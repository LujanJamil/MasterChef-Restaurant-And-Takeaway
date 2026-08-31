use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DeliveryError {
    #[error("database error")]
    Database(#[from] rusqlite::Error),
    #[error("validation: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("invalid transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
    #[error("order already has an open delivery")]
    DuplicateDelivery,
    #[error("forbidden")]
    Forbidden,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delivery {
    pub id: String,
    pub order_id: String,
    pub rider_id: String,
    pub status: String,
    pub assigned_by: String,
    pub collected_minor: Option<i64>,
    pub handover_minor: Option<i64>,
    pub currency: String,
    pub notes: Option<String>,
    pub device_id: String,
    pub assigned_at: i64,
    pub picked_up_at: Option<i64>,
    pub delivered_at: Option<i64>,
    pub handover_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RiderReconciliation {
    pub open_deliveries: i64,
    pub expected_handover_minor: i64,
    pub settled_minor: i64,
}

fn now() -> i64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64 }

fn row_delivery(r: &rusqlite::Row<'_>) -> rusqlite::Result<Delivery> {
    Ok(Delivery {
        id: r.get(0)?, order_id: r.get(1)?, rider_id: r.get(2)?, status: r.get(3)?,
        assigned_by: r.get(4)?, collected_minor: r.get(5)?, handover_minor: r.get(6)?,
        currency: r.get(7)?, notes: r.get(8)?, device_id: r.get(9)?, assigned_at: r.get(10)?,
        picked_up_at: r.get(11)?, delivered_at: r.get(12)?, handover_at: r.get(13)?,
        created_at: r.get(14)?, updated_at: r.get(15)?,
    })
}

const COLS: &str = "id,order_id,rider_id,status,assigned_by,collected_minor,handover_minor,currency,notes,device_id,assigned_at,picked_up_at,delivered_at,handover_at,created_at,updated_at";

pub struct DeliveryStore { conn: Mutex<Connection> }

impl DeliveryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DeliveryError> {
        let c = Connection::open(path)?;
        c.pragma_update(None, "journal_mode", "WAL")?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        let s = Self { conn: Mutex::new(c) };
        s.migrate()?;
        Ok(s)
    }
    pub fn open_in_memory() -> Result<Self, DeliveryError> {
        let c = Connection::open_in_memory()?;
        let s = Self { conn: Mutex::new(c) };
        s.migrate()?;
        Ok(s)
    }
    fn migrate(&self) -> Result<(), DeliveryError> {
        let c = self.conn.lock().unwrap();
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS deliveries(
                id TEXT PRIMARY KEY, order_id TEXT NOT NULL, rider_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'ASSIGNED' CHECK(status IN ('ASSIGNED','PICKED_UP','DELIVERED','SETTLED','CANCELLED')),
                assigned_by TEXT NOT NULL, collected_minor INTEGER, handover_minor INTEGER,
                currency TEXT NOT NULL DEFAULT 'UGX', notes TEXT, device_id TEXT NOT NULL,
                assigned_at INTEGER NOT NULL, picked_up_at INTEGER, delivered_at INTEGER, handover_at INTEGER,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS deliveries_rider ON deliveries(rider_id,status);
             CREATE INDEX IF NOT EXISTS deliveries_order ON deliveries(order_id);",
        )?;
        Ok(())
    }

    fn get(&self, c: &Connection, id: &str) -> Result<Delivery, DeliveryError> {
        c.query_row(&format!("SELECT {COLS} FROM deliveries WHERE id=?"), params![id], row_delivery).optional()?.ok_or(DeliveryError::NotFound)
    }

    pub fn assign(&self, order_id: &str, rider_id: &str, notes: Option<&str>, assigned_by: &str, device_id: &str) -> Result<Delivery, DeliveryError> {
        if order_id.trim().is_empty() || rider_id.trim().is_empty() { return Err(DeliveryError::Validation("order and rider are required".into())); }
        let c = self.conn.lock().unwrap();
        let open: Option<String> = c.query_row("SELECT id FROM deliveries WHERE order_id=?1 AND status NOT IN ('CANCELLED','SETTLED')", params![order_id], |r| r.get(0)).optional()?;
        if open.is_some() { return Err(DeliveryError::DuplicateDelivery); }
        let did = Uuid::new_v4().to_string();
        let t = now();
        c.execute(
            "INSERT INTO deliveries(id,order_id,rider_id,status,assigned_by,currency,notes,device_id,assigned_at,created_at,updated_at) VALUES(?,?,?,'ASSIGNED',?,'UGX',?,?,?,?,?)",
            params![did, order_id, rider_id, assigned_by, notes, device_id, t, t, t],
        )?;
        self.get(&c, &did)
    }

    fn transition(&self, id: &str, actor: &str, expect: &str, to: &str, enforce_rider: bool) -> Result<Delivery, DeliveryError> {
        let c = self.conn.lock().unwrap();
        let d = self.get(&c, id)?;
        if d.status != expect { return Err(DeliveryError::InvalidTransition { from: d.status, to: to.into() }); }
        if enforce_rider && d.rider_id != actor { return Err(DeliveryError::Forbidden); }
        let t = now();
        let col = match to { "PICKED_UP" => "picked_up_at", "DELIVERED" => "delivered_at", _ => "handover_at" };
        c.execute(&format!("UPDATE deliveries SET status=?2,{col}=?3,updated_at=?3 WHERE id=?1"), params![id, to, t])?;
        self.get(&c, id)
    }

    pub fn pickup(&self, id: &str, rider_id: &str) -> Result<Delivery, DeliveryError> { self.transition(id, rider_id, "ASSIGNED", "PICKED_UP", true) }

    pub fn deliver(&self, id: &str, rider_id: &str, collected_minor: i64) -> Result<Delivery, DeliveryError> {
        if collected_minor < 0 { return Err(DeliveryError::Validation("collected amount must be >= 0".into())); }
        let d = self.transition(id, rider_id, "PICKED_UP", "DELIVERED", true)?;
        let c = self.conn.lock().unwrap();
        c.execute("UPDATE deliveries SET collected_minor=?2 WHERE id=?1", params![id, collected_minor])?;
        self.get(&c, id).map(|mut x| { x.collected_minor = Some(collected_minor); x })
            .map(|x| { let _ = d; x })
    }

    pub fn reconcile(&self, id: &str, handover_minor: i64) -> Result<Delivery, DeliveryError> {
        if handover_minor < 0 { return Err(DeliveryError::Validation("handover must be >= 0".into())); }
        let d = self.transition(id, "", "DELIVERED", "SETTLED", false)?;
        let c = self.conn.lock().unwrap();
        c.execute("UPDATE deliveries SET handover_minor=?2 WHERE id=?1", params![id, handover_minor])?;
        self.get(&c, id).map(|x| { let _ = d; x })
    }

    pub fn cancel(&self, id: &str) -> Result<Delivery, DeliveryError> {
        let c = self.conn.lock().unwrap();
        let d = self.get(&c, id)?;
        if d.status == "SETTLED" || d.status == "CANCELLED" { return Err(DeliveryError::InvalidTransition { from: d.status, to: "CANCELLED".into() }); }
        c.execute("UPDATE deliveries SET status='CANCELLED',updated_at=?2 WHERE id=?1", params![id, now()])?;
        self.get(&c, id)
    }

    pub fn list(&self, rider_id: Option<&str>, status: Option<&str>, limit: usize) -> Result<Vec<Delivery>, DeliveryError> {
        let c = self.conn.lock().unwrap();
        let mut s = c.prepare(&format!("SELECT {COLS} FROM deliveries WHERE (?1 IS NULL OR rider_id=?1) AND (?2 IS NULL OR status=?2) ORDER BY created_at DESC LIMIT ?3"))?;
        let mut o = Vec::new();
        for r in s.query_map(params![rider_id, status, limit.clamp(1, 500) as i64], row_delivery)? { o.push(r?); }
        Ok(o)
    }

    pub fn reconciliation(&self, rider_id: &str) -> Result<RiderReconciliation, DeliveryError> {
        let c = self.conn.lock().unwrap();
        let open: i64 = c.query_row("SELECT COUNT(*) FROM deliveries WHERE rider_id=?1 AND status IN ('ASSIGNED','PICKED_UP')", params![rider_id], |r| r.get(0))?;
        let expected: i64 = c.query_row("SELECT COALESCE(SUM(collected_minor),0) FROM deliveries WHERE rider_id=?1 AND status='DELIVERED'", params![rider_id], |r| r.get(0))?;
        let settled: i64 = c.query_row("SELECT COALESCE(SUM(handover_minor),0) FROM deliveries WHERE rider_id=?1 AND status='SETTLED'", params![rider_id], |r| r.get(0))?;
        Ok(RiderReconciliation { open_deliveries: open, expected_handover_minor: expected, settled_minor: settled })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> DeliveryStore { DeliveryStore::open_in_memory().unwrap() }

    #[test]
    fn full_lifecycle_and_reconciliation() {
        let s = store();
        let d = s.assign("order-1", "rider-1", None, "manager", "dev").unwrap();
        assert_eq!(d.status, "ASSIGNED");
        assert!(s.assign("order-1", "rider-2", None, "manager", "dev").is_err());
        assert!(matches!(s.pickup(&d.id, "rider-2"), Err(DeliveryError::Forbidden)));
        let d = s.pickup(&d.id, "rider-1").unwrap();
        assert_eq!(d.status, "PICKED_UP");
        let d = s.deliver(&d.id, "rider-1", 45000).unwrap();
        assert_eq!(d.status, "DELIVERED");
        assert_eq!(d.collected_minor, Some(45000));
        let rec = s.reconciliation("rider-1").unwrap();
        assert_eq!(rec.expected_handover_minor, 45000);
        let d = s.reconcile(&d.id, 45000).unwrap();
        assert_eq!(d.status, "SETTLED");
        let rec = s.reconciliation("rider-1").unwrap();
        assert_eq!(rec.settled_minor, 45000);
        assert_eq!(rec.expected_handover_minor, 0);
    }

    #[test]
    fn cancel_blocks_further_progress() {
        let s = store();
        let d = s.assign("order-9", "rider-1", None, "manager", "dev").unwrap();
        s.cancel(&d.id).unwrap();
        assert!(s.pickup(&d.id, "rider-1").is_err());
        assert!(s.assign("order-9", "rider-1", None, "manager", "dev").is_ok());
    }
}
