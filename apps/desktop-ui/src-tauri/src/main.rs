use radsuite_desktop::{AppStatus, DesktopState};

#[tauri::command]
fn get_app_status(state: tauri::State<'_, DesktopState>) -> AppStatus {
    radsuite_desktop::get_app_status(&state)
}

fn main() {
    tauri::Builder::default()
        .manage(DesktopState::for_tests())
        .invoke_handler(tauri::generate_handler![get_app_status])
        .run(tauri::generate_context!())
        .expect("failed to run RADsuite desktop app");
}
