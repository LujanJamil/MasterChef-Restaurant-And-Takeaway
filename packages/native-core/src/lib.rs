use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use thiserror::Error;
use uuid::Uuid;

const SESSION_TTL: i64 = 8 * 60 * 60;
const SESSION_IDLE_TTL: i64 = 30 * 60;
const MAX_FAILED_LOGINS: i64 = 5;
const LOCKOUT_SECONDS: i64 = 15 * 60;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("account is temporarily locked")]
    AccountLocked,
    #[error("account is not active")]
    AccountInactive,
    #[error("not authorized")]
    Unauthorized,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("not initialized")]
    NotInitialized,
    #[error("resource not found")]
    NotFound,
    #[error("database error")]
    Database(#[from] rusqlite::Error),
    #[error("password hashing error")]
    PasswordHash(#[from] argon2::password_hash::Error),
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus { Active, Suspended, Inactive }

impl UserStatus {
    fn as_str(&self) -> &'static str { match self { Self::Active => "ACTIVE", Self::Suspended => "SUSPENDED", Self::Inactive => "INACTIVE" } }
}

impl TryFrom<&str> for UserStatus {
    type Error = AuthError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value { "ACTIVE" => Ok(Self::Active), "SUSPENDED" => Ok(Self::Suspended), "INACTIVE" => Ok(Self::Inactive), _ => Err(AuthError::Validation("invalid user status".into())) }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub status: UserStatus,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthSession {
    pub token: String,
    pub user: UserSummary,
    pub expires_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub actor_user_id: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub details: serde_json::Value,
    pub device_id: String,
    pub created_at: i64,
}

pub struct AuthStore { conn: Mutex<Connection> }

impl AuthStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AuthError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let store = Self { conn: Mutex::new(conn) };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, AuthError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self { conn: Mutex::new(conn) };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), AuthError> {
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                role TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('ACTIVE','SUSPENDED','INACTIVE')),
                password_hash TEXT NOT NULL,
                auth_version INTEGER NOT NULL DEFAULT 1,
                failed_attempts INTEGER NOT NULL DEFAULT 0,
                locked_until INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                token_hash TEXT NOT NULL UNIQUE,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                auth_version INTEGER NOT NULL,
                device_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_token_hash ON sessions(token_hash);
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE TABLE IF NOT EXISTS permission_overrides (
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                permission TEXT NOT NULL,
                effect TEXT NOT NULL CHECK(effect IN ('GRANT','REVOKE')),
                PRIMARY KEY(user_id, permission)
            );
            CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY,
                actor_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
                action TEXT NOT NULL,
                target_type TEXT,
                target_id TEXT,
                details_json TEXT NOT NULL,
                device_id TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_created_at ON audit_events(created_at DESC);
        "#)?;
        Ok(())
    }

    pub fn bootstrap_ceo(&self, username: &str, display_name: &str, password: &str, device_id: &str) -> Result<UserSummary, AuthError> {
        validate_username(username)?; validate_display_name(display_name)?; validate_password(password)?; validate_device(device_id)?;
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
        if count != 0 { return Err(AuthError::Unauthorized); }
        let id = Uuid::new_v4().to_string(); let now = now();
        let hash = hash_password(password)?;
        conn.execute("INSERT INTO users(id,username,display_name,role,status,password_hash,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)",
            params![id, normalize_username(username), display_name.trim(), "CEO", "ACTIVE", hash, now, now])?;
        drop(conn);
        self.audit(None, "CEO_BOOTSTRAPPED", Some("USER"), Some(&id), serde_json::json!({"username": normalize_username(username)}), device_id)?;
        self.get_user(&id)
    }

    pub fn login(&self, username: &str, password: &str, device_id: &str) -> Result<AuthSession, AuthError> {
        validate_username(username)?; validate_device(device_id)?;
        let uname = normalize_username(username); let now = now();
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        let row = conn.query_row("SELECT id,display_name,role,status,password_hash,auth_version,failed_attempts,locked_until,created_at FROM users WHERE username=?1", params![uname], |r| {
            Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,String>(2)?, r.get::<_,String>(3)?, r.get::<_,String>(4)?, r.get::<_,i64>(5)?, r.get::<_,i64>(6)?, r.get::<_,Option<i64>>(7)?, r.get::<_,i64>(8)?))
        }).optional()?;
        let Some((id, display_name, role, status, password_hash, auth_version, failed_attempts, locked_until, created_at)) = row else { return Err(AuthError::InvalidCredentials); };
        if locked_until.is_some_and(|until| until > now) { return Err(AuthError::AccountLocked); }
        if status != "ACTIVE" { return Err(AuthError::InvalidCredentials); }
        let valid = verify_password(password, &password_hash);
        if !valid {
            let next = failed_attempts + 1;
            let lock = if next >= MAX_FAILED_LOGINS { Some(now + LOCKOUT_SECONDS) } else { None };
            conn.execute("UPDATE users SET failed_attempts=?1,locked_until=?2,updated_at=?3 WHERE id=?4", params![if lock.is_some(){0}else{next}, lock, now, id])?;
            drop(conn);
            self.audit(None, "LOGIN_FAILED", Some("USER"), Some(&id), serde_json::json!({"reason":"invalid_credentials"}), device_id)?;
            return Err(if lock.is_some() { AuthError::AccountLocked } else { AuthError::InvalidCredentials });
        }
        conn.execute("UPDATE users SET failed_attempts=0,locked_until=NULL,updated_at=?1 WHERE id=?2", params![now, id])?;
        let mut raw = [0u8; 32]; OsRng.fill_bytes(&mut raw); let token = URL_SAFE_NO_PAD.encode(raw); let token_hash = hash_token(&token);
        let session_id = Uuid::new_v4().to_string(); let expires = now + SESSION_TTL;
        conn.execute("INSERT INTO sessions(id,token_hash,user_id,auth_version,device_id,created_at,expires_at,last_seen_at) VALUES(?,?,?,?,?,?,?,?)", params![session_id,token_hash,id,auth_version,device_id,now,expires,now])?;
        conn.execute("DELETE FROM sessions WHERE expires_at <= ?1 OR last_seen_at <= ?2", params![now, now-SESSION_IDLE_TTL])?;
        drop(conn);
        self.audit(Some(&id), "LOGIN_SUCCEEDED", Some("USER"), Some(&id), serde_json::json!({}), device_id)?;
        Ok(AuthSession { token, user: UserSummary { id, username: uname, display_name, role, status: UserStatus::Active, created_at }, expires_at: expires })
    }

    pub fn logout(&self, token: &str, device_id: &str) -> Result<(), AuthError> {
        let user = self.authenticate(token)?;
        let hash = hash_token(token); let conn = self.conn.lock().expect("auth store mutex poisoned");
        conn.execute("DELETE FROM sessions WHERE token_hash=?1", params![hash])?; drop(conn);
        self.audit(Some(&user.id), "LOGOUT", Some("USER"), Some(&user.id), serde_json::json!({}), device_id)?;
        Ok(())
    }

    pub fn authenticate(&self, token: &str) -> Result<UserSummary, AuthError> {
        if token.len() < 32 || token.len() > 128 { return Err(AuthError::Unauthorized); }
        let now = now(); let hash = hash_token(token);
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        let row = conn.query_row("SELECT u.id,u.username,u.display_name,u.role,u.status,u.auth_version,u.created_at,s.expires_at,s.last_seen_at FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=?1", params![hash], |r| {
            Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,i64>(8)?))
        }).optional()?;
        let Some((id, username, display_name, role, status, auth_version, created_at, expires_at, last_seen)) = row else { return Err(AuthError::Unauthorized); };
        if expires_at <= now || last_seen + SESSION_IDLE_TTL <= now { conn.execute("DELETE FROM sessions WHERE token_hash=?1", params![hash])?; return Err(AuthError::Unauthorized); }
        if status != "ACTIVE" { return Err(AuthError::AccountInactive); }
        let session_version: i64 = conn.query_row("SELECT auth_version FROM users WHERE id=?1", params![id], |r| r.get(0))?;
        if session_version != auth_version { conn.execute("DELETE FROM sessions WHERE token_hash=?1", params![hash])?; return Err(AuthError::Unauthorized); }
        conn.execute("UPDATE sessions SET last_seen_at=?1 WHERE token_hash=?2", params![now,hash])?;
        Ok(UserSummary { id, username, display_name, role, status: UserStatus::Active, created_at })
    }

    pub fn list_users(&self, token: &str) -> Result<Vec<UserSummary>, AuthError> {
        let actor = self.authenticate(token)?; self.require(&actor, "users.manage")?;
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        let mut stmt = conn.prepare("SELECT id,username,display_name,role,status,created_at FROM users ORDER BY display_name COLLATE NOCASE")?;
        let rows = stmt.query_map([], |r| Ok(UserSummary { id:r.get(0)?, username:r.get(1)?, display_name:r.get(2)?, role:r.get(3)?, status: UserStatus::try_from(r.get::<_,String>(4)?.as_str()).unwrap_or(UserStatus::Inactive), created_at:r.get(5)? }))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn create_user(&self, token: &str, username: &str, display_name: &str, role: &str, password: &str, device_id: &str) -> Result<UserSummary, AuthError> {
        let actor = self.authenticate(token)?; self.require(&actor, "users.manage")?;
        validate_username(username)?; validate_display_name(display_name)?; validate_password(password)?; validate_role(role)?; validate_device(device_id)?;
        if role == "CEO" && actor.role != "CEO" { return Err(AuthError::Unauthorized); }
        let id = Uuid::new_v4().to_string(); let now = now(); let hash = hash_password(password)?;
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        conn.execute("INSERT INTO users(id,username,display_name,role,status,password_hash,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)", params![id,normalize_username(username),display_name.trim(),role,"ACTIVE",hash,now,now])?;
        drop(conn);
        self.audit(Some(&actor.id), "USER_CREATED", Some("USER"), Some(&id), serde_json::json!({"role":role,"username":normalize_username(username)}), device_id)?;
        self.get_user(&id)
    }

    pub fn set_status(&self, token: &str, user_id: &str, status: UserStatus, device_id: &str) -> Result<(), AuthError> {
        let actor = self.authenticate(token)?; self.require(&actor, "users.manage")?; validate_device(device_id)?;
        if actor.id == user_id && status != UserStatus::Active { return Err(AuthError::Validation("CEO cannot disable the current account".into())); }
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        let changed = conn.execute("UPDATE users SET status=?1,auth_version=auth_version+1,updated_at=?2 WHERE id=?3", params![status.as_str(),now(),user_id])?;
        if changed == 0 { return Err(AuthError::NotFound); } drop(conn);
        self.audit(Some(&actor.id), "USER_STATUS_CHANGED", Some("USER"), Some(user_id), serde_json::json!({"status":status.as_str()}), device_id)?; Ok(())
    }

    pub fn change_password(&self, token: &str, user_id: &str, new_password: &str, device_id: &str) -> Result<(), AuthError> {
        let actor = self.authenticate(token)?;
        if actor.id != user_id { self.require(&actor, "users.manage")?; }
        validate_password(new_password)?; validate_device(device_id)?;
        let hash = hash_password(new_password)?; let conn = self.conn.lock().expect("auth store mutex poisoned");
        let changed = conn.execute("UPDATE users SET password_hash=?1,auth_version=auth_version+1,failed_attempts=0,locked_until=NULL,updated_at=?2 WHERE id=?3", params![hash,now(),user_id])?;
        if changed == 0 { return Err(AuthError::NotFound); } drop(conn);
        self.audit(Some(&actor.id), "PASSWORD_CHANGED", Some("USER"), Some(user_id), serde_json::json!({"self_change": actor.id == user_id}), device_id)?; Ok(())
    }

    pub fn set_permission_override(&self, token: &str, user_id: &str, permission: &str, grant: bool, device_id: &str) -> Result<(), AuthError> {
        let actor = self.authenticate(token)?; self.require(&actor, "users.manage")?; validate_permission(permission)?; validate_device(device_id)?;
        if actor.id == user_id && permission == "system.manage" && !grant { return Err(AuthError::Validation("cannot revoke CEO system authority".into())); }
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        conn.execute("INSERT INTO permission_overrides(user_id,permission,effect) VALUES(?1,?2,?3) ON CONFLICT(user_id,permission) DO UPDATE SET effect=excluded.effect", params![user_id,permission,if grant{"GRANT"}else{"REVOKE"}])?; drop(conn);
        self.audit(Some(&actor.id), "PERMISSION_OVERRIDE_CHANGED", Some("USER"), Some(user_id), serde_json::json!({"permission":permission,"effect":if grant{"GRANT"}else{"REVOKE"}}), device_id)?; Ok(())
    }

    pub fn has_permission(&self, token: &str, permission: &str) -> Result<bool, AuthError> {
        validate_permission(permission)?; let actor = self.authenticate(token)?; Ok(self.permission_for(&actor, permission))
    }

    pub fn list_audit(&self, token: &str, limit: i64) -> Result<Vec<AuditEvent>, AuthError> {
        let actor = self.authenticate(token)?; self.require(&actor, "audit.view")?;
        let limit = limit.clamp(1, 200); let conn = self.conn.lock().expect("auth store mutex poisoned");
        let mut stmt = conn.prepare("SELECT id,actor_user_id,action,target_type,target_id,details_json,device_id,created_at FROM audit_events ORDER BY created_at DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit], |r| {
            let details: String = r.get(5)?; Ok(AuditEvent { id:r.get(0)?, actor_user_id:r.get(1)?, action:r.get(2)?, target_type:r.get(3)?, target_id:r.get(4)?, details:serde_json::from_str(&details).unwrap_or(serde_json::json!({"parse_error":true})), device_id:r.get(6)?, created_at:r.get(7)? })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    fn require(&self, actor: &UserSummary, permission: &str) -> Result<(), AuthError> { if self.permission_for(actor, permission) { Ok(()) } else { Err(AuthError::Unauthorized) } }
    fn permission_for(&self, actor: &UserSummary, permission: &str) -> bool {
        if actor.role == "CEO" { return true; }
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        let override_effect: Option<String> = conn.query_row("SELECT effect FROM permission_overrides WHERE user_id=?1 AND permission=?2", params![actor.id,permission], |r| r.get(0)).optional().unwrap_or(None);
        match override_effect.as_deref() { Some("GRANT") => true, Some("REVOKE") => false, _ => role_has_permission(&actor.role, permission) }
    }

    fn get_user(&self, id: &str) -> Result<UserSummary, AuthError> {
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        conn.query_row("SELECT id,username,display_name,role,status,created_at FROM users WHERE id=?1", params![id], |r| Ok(UserSummary{id:r.get(0)?,username:r.get(1)?,display_name:r.get(2)?,role:r.get(3)?,status:UserStatus::try_from(r.get::<_,String>(4)?.as_str()).unwrap_or(UserStatus::Inactive),created_at:r.get(5)?})).optional()?.ok_or(AuthError::NotFound)
    }

    fn audit(&self, actor: Option<&str>, action: &str, target_type: Option<&str>, target_id: Option<&str>, details: serde_json::Value, device_id: &str) -> Result<(), AuthError> {
        let conn = self.conn.lock().expect("auth store mutex poisoned");
        conn.execute("INSERT INTO audit_events(id,actor_user_id,action,target_type,target_id,details_json,device_id,created_at) VALUES(?,?,?,?,?,?,?,?)", params![Uuid::new_v4().to_string(),actor,action,target_type,target_id,serde_json::to_string(&details)?,device_id,now()])?; Ok(())
    }
}

fn role_has_permission(role: &str, permission: &str) -> bool {
    match role {
        "MANAGER" => matches!(permission,"workers.view"|"orders.view"|"orders.create"|"orders.edit"|"payments.view"|"payments.receive"|"cash.open"|"cash.close"|"cash.reconcile"|"inventory.view"|"attendance.view"|"attendance.manage"|"reports.view"|"delivery.manage"),
        "WAITER" => matches!(permission,"orders.view"|"orders.create"|"orders.edit"|"payments.receive"),
        "COOK" => permission == "orders.view",
        "CASHIER" => matches!(permission,"orders.view"|"payments.view"|"payments.receive"|"payments.refund"|"cash.open"|"cash.close"|"cash.reconcile"),
        "STOREKEEPER" => matches!(permission,"inventory.view"|"inventory.issue"),
        "DELIVERY_RIDER" => matches!(permission,"orders.view"|"delivery.manage"),
        _ => false,
    }
}

fn validate_username(v: &str) -> Result<(), AuthError> {
    let s = v.trim(); if !(3..=32).contains(&s.len()) || !s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c,'.'|'_'|'-')) { return Err(AuthError::Validation("username must be 3-32 ASCII letters, numbers, '.', '_' or '-'".into())); } Ok(())
}
fn validate_display_name(v: &str) -> Result<(), AuthError> { let n=v.trim(); if !(1..=80).contains(&n.len()) { return Err(AuthError::Validation("display name must be 1-80 characters".into())); } Ok(()) }
fn validate_password(v: &str) -> Result<(), AuthError> { if !(12..=128).contains(&v.len()) || !v.chars().any(|c|c.is_ascii_uppercase()) || !v.chars().any(|c|c.is_ascii_lowercase()) || !v.chars().any(|c|c.is_ascii_digit()) { return Err(AuthError::Validation("password must be 12-128 characters and contain upper, lower and numeric characters".into())); } Ok(()) }
fn validate_device(v: &str) -> Result<(), AuthError> { if !(8..=128).contains(&v.len()) { Err(AuthError::Validation("invalid device identifier".into())) } else { Ok(()) } }
fn validate_role(v: &str) -> Result<(), AuthError> { if matches!(v,"CEO"|"MANAGER"|"WAITER"|"COOK"|"CASHIER"|"STOREKEEPER"|"DELIVERY_RIDER") { Ok(()) } else { Err(AuthError::Validation("invalid worker role".into())) } }
fn validate_permission(v: &str) -> Result<(), AuthError> { if v.len() >= 3 && v.len() <= 80 && v.chars().all(|c|c.is_ascii_alphanumeric() || matches!(c,'.'|'_'|'-')) { Ok(()) } else { Err(AuthError::Validation("invalid permission".into())) } }
fn normalize_username(v: &str) -> String { v.trim().to_ascii_lowercase() }
fn hash_password(password: &str) -> Result<String, AuthError> { let salt = SaltString::generate(&mut OsRng); Ok(Argon2::default().hash_password(password.as_bytes(), &salt)?.to_string()) }
fn verify_password(password: &str, hash: &str) -> bool { PasswordHash::new(hash).ok().and_then(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).ok()).is_some() }
fn hash_token(token: &str) -> String { let mut h=Sha256::new(); h.update(token.as_bytes()); format!("{:x}",h.finalize()) }
fn now() -> i64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64 }

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> AuthStore { AuthStore::open_in_memory().unwrap() }
    #[test] fn bootstrap_and_login_work() { let s=setup(); let u=s.bootstrap_ceo("ceo","MasterChef CEO","SecurePassword123","device-0001").unwrap(); assert_eq!(u.role,"CEO"); let session=s.login("CEO","SecurePassword123","device-0001").unwrap(); assert_eq!(session.user.id,u.id); assert!(s.has_permission(&session.token,"anything").unwrap()); }
    #[test] fn weak_password_rejected() { let s=setup(); assert!(s.bootstrap_ceo("ceo","CEO","weak","device-0001").is_err()); }
    #[test] fn lockout_after_repeated_failures() { let s=setup(); s.bootstrap_ceo("ceo","CEO","SecurePassword123","device-0001").unwrap(); for _ in 0..4 { assert!(matches!(s.login("ceo","WrongPassword123","device-0001"),Err(AuthError::InvalidCredentials))); } assert!(matches!(s.login("ceo","WrongPassword123","device-0001"),Err(AuthError::AccountLocked))); }
    #[test] fn password_change_invalidates_old_session() { let s=setup(); s.bootstrap_ceo("ceo","CEO","SecurePassword123","device-0001").unwrap(); let session=s.login("ceo","SecurePassword123","device-0001").unwrap(); s.change_password(&session.token,&session.user.id,"NewPassword123","device-0001").unwrap(); assert!(matches!(s.authenticate(&session.token),Err(AuthError::Unauthorized))); assert!(s.login("ceo","NewPassword123","device-0001").is_ok()); }
    #[test] fn non_ceo_cannot_escalate_to_ceo() { let s=setup(); s.bootstrap_ceo("ceo","CEO","SecurePassword123","device-0001").unwrap(); let ceo=s.login("ceo","SecurePassword123","device-0001").unwrap(); let m=s.create_user(&ceo.token,"manager","Manager","MANAGER","ManagerPassword123","device-0001").unwrap(); let ms=s.login("manager","ManagerPassword123","device-0001").unwrap(); assert!(s.create_user(&ms.token,"other","Other","CEO","OtherPassword123","device-0001").is_err()); assert_eq!(m.role,"MANAGER"); }
}
