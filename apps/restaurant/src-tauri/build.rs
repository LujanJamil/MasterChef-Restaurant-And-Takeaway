fn main() {
    const COMMANDS: &[&str] = ["bootstrap_ceo", "login", "logout", "current_user"].as_slice();
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to generate Tauri command permissions");
}
