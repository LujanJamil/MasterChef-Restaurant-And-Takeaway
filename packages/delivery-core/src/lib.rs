<<<<<<< HEAD
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeliveryStatus { Pending, Assigned, PickedUp, OutForDelivery, Delivered, Failed, Cancelled, Returned }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollectionMethod { Cash, MtnMobileMoney, AirtelMoney, Prepaid }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Delivery { pub id: Uuid, pub order_id: Uuid, pub rider_id: Option<Uuid>, pub status: DeliveryStatus, pub address: String, pub customer_name: Option<String>, pub customer_phone: Option<String>, pub collection_method: Option<CollectionMethod>, pub amount_due_minor: i64, pub amount_collected_minor: i64, pub proof_reference: Option<String>, pub failure_reason: Option<String>, pub created_at: i64, pub updated_at: i64 }

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeliveryError { #[error("address is required")] MissingAddress, #[error("amount due cannot be negative")] NegativeAmount, #[error("collection exceeds amount due")] OverCollection, #[error("rider is required for this transition")] RiderRequired, #[error("invalid delivery state transition")] InvalidTransition, #[error("proof of delivery is required")] ProofRequired, #[error("failure reason is required")] FailureReasonRequired, #[error("duplicate operation")] DuplicateOperation }

impl Delivery {
 pub fn new(order_id: Uuid,address:String,amount_due_minor:i64,now:i64)->Result<Self,DeliveryError>{ if address.trim().is_empty(){return Err(DeliveryError::MissingAddress)} if amount_due_minor<0{return Err(DeliveryError::NegativeAmount)} Ok(Self{id:Uuid::new_v4(),order_id,rider_id:None,status:DeliveryStatus::Pending,address,customer_name:None,customer_phone:None,collection_method:None,amount_due_minor,amount_collected_minor:0,proof_reference:None,failure_reason:None,created_at:now,updated_at:now}) }
 pub fn assign(&mut self,rider_id:Uuid,now:i64)->Result<(),DeliveryError>{if !matches!(self.status,DeliveryStatus::Pending|DeliveryStatus::Assigned){return Err(DeliveryError::InvalidTransition)} self.rider_id=Some(rider_id);self.status=DeliveryStatus::Assigned;self.updated_at=now;Ok(())}
 pub fn pickup(&mut self,now:i64)->Result<(),DeliveryError>{if self.rider_id.is_none(){return Err(DeliveryError::RiderRequired)} if !matches!(self.status,DeliveryStatus::Assigned){return Err(DeliveryError::InvalidTransition)} self.status=DeliveryStatus::PickedUp;self.updated_at=now;Ok(())}
 pub fn dispatch(&mut self,now:i64)->Result<(),DeliveryError>{if !matches!(self.status,DeliveryStatus::PickedUp){return Err(DeliveryError::InvalidTransition)} self.status=DeliveryStatus::OutForDelivery;self.updated_at=now;Ok(())}
 pub fn collect(&mut self,method:CollectionMethod,amount_minor:i64,now:i64)->Result<(),DeliveryError>{if amount_minor<0||self.amount_collected_minor.saturating_add(amount_minor)>self.amount_due_minor{return Err(DeliveryError::OverCollection)} self.collection_method=Some(method);self.amount_collected_minor+=amount_minor;self.updated_at=now;Ok(())}
 pub fn deliver(&mut self,proof_reference:String,now:i64)->Result<(),DeliveryError>{if !matches!(self.status,DeliveryStatus::OutForDelivery|DeliveryStatus::PickedUp){return Err(DeliveryError::InvalidTransition)} if proof_reference.trim().is_empty(){return Err(DeliveryError::ProofRequired)} if self.amount_collected_minor<self.amount_due_minor && self.collection_method.is_none(){return Err(DeliveryError::OverCollection)} self.proof_reference=Some(proof_reference);self.status=DeliveryStatus::Delivered;self.updated_at=now;Ok(())}
 pub fn fail(&mut self,reason:String,now:i64)->Result<(),DeliveryError>{if reason.trim().is_empty(){return Err(DeliveryError::FailureReasonRequired)} if matches!(self.status,DeliveryStatus::Delivered|DeliveryStatus::Cancelled|DeliveryStatus::Returned){return Err(DeliveryError::InvalidTransition)} self.failure_reason=Some(reason);self.status=DeliveryStatus::Failed;self.updated_at=now;Ok(())}
 pub fn cancel(&mut self,reason:String,now:i64)->Result<(),DeliveryError>{if reason.trim().is_empty(){return Err(DeliveryError::FailureReasonRequired)} if matches!(self.status,DeliveryStatus::Delivered|DeliveryStatus::Cancelled){return Err(DeliveryError::InvalidTransition)} self.failure_reason=Some(reason);self.status=DeliveryStatus::Cancelled;self.updated_at=now;Ok(())}
}

pub struct DeliveryStore { conn: Connection }
impl DeliveryStore {
 pub fn open(path: impl AsRef<Path>) -> Result<Self, String> { let conn=Connection::open(path).map_err(|e|e.to_string())?; conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS deliveries(id TEXT PRIMARY KEY, order_id TEXT NOT NULL, payload TEXT NOT NULL, updated_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS idx_deliveries_status ON deliveries(json_extract(payload, '$.status')); ").map_err(|e|e.to_string())?; Ok(Self{conn}) }
 pub fn save(&self,d:&Delivery)->Result<(),String>{let payload=serde_json::to_string(d).map_err(|e|e.to_string())?;self.conn.execute("INSERT INTO deliveries(id,order_id,payload,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,updated_at=excluded.updated_at",params![d.id.to_string(),d.order_id.to_string(),payload,d.updated_at]).map_err(|e|e.to_string())?;Ok(())}
 pub fn get(&self,id:&Uuid)->Result<Option<Delivery>,String>{let mut s=self.conn.prepare("SELECT payload FROM deliveries WHERE id=?1").map_err(|e|e.to_string())?;let mut rows=s.query(params![id.to_string()]).map_err(|e|e.to_string())?;match rows.next().map_err(|e|e.to_string())?{Some(r)=>{let p:String=r.get(0).map_err(|e|e.to_string())?;Ok(Some(serde_json::from_str(&p).map_err(|e|e.to_string())?))},None=>Ok(None)}}
 pub fn list(&self,limit:usize)->Result<Vec<Delivery>,String>{let mut s=self.conn.prepare("SELECT payload FROM deliveries ORDER BY updated_at DESC LIMIT ?1").map_err(|e|e.to_string())?;let rows=s.query_map(params![limit as i64],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;let mut out=Vec::new();for row in rows{let p=row.map_err(|e|e.to_string())?;out.push(serde_json::from_str(&p).map_err(|e|e.to_string())?);}Ok(out)}
}

#[derive(Default)] pub struct OperationGuard{seen:HashSet<Uuid>}
impl OperationGuard{pub fn accept(&mut self,operation_id:Uuid)->Result<(),DeliveryError>{if !self.seen.insert(operation_id){return Err(DeliveryError::DuplicateOperation)}Ok(())}}

#[cfg(test)] mod tests{use super::*;#[test]fn lifecycle(){let r=Uuid::new_v4();let mut d=Delivery::new(Uuid::new_v4(),"Najjera II Central Market".into(),15000,1).unwrap();d.assign(r,2).unwrap();d.pickup(3).unwrap();d.dispatch(4).unwrap();d.collect(CollectionMethod::Cash,15000,5).unwrap();d.deliver("proof-1".into(),6).unwrap();assert_eq!(d.status,DeliveryStatus::Delivered)}#[test]fn duplicate(){let mut g=OperationGuard::default();let id=Uuid::new_v4();assert!(g.accept(id).is_ok());assert_eq!(g.accept(id),Err(DeliveryError::DuplicateOperation))}#[test]fn illegal(){let mut d=Delivery::new(Uuid::new_v4(),"address".into(),1,1).unwrap();assert_eq!(d.dispatch(2),Err(DeliveryError::InvalidTransition))}#[test]fn persists(){let path=std::env::temp_dir().join(format!("mc-delivery-{}.sqlite",Uuid::new_v4()));let s=DeliveryStore::open(&path).unwrap();let d=Delivery::new(Uuid::new_v4(),"address".into(),10,1).unwrap();s.save(&d).unwrap();assert_eq!(s.get(&d.id).unwrap().unwrap().id,d.id);drop(s);let _=std::fs::remove_file(path);}}
=======
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
>>>>>>> c04e40f12226b5924f5dc826d1729f2e6f814588
