use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum WorkforceError {
    #[error("database error")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("already clocked in")]
    AlreadyClockedIn,
    #[error("not clocked in")]
    NotClockedIn,
    #[error("unauthorized")]
    Unauthorized,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttendanceStatus {
    Open,
    Closed,
    Adjusted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub id: String,
    pub worker_id: String,
    pub device_id: String,
    pub clock_in_at: i64,
    pub clock_out_at: Option<i64>,
    pub break_minutes: i64,
    pub status: AttendanceStatus,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shift {
    pub id: String,
    pub worker_id: String,
    pub role: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub location: String,
    pub created_by: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PayRecord {
    pub id: String,
    pub worker_id: String,
    pub period_start: i64,
    pub period_end: i64,
    pub hours_worked: f64,
    pub rate_minor: i64,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub created_by: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct WorkforceStore {
    conn: Mutex<Connection>,
}

impl WorkforceStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WorkforceError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let store = Self { conn: Mutex::new(conn) };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, WorkforceError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let s = Self { conn: Mutex::new(conn) };
        s.migrate()?;
        Ok(s)
    }

    fn migrate(&self) -> Result<(), WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        c.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS attendance (
              id TEXT PRIMARY KEY,
              worker_id TEXT NOT NULL,
              device_id TEXT NOT NULL,
              clock_in_at INTEGER NOT NULL,
              clock_out_at INTEGER,
              break_minutes INTEGER NOT NULL DEFAULT 0,
              status TEXT NOT NULL CHECK(status IN ('OPEN','CLOSED','ADJUSTED')),
              notes TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_att_worker ON attendance(worker_id, clock_in_at DESC);
            CREATE INDEX IF NOT EXISTS idx_att_open ON attendance(worker_id, status) WHERE status = 'OPEN';

            CREATE TABLE IF NOT EXISTS shifts (
              id TEXT PRIMARY KEY,
              worker_id TEXT NOT NULL,
              role TEXT NOT NULL,
              starts_at INTEGER NOT NULL,
              ends_at INTEGER NOT NULL,
              location TEXT NOT NULL,
              created_by TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              CHECK(ends_at > starts_at)
            );
            CREATE INDEX IF NOT EXISTS idx_shift_worker ON shifts(worker_id, starts_at);

            CREATE TABLE IF NOT EXISTS pay_records (
              id TEXT PRIMARY KEY,
              worker_id TEXT NOT NULL,
              period_start INTEGER NOT NULL,
              period_end INTEGER NOT NULL,
              hours_worked REAL NOT NULL,
              rate_minor INTEGER NOT NULL,
              amount_minor INTEGER NOT NULL,
              currency TEXT NOT NULL DEFAULT 'UGX',
              status TEXT NOT NULL CHECK(status IN ('DRAFT','APPROVED','PAID')),
              created_by TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              CHECK(period_end > period_start),
              UNIQUE(worker_id, period_start, period_end)
            );
            CREATE INDEX IF NOT EXISTS idx_pay_worker ON pay_records(worker_id, period_start);
            "#,
        )?;
        Ok(())
    }

    pub fn clock_in(
        &self,
        worker_id: &str,
        device_id: &str,
        notes: Option<&str>,
    ) -> Result<AttendanceRecord, WorkforceError> {
        validate_id(worker_id, "worker_id")?;
        validate_id(device_id, "device_id")?;
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let open: Option<String> = c
            .query_row(
                "SELECT id FROM attendance WHERE worker_id=?1 AND status='OPEN' LIMIT 1",
                params![worker_id],
                |r| r.get(0),
            )
            .optional()?;
        if open.is_some() {
            return Err(WorkforceError::AlreadyClockedIn);
        }
        let now = now();
        let id = Uuid::new_v4().to_string();
        c.execute(
            "INSERT INTO attendance(id, worker_id, device_id, clock_in_at, break_minutes, status, notes, created_at, updated_at)
             VALUES (?,?,?,?,0,'OPEN',?,?,?)",
            params![id, worker_id, device_id, now, notes, now, now],
        )?;
        drop(c);
        self.get_attendance(&id)
    }

    pub fn clock_out(
        &self,
        worker_id: &str,
        break_minutes: i64,
        notes: Option<&str>,
    ) -> Result<AttendanceRecord, WorkforceError> {
        validate_id(worker_id, "worker_id")?;
        if break_minutes < 0 || break_minutes > 24 * 60 {
            return Err(WorkforceError::Validation("invalid break_minutes".into()));
        }
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let open_id: Option<String> = c
            .query_row(
                "SELECT id FROM attendance WHERE worker_id=?1 AND status='OPEN' ORDER BY clock_in_at DESC LIMIT 1",
                params![worker_id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(id) = open_id else {
            return Err(WorkforceError::NotClockedIn);
        };
        let now = now();
        c.execute(
            "UPDATE attendance SET clock_out_at=?1, break_minutes=?2, status='CLOSED', notes=COALESCE(?3, notes), updated_at=?1 WHERE id=?4",
            params![now, break_minutes, notes, id],
        )?;
        drop(c);
        self.get_attendance(&id)
    }

    pub fn get_attendance(&self, id: &str) -> Result<AttendanceRecord, WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        c.query_row(
            "SELECT id, worker_id, device_id, clock_in_at, clock_out_at, break_minutes, status, notes, created_at, updated_at
             FROM attendance WHERE id=?1",
            params![id],
            row_att,
        )
        .optional()?
        .ok_or(WorkforceError::NotFound)
    }

    pub fn list_attendance(
        &self,
        worker_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<AttendanceRecord>, WorkforceError> {
        let n = limit.clamp(1, 500) as i64;
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let mut out = Vec::new();
        if let Some(wid) = worker_id {
            let mut st = c.prepare(
                "SELECT id, worker_id, device_id, clock_in_at, clock_out_at, break_minutes, status, notes, created_at, updated_at
                 FROM attendance WHERE worker_id=?1 ORDER BY clock_in_at DESC LIMIT ?2",
            )?;
            let rows = st.query_map(params![wid, n], row_att)?;
            for r in rows {
                out.push(r?);
            }
        } else {
            let mut st = c.prepare(
                "SELECT id, worker_id, device_id, clock_in_at, clock_out_at, break_minutes, status, notes, created_at, updated_at
                 FROM attendance ORDER BY clock_in_at DESC LIMIT ?1",
            )?;
            let rows = st.query_map(params![n], row_att)?;
            for r in rows {
                out.push(r?);
            }
        }
        Ok(out)
    }

    pub fn open_attendance(&self, worker_id: &str) -> Result<Option<AttendanceRecord>, WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let id: Option<String> = c
            .query_row(
                "SELECT id FROM attendance WHERE worker_id=?1 AND status='OPEN' LIMIT 1",
                params![worker_id],
                |r| r.get(0),
            )
            .optional()?;
        drop(c);
        match id {
            Some(i) => Ok(Some(self.get_attendance(&i)?)),
            None => Ok(None),
        }
    }

    pub fn create_shift(
        &self,
        worker_id: &str,
        role: &str,
        starts_at: i64,
        ends_at: i64,
        location: &str,
        created_by: &str,
    ) -> Result<Shift, WorkforceError> {
        validate_id(worker_id, "worker_id")?;
        validate_id(role, "role")?;
        validate_id(location, "location")?;
        validate_id(created_by, "created_by")?;
        if ends_at <= starts_at {
            return Err(WorkforceError::Validation("ends_at must be after starts_at".into()));
        }
        let now = now();
        let id = Uuid::new_v4().to_string();
        let c = self.conn.lock().expect("workforce mutex poisoned");
        c.execute(
            "INSERT INTO shifts(id, worker_id, role, starts_at, ends_at, location, created_by, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?,?,?)",
            params![id, worker_id, role, starts_at, ends_at, location, created_by, now, now],
        )?;
        drop(c);
        self.get_shift(&id)
    }

    pub fn get_shift(&self, id: &str) -> Result<Shift, WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        c.query_row(
            "SELECT id, worker_id, role, starts_at, ends_at, location, created_by, created_at, updated_at FROM shifts WHERE id=?1",
            params![id],
            row_shift,
        )
        .optional()?
        .ok_or(WorkforceError::NotFound)
    }

    pub fn list_shifts(
        &self,
        worker_id: Option<&str>,
        from: i64,
        to: i64,
        limit: usize,
    ) -> Result<Vec<Shift>, WorkforceError> {
        let n = limit.clamp(1, 500) as i64;
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let mut out = Vec::new();
        if let Some(wid) = worker_id {
            let mut st = c.prepare(
                "SELECT id, worker_id, role, starts_at, ends_at, location, created_by, created_at, updated_at
                 FROM shifts WHERE worker_id=?1 AND starts_at >= ?2 AND starts_at < ?3 ORDER BY starts_at LIMIT ?4",
            )?;
            let rows = st.query_map(params![wid, from, to, n], row_shift)?;
            for r in rows {
                out.push(r?);
            }
        } else {
            let mut st = c.prepare(
                "SELECT id, worker_id, role, starts_at, ends_at, location, created_by, created_at, updated_at
                 FROM shifts WHERE starts_at >= ?1 AND starts_at < ?2 ORDER BY starts_at LIMIT ?3",
            )?;
            let rows = st.query_map(params![from, to, n], row_shift)?;
            for r in rows {
                out.push(r?);
            }
        }
        Ok(out)
    }

    pub fn create_pay_record(
        &self,
        worker_id: &str,
        period_start: i64,
        period_end: i64,
        hours_worked: f64,
        rate_minor: i64,
        created_by: &str,
    ) -> Result<PayRecord, WorkforceError> {
        validate_id(worker_id, "worker_id")?;
        validate_id(created_by, "created_by")?;
        if period_end <= period_start {
            return Err(WorkforceError::Validation("invalid period".into()));
        }
        if hours_worked < 0.0 || rate_minor < 0 {
            return Err(WorkforceError::Validation("invalid hours or rate".into()));
        }
        let amount = ((hours_worked * rate_minor as f64).round()) as i64;
        let now = now();
        let id = Uuid::new_v4().to_string();
        let c = self.conn.lock().expect("workforce mutex poisoned");
        c.execute(
            "INSERT INTO pay_records(id, worker_id, period_start, period_end, hours_worked, rate_minor, amount_minor, currency, status, created_by, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?,'UGX','DRAFT',?,?,?)",
            params![id, worker_id, period_start, period_end, hours_worked, rate_minor, amount, created_by, now, now],
        )?;
        drop(c);
        self.get_pay(&id)
    }

    pub fn approve_pay(&self, id: &str) -> Result<PayRecord, WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let n = c.execute(
            "UPDATE pay_records SET status='APPROVED', updated_at=?1 WHERE id=?2 AND status='DRAFT'",
            params![now(), id],
        )?;
        if n == 0 {
            return Err(WorkforceError::NotFound);
        }
        drop(c);
        self.get_pay(id)
    }

    pub fn get_pay(&self, id: &str) -> Result<PayRecord, WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        c.query_row(
            "SELECT id, worker_id, period_start, period_end, hours_worked, rate_minor, amount_minor, currency, status, created_by, created_at, updated_at
             FROM pay_records WHERE id=?1",
            params![id],
            row_pay,
        )
        .optional()?
        .ok_or(WorkforceError::NotFound)
    }

    pub fn list_pay(
        &self,
        worker_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PayRecord>, WorkforceError> {
        let n = limit.clamp(1, 500) as i64;
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let mut out = Vec::new();
        if let Some(wid) = worker_id {
            let mut st = c.prepare(
                "SELECT id, worker_id, period_start, period_end, hours_worked, rate_minor, amount_minor, currency, status, created_by, created_at, updated_at
                 FROM pay_records WHERE worker_id=?1 ORDER BY period_start DESC LIMIT ?2",
            )?;
            let rows = st.query_map(params![wid, n], row_pay)?;
            for r in rows {
                out.push(r?);
            }
        } else {
            let mut st = c.prepare(
                "SELECT id, worker_id, period_start, period_end, hours_worked, rate_minor, amount_minor, currency, status, created_by, created_at, updated_at
                 FROM pay_records ORDER BY period_start DESC LIMIT ?1",
            )?;
            let rows = st.query_map(params![n], row_pay)?;
            for r in rows {
                out.push(r?);
            }
        }
        Ok(out)
    }

    /// Sum closed attendance hours for a worker in [from, to).
    pub fn hours_in_period(&self, worker_id: &str, from: i64, to: i64) -> Result<f64, WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        let mut st = c.prepare(
            "SELECT clock_in_at, clock_out_at, break_minutes FROM attendance
             WHERE worker_id=?1 AND status IN ('CLOSED','ADJUSTED') AND clock_in_at >= ?2 AND clock_in_at < ?3 AND clock_out_at IS NOT NULL",
        )?;
        let rows = st.query_map(params![worker_id, from, to], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        })?;
        let mut total_secs: i64 = 0;
        for row in rows {
            let (cin, cout, brk) = row?;
            let worked = (cout - cin).max(0) - (brk * 60).max(0);
            total_secs += worked.max(0);
        }
        Ok(total_secs as f64 / 3600.0)
    }
}

