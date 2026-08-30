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
    #[error("validation: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("already clocked in")]
    AlreadyClockedIn,
    #[error("not clocked in")]
    NotClockedIn,
    #[error("conflict")]
    Conflict,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttendanceStatus { Open, Closed, Adjusted }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub id: String, pub worker_id: String, pub device_id: String,
    pub clock_in_at: i64, pub clock_out_at: Option<i64>, pub break_minutes: i64,
    pub status: AttendanceStatus, pub notes: Option<String>, pub created_at: i64, pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvailabilityRule {
    pub id: String, pub worker_id: String, pub day_of_week: i64,
    pub available_from_minute: i64, pub available_to_minute: i64, pub preferred_shift: Option<String>,
    pub active: bool, pub valid_from: i64, pub valid_to: Option<i64>, pub created_at: i64, pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shift {
    pub id: String, pub worker_id: String, pub role: String, pub shift_type: String,
    pub starts_at: i64, pub ends_at: i64, pub location: String, pub status: String,
    pub created_by: String, pub created_at: i64, pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShiftSwap {
    pub id: String, pub shift_id: String, pub from_worker_id: String, pub to_worker_id: String,
    pub reason: Option<String>, pub status: String, pub requested_by: String,
    pub approved_by: Option<String>, pub created_at: i64, pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PayRate {
    pub id: String, pub worker_id: String, pub role: String, pub shift_type: String,
    pub rate_minor: i64, pub currency: String, pub effective_from: i64, pub effective_to: Option<i64>,
    pub created_by: String, pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PayRecord {
    pub id: String, pub worker_id: String, pub period_start: i64, pub period_end: i64,
    pub hours_worked: f64, pub rate_minor: i64, pub amount_minor: i64, pub currency: String,
    pub status: String, pub created_by: String, pub created_at: i64, pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PayrollAdjustment {
    pub id: String, pub worker_id: String, pub pay_record_id: Option<String>,
    pub kind: String, pub amount_minor: i64, pub reason: String, pub created_by: String, pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerAdvance {
    pub id: String, pub worker_id: String, pub amount_minor: i64, pub currency: String,
    pub reason: String, pub status: String, pub created_by: String, pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerPayment {
    pub id: String, pub worker_id: String, pub amount_minor: i64, pub currency: String,
    pub payment_method: String, pub reference: Option<String>, pub paid_by: String, pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PayrollSummary {
    pub gross_minor: i64, pub adjustments_minor: i64, pub advances_minor: i64,
    pub paid_minor: i64, pub balance_minor: i64,
}

pub struct WorkforceStore { conn: Mutex<Connection> }

impl WorkforceStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WorkforceError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let store = Self { conn: Mutex::new(conn) }; store.migrate()?; Ok(store)
    }
    pub fn open_in_memory() -> Result<Self, WorkforceError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let s = Self { conn: Mutex::new(conn) }; s.migrate()?; Ok(s)
    }
    fn migrate(&self) -> Result<(), WorkforceError> {
        let c = self.conn.lock().expect("workforce mutex poisoned");
        c.execute_batch(r#"
          CREATE TABLE IF NOT EXISTS attendance(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, device_id TEXT NOT NULL,
            clock_in_at INTEGER NOT NULL, clock_out_at INTEGER, break_minutes INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL CHECK(status IN('OPEN','CLOSED','ADJUSTED')), notes TEXT,
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
          CREATE UNIQUE INDEX IF NOT EXISTS uq_att_open ON attendance(worker_id) WHERE status='OPEN';
          CREATE INDEX IF NOT EXISTS idx_att_worker ON attendance(worker_id,clock_in_at DESC);
          CREATE TABLE IF NOT EXISTS availability(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, day_of_week INTEGER NOT NULL CHECK(day_of_week BETWEEN 0 AND 6),
            available_from_minute INTEGER NOT NULL CHECK(available_from_minute BETWEEN 0 AND 1439),
            available_to_minute INTEGER NOT NULL CHECK(available_to_minute BETWEEN 1 AND 1440),
            preferred_shift TEXT, active INTEGER NOT NULL DEFAULT 1, valid_from INTEGER NOT NULL,
            valid_to INTEGER, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
            CHECK(available_to_minute>available_from_minute));
          CREATE INDEX IF NOT EXISTS idx_availability_worker ON availability(worker_id,day_of_week,active);
          CREATE TABLE IF NOT EXISTS shifts(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, role TEXT NOT NULL,
            shift_type TEXT NOT NULL CHECK(shift_type IN('DAY','NIGHT','CUSTOM')),
            starts_at INTEGER NOT NULL, ends_at INTEGER NOT NULL, location TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN('SCHEDULED','PRESENT','ABSENT','EXCUSED','CANCELLED','REPLACED','COMPLETED')),
            created_by TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
            CHECK(ends_at>starts_at));
          CREATE INDEX IF NOT EXISTS idx_shift_worker ON shifts(worker_id,starts_at);
          CREATE TABLE IF NOT EXISTS shift_swaps(
            id TEXT PRIMARY KEY, shift_id TEXT NOT NULL, from_worker_id TEXT NOT NULL, to_worker_id TEXT NOT NULL,
            reason TEXT, status TEXT NOT NULL CHECK(status IN('REQUESTED','APPROVED','REJECTED','CANCELLED')),
            requested_by TEXT NOT NULL, approved_by TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
            FOREIGN KEY(shift_id) REFERENCES shifts(id), CHECK(from_worker_id<>to_worker_id));
          CREATE TABLE IF NOT EXISTS pay_rates(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, role TEXT NOT NULL, shift_type TEXT NOT NULL,
            rate_minor INTEGER NOT NULL CHECK(rate_minor>=0), currency TEXT NOT NULL DEFAULT 'UGX',
            effective_from INTEGER NOT NULL, effective_to INTEGER, created_by TEXT NOT NULL, created_at INTEGER NOT NULL,
            CHECK(effective_to IS NULL OR effective_to>effective_from));
          CREATE INDEX IF NOT EXISTS idx_rate_worker ON pay_rates(worker_id,shift_type,effective_from DESC);
          CREATE TABLE IF NOT EXISTS pay_records(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, period_start INTEGER NOT NULL, period_end INTEGER NOT NULL,
            hours_worked REAL NOT NULL CHECK(hours_worked>=0), rate_minor INTEGER NOT NULL CHECK(rate_minor>=0),
            amount_minor INTEGER NOT NULL CHECK(amount_minor>=0), currency TEXT NOT NULL DEFAULT 'UGX',
            status TEXT NOT NULL CHECK(status IN('DRAFT','APPROVED','PAID','VOIDED')),
            created_by TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
            CHECK(period_end>period_start), UNIQUE(worker_id,period_start,period_end));
          CREATE INDEX IF NOT EXISTS idx_pay_worker ON pay_records(worker_id,period_start);
          CREATE TABLE IF NOT EXISTS payroll_adjustments(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, pay_record_id TEXT, kind TEXT NOT NULL,
            amount_minor INTEGER NOT NULL CHECK(amount_minor<>0), reason TEXT NOT NULL, created_by TEXT NOT NULL, created_at INTEGER NOT NULL,
            FOREIGN KEY(pay_record_id) REFERENCES pay_records(id));
          CREATE TABLE IF NOT EXISTS worker_advances(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, amount_minor INTEGER NOT NULL CHECK(amount_minor>0),
            currency TEXT NOT NULL DEFAULT 'UGX', reason TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN('OUTSTANDING','SETTLED','VOIDED')),
            created_by TEXT NOT NULL, created_at INTEGER NOT NULL);
          CREATE TABLE IF NOT EXISTS worker_payments(
            id TEXT PRIMARY KEY, worker_id TEXT NOT NULL, amount_minor INTEGER NOT NULL CHECK(amount_minor>0),
            currency TEXT NOT NULL DEFAULT 'UGX', payment_method TEXT NOT NULL CHECK(payment_method IN('CASH','MTN_MOMO','AIRTEL_MONEY','OTHER')),
            reference TEXT, paid_by TEXT NOT NULL, created_at INTEGER NOT NULL);
        "#)?;
        Ok(())
    }

    pub fn clock_in(&self, worker_id:&str, device_id:&str, notes:Option<&str>) -> Result<AttendanceRecord,WorkforceError> {
        validate_id(worker_id,"worker_id")?; validate_id(device_id,"device_id")?;
        let c=self.conn.lock().expect("workforce mutex poisoned");
        let exists:Option<String>=c.query_row("SELECT id FROM attendance WHERE worker_id=?1 AND status='OPEN'",params![worker_id],|r|r.get(0)).optional()?;
        if exists.is_some(){return Err(WorkforceError::AlreadyClockedIn)}
        let now=now(); let id=Uuid::new_v4().to_string();
        c.execute("INSERT INTO attendance(id,worker_id,device_id,clock_in_at,break_minutes,status,notes,created_at,updated_at) VALUES(?,?,?,?,0,'OPEN',?,?,?)",params![id,worker_id,device_id,now,notes,now,now])?;
        drop(c); self.get_attendance(&id)
    }
    pub fn clock_out(&self,worker_id:&str,break_minutes:i64,notes:Option<&str>)->Result<AttendanceRecord,WorkforceError>{
        validate_id(worker_id,"worker_id")?; if !(0..=1440).contains(&break_minutes){return Err(WorkforceError::Validation("invalid break_minutes".into()))}
        let c=self.conn.lock().expect("workforce mutex poisoned");
        let id:Option<String>=c.query_row("SELECT id FROM attendance WHERE worker_id=?1 AND status='OPEN' ORDER BY clock_in_at DESC LIMIT 1",params![worker_id],|r|r.get(0)).optional()?;
        let Some(id)=id else{return Err(WorkforceError::NotClockedIn)}; let t=now();
        c.execute("UPDATE attendance SET clock_out_at=?1,break_minutes=?2,status='CLOSED',notes=COALESCE(?3,notes),updated_at=?1 WHERE id=?4",params![t,break_minutes,notes,id])?;
        drop(c); self.get_attendance(&id)
    }
    pub fn get_attendance(&self,id:&str)->Result<AttendanceRecord,WorkforceError>{
        let c=self.conn.lock().expect("workforce mutex poisoned"); c.query_row("SELECT id,worker_id,device_id,clock_in_at,clock_out_at,break_minutes,status,notes,created_at,updated_at FROM attendance WHERE id=?1",params![id],row_att).optional()?.ok_or(WorkforceError::NotFound)
    }
    pub fn list_attendance(&self,worker_id:Option<&str>,limit:usize)->Result<Vec<AttendanceRecord>,WorkforceError>{
        let c=self.conn.lock().expect("workforce mutex poisoned"); let n=limit.clamp(1,500) as i64; let mut out=Vec::new();
        if let Some(w)=worker_id{let mut s=c.prepare("SELECT id,worker_id,device_id,clock_in_at,clock_out_at,break_minutes,status,notes,created_at,updated_at FROM attendance WHERE worker_id=?1 ORDER BY clock_in_at DESC LIMIT ?2")?; for r in s.query_map(params![w,n],row_att)?{out.push(r?)} }
        else {let mut s=c.prepare("SELECT id,worker_id,device_id,clock_in_at,clock_out_at,break_minutes,status,notes,created_at,updated_at FROM attendance ORDER BY clock_in_at DESC LIMIT ?1")?; for r in s.query_map(params![n],row_att)?{out.push(r?)} }
        Ok(out)
    }
    pub fn open_attendance(&self,worker_id:&str)->Result<Option<AttendanceRecord>,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let id:Option<String>=c.query_row("SELECT id FROM attendance WHERE worker_id=?1 AND status='OPEN'",params![worker_id],|r|r.get(0)).optional()?;drop(c);id.map(|i|self.get_attendance(&i)).transpose()}

    pub fn set_availability(&self,worker_id:&str,day_of_week:i64,from_minute:i64,to_minute:i64,preferred_shift:Option<&str>,valid_from:i64,valid_to:Option<i64>,created_by:&str)->Result<AvailabilityRule,WorkforceError>{
        validate_id(worker_id,"worker_id")?;validate_id(created_by,"created_by")?;if !(0..=6).contains(&day_of_week)||from_minute<0||to_minute>1440||to_minute<=from_minute{return Err(WorkforceError::Validation("invalid availability window".into()))};if let Some(v)=valid_to{if v<=valid_from{return Err(WorkforceError::Validation("invalid availability validity".into()))}}
        let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("INSERT INTO availability(id,worker_id,day_of_week,available_from_minute,available_to_minute,preferred_shift,active,valid_from,valid_to,created_at,updated_at) VALUES(?,?,?,?,?,?,1,?,?,?,?,?)",params![id,worker_id,day_of_week,from_minute,to_minute,preferred_shift,valid_from,valid_to,t,t])?;drop(c);self.get_availability(&id)
    }
    pub fn get_availability(&self,id:&str)->Result<AvailabilityRule,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,day_of_week,available_from_minute,available_to_minute,preferred_shift,active,valid_from,valid_to,created_at,updated_at FROM availability WHERE id=?1",params![id],row_availability).optional()?.ok_or(WorkforceError::NotFound)}
    pub fn list_availability(&self,worker_id:&str)->Result<Vec<AvailabilityRule>,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let mut s=c.prepare("SELECT id,worker_id,day_of_week,available_from_minute,available_to_minute,preferred_shift,active,valid_from,valid_to,created_at,updated_at FROM availability WHERE worker_id=?1 ORDER BY day_of_week,available_from_minute")?;let mut out=Vec::new();for r in s.query_map(params![worker_id],row_availability)?{out.push(r?)}Ok(out)}

    pub fn create_shift(&self,worker_id:&str,role:&str,starts_at:i64,ends_at:i64,location:&str,created_by:&str)->Result<Shift,WorkforceError>{self.create_shift_typed(worker_id,role,"CUSTOM",starts_at,ends_at,location,created_by)}
    pub fn create_shift_typed(&self,worker_id:&str,role:&str,shift_type:&str,starts_at:i64,ends_at:i64,location:&str,created_by:&str)->Result<Shift,WorkforceError>{
        validate_id(worker_id,"worker_id")?;validate_id(role,"role")?;validate_id(location,"location")?;validate_id(created_by,"created_by")?;if !matches!(shift_type,"DAY"|"NIGHT"|"CUSTOM"){return Err(WorkforceError::Validation("invalid shift type".into()))};if ends_at<=starts_at{return Err(WorkforceError::Validation("ends_at must be after starts_at".into()))}
        let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("INSERT INTO shifts(id,worker_id,role,shift_type,starts_at,ends_at,location,status,created_by,created_at,updated_at) VALUES(?,?,?,?,?,?,?,'SCHEDULED',?,?,?)",params![id,worker_id,role,shift_type,starts_at,ends_at,location,created_by,t,t])?;drop(c);self.get_shift(&id)
    }
    pub fn get_shift(&self,id:&str)->Result<Shift,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,role,shift_type,starts_at,ends_at,location,status,created_by,created_at,updated_at FROM shifts WHERE id=?1",params![id],row_shift).optional()?.ok_or(WorkforceError::NotFound)}
    pub fn list_shifts(&self,worker_id:Option<&str>,from:i64,to:i64,limit:usize)->Result<Vec<Shift>,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let n=limit.clamp(1,500) as i64;let mut out=Vec::new();if let Some(w)=worker_id{let mut s=c.prepare("SELECT id,worker_id,role,shift_type,starts_at,ends_at,location,status,created_by,created_at,updated_at FROM shifts WHERE worker_id=?1 AND starts_at>=?2 AND starts_at<?3 ORDER BY starts_at LIMIT ?4")?;for r in s.query_map(params![w,from,to,n],row_shift)?{out.push(r?)}}else{let mut s=c.prepare("SELECT id,worker_id,role,shift_type,starts_at,ends_at,location,status,created_by,created_at,updated_at FROM shifts WHERE starts_at>=?1 AND starts_at<?2 ORDER BY starts_at LIMIT ?3")?;for r in s.query_map(params![from,to,n],row_shift)?{out.push(r?)}}Ok(out)}
    pub fn mark_shift_status(&self,id:&str,status:&str)->Result<Shift,WorkforceError>{if !matches!(status,"SCHEDULED"|"PRESENT"|"ABSENT"|"EXCUSED"|"CANCELLED"|"REPLACED"|"COMPLETED"){return Err(WorkforceError::Validation("invalid shift status".into()))}let c=self.conn.lock().expect("workforce mutex poisoned");let n=c.execute("UPDATE shifts SET status=?1,updated_at=?2 WHERE id=?3",params![status,now(),id])?;if n==0{return Err(WorkforceError::NotFound)}drop(c);self.get_shift(id)}

    pub fn request_shift_swap(&self,shift_id:&str,to_worker_id:&str,requested_by:&str,reason:Option<&str>)->Result<ShiftSwap,WorkforceError>{
        validate_id(shift_id,"shift_id")?;validate_id(to_worker_id,"to_worker_id")?;validate_id(requested_by,"requested_by")?;let shift=self.get_shift(shift_id)?;if shift.worker_id==to_worker_id{return Err(WorkforceError::Validation("replacement must differ from current worker".into()))};let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("INSERT INTO shift_swaps(id,shift_id,from_worker_id,to_worker_id,reason,status,requested_by,created_at,updated_at) VALUES(?,?,?,?,?,'REQUESTED',?,?,?)",params![id,shift_id,shift.worker_id,to_worker_id,reason,requested_by,t,t])?;drop(c);self.get_swap(&id)}
    pub fn approve_shift_swap(&self,id:&str,approved_by:&str)->Result<ShiftSwap,WorkforceError>{validate_id(approved_by,"approved_by")?;let c=self.conn.lock().expect("workforce mutex poisoned");let swap=self.get_swap(id)?;let tx=c.unchecked_transaction()?;tx.execute("UPDATE shift_swaps SET status='APPROVED',approved_by=?1,updated_at=?2 WHERE id=?3 AND status='REQUESTED'",params![approved_by,now(),id])?;tx.execute("UPDATE shifts SET worker_id=?1,status='SCHEDULED',updated_at=?2 WHERE id=?3",params![swap.to_worker_id,now(),swap.shift_id])?;tx.commit()?;drop(c);self.get_swap(id)}
    pub fn get_swap(&self,id:&str)->Result<ShiftSwap,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,shift_id,from_worker_id,to_worker_id,reason,status,requested_by,approved_by,created_at,updated_at FROM shift_swaps WHERE id=?1",params![id],row_swap).optional()?.ok_or(WorkforceError::NotFound)}

    pub fn set_pay_rate(&self,worker_id:&str,role:&str,shift_type:&str,rate_minor:i64,currency:&str,effective_from:i64,created_by:&str)->Result<PayRate,WorkforceError>{validate_id(worker_id,"worker_id")?;validate_id(role,"role")?;validate_id(currency,"currency")?;validate_id(created_by,"created_by")?;if rate_minor<0{return Err(WorkforceError::Validation("negative rate".into()))};if !matches!(shift_type,"DAY"|"NIGHT"|"CUSTOM"){return Err(WorkforceError::Validation("invalid shift type".into()))};let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("UPDATE pay_rates SET effective_to=?1 WHERE worker_id=?2 AND shift_type=?3 AND effective_to IS NULL AND effective_from<?4",params![effective_from,worker_id,shift_type,effective_from])?;c.execute("INSERT INTO pay_rates(id,worker_id,role,shift_type,rate_minor,currency,effective_from,created_by,created_at) VALUES(?,?,?,?,?,?,?,?,?)",params![id,worker_id,role,shift_type,rate_minor,currency,effective_from,created_by,t])?;drop(c);self.get_rate(&id)}
    pub fn get_rate(&self,id:&str)->Result<PayRate,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,role,shift_type,rate_minor,currency,effective_from,effective_to,created_by,created_at FROM pay_rates WHERE id=?1",params![id],row_rate).optional()?.ok_or(WorkforceError::NotFound)}
    pub fn rate_at(&self,worker_id:&str,shift_type:&str,at:i64)->Result<Option<PayRate>,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,role,shift_type,rate_minor,currency,effective_from,effective_to,created_by,created_at FROM pay_rates WHERE worker_id=?1 AND shift_type=?2 AND effective_from<=?3 AND (effective_to IS NULL OR effective_to>?3) ORDER BY effective_from DESC LIMIT 1",params![worker_id,shift_type,at],row_rate).optional().map_err(Into::into)}

    pub fn create_pay_record(&self,worker_id:&str,period_start:i64,period_end:i64,hours_worked:f64,rate_minor:i64,created_by:&str)->Result<PayRecord,WorkforceError>{validate_id(worker_id,"worker_id")?;validate_id(created_by,"created_by")?;if period_end<=period_start||!hours_worked.is_finite()||hours_worked<0.0||rate_minor<0{return Err(WorkforceError::Validation("invalid pay period, hours or rate".into()))};let amount=((hours_worked*rate_minor as f64).round()) as i64;let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("INSERT INTO pay_records(id,worker_id,period_start,period_end,hours_worked,rate_minor,amount_minor,currency,status,created_by,created_at,updated_at) VALUES(?,?,?,?,?,?,?,'UGX','DRAFT',?,?,?)",params![id,worker_id,period_start,period_end,hours_worked,rate_minor,amount,created_by,t,t])?;drop(c);self.get_pay(&id)}
    pub fn create_shift_pay(&self,worker_id:&str,shift_id:&str,created_by:&str)->Result<PayRecord,WorkforceError>{let sh=self.get_shift(shift_id)?;if sh.worker_id!=worker_id{return Err(WorkforceError::Conflict)};let attendance=self.attendance_overlapping(worker_id,sh.starts_at,sh.ends_at)?;let hours=attendance.iter().map(|a|worked_hours(a)).sum::<f64>();let rate=self.rate_at(worker_id,&sh.shift_type,sh.starts_at)?.ok_or_else(||WorkforceError::Validation("no effective pay rate".into()))?;self.create_pay_record(worker_id,sh.starts_at,sh.ends_at,hours,rate.rate_minor,created_by)}
    pub fn approve_pay(&self,id:&str)->Result<PayRecord,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let n=c.execute("UPDATE pay_records SET status='APPROVED',updated_at=?1 WHERE id=?2 AND status='DRAFT'",params![now(),id])?;if n==0{return Err(WorkforceError::NotFound)}drop(c);self.get_pay(id)}
    pub fn mark_pay_paid(&self,id:&str)->Result<PayRecord,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let n=c.execute("UPDATE pay_records SET status='PAID',updated_at=?1 WHERE id=?2 AND status='APPROVED'",params![now(),id])?;if n==0{return Err(WorkforceError::NotFound)}drop(c);self.get_pay(id)}
    pub fn get_pay(&self,id:&str)->Result<PayRecord,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,period_start,period_end,hours_worked,rate_minor,amount_minor,currency,status,created_by,created_at,updated_at FROM pay_records WHERE id=?1",params![id],row_pay).optional()?.ok_or(WorkforceError::NotFound)}
    pub fn list_pay(&self,worker_id:Option<&str>,limit:usize)->Result<Vec<PayRecord>,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let n=limit.clamp(1,500) as i64;let mut out=Vec::new();if let Some(w)=worker_id{let mut s=c.prepare("SELECT id,worker_id,period_start,period_end,hours_worked,rate_minor,amount_minor,currency,status,created_by,created_at,updated_at FROM pay_records WHERE worker_id=?1 ORDER BY period_start DESC LIMIT ?2")?;for r in s.query_map(params![w,n],row_pay)?{out.push(r?)}}else{let mut s=c.prepare("SELECT id,worker_id,period_start,period_end,hours_worked,rate_minor,amount_minor,currency,status,created_by,created_at,updated_at FROM pay_records ORDER BY period_start DESC LIMIT ?1")?;for r in s.query_map(params![n],row_pay)?{out.push(r?)}}Ok(out)}

    pub fn add_adjustment(&self,worker_id:&str,pay_record_id:Option<&str>,kind:&str,amount_minor:i64,reason:&str,created_by:&str)->Result<PayrollAdjustment,WorkforceError>{validate_id(worker_id,"worker_id")?;validate_id(kind,"kind")?;if reason.trim().is_empty(){return Err(WorkforceError::Validation("reason required".into()))};if amount_minor==0{return Err(WorkforceError::Validation("zero adjustment".into()))};let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("INSERT INTO payroll_adjustments(id,worker_id,pay_record_id,kind,amount_minor,reason,created_by,created_at) VALUES(?,?,?,?,?,?,?,?)",params![id,worker_id,pay_record_id,kind,amount_minor,reason,created_by,t])?;drop(c);self.get_adjustment(&id)}
    pub fn get_adjustment(&self,id:&str)->Result<PayrollAdjustment,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,pay_record_id,kind,amount_minor,reason,created_by,created_at FROM payroll_adjustments WHERE id=?1",params![id],row_adjustment).optional()?.ok_or(WorkforceError::NotFound)}
    pub fn issue_advance(&self,worker_id:&str,amount_minor:i64,reason:&str,created_by:&str)->Result<WorkerAdvance,WorkforceError>{validate_id(worker_id,"worker_id")?;if amount_minor<=0||reason.trim().is_empty(){return Err(WorkforceError::Validation("invalid advance".into()))};let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("INSERT INTO worker_advances(id,worker_id,amount_minor,currency,reason,status,created_by,created_at) VALUES(?,?,'',?,'')",params![]).ok();
        c.execute("INSERT INTO worker_advances(id,worker_id,amount_minor,currency,reason,status,created_by,created_at) VALUES(?,?,?,'UGX',?,'OUTSTANDING',?,?)",params![id,worker_id,amount_minor,reason,created_by,t])?;drop(c);self.get_advance(&id)}
    pub fn get_advance(&self,id:&str)->Result<WorkerAdvance,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,amount_minor,currency,reason,status,created_by,created_at FROM worker_advances WHERE id=?1",params![id],row_advance).optional()?.ok_or(WorkforceError::NotFound)}
    pub fn record_payment(&self,worker_id:&str,amount_minor:i64,method:&str,reference:Option<&str>,paid_by:&str)->Result<WorkerPayment,WorkforceError>{validate_id(worker_id,"worker_id")?;validate_id(paid_by,"paid_by")?;if amount_minor<=0||!matches!(method,"CASH"|"MTN_MOMO"|"AIRTEL_MONEY"|"OTHER"){return Err(WorkforceError::Validation("invalid payment".into()))};let t=now();let id=Uuid::new_v4().to_string();let c=self.conn.lock().expect("workforce mutex poisoned");c.execute("INSERT INTO worker_payments(id,worker_id,amount_minor,currency,payment_method,reference,paid_by,created_at) VALUES(?,?,?,'UGX',?,?,?,?)",params![id,worker_id,amount_minor,method,reference,paid_by,t])?;drop(c);self.get_payment(&id)}
    pub fn get_payment(&self,id:&str)->Result<WorkerPayment,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");c.query_row("SELECT id,worker_id,amount_minor,currency,payment_method,reference,paid_by,created_at FROM worker_payments WHERE id=?1",params![id],row_payment).optional()?.ok_or(WorkforceError::NotFound)}
    pub fn payroll_summary(&self,worker_id:&str)->Result<PayrollSummary,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let gross:i64=c.query_row("SELECT COALESCE(SUM(amount_minor),0) FROM pay_records WHERE worker_id=?1 AND status<>'VOIDED'",params![worker_id],|r|r.get(0))?;let adj:i64=c.query_row("SELECT COALESCE(SUM(amount_minor),0) FROM payroll_adjustments WHERE worker_id=?1",params![worker_id],|r|r.get(0))?;let adv:i64=c.query_row("SELECT COALESCE(SUM(amount_minor),0) FROM worker_advances WHERE worker_id=?1 AND status='OUTSTANDING'",params![worker_id],|r|r.get(0))?;let paid:i64=c.query_row("SELECT COALESCE(SUM(amount_minor),0) FROM worker_payments WHERE worker_id=?1",params![worker_id],|r|r.get(0))?;Ok(PayrollSummary{gross_minor:gross,adjustments_minor:adj,advances_minor:adv,paid_minor:paid,balance_minor:gross+adj-adv-paid})}
    pub fn hours_in_period(&self,worker_id:&str,from:i64,to:i64)->Result<f64,WorkforceError>{self.attendance_overlapping(worker_id,from,to).map(|v|v.iter().map(worked_hours).sum())}
    fn attendance_overlapping(&self,worker_id:&str,from:i64,to:i64)->Result<Vec<AttendanceRecord>,WorkforceError>{let c=self.conn.lock().expect("workforce mutex poisoned");let mut s=c.prepare("SELECT id,worker_id,device_id,clock_in_at,clock_out_at,break_minutes,status,notes,created_at,updated_at FROM attendance WHERE worker_id=?1 AND status IN('CLOSED','ADJUSTED') AND clock_out_at IS NOT NULL AND clock_out_at>?2 AND clock_in_at<?3 ORDER BY clock_in_at")?;let mut out=Vec::new();for r in s.query_map(params![worker_id,from,to],row_att)?{out.push(r?)}Ok(out)}
}

