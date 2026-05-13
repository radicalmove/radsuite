use radsuite_desktop::{
    AddManualCitationRequest, AnalyseDocxRequest, AnalyseDocxResponse, AnalyseDocxReviewResponse,
    AppStatus, DesktopState, UpdateParagraphReviewRequest,
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

#[tauri::command]
async fn mark_radcite_paragraph_resolved(
    state: tauri::State<'_, DesktopState>,
    request: UpdateParagraphReviewRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::mark_paragraph_resolved_for_review(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn verify_radcite_paragraph_citations(
    state: tauri::State<'_, DesktopState>,
    request: UpdateParagraphReviewRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::verify_paragraph_citations_for_review(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn add_radcite_manual_citation(
    state: tauri::State<'_, DesktopState>,
    request: AddManualCitationRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::add_manual_citation_for_review(&state, request)
        .await
        .map_err(|error| error.to_string())
}

fn main() {
    let state = tauri::async_runtime::block_on(DesktopState::for_app("RADsuite"))
        .expect("initialize RADsuite desktop state");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            analyse_docx_path,
            analyse_docx_for_review,
            mark_radcite_paragraph_resolved,
            verify_radcite_paragraph_citations,
            add_radcite_manual_citation
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RADsuite desktop app");
}
