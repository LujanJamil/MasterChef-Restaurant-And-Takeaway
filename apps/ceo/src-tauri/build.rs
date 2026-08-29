fn main() {
    const COMMANDS: &[&str] = &[
        "bootstrap_ceo", "login", "logout", "current_user", "list_users",
        "create_user", "set_user_status", "change_password",
        "set_permission_override", "has_permission", "audit_events",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    ).expect("failed to generate Tauri command permissions");
}
