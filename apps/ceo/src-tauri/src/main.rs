use masterchef_native_core::{AuthSession, AuthStore, AuditEvent, UserStatus, UserSummary};
use std::{fs, path::PathBuf};
use tauri::{Manager, State};

struct AppState { auth: AuthStore, device_id: String }
fn err<E: std::fmt::Display>(e: E) -> String { e.to_string() }
fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> { let dir=app.path().app_data_dir().map_err(err)?; fs::create_dir_all(&dir).map_err(err)?; Ok(dir.join("masterchef-security.sqlite3")) }
fn device_id(app: &tauri::AppHandle) -> Result<String, String> {
    let dir=app.path().app_data_dir().map_err(err)?; fs::create_dir_all(&dir).map_err(err)?; let path=dir.join("device.id");
    if let Ok(value)=fs::read_to_string(&path) { let value=value.trim().to_string(); if value.len()>=8 { return Ok(value); } }
    let value=uuid::Uuid::new_v4().to_string(); fs::write(path,&value).map_err(err)?; Ok(value)
}

#[tauri::command]
fn bootstrap_ceo(state: State<'_, AppState>, username: String, display_name: String, password: String) -> Result<UserSummary,String> { state.auth.bootstrap_ceo(&username,&display_name,&password,&state.device_id).map_err(err) }
#[tauri::command]
fn login(state: State<'_, AppState>, username: String, password: String) -> Result<AuthSession,String> { state.auth.login(&username,&password,&state.device_id).map_err(err) }
#[tauri::command]
fn logout(state: State<'_, AppState>, token: String) -> Result<(),String> { state.auth.logout(&token,&state.device_id).map_err(err) }
#[tauri::command]
fn current_user(state: State<'_, AppState>, token: String) -> Result<UserSummary,String> { state.auth.authenticate(&token).map_err(err) }
#[tauri::command]
fn list_users(state: State<'_, AppState>, token: String) -> Result<Vec<UserSummary>,String> { state.auth.list_users(&token).map_err(err) }
#[tauri::command]
fn create_user(state: State<'_, AppState>, token: String, username: String, display_name: String, role: String, password: String) -> Result<UserSummary,String> { state.auth.create_user(&token,&username,&display_name,&role,&password,&state.device_id).map_err(err) }
#[tauri::command]
fn set_user_status(state: State<'_, AppState>, token: String, user_id: String, status: UserStatus) -> Result<(),String> { state.auth.set_status(&token,&user_id,status,&state.device_id).map_err(err) }
#[tauri::command]
fn change_password(state: State<'_, AppState>, token: String, user_id: String, new_password: String) -> Result<(),String> { state.auth.change_password(&token,&user_id,&new_password,&state.device_id).map_err(err) }
#[tauri::command]
fn set_permission_override(state: State<'_, AppState>, token: String, user_id: String, permission: String, grant: bool) -> Result<(),String> { state.auth.set_permission_override(&token,&user_id,&permission,grant,&state.device_id).map_err(err) }
#[tauri::command]
fn has_permission(state: State<'_, AppState>, token: String, permission: String) -> Result<bool,String> { state.auth.has_permission(&token,&permission).map_err(err) }
#[tauri::command]
fn audit_events(state: State<'_, AppState>, token: String, limit: Option<i64>) -> Result<Vec<AuditEvent>,String> { state.auth.list_audit(&token,limit.unwrap_or(50)).map_err(err) }

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let path=db_path(app.handle()).map_err(std::io::Error::other)?;
            let auth=AuthStore::open(path).map_err(|e| std::io::Error::other(e.to_string()))?;
            let device=device_id(app.handle()).map_err(std::io::Error::other)?;
            app.manage(AppState{auth,device_id:device}); Ok(())
        })
        .invoke_handler(tauri::generate_handler![bootstrap_ceo,login,logout,current_user,list_users,create_user,set_user_status,change_password,set_permission_override,has_permission,audit_events])
        .run(tauri::generate_context!())
        .expect("error while running MasterChef CEO application");
}
