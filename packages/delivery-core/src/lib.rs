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
