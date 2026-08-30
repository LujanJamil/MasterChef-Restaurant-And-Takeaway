use base64::{engine::general_purpose::STANDARD, Engine};
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
pub enum SyncState {
    Pending,
    InFlight,
    Synced,
    Failed,
    Conflict,
}

impl SyncState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InFlight => "IN_FLIGHT",
            Self::Synced => "SYNCED",
            Self::Failed => "FAILED",
            Self::Conflict => "CONFLICT",
        }
    }
}

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
pub struct SyncCursor {
    pub device_id: String,
    pub last_sequence: i64,
}

pub struct SyncStore {
    conn: Mutex<Connection>,
}

impl SyncStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SyncError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, SyncError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let s = Self {
            conn: Mutex::new(conn),
        };
        s.migrate()?;
        Ok(s)
    }

    fn migrate(&self) -> Result<(), SyncError> {
        let c = self.conn.lock().expect("sync store mutex poisoned");
        let actual = hash_payload(&serde_json::to_string(&op.payload)?);
        if actual != op.payload_hash {
            return Err(SyncError::Validation("payload hash mismatch".into()));
        }
        let local: Option<String> = c
            .query_row("SELECT id FROM sync_meta WHERE id=?1 AND state<>'SYNCED'", params![op.id], |r| r.get(0))
            .optional()?;
        if local.is_some() {
            return Ok(false);
        }
        let existing: Option<String> = c
            .query_row(
                "SELECT payload_hash FROM sync_inbox WHERE operation_id=?1",
                params![op.id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(h) = existing {
            if h == op.payload_hash {
                return Ok(false);
            }
            return Err(SyncError::Validation(
                "operation id reused with different payload".into(),
            ));
        }
        c.execute(
            "INSERT INTO sync_inbox(operation_id, device_id, received_at, payload_hash) VALUES (?,?,?,?)",
            params![op.id, op.device_id, now(), op.payload_hash],
        )?;
        Ok(true)
    }

    pub fn compact_synced(&self, before: i64) -> Result<usize, SyncError> {
        let c = self.conn.lock().expect("sync store mutex poisoned");
        Ok(c.execute(
            "DELETE FROM sync_operations WHERE state='SYNCED' AND updated_at < ?1",
            params![before],
        )?)
    }

    pub fn get(&self, id: &str) -> Result<SyncOperation, SyncError> {
        let c = self.conn.lock().expect("sync store mutex poisoned");
        c.query_row(
            "SELECT id, device_id, entity_type, entity_id, operation, payload_json, payload_hash,
                    sequence, state, attempts, next_attempt_at, created_at, updated_at
             FROM sync_operations WHERE id=?1",
            params![id],
            row_op,
        )
        .optional()?
        .ok_or(SyncError::NotFound)
    }

    pub fn pending_count(&self) -> Result<i64, SyncError> {
        let c = self.conn.lock().expect("sync store mutex poisoned");
        Ok(c.query_row(
            "SELECT COUNT(*) FROM sync_operations
             WHERE state IN ('PENDING','FAILED','IN_FLIGHT','CONFLICT')",
            [],
            |r| r.get(0),
        )?)
    }

    /// Deterministic SHA-256 + standard base64 (matches Postgres encode(..., 'base64')).
    pub fn compute_payload_hash(payload: &Value) -> Result<String, SyncError> {
        let encoded = serde_json::to_string(payload)?;
        Ok(hash_payload(&encoded))
    }
}

fn row_op(r: &rusqlite::Row<'_>) -> Result<SyncOperation, rusqlite::Error> {
    let state: String = r.get(8)?;
    let state = match state.as_str() {
        "PENDING" => SyncState::Pending,
        "IN_FLIGHT" => SyncState::InFlight,
        "SYNCED" => SyncState::Synced,
        "FAILED" => SyncState::Failed,
        "CONFLICT" => SyncState::Conflict,
        _ => SyncState::Failed,
    };
    Ok(SyncOperation {
        id: r.get(0)?,
        device_id: r.get(1)?,
        entity_type: r.get(2)?,
        entity_id: r.get(3)?,
        operation: r.get(4)?,
        payload: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or(Value::Null),
        payload_hash: r.get(6)?,
        sequence: r.get(7)?,
        state,
        attempts: r.get(9)?,
        next_attempt_at: r.get(10)?,
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

fn validate_id(v: &str, name: &str) -> Result<(), SyncError> {
    if v.trim().is_empty() || v.len() > 128 {
        return Err(SyncError::Validation(format!("invalid {name}")));
    }
    Ok(())
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Standard base64 (with padding) to match PostgreSQL encode(digest(...), 'base64').
fn hash_payload(v: &str) -> String {
    let mut h = Sha256::new();
    h.update(v.as_bytes());
    STANDARD.encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_is_ordered_and_idempotent_ingest_is_replay_safe() {
        let s = SyncStore::open_in_memory().unwrap();
        let a = s
            .enqueue(
                "device-1",
                "ORDER",
                "order-1",
                "CREATE",
                &serde_json::json!({"total_minor": 1200}),
            )
            .unwrap();
        let b = s
            .enqueue(
                "device-1",
                "ORDER",
                "order-2",
                "CREATE",
                &serde_json::json!({"total_minor": 800}),
            )
            .unwrap();
        assert_eq!(a.sequence, 1);
        assert_eq!(b.sequence, 2);
        let due = s.due_batch(now(), 10).unwrap();
        assert_eq!(due.len(), 2);
        s.claim(&[a.id.clone(), b.id.clone()]).unwrap();
        s.ack(&a.id).unwrap();
        assert_eq!(s.pending_count().unwrap(), 1);
        assert!(!s.ingest_remote(&b).unwrap());
        assert!(s.ingest_remote(&a).unwrap());
        assert!(!s.ingest_remote(&a).unwrap());
    }

    #[test]
    fn payload_tampering_is_rejected() {
        let s = SyncStore::open_in_memory().unwrap();
        let mut op = s
            .enqueue(
                "device-1",
                "ORDER",
                "order-1",
                "CREATE",
                &serde_json::json!({"x": 1}),
            )
            .unwrap();
        op.payload = serde_json::json!({"x": 2});
        assert!(s.ingest_remote(&op).is_err());
    }

    #[test]
    fn retry_uses_bounded_exponential_backoff() {
        let s = SyncStore::open_in_memory().unwrap();
        let op = s
            .enqueue(
                "device-1",
                "ORDER",
                "order-1",
                "CREATE",
                &serde_json::json!({}),
            )
            .unwrap();
        s.claim(&[op.id.clone()]).unwrap();
        s.fail(&op.id, "network down").unwrap();
        let current = s.get(&op.id).unwrap();
        assert!(current.next_attempt_at > now());
        assert_eq!(current.state, SyncState::Failed);
    }

    #[test]
    fn hash_matches_standard_base64() {
        let payload = serde_json::json!({"total_minor": 1200});
        let h = SyncStore::compute_payload_hash(&payload).unwrap();
        // Must be standard base64 (contains +/ and possibly = padding)
        assert!(!h.contains('-'));
        assert!(!h.contains('_'));
    }
}