fn row_att(r:&rusqlite::Row<'_>)->Result<AttendanceRecord,rusqlite::Error>{Ok(AttendanceRecord{id:r.get(0)?,worker_id:r.get(1)?,device_id:r.get(2)?,clock_in_at:r.get(3)?,clock_out_at:r.get(4)?,break_minutes:r.get(5)?,status:match r.get::<_,String>(6)?.as_str(){"OPEN"=>AttendanceStatus::Open,"CLOSED"=>AttendanceStatus::Closed,"ADJUSTED"=>AttendanceStatus::Adjusted,_=>AttendanceStatus::Closed},notes:r.get(7)?,created_at:r.get(8)?,updated_at:r.get(9)?})}
fn row_availability(r:&rusqlite::Row<'_>)->Result<AvailabilityRule,rusqlite::Error>{Ok(AvailabilityRule{id:r.get(0)?,worker_id:r.get(1)?,day_of_week:r.get(2)?,available_from_minute:r.get(3)?,available_to_minute:r.get(4)?,preferred_shift:r.get(5)?,active:r.get::<_,i64>(6)?!=0,valid_from:r.get(7)?,valid_to:r.get(8)?,created_at:r.get(9)?,updated_at:r.get(10)?})}
fn row_shift(r:&rusqlite::Row<'_>)->Result<Shift,rusqlite::Error>{Ok(Shift{id:r.get(0)?,worker_id:r.get(1)?,role:r.get(2)?,shift_type:r.get(3)?,starts_at:r.get(4)?,ends_at:r.get(5)?,location:r.get(6)?,status:r.get(7)?,created_by:r.get(8)?,created_at:r.get(9)?,updated_at:r.get(10)?})}
fn row_swap(r:&rusqlite::Row<'_>)->Result<ShiftSwap,rusqlite::Error>{Ok(ShiftSwap{id:r.get(0)?,shift_id:r.get(1)?,from_worker_id:r.get(2)?,to_worker_id:r.get(3)?,reason:r.get(4)?,status:r.get(5)?,requested_by:r.get(6)?,approved_by:r.get(7)?,created_at:r.get(8)?,updated_at:r.get(9)?})}
fn row_rate(r:&rusqlite::Row<'_>)->Result<PayRate,rusqlite::Error>{Ok(PayRate{id:r.get(0)?,worker_id:r.get(1)?,role:r.get(2)?,shift_type:r.get(3)?,rate_minor:r.get(4)?,currency:r.get(5)?,effective_from:r.get(6)?,effective_to:r.get(7)?,created_by:r.get(8)?,created_at:r.get(9)?})}
fn row_pay(r:&rusqlite::Row<'_>)->Result<PayRecord,rusqlite::Error>{Ok(PayRecord{id:r.get(0)?,worker_id:r.get(1)?,period_start:r.get(2)?,period_end:r.get(3)?,hours_worked:r.get(4)?,rate_minor:r.get(5)?,amount_minor:r.get(6)?,currency:r.get(7)?,status:r.get(8)?,created_by:r.get(9)?,created_at:r.get(10)?,updated_at:r.get(11)?})}
fn row_adjustment(r:&rusqlite::Row<'_>)->Result<PayrollAdjustment,rusqlite::Error>{Ok(PayrollAdjustment{id:r.get(0)?,worker_id:r.get(1)?,pay_record_id:r.get(2)?,kind:r.get(3)?,amount_minor:r.get(4)?,reason:r.get(5)?,created_by:r.get(6)?,created_at:r.get(7)?})}
fn row_advance(r:&rusqlite::Row<'_>)->Result<WorkerAdvance,rusqlite::Error>{Ok(WorkerAdvance{id:r.get(0)?,worker_id:r.get(1)?,amount_minor:r.get(2)?,currency:r.get(3)?,reason:r.get(4)?,status:r.get(5)?,created_by:r.get(6)?,created_at:r.get(7)?})}
fn row_payment(r:&rusqlite::Row<'_>)->Result<WorkerPayment,rusqlite::Error>{Ok(WorkerPayment{id:r.get(0)?,worker_id:r.get(1)?,amount_minor:r.get(2)?,currency:r.get(3)?,payment_method:r.get(4)?,reference:r.get(5)?,paid_by:r.get(6)?,created_at:r.get(7)?})}
fn validate_id(v:&str,name:&str)->Result<(),WorkforceError>{if v.trim().is_empty()||v.len()>128{Err(WorkforceError::Validation(format!("invalid {name}")))}else{Ok(())}}
fn worked_hours(a:&AttendanceRecord)->f64{match a.clock_out_at{Some(out)=>(((out-a.clock_in_at).max(0)-(a.break_minutes.max(0)*60)).max(0) as f64)/3600.0,None=>0.0}}
fn now()->i64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64}

#[cfg(test)]
mod tests{
 use super::*;
 #[test] fn attendance_rules(){let s=WorkforceStore::open_in_memory().unwrap();let a=s.clock_in("w1","d1",None).unwrap();assert_eq!(a.status,AttendanceStatus::Open);assert!(matches!(s.clock_in("w1","d1",None),Err(WorkforceError::AlreadyClockedIn)));let c=s.clock_out("w1",15,None).unwrap();assert_eq!(c.status,AttendanceStatus::Closed);assert_eq!(c.break_minutes,15);assert!(s.clock_out("w1",0,None).is_err());}
 #[test] fn availability_and_shifts(){let s=WorkforceStore::open_in_memory().unwrap();let a=s.set_availability("w1",1,480,1260,Some("DAY"),100,None,"ceo").unwrap();assert!(a.active);let sh=s.create_shift_typed("w1","WAITER","DAY",200,1000,"Najjera II Central Market","ceo").unwrap();assert_eq!(sh.status,"SCHEDULED");}
 #[test] fn swap_changes_paid_worker(){let s=WorkforceStore::open_in_memory().unwrap();let sh=s.create_shift("w1","WAITER",100,200,"Najjera II Central Market","ceo").unwrap();let sw=s.request_shift_swap(&sh.id,"w2","w1",Some("cover")).unwrap();let done=s.approve_shift_swap(&sw.id,"ceo").unwrap();assert_eq!(done.status,"APPROVED");assert_eq!(s.get_shift(&sh.id).unwrap().worker_id,"w2");}
 #[test] fn historical_rate_and_payroll(){let s=WorkforceStore::open_in_memory().unwrap();s.set_pay_rate("w1","WAITER","DAY",20000,"UGX",100,"ceo").unwrap();let sh=s.create_shift_typed("w1","WAITER","DAY",100,3700,"Najjera II Central Market","ceo").unwrap();s.clock_in("w1","d1",None).unwrap();s.clock_out("w1",0,None).unwrap();let pay=s.create_pay_record("w1",100,3700,2.0,20000,"ceo").unwrap();assert_eq!(pay.amount_minor,40000);assert_eq!(s.approve_pay(&pay.id).unwrap().status,"APPROVED");assert_eq!(s.payroll_summary("w1").unwrap().gross_minor,40000);let _=sh;}
 #[test] fn advances_and_payments(){let s=WorkforceStore::open_in_memory().unwrap();s.create_pay_record("w1",1,2,1.0,10000,"ceo").unwrap();s.issue_advance("w1",2000,"food","ceo").unwrap();s.record_payment("w1",5000,"CASH",None,"ceo").unwrap();let p=s.payroll_summary("w1").unwrap();assert_eq!(p.balance_minor,3000);}
 #[test] fn invalid_inputs(){let s=WorkforceStore::open_in_memory().unwrap();assert!(s.create_shift("","WAITER",1,2,"x","ceo").is_err());assert!(s.create_pay_record("w",2,1,1.0,1,"ceo").is_err());assert!(s.set_availability("w",7,1,2,None,1,None,"ceo").is_err());}
}
