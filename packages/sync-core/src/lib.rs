use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use thiserror::Error;
use uuid::Uuid;

const MAX_BACKOFF_SECONDS: i64 = 30 * 60;
const MAX_BATCH: usize = 250;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("database error")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("sync operation not found")]
    NotFound,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyncState { Pending, InFlight, Synced, Failed, Conflict }

impl SyncState { fn as_str(&self)->&'static str{match self{Self::Pending=>"PENDING",Self::InFlight=>"IN_FLIGHT",Self::Synced=>"SYNCED",Self::Failed=>"FAILED",Self::Conflict=>"CONFLICT"}} }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncOperation {
    pub id: String,
    pub device_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: String,
    pub payload: Value,
    pub payload_hash: String,
    pub sequence: i64,
    pub state: SyncState,
    pub attempts: i64,
    pub next_attempt_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncCursor { pub device_id:String, pub last_sequence:i64 }

pub struct SyncStore { conn: Mutex<Connection> }

impl SyncStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SyncError> {
        let conn=Connection::open(path)?;
        conn.pragma_update(None,"journal_mode","WAL")?;
        conn.pragma_update(None,"foreign_keys","ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let store=Self{conn:Mutex::new(conn)}; store.migrate()?; Ok(store)
    }

    pub fn open_in_memory()->Result<Self,SyncError>{let conn=Connection::open_in_memory()?;conn.pragma_update(None,"foreign_keys","ON")?;let s=Self{conn:Mutex::new(conn)};s.migrate()?;Ok(s)}

