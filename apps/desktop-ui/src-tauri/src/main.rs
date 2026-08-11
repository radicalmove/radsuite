use std::path::Path;

use radsuite_desktop::{
    AddCourseReferenceRequest, AddManualCitationRequest, AddModuleReadingRequest,
    AddRadciteModuleRequest, AnalyseDocxRequest, AnalyseDocxResponse, AnalyseDocxReviewResponse,
    AnalysePdfRequest, AppStatus, ArchiveCourseReferenceRequest, ArchiveModuleReadingRequest,
    ArchiveRadciteDocumentRequest, ArchiveRadciteModuleRequest, ArchiveRadciteProjectRequest,
    CourseModuleSummary, CourseReferenceSummary, CourseReferencesExport,
    CreateRadciteProjectRequest, DeleteRadcastAudioRequest, DesktopState,
    ExportCourseReferencesRequest, ExportModuleReadingsRequest, ExportRadciteReviewReportRequest,
    ImportDocumentReadingsRequest, ImportDocumentReadingsResponse, ImportRadcastAudioLinkRequest,
    ImportRadcastAudioRequest, LinkCitationReferenceRequest, ListCourseReferencesRequest,
    ListModuleReadingsRequest, ListRadcastAudioRequest, ListRadciteArchiveRequest,
    ListRadciteModulesRequest, ListRadtTsOutputsRequest, ListSavedReviewsRequest,
    LoadSavedReviewRequest, MergeCourseReferencesRequest, ModuleReadingImportCandidateSummary,
    ModuleReadingSummary, ModuleReadingsExport, ModuleReadingsPdfImportPreview,
    PreviewModuleReadingsCsvImportRequest, PreviewModuleReadingsImportRequest,
    PreviewModuleReadingsPdfImportRequest, ProcessRadcastAudioRequest, RadcastAudioListing,
    RadcastAudioOutput, RadcastAudioSource, RadcastCapabilityStatus, RadcastJobStatus,
    RadciteArchiveItem, RadciteProjectSummary, RadciteReviewReportExport, RadtTsCapabilityStatus,
    RadtTsJobStatus, RadtTsMediaJobStatus, RadtTsMediaOutputListing, RadtTsOutputListing,
    RestoreRadciteArchiveItemRequest, RestoreRadciteProjectRequest,
    SaveModuleReadingsImportRequest, SaveRadcastSettingsRequest, SavedRadciteReviewSummary,
    StartRadtTsClipRequest, StartRadtTsSynthesisRequest, StartRadtTsTranscriptionRequest,
    UpdateCourseReferenceRequest, UpdateModuleReadingRequest, UpdateParagraphReviewRequest,
    UpdateRadciteDocumentRequest, UpdateRadciteModuleRequest, UpdateRadciteProjectRequest,
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
async fn analyse_pdf_for_review(
    state: tauri::State<'_, DesktopState>,
    request: AnalysePdfRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::analyse_pdf_for_review(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_radcite_review_report(
    state: tauri::State<'_, DesktopState>,
    request: ExportRadciteReviewReportRequest,
) -> Result<RadciteReviewReportExport, String> {
    radsuite_desktop::export_radcite_review_report(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_radcast_audio(
    state: tauri::State<'_, DesktopState>,
    request: ListRadcastAudioRequest,
) -> Result<RadcastAudioListing, String> {
    radsuite_desktop::list_radcast_audio(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_radcast_audio(
    state: tauri::State<'_, DesktopState>,
    request: DeleteRadcastAudioRequest,
) -> Result<(), String> {
    radsuite_desktop::delete_radcast_audio(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_local_file(source_path: String, destination_path: String) -> Result<(), String> {
    radsuite_desktop::copy_local_file(Path::new(&source_path), Path::new(&destination_path))
        .map_err(|error| format!("could not save file: {error}"))
}

#[tauri::command]
fn write_local_text_file(destination_path: String, contents: String) -> Result<(), String> {
    radsuite_desktop::write_local_text_file(Path::new(&destination_path), &contents)
        .map_err(|error| format!("could not save text file: {error}"))
}

#[tauri::command]
async fn save_radcast_settings(
    state: tauri::State<'_, DesktopState>,
    request: SaveRadcastSettingsRequest,
) -> Result<radsuite_desktop::RadcastProjectSettings, String> {
    radsuite_desktop::save_radcast_settings(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn import_radcast_audio(
    state: tauri::State<'_, DesktopState>,
    request: ImportRadcastAudioRequest,
) -> Result<RadcastAudioSource, String> {
    radsuite_desktop::import_radcast_audio(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn import_radcast_audio_from_link(
    state: tauri::State<'_, DesktopState>,
    request: ImportRadcastAudioLinkRequest,
) -> Result<RadcastAudioSource, String> {
    radsuite_desktop::import_radcast_audio_from_link(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn process_radcast_audio(
    state: tauri::State<'_, DesktopState>,
    request: ProcessRadcastAudioRequest,
) -> Result<RadcastAudioOutput, String> {
    radsuite_desktop::process_radcast_audio(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_radcast_audio(
    state: tauri::State<'_, DesktopState>,
    request: ProcessRadcastAudioRequest,
) -> Result<RadcastJobStatus, String> {
    radsuite_desktop::start_radcast_audio(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cancel_radcast_audio(
    state: tauri::State<'_, DesktopState>,
    job_id: String,
) -> Result<RadcastJobStatus, String> {
    radsuite_desktop::cancel_radcast_audio(&state, job_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_radcast_audio_job(
    state: tauri::State<'_, DesktopState>,
    job_id: String,
) -> Result<RadcastJobStatus, String> {
    radsuite_desktop::get_radcast_audio_job(&state, job_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_radcast_capabilities() -> RadcastCapabilityStatus {
    radsuite_desktop::get_radcast_capabilities()
}

#[tauri::command]
fn get_radt_ts_capabilities() -> RadtTsCapabilityStatus {
    radsuite_desktop::get_radt_ts_capabilities()
}

#[tauri::command]
async fn list_radt_ts_outputs(
    state: tauri::State<'_, DesktopState>,
    request: ListRadtTsOutputsRequest,
) -> Result<RadtTsOutputListing, String> {
    radsuite_desktop::list_radt_ts_outputs(&state, request).await
}

#[tauri::command]
async fn start_radt_ts_synthesis(
    state: tauri::State<'_, DesktopState>,
    request: StartRadtTsSynthesisRequest,
) -> Result<RadtTsJobStatus, String> {
    radsuite_desktop::start_radt_ts_synthesis(&state, request).await
}

#[tauri::command]
fn get_radt_ts_job(
    state: tauri::State<'_, DesktopState>,
    job_id: String,
) -> Result<RadtTsJobStatus, String> {
    radsuite_desktop::get_radt_ts_job(&state, job_id)
}

#[tauri::command]
fn cancel_radt_ts_job(
    state: tauri::State<'_, DesktopState>,
    job_id: String,
) -> Result<RadtTsJobStatus, String> {
    radsuite_desktop::cancel_radt_ts_job(&state, job_id)
}

#[tauri::command]
async fn list_radt_ts_media_outputs(
    state: tauri::State<'_, DesktopState>,
    request: ListRadtTsOutputsRequest,
) -> Result<RadtTsMediaOutputListing, String> {
    radsuite_desktop::list_radt_ts_media_outputs(&state, request).await
}

#[tauri::command]
async fn start_radt_ts_transcription(
    state: tauri::State<'_, DesktopState>,
    request: StartRadtTsTranscriptionRequest,
) -> Result<RadtTsMediaJobStatus, String> {
    radsuite_desktop::start_radt_ts_transcription(&state, request).await
}

#[tauri::command]
async fn start_radt_ts_clip(
    state: tauri::State<'_, DesktopState>,
    request: StartRadtTsClipRequest,
) -> Result<RadtTsMediaJobStatus, String> {
    radsuite_desktop::start_radt_ts_clip(&state, request).await
}

#[tauri::command]
fn get_radt_ts_media_job(
    state: tauri::State<'_, DesktopState>,
    job_id: String,
) -> Result<RadtTsMediaJobStatus, String> {
    radsuite_desktop::get_radt_ts_media_job(&state, job_id)
}

#[tauri::command]
fn cancel_radt_ts_media_job(
    state: tauri::State<'_, DesktopState>,
    job_id: String,
) -> Result<RadtTsMediaJobStatus, String> {
    radsuite_desktop::cancel_radt_ts_media_job(&state, job_id)
}

#[tauri::command]
async fn list_radcite_projects(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<RadciteProjectSummary>, String> {
    radsuite_desktop::list_radcite_projects(&state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_radcite_project(
    state: tauri::State<'_, DesktopState>,
    request: CreateRadciteProjectRequest,
) -> Result<RadciteProjectSummary, String> {
    radsuite_desktop::create_radcite_project(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_radcite_project(
    state: tauri::State<'_, DesktopState>,
    request: UpdateRadciteProjectRequest,
) -> Result<RadciteProjectSummary, String> {
    radsuite_desktop::update_radcite_project(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn archive_radcite_project(
    state: tauri::State<'_, DesktopState>,
    request: ArchiveRadciteProjectRequest,
) -> Result<RadciteProjectSummary, String> {
    radsuite_desktop::archive_radcite_project(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn restore_radcite_project(
    state: tauri::State<'_, DesktopState>,
    request: RestoreRadciteProjectRequest,
) -> Result<RadciteProjectSummary, String> {
    radsuite_desktop::restore_radcite_project(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_saved_radcite_reviews(
    state: tauri::State<'_, DesktopState>,
    request: Option<ListSavedReviewsRequest>,
) -> Result<Vec<SavedRadciteReviewSummary>, String> {
    radsuite_desktop::list_saved_radcite_reviews(&state, request.unwrap_or_default())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn archive_radcite_document(
    state: tauri::State<'_, DesktopState>,
    request: ArchiveRadciteDocumentRequest,
) -> Result<SavedRadciteReviewSummary, String> {
    radsuite_desktop::archive_radcite_document(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn update_radcite_document(
    state: tauri::State<'_, DesktopState>,
    request: UpdateRadciteDocumentRequest,
) -> Result<SavedRadciteReviewSummary, String> {
    radsuite_desktop::update_radcite_document(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_radcite_archive(
    state: tauri::State<'_, DesktopState>,
    request: Option<ListRadciteArchiveRequest>,
) -> Result<Vec<RadciteArchiveItem>, String> {
    radsuite_desktop::list_radcite_archive(&state, request.unwrap_or_default())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn restore_radcite_archive_item(
    state: tauri::State<'_, DesktopState>,
    request: RestoreRadciteArchiveItemRequest,
) -> Result<Vec<RadciteArchiveItem>, String> {
    radsuite_desktop::restore_radcite_archive_item(&state, request)
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
    request: Option<ListCourseReferencesRequest>,
) -> Result<Vec<CourseReferenceSummary>, String> {
    radsuite_desktop::list_course_references(&state, request.unwrap_or_default())
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
async fn update_course_reference(
    state: tauri::State<'_, DesktopState>,
    request: UpdateCourseReferenceRequest,
) -> Result<CourseReferenceSummary, String> {
    radsuite_desktop::update_course_reference(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn archive_course_reference(
    state: tauri::State<'_, DesktopState>,
    request: ArchiveCourseReferenceRequest,
) -> Result<CourseReferenceSummary, String> {
    radsuite_desktop::archive_course_reference(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn merge_course_references(
    state: tauri::State<'_, DesktopState>,
    request: MergeCourseReferencesRequest,
) -> Result<CourseReferenceSummary, String> {
    radsuite_desktop::merge_course_references(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_radcite_modules(
    state: tauri::State<'_, DesktopState>,
    request: Option<ListRadciteModulesRequest>,
) -> Result<Vec<CourseModuleSummary>, String> {
    radsuite_desktop::list_radcite_modules(&state, request.unwrap_or_default())
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
async fn preview_module_readings_csv_import(
    state: tauri::State<'_, DesktopState>,
    request: PreviewModuleReadingsCsvImportRequest,
) -> Result<Vec<ModuleReadingImportCandidateSummary>, String> {
    radsuite_desktop::preview_module_readings_csv_import(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn preview_module_readings_pdf_import(
    state: tauri::State<'_, DesktopState>,
    request: PreviewModuleReadingsPdfImportRequest,
) -> Result<ModuleReadingsPdfImportPreview, String> {
    radsuite_desktop::preview_module_readings_pdf_import(&state, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn import_document_readings(
    state: tauri::State<'_, DesktopState>,
    request: ImportDocumentReadingsRequest,
) -> Result<ImportDocumentReadingsResponse, String> {
    radsuite_desktop::import_document_readings(&state, request)
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
    let shutdown_state = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            analyse_docx_path,
            analyse_docx_for_review,
            analyse_pdf_for_review,
            export_radcite_review_report,
            list_radcast_audio,
            delete_radcast_audio,
            save_local_file,
            write_local_text_file,
            save_radcast_settings,
            import_radcast_audio,
            import_radcast_audio_from_link,
            process_radcast_audio,
            start_radcast_audio,
            cancel_radcast_audio,
            get_radcast_audio_job,
            get_radcast_capabilities,
            get_radt_ts_capabilities,
            list_radt_ts_outputs,
            start_radt_ts_synthesis,
            get_radt_ts_job,
            cancel_radt_ts_job,
            list_radt_ts_media_outputs,
            start_radt_ts_transcription,
            start_radt_ts_clip,
            get_radt_ts_media_job,
            cancel_radt_ts_media_job,
            list_radcite_projects,
            create_radcite_project,
            update_radcite_project,
            archive_radcite_project,
            restore_radcite_project,
            list_saved_radcite_reviews,
            archive_radcite_document,
            update_radcite_document,
            list_radcite_archive,
            restore_radcite_archive_item,
            load_saved_radcite_review,
            list_course_references,
            add_course_reference,
            update_course_reference,
            archive_course_reference,
            merge_course_references,
            list_radcite_modules,
            add_radcite_module,
            update_radcite_module,
            archive_radcite_module,
            list_module_readings,
            add_module_reading,
            update_module_reading,
            archive_module_reading,
            preview_module_readings_import,
            preview_module_readings_csv_import,
            preview_module_readings_pdf_import,
            import_document_readings,
            save_module_readings_import,
            export_course_references,
            export_module_readings,
            mark_radcite_paragraph_resolved,
            verify_radcite_paragraph_citations,
            add_radcite_manual_citation,
            link_radcite_citation_reference
        ])
        .build(tauri::generate_context!())
        .expect("failed to build RADsuite desktop app")
        .run(move |_app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                radsuite_desktop::shutdown_radt_ts_jobs(&shutdown_state);
                radsuite_desktop::shutdown_radt_ts_media_jobs(&shutdown_state);
            }
        });
}
