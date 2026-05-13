use radsuite_desktop::{
    AnalyseDocxRequest, AnalyseDocxResponse, AnalyseDocxReviewResponse, AppStatus, DesktopState,
};

#[tauri::command]
fn get_app_status(state: tauri::State<'_, DesktopState>) -> AppStatus {
    radsuite_desktop::get_app_status(&state)
}

#[tauri::command]
async fn analyse_docx_path(
    state: tauri::State<'_, DesktopState>,
    request: AnalyseDocxRequest,
) -> Result<AnalyseDocxResponse, String> {
    radsuite_desktop::analyse_docx_path(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn analyse_docx_for_review(
    state: tauri::State<'_, DesktopState>,
    request: AnalyseDocxRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::analyse_docx_for_review(&state, request)
        .await
        .map_err(|error| error.to_string())
}

fn main() {
    let state = tauri::async_runtime::block_on(DesktopState::for_app("RADsuite"))
        .expect("initialize RADsuite desktop state");

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            analyse_docx_path,
            analyse_docx_for_review
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RADsuite desktop app");
}