fn row_att(r: &rusqlite::Row<'_>) -> Result<AttendanceRecord, rusqlite::Error> {
    let status: String = r.get(6)?;
    let status = match status.as_str() {
        "OPEN" => AttendanceStatus::Open,
        "CLOSED" => AttendanceStatus::Closed,
        "ADJUSTED" => AttendanceStatus::Adjusted,
        _ => AttendanceStatus::Closed,
    };
    Ok(AttendanceRecord {
        id: r.get(0)?,
        worker_id: r.get(1)?,
        device_id: r.get(2)?,
        clock_in_at: r.get(3)?,
        clock_out_at: r.get(4)?,
        break_minutes: r.get(5)?,
        status,
        notes: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

fn row_shift(r: &rusqlite::Row<'_>) -> Result<Shift, rusqlite::Error> {
    Ok(Shift {
        id: r.get(0)?,
        worker_id: r.get(1)?,
        role: r.get(2)?,
        starts_at: r.get(3)?,
        ends_at: r.get(4)?,
        location: r.get(5)?,
        created_by: r.get(6)?,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
    })
}

fn row_pay(r: &rusqlite::Row<'_>) -> Result<PayRecord, rusqlite::Error> {
    Ok(PayRecord {
        id: r.get(0)?,
        worker_id: r.get(1)?,
        period_start: r.get(2)?,
        period_end: r.get(3)?,
        hours_worked: r.get(4)?,
        rate_minor: r.get(5)?,
        amount_minor: r.get(6)?,
        currency: r.get(7)?,
        status: r.get(8)?,
        created_by: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}

fn validate_id(v: &str, name: &str) -> Result<(), WorkforceError> {
    if v.trim().is_empty() || v.len() > 128 {
        return Err(WorkforceError::Validation(format!("invalid {name}")));
    }
    Ok(())
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_in_out_and_hours() {
        let s = WorkforceStore::open_in_memory().unwrap();
        let a = s.clock_in("w1", "dev1", None).unwrap();
        assert_eq!(a.status, AttendanceStatus::Open);
        assert!(s.clock_in("w1", "dev1", None).is_err());
        let closed = s.clock_out("w1", 15, Some("lunch")).unwrap();
        assert_eq!(closed.status, AttendanceStatus::Closed);
        assert_eq!(closed.break_minutes, 15);
        assert!(closed.clock_out_at.is_some());
    }

    #[test]
    fn shift_and_pay() {
        let s = WorkforceStore::open_in_memory().unwrap();
        let sh = s
            .create_shift("w1", "WAITER", 1_700_000_000, 1_700_003_600, "Najjera", "ceo1")
            .unwrap();
        assert_eq!(sh.role, "WAITER");
        let pay = s
            .create_pay_record("w1", 1_700_000_000, 1_700_086_400, 40.0, 5000, "ceo1")
            .unwrap();
        assert_eq!(pay.amount_minor, 200_000);
        assert_eq!(pay.status, "DRAFT");
        let approved = s.approve_pay(&pay.id).unwrap();
        assert_eq!(approved.status, "APPROVED");
    }

    #[test]
    fn cannot_clock_out_without_in() {
        let s = WorkforceStore::open_in_memory().unwrap();
        assert!(matches!(s.clock_out("w1", 0, None), Err(WorkforceError::NotClockedIn)));
    }
}
