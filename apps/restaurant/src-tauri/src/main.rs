use masterchef_native_core::{AuthSession, AuthStore, UserSummary};
use masterchef_sync_core::{SyncOperation, SyncStore};
use serde_json::Value;
use std::{fs, path::PathBuf};
use tauri::{Manager, State};

struct AppState { auth: AuthStore, sync: SyncStore, device_id: String }
fn err<E: std::fmt::Display>(e:E)->String{e.to_string()}
fn setup_paths(app:&tauri::AppHandle)->Result<(PathBuf,PathBuf,String),String>{
    let dir=app.path().app_data_dir().map_err(err)?; fs::create_dir_all(&dir).map_err(err)?;
    let db=dir.join("masterchef-restaurant-security.sqlite3");
    let sync_db=dir.join("masterchef-restaurant-sync.sqlite3");
    let id_file=dir.join("device.id");
    let device=match fs::read_to_string(&id_file){Ok(v) if v.trim().len()>=8=>v.trim().to_string(),_=>{let v=uuid::Uuid::new_v4().to_string();fs::write(&id_file,&v).map_err(err)?;v}};
    Ok((db,sync_db,device))
}
#[tauri::command]
fn bootstrap_ceo(state:State<'_,AppState>,username:String,display_name:String,password:String)->Result<UserSummary,String>{state.auth.bootstrap_ceo(&username,&display_name,&password,&state.device_id).map_err(err)}
#[tauri::command]
fn login(state:State<'_,AppState>,username:String,password:String)->Result<AuthSession,String>{state.auth.login(&username,&password,&state.device_id).map_err(err)}
#[tauri::command]
fn logout(state:State<'_,AppState>,token:String)->Result<(),String>{state.auth.logout(&token,&state.device_id).map_err(err)}
#[tauri::command]
fn current_user(state:State<'_,AppState>,token:String)->Result<UserSummary,String>{state.auth.authenticate(&token).map_err(err)}
#[tauri::command]
fn sync_enqueue(state:State<'_,AppState>,token:String,entity_type:String,entity_id:String,operation:String,payload:Value)->Result<SyncOperation,String>{state.auth.authenticate(&token).map_err(err)?;state.sync.enqueue(&state.device_id,&entity_type,&entity_id,&operation,&payload).map_err(err)}
#[tauri::command]
fn sync_due(state:State<'_,AppState>,token:String,limit:usize)->Result<Vec<SyncOperation>,String>{state.auth.authenticate(&token).map_err(err)?;state.sync.due_batch(now(),limit).map_err(err)}
#[tauri::command]
fn sync_claim(state:State<'_,AppState>,token:String,ids:Vec<String>)->Result<usize,String>{state.auth.authenticate(&token).map_err(err)?;state.sync.claim(&ids).map_err(err)}
#[tauri::command]
fn sync_ack(state:State<'_,AppState>,token:String,id:String)->Result<(),String>{state.auth.authenticate(&token).map_err(err)?;state.sync.ack(&id).map_err(err)}
#[tauri::command]
fn sync_fail(state:State<'_,AppState>,token:String,id:String,error:String)->Result<(),String>{state.auth.authenticate(&token).map_err(err)?;state.sync.fail(&id,&error).map_err(err)}
#[tauri::command]
fn sync_pending_count(state:State<'_,AppState>,token:String)->Result<i64,String>{state.auth.authenticate(&token).map_err(err)?;state.sync.pending_count().map_err(err)}
fn now()->i64{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64}

fn main(){
    tauri::Builder::default()
      .setup(|app|{let (db,sync_db,device)=setup_paths(app.handle()).map_err(std::io::Error::other)?;let auth=AuthStore::open(db).map_err(|e|std::io::Error::other(e.to_string()))?;let sync=SyncStore::open(sync_db).map_err(|e|std::io::Error::other(e.to_string()))?;app.manage(AppState{auth,sync,device_id:device});Ok(())})
      .invoke_handler(tauri::generate_handler![bootstrap_ceo,login,logout,current_user,sync_enqueue,sync_due,sync_claim,sync_ack,sync_fail,sync_pending_count])
      .run(tauri::generate_context!())
      .expect("error while running MasterChef Restaurant application");
}
