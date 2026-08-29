use masterchef_native_core::{AuthSession, AuthStore, UserSummary};
use std::{fs, path::PathBuf};
use tauri::{Manager, State};

struct AppState { auth: AuthStore, device_id: String }
fn err<E: std::fmt::Display>(e:E)->String{e.to_string()}
fn setup_paths(app:&tauri::AppHandle)->Result<(PathBuf,String),String>{
    let dir=app.path().app_data_dir().map_err(err)?; fs::create_dir_all(&dir).map_err(err)?;
    let db=dir.join("masterchef-restaurant-security.sqlite3"); let id_file=dir.join("device.id");
    let device=match fs::read_to_string(&id_file){Ok(v) if v.trim().len()>=8=>v.trim().to_string(),_=>{let v=uuid::Uuid::new_v4().to_string();fs::write(&id_file,&v).map_err(err)?;v}};
    Ok((db,device))
}
#[tauri::command]
fn bootstrap_ceo(state:State<'_,AppState>,username:String,display_name:String,password:String)->Result<UserSummary,String>{state.auth.bootstrap_ceo(&username,&display_name,&password,&state.device_id).map_err(err)}
#[tauri::command]
fn login(state:State<'_,AppState>,username:String,password:String)->Result<AuthSession,String>{state.auth.login(&username,&password,&state.device_id).map_err(err)}
#[tauri::command]
fn logout(state:State<'_,AppState>,token:String)->Result<(),String>{state.auth.logout(&token,&state.device_id).map_err(err)}
#[tauri::command]
fn current_user(state:State<'_,AppState>,token:String)->Result<UserSummary,String>{state.auth.authenticate(&token).map_err(err)}

fn main(){
    tauri::Builder::default()
      .setup(|app|{let (db,device)=setup_paths(app.handle()).map_err(std::io::Error::other)?;let auth=AuthStore::open(db).map_err(|e|std::io::Error::other(e.to_string()))?;app.manage(AppState{auth,device_id:device});Ok(())})
      .invoke_handler(tauri::generate_handler![bootstrap_ceo,login,logout,current_user])
      .run(tauri::generate_context!())
      .expect("error while running MasterChef Restaurant application");
}