    fn migrate(&self)->Result<(),SyncError>{
        let c=self.conn.lock().expect("sync store mutex poisoned");
        c.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS sync_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS sync_operations(
          id TEXT PRIMARY KEY,
          device_id TEXT NOT NULL,
          entity_type TEXT NOT NULL,
          entity_id TEXT NOT NULL,
          operation TEXT NOT NULL CHECK(operation IN ('CREATE','UPDATE','DELETE','ADJUST')),
          payload_json TEXT NOT NULL,
          payload_hash TEXT NOT NULL,
          sequence INTEGER NOT NULL,
          state TEXT NOT NULL CHECK(state IN ('PENDING','IN_FLIGHT','SYNCED','FAILED','CONFLICT')),
          attempts INTEGER NOT NULL DEFAULT 0,
          next_attempt_at INTEGER NOT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          UNIQUE(device_id,sequence)
        );
        CREATE INDEX IF NOT EXISTS idx_sync_due ON sync_operations(state,next_attempt_at,sequence);
        CREATE INDEX IF NOT EXISTS idx_sync_entity ON sync_operations(entity_type,entity_id,sequence);
        CREATE TABLE IF NOT EXISTS sync_inbox(
          operation_id TEXT PRIMARY KEY,
          device_id TEXT NOT NULL,
          received_at INTEGER NOT NULL,
          payload_hash TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_conflicts(
          id TEXT PRIMARY KEY,
          operation_id TEXT NOT NULL,
          entity_type TEXT NOT NULL,
          entity_id TEXT NOT NULL,
          local_payload_json TEXT,
          remote_payload_json TEXT,
          detected_at INTEGER NOT NULL,
          resolved_at INTEGER,
          resolution TEXT
        );
        CREATE TABLE IF NOT EXISTS sync_peers(device_id TEXT PRIMARY KEY,last_remote_sequence INTEGER NOT NULL DEFAULT 0,last_sync_at INTEGER);
        "#)?; Ok(())
    }

    pub fn enqueue(&self,device_id:&str,entity_type:&str,entity_id:&str,operation:&str,payload:&Value)->Result<SyncOperation,SyncError>{
        validate_id(device_id,"device_id")?;validate_id(entity_type,"entity_type")?;validate_id(entity_id,"entity_id")?;
        if !matches!(operation,"CREATE"|"UPDATE"|"DELETE"|"ADJUST"){return Err(SyncError::Validation("unsupported operation".into()));}
        let encoded=serde_json::to_string(payload)?;let hash=hash(&encoded);let now=now();let id=Uuid::new_v4().to_string();
        let c=self.conn.lock().expect("sync store mutex poisoned");
        let seq:i64=c.query_row("SELECT COALESCE(MAX(sequence),0)+1 FROM sync_operations WHERE device_id=?1",params![device_id],|r|r.get(0))?;
        c.execute("INSERT INTO sync_operations(id,device_id,entity_type,entity_id,operation,payload_json,payload_hash,sequence,state,attempts,next_attempt_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",params![id,device_id,entity_type,entity_id,operation,encoded,hash,seq,"PENDING",0,now,now,now])?;drop(c);self.get(&id)
    }

    pub fn due_batch(&self,now:i64,limit:usize)->Result<Vec<SyncOperation>,SyncError>{
        let n=limit.clamp(1,MAX_BATCH);let c=self.conn.lock().expect("sync store mutex poisoned");let mut st=c.prepare("SELECT id,device_id,entity_type,entity_id,operation,payload_json,payload_hash,sequence,state,attempts,next_attempt_at,created_at,updated_at FROM sync_operations WHERE state IN ('PENDING','FAILED') AND next_attempt_at<=?1 ORDER BY sequence LIMIT ?2")?;
        let rows=st.query_map(params![now,n as i64],row_op)?;Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn claim(&self,ids:&[String])->Result<usize,SyncError>{if ids.is_empty(){return Ok(0)}let c=self.conn.lock().expect("sync store mutex poisoned");let tx=c.unchecked_transaction()?;let now=now();let mut count=0;for id in ids{count+=tx.execute("UPDATE sync_operations SET state='IN_FLIGHT',attempts=attempts+1,updated_at=?1 WHERE id=?2 AND state IN ('PENDING','FAILED')",params![now,id])?;}tx.commit()?;Ok(count)}

    pub fn ack(&self,id:&str)->Result<(),SyncError>{let c=self.conn.lock().expect("sync store mutex poisoned");let n=c.execute("UPDATE sync_operations SET state='SYNCED',updated_at=?1 WHERE id=?2 AND state='IN_FLIGHT'",params![now(),id])?;if n==0{return Err(SyncError::NotFound)}Ok(())}

    pub fn fail(&self,id:&str,error:&str)->Result<(),SyncError>{let safe=error.chars().take(500).collect::<String>();let c=self.conn.lock().expect("sync store mutex poisoned");let row:Option<(i64,i64)>=c.query_row("SELECT attempts,created_at FROM sync_operations WHERE id=?1",params![id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;let Some((attempts,_))=row else{return Err(SyncError::NotFound)};let delay=(2_i64.saturating_pow(attempts.clamp(0,10) as u32)).min(MAX_BACKOFF_SECONDS);let next=now()+delay;c.execute("UPDATE sync_operations SET state='FAILED',next_attempt_at=?1,updated_at=?2 WHERE id=?3",params![next,now(),id])?;c.execute("INSERT OR REPLACE INTO sync_meta(key,value) VALUES('last_error_'||?1,?2)",params![id,safe])?;Ok(())}

    pub fn mark_conflict(&self,id:&str,local:&Value,remote:&Value)->Result<(),SyncError>{let op=self.get(id)?;let c=self.conn.lock().expect("sync store mutex poisoned");c.execute("UPDATE sync_operations SET state='CONFLICT',updated_at=?1 WHERE id=?2",params![now(),id])?;c.execute("INSERT INTO sync_conflicts(id,operation_id,entity_type,entity_id,local_payload_json,remote_payload_json,detected_at) VALUES(?,?,?,?,?,?,?)",params![Uuid::new_v4().to_string(),id,op.entity_type,op.entity_id,serde_json::to_string(local)?,serde_json::to_string(remote)?,now()])?;Ok(())}

    pub fn ingest_remote(&self,op:&SyncOperation)->Result<bool,SyncError>{
        let c=self.conn.lock().expect("sync store mutex poisoned");let existing:Option<String>=c.query_row("SELECT payload_hash FROM sync_inbox WHERE operation_id=?1",params![op.id],|r|r.get(0)).optional()?;
        if let Some(h)=existing{if h==op.payload_hash{return Ok(false)}return Err(SyncError::Validation("operation id reused with different payload".into()));}
        let actual=hash(&serde_json::to_string(&op.payload)?);if actual!=op.payload_hash{return Err(SyncError::Validation("payload hash mismatch".into()));}
        c.execute("INSERT INTO sync_inbox(operation_id,device_id,received_at,payload_hash) VALUES(?,?,?,?)",params![op.id,op.device_id,now(),op.payload_hash])?;Ok(true)
    }

    pub fn compact_synced(&self,before:i64)->Result<usize,SyncError>{let c=self.conn.lock().expect("sync store mutex poisoned");Ok(c.execute("DELETE FROM sync_operations WHERE state='SYNCED' AND updated_at<?1",params![before])?)}
    pub fn get(&self,id:&str)->Result<SyncOperation,SyncError>{let c=self.conn.lock().expect("sync store mutex poisoned");c.query_row("SELECT id,device_id,entity_type,entity_id,operation,payload_json,payload_hash,sequence,state,attempts,next_attempt_at,created_at,updated_at FROM sync_operations WHERE id=?1",params![id],row_op).optional()?.ok_or(SyncError::NotFound)}
    pub fn pending_count(&self)->Result<i64,SyncError>{let c=self.conn.lock().expect("sync store mutex poisoned");Ok(c.query_row("SELECT COUNT(*) FROM sync_operations WHERE state IN ('PENDING','FAILED','IN_FLIGHT','CONFLICT')",[],|r|r.get(0))?)}
}

fn row_op(r:&rusqlite::Row<'_>)->Result<SyncOperation,rusqlite::Error>{let state:String=r.get(8)?;let state=match state.as_str(){"PENDING"=>SyncState::Pending,"IN_FLIGHT"=>SyncState::InFlight,"SYNCED"=>SyncState::Synced,"FAILED"=>SyncState::Failed,"CONFLICT"=>SyncState::Conflict,_=>SyncState::Failed};Ok(SyncOperation{id:r.get(0)?,device_id:r.get(1)?,entity_type:r.get(2)?,entity_id:r.get(3)?,operation:r.get(4)?,payload:serde_json::from_str(&r.get::<_,String>(5)?).unwrap_or(Value::Null),payload_hash:r.get(6)?,sequence:r.get(7)?,state,attempts:r.get(9)?,next_attempt_at:r.get(10)?,created_at:r.get(11)?,updated_at:r.get(12)?})}
fn validate_id(v:&str,name:&str)->Result<(),SyncError>{if v.trim().is_empty()||v.len()>128{return Err(SyncError::Validation(format!("invalid {name}")))}Ok(())}
fn now()->i64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64}
fn hash(v:&str)->String{let mut h=Sha256::new();h.update(v.as_bytes());URL_SAFE_NO_PAD.encode(h.finalize())}

#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn enqueue_is_ordered_and_idempotent_ingest_is_replay_safe(){let s=SyncStore::open_in_memory().unwrap();let a=s.enqueue("device-1","ORDER","order-1","CREATE",&serde_json::json!({"total_minor":1200})).unwrap();let b=s.enqueue("device-1","ORDER","order-2","CREATE",&serde_json::json!({"total_minor":800})).unwrap();assert_eq!(a.sequence,1);assert_eq!(b.sequence,2);let due=s.due_batch(now(),10).unwrap();assert_eq!(due.len(),2);s.claim(&[a.id.clone(),b.id.clone()]).unwrap();s.ack(&a.id).unwrap();assert_eq!(s.pending_count().unwrap(),1);assert!(!s.ingest_remote(&b).unwrap());assert!(s.ingest_remote(&a).unwrap());assert!(!s.ingest_remote(&a).unwrap());}
 #[test]fn payload_tampering_is_rejected(){let s=SyncStore::open_in_memory().unwrap();let mut op=s.enqueue("device-1","ORDER","order-1","CREATE",&serde_json::json!({"x":1})).unwrap();op.payload=serde_json::json!({"x":2});assert!(s.ingest_remote(&op).is_err());}
 #[test]fn retry_uses_bounded_exponential_backoff(){let s=SyncStore::open_in_memory().unwrap();let op=s.enqueue("device-1","ORDER","order-1","CREATE",&serde_json::json!({})).unwrap();s.claim(&[op.id.clone()]).unwrap();s.fail(&op.id,"network down").unwrap();let current=s.get(&op.id).unwrap();assert!(current.next_attempt_at>now());assert_eq!(current.state,SyncState::Failed);}
}
