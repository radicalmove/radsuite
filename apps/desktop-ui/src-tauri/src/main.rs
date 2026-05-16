use radsuite_desktop::{
    AddCourseReferenceRequest, AddManualCitationRequest, AddModuleReadingRequest,
    AddRadciteModuleRequest, AnalyseDocxRequest, AnalyseDocxResponse, AnalyseDocxReviewResponse,
    AppStatus, ArchiveModuleReadingRequest, ArchiveRadciteModuleRequest, CourseModuleSummary,
    CourseReferenceSummary, CourseReferencesExport, DesktopState, ExportCourseReferencesRequest,
    ExportModuleReadingsRequest, LinkCitationReferenceRequest, ListModuleReadingsRequest,
    LoadSavedReviewRequest, ModuleReadingImportCandidateSummary, ModuleReadingSummary,
    ModuleReadingsExport, PreviewModuleReadingsImportRequest, SaveModuleReadingsImportRequest,
    SavedRadciteReviewSummary, UpdateModuleReadingRequest, UpdateParagraphReviewRequest,
    UpdateRadciteModuleRequest,
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
async fn list_saved_radcite_reviews(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<SavedRadciteReviewSummary>, String> {
    radsuite_desktop::list_saved_radcite_reviews(&state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn load_saved_radcite_review(
    state: tauri::State<'_, DesktopState>,
    request: LoadSavedReviewRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::load_saved_radcite_review(&state, request.document_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_course_references(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<CourseReferenceSummary>, String> {
    radsuite_desktop::list_course_references(&state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn add_course_reference(
    state: tauri::State<'_, DesktopState>,
    request: AddCourseReferenceRequest,
) -> Result<CourseReferenceSummary, String> {
    radsuite_desktop::add_course_reference(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_radcite_modules(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<CourseModuleSummary>, String> {
    radsuite_desktop::list_radcite_modules(&state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn add_radcite_module(
    state: tauri::State<'_, DesktopState>,
    request: AddRadciteModuleRequest,
) -> Result<CourseModuleSummary, String> {
    radsuite_desktop::add_radcite_module(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_radcite_module(
    state: tauri::State<'_, DesktopState>,
    request: UpdateRadciteModuleRequest,
) -> Result<CourseModuleSummary, String> {
    radsuite_desktop::update_radcite_module(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn archive_radcite_module(
    state: tauri::State<'_, DesktopState>,
    request: ArchiveRadciteModuleRequest,
) -> Result<CourseModuleSummary, String> {
    radsuite_desktop::archive_radcite_module(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_module_readings(
    state: tauri::State<'_, DesktopState>,
    request: ListModuleReadingsRequest,
) -> Result<Vec<ModuleReadingSummary>, String> {
    radsuite_desktop::list_module_readings(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn add_module_reading(
    state: tauri::State<'_, DesktopState>,
    request: AddModuleReadingRequest,
) -> Result<ModuleReadingSummary, String> {
    radsuite_desktop::add_module_reading(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_module_reading(
    state: tauri::State<'_, DesktopState>,
    request: UpdateModuleReadingRequest,
) -> Result<ModuleReadingSummary, String> {
    radsuite_desktop::update_module_reading(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn archive_module_reading(
    state: tauri::State<'_, DesktopState>,
    request: ArchiveModuleReadingRequest,
) -> Result<ModuleReadingSummary, String> {
    radsuite_desktop::archive_module_reading(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn preview_module_readings_import(
    state: tauri::State<'_, DesktopState>,
    request: PreviewModuleReadingsImportRequest,
) -> Result<Vec<ModuleReadingImportCandidateSummary>, String> {
    radsuite_desktop::preview_module_readings_import(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn save_module_readings_import(
    state: tauri::State<'_, DesktopState>,
    request: SaveModuleReadingsImportRequest,
) -> Result<Vec<ModuleReadingSummary>, String> {
    radsuite_desktop::save_module_readings_import(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_course_references(
    state: tauri::State<'_, DesktopState>,
    request: ExportCourseReferencesRequest,
) -> Result<CourseReferencesExport, String> {
    radsuite_desktop::export_course_references(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_module_readings(
    state: tauri::State<'_, DesktopState>,
    request: ExportModuleReadingsRequest,
) -> Result<ModuleReadingsExport, String> {
    radsuite_desktop::export_module_readings(&state, request)
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

#[tauri::command]
async fn link_radcite_citation_reference(
    state: tauri::State<'_, DesktopState>,
    request: LinkCitationReferenceRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::link_citation_to_reference_for_review(&state, request)
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
            list_saved_radcite_reviews,
            load_saved_radcite_review,
            list_course_references,
            add_course_reference,
            list_radcite_modules,
            add_radcite_module,
            update_radcite_module,
            archive_radcite_module,
            list_module_readings,
            add_module_reading,
            update_module_reading,
            archive_module_reading,
            preview_module_readings_import,
            save_module_readings_import,
            export_course_references,
            export_module_readings,
            mark_radcite_paragraph_resolved,
            verify_radcite_paragraph_citations,
            add_radcite_manual_citation,
            link_radcite_citation_reference
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RADsuite desktop app");
}
