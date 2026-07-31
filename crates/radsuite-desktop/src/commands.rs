use std::{cmp::Ordering, collections::HashSet, path::PathBuf, time::Instant};

use chrono::Utc;
use radsuite_cite::{
    CsvReadingExtractionRequest, CsvReadingImportError, DocxIngestionError, DocxIngestionRequest,
    DocxReadingExtractionRequest, PdfIngestionError, PdfIngestionRequest,
    PdfReadingExtractionError, PdfReadingExtractionRequest, ReadingImportCandidate,
    extract_csv_reading_candidates, extract_docx_reading_candidates,
    extract_pdf_reading_candidates_with_report, ingest_docx, ingest_pdf,
};
use radsuite_core::{
    ApaValidationStatus, Citation, CitationId, CourseModule, Document, DocumentFileType,
    DocumentId, DocumentVariant, ModuleId, Paragraph, ParagraphId, Project, ProjectId,
    ReadingCategory, ReferenceEntry, ReferenceEntryId, ReferenceEntryType, UserId,
};
use radsuite_db::{
    CitationDocumentRepository, CourseModuleRepository, DbError, ProjectRepository,
    ReferenceEntryRepository, SqliteCitationDocumentRepository, SqliteCourseModuleRepository,
    SqliteProjectRepository, SqliteReferenceEntryRepository,
};
use radsuite_engines::EngineStatus;
use radsuite_engines::{AudioProcessor, CaptionProcessor, EnhancementProcessor};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    DesktopState,
    document_store::{DocumentStorageError, store_source, validate_source},
    library_links::build_uc_library_link,
};

pub use crate::radcast::{
    DeleteRadcastAudioRequest, ImportRadcastAudioRequest, ListRadcastAudioRequest,
    ProcessRadcastAudioRequest, RadcastAudioListing, RadcastAudioOutput, RadcastAudioSource,
    RadcastProcessingPhase, RadcastProjectSettings, RadcastStorageError,
};
pub use crate::radt_ts::{
    ListRadtTsOutputsRequest, RadtTsCapabilityStatus, RadtTsJobStatus, RadtTsOutputListing,
    StartRadtTsSynthesisRequest,
};
pub use crate::radt_ts_tools::{
    RadtTsMediaJobStatus, RadtTsMediaOutputListing, StartRadtTsClipRequest,
    StartRadtTsTranscriptionRequest,
};
pub use radsuite_engines::{AudioOutputFormat, CaptionFormat};

const LOCAL_RADCITE_PROJECT_CODE: &str = "CRJU150";
const LOCAL_RADCITE_PROJECT_TITLE: &str = "RADcite Functional Testing";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    pub app_name: String,
    pub database_ready: bool,
    pub sync_configured: bool,
    pub engines: Vec<EngineStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadcastCapabilityStatus {
    pub caption_available: bool,
    pub caption_detail: String,
    pub optimized_available: bool,
    pub optimized_detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveRadcastSettingsRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub settings: RadcastProjectSettings,
}

pub fn get_radcast_capabilities() -> RadcastCapabilityStatus {
    get_radcast_capabilities_with_processors(
        CaptionProcessor::default(),
        EnhancementProcessor::default(),
    )
}

pub fn get_radcast_capabilities_with_processor(
    processor: CaptionProcessor,
) -> RadcastCapabilityStatus {
    get_radcast_capabilities_with_processors(processor, EnhancementProcessor::default())
}

pub fn get_radcast_capabilities_with_processors(
    processor: CaptionProcessor,
    enhancement_processor: EnhancementProcessor,
) -> RadcastCapabilityStatus {
    let (caption_available, caption_detail) = if processor.is_available() {
        (
            true,
            "Native captions are available through whisper.cpp.".to_string(),
        )
    } else {
        (
            false,
            format!(
                "Install whisper.cpp and a local speech model before generating captions. Expected model: {}.",
                processor.model_path().display()
            ),
        )
    };
    let (optimized_available, optimized_detail) = if enhancement_processor.is_available() {
        (true, optimized_capability_detail())
    } else {
        (
            false,
            format!(
                "Install the local RADcast Studio helper to enable RADcast Optimized. Expected command: {}.",
                enhancement_processor.command_path().display()
            ),
        )
    };
    RadcastCapabilityStatus {
        caption_available,
        caption_detail,
        optimized_available,
        optimized_detail,
    }
}

pub fn get_radt_ts_capabilities() -> RadtTsCapabilityStatus {
    crate::radt_ts::discover_radt_ts_cli()
}

pub async fn list_radt_ts_outputs(
    state: &DesktopState,
    request: ListRadtTsOutputsRequest,
) -> Result<RadtTsOutputListing, String> {
    let project = load_requested_or_local_radcite_project(state, request.project_id)
        .await
        .map_err(|error| error.to_string())?;
    crate::radt_ts::list_radt_ts_outputs_for_project(state, project.id)
        .map_err(|error| error.to_string())
}

pub async fn start_radt_ts_synthesis(
    state: &DesktopState,
    request: StartRadtTsSynthesisRequest,
) -> Result<RadtTsJobStatus, String> {
    let project = load_requested_or_local_radcite_project(state, request.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let request = crate::radt_ts::RadtTsSynthesisRequest {
        project_id: project.id,
        text: request.text,
        reference_audio_path: PathBuf::from(request.reference_audio_path),
        quality: request.quality,
        chunk_mode: request.chunk_mode,
        pause_min_seconds: request.pause_min_seconds,
        pause_max_seconds: request.pause_max_seconds,
        output_format: request.output_format,
        output_name: request.output_name,
        acknowledge_voice_clone: request.acknowledge_voice_clone,
    };
    crate::radt_ts::start_radt_ts_synthesis(state, request)
        .await
        .map_err(|error| error.to_string())
}

pub fn get_radt_ts_job(state: &DesktopState, job_id: String) -> Result<RadtTsJobStatus, String> {
    crate::radt_ts::get_radt_ts_job(state, &job_id).map_err(|error| error.to_string())
}

pub fn cancel_radt_ts_job(state: &DesktopState, job_id: String) -> Result<RadtTsJobStatus, String> {
    crate::radt_ts::cancel_radt_ts_job(state, &job_id).map_err(|error| error.to_string())
}

pub async fn list_radt_ts_media_outputs(
    state: &DesktopState,
    request: ListRadtTsOutputsRequest,
) -> Result<RadtTsMediaOutputListing, String> {
    let project = load_requested_or_local_radcite_project(state, request.project_id)
        .await
        .map_err(|error| error.to_string())?;
    crate::radt_ts_tools::list_radt_ts_media_outputs(state, project.id)
        .map_err(|error| error.to_string())
}

pub async fn start_radt_ts_transcription(
    state: &DesktopState,
    mut request: StartRadtTsTranscriptionRequest,
) -> Result<RadtTsMediaJobStatus, String> {
    let project = load_requested_or_local_radcite_project(state, request.project_id)
        .await
        .map_err(|error| error.to_string())?;
    request.project_id = Some(project.id);
    crate::radt_ts_tools::start_radt_ts_transcription(state, request)
        .await
        .map_err(|error| error.to_string())
}

pub async fn start_radt_ts_clip(
    state: &DesktopState,
    mut request: StartRadtTsClipRequest,
) -> Result<RadtTsMediaJobStatus, String> {
    let project = load_requested_or_local_radcite_project(state, request.project_id)
        .await
        .map_err(|error| error.to_string())?;
    request.project_id = Some(project.id);
    crate::radt_ts_tools::start_radt_ts_clip(state, request)
        .await
        .map_err(|error| error.to_string())
}

pub fn get_radt_ts_media_job(
    state: &DesktopState,
    job_id: String,
) -> Result<RadtTsMediaJobStatus, String> {
    crate::radt_ts_tools::get_radt_ts_media_job(state, &job_id).map_err(|error| error.to_string())
}

pub fn cancel_radt_ts_media_job(
    state: &DesktopState,
    job_id: String,
) -> Result<RadtTsMediaJobStatus, String> {
    crate::radt_ts_tools::cancel_radt_ts_media_job(state, &job_id)
        .map_err(|error| error.to_string())
}

fn optimized_capability_detail() -> String {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "RADcast Optimized is available locally. This Apple Silicon build uses the validated CPU profile and all available CPU threads; no server connection is required.".to_string()
    } else {
        "RADcast Optimized is available through the local Studio helper; no server connection is required.".to_string()
    }
}

pub fn get_app_status(state: &DesktopState) -> AppStatus {
    AppStatus {
        app_name: state.app_name.clone(),
        database_ready: state.database_ready,
        sync_configured: state.sync_configured,
        engines: state.engine_registry.list(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadciteProjectSummary {
    pub id: ProjectId,
    pub code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub structure_mode: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRadciteProjectRequest {
    pub code: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRadciteProjectRequest {
    pub project_id: ProjectId,
    pub code: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub structure_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRadciteProjectRequest {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRadciteProjectRequest {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyseDocxRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub path: String,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysePdfRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub path: String,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyseDocxResponse {
    pub project_id: ProjectId,
    pub project_title: String,
    pub document_id: DocumentId,
    pub original_filename: String,
    pub source_path: Option<String>,
    pub source_file_type: String,
    pub paragraph_count: usize,
    pub citation_count: usize,
    pub missing_citation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyseDocxReviewResponse {
    pub project_id: ProjectId,
    pub project_title: String,
    pub document_id: DocumentId,
    pub original_filename: String,
    pub display_name: String,
    pub source_path: Option<String>,
    pub source_file_type: String,
    pub doc_variant: String,
    pub doc_number: Option<i32>,
    pub exclude_from_references: bool,
    pub summary: AnalyseDocxSummary,
    pub paragraphs: Vec<ReviewParagraph>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyseDocxSummary {
    pub paragraph_count: usize,
    pub citation_count: usize,
    pub cited_paragraph_count: usize,
    pub missing_citation_count: usize,
    pub linked_citation_count: usize,
    pub suggested_citation_count: usize,
    pub unlinked_citation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedRadciteReviewSummary {
    pub document_id: DocumentId,
    pub project_id: ProjectId,
    pub original_filename: String,
    pub display_name: String,
    pub source_path: Option<String>,
    pub source_file_type: String,
    pub doc_variant: String,
    pub doc_number: Option<i32>,
    pub exclude_from_references: bool,
    pub paragraph_count: usize,
    pub citation_count: usize,
    pub missing_citation_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadciteArchiveItemKind {
    Document,
    Module,
    CourseReference,
    ModuleReading,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadciteArchiveItem {
    pub id: String,
    pub kind: RadciteArchiveItemKind,
    pub label: String,
    pub detail: Option<String>,
    pub archived_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRadciteArchiveRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRadciteArchiveItemRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub kind: RadciteArchiveItemKind,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRadciteDocumentRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub document_id: DocumentId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRadciteDocumentRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub document_id: DocumentId,
    pub display_name: String,
    pub doc_number: Option<i32>,
    pub doc_variant: DocumentVariant,
    pub exclude_from_references: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSavedReviewsRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListCourseReferencesRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseReferenceSummary {
    pub id: ReferenceEntryId,
    pub project_id: ProjectId,
    pub reference_type: String,
    pub apa_citation: Option<String>,
    pub citation_text: Option<String>,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub publication_year: Option<String>,
    pub source: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub validation_status: String,
    pub validation_report: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseModuleSummary {
    pub id: ModuleId,
    pub project_id: ProjectId,
    pub code: Option<String>,
    pub title: String,
    pub order_index: Option<i32>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRadciteModulesRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleReadingSummary {
    pub id: ReferenceEntryId,
    pub project_id: ProjectId,
    pub module_id: ModuleId,
    pub reading_category: String,
    pub lesson_code: Option<String>,
    pub apa_citation: Option<String>,
    pub citation_text: Option<String>,
    pub title: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub reading_notes: Option<String>,
    pub estimated_reading_time: Option<String>,
    pub validation_status: String,
    pub validation_report: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewParagraph {
    pub id: ParagraphId,
    pub order_index: i32,
    pub page: Option<i32>,
    pub text: String,
    pub formatted_text: Option<String>,
    pub is_table: bool,
    pub needs_citation: bool,
    pub citations: Vec<ReviewCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCitation {
    pub id: CitationId,
    pub text: String,
    pub start: Option<i32>,
    pub end: Option<i32>,
    pub verified: bool,
    pub reference_entry_id: Option<ReferenceEntryId>,
    pub reference_suggestions: Vec<ReviewCitationReferenceSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCitationReferenceSuggestion {
    pub reference_entry_id: ReferenceEntryId,
    pub label: String,
    pub confidence: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateParagraphReviewRequest {
    pub document_id: DocumentId,
    pub paragraph_id: ParagraphId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddManualCitationRequest {
    pub document_id: DocumentId,
    pub paragraph_id: ParagraphId,
    pub citation_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCitationReferenceRequest {
    pub document_id: DocumentId,
    pub citation_id: CitationId,
    pub reference_entry_id: ReferenceEntryId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadSavedReviewRequest {
    pub document_id: DocumentId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddCourseReferenceRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub apa_citation: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCourseReferenceRequest {
    pub reference_id: ReferenceEntryId,
    pub apa_citation: String,
    pub notes: Option<String>,
    pub citation_text: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveCourseReferenceRequest {
    pub reference_id: ReferenceEntryId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeCourseReferencesRequest {
    pub primary_reference_id: ReferenceEntryId,
    pub merge_reference_ids: Vec<ReferenceEntryId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddRadciteModuleRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub title: String,
    pub code: Option<String>,
    pub order_index: Option<i32>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRadciteModuleRequest {
    pub module_id: ModuleId,
    pub title: String,
    pub code: Option<String>,
    pub order_index: Option<i32>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRadciteModuleRequest {
    pub module_id: ModuleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListModuleReadingsRequest {
    pub module_id: ModuleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddModuleReadingRequest {
    pub module_id: ModuleId,
    pub reading_category: String,
    pub lesson_code: Option<String>,
    pub apa_citation: Option<String>,
    pub citation_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub reading_notes: Option<String>,
    pub estimated_reading_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateModuleReadingRequest {
    pub reading_id: ReferenceEntryId,
    pub reading_category: String,
    pub lesson_code: Option<String>,
    pub apa_citation: Option<String>,
    pub citation_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub reading_notes: Option<String>,
    pub estimated_reading_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveModuleReadingRequest {
    pub reading_id: ReferenceEntryId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewModuleReadingsImportRequest {
    pub path: String,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewModuleReadingsCsvImportRequest {
    pub path: String,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewModuleReadingsPdfImportRequest {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDocumentReadingsRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub path: String,
    pub source_file_type: DocumentFileType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDocumentReadingsResponse {
    pub candidate_count: usize,
    pub saved_count: usize,
    pub created_module_count: usize,
    pub unassigned_count: usize,
    pub failed_file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleReadingImportCandidateSummary {
    pub source_path: Option<String>,
    pub source_filename: Option<String>,
    pub module_order: Option<i32>,
    pub module_title: Option<String>,
    pub reading_category: String,
    pub lesson_code: Option<String>,
    pub apa_citation: String,
    pub citation_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleReadingsPdfImportPreview {
    pub candidates: Vec<ModuleReadingImportCandidateSummary>,
    pub failures: Vec<ModuleReadingsPdfImportFailureSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleReadingsPdfImportFailureSummary {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveModuleReadingsImportRequest {
    pub candidates: Vec<SaveModuleReadingsImportCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveModuleReadingsImportCandidate {
    pub module_id: ModuleId,
    pub reading_category: String,
    pub lesson_code: Option<String>,
    pub apa_citation: Option<String>,
    pub citation_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub reading_notes: Option<String>,
    pub estimated_reading_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportCourseReferencesRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub for_ako_learn: bool,
    #[serde(default)]
    pub allow_incomplete: bool,
    #[serde(default)]
    pub use_library_links: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportModuleReadingsRequest {
    pub module_id: ModuleId,
    pub for_ako_learn: bool,
    #[serde(default)]
    pub use_library_links: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseReferencesExport {
    pub filename: String,
    pub content_type: String,
    pub html: String,
    pub reference_count: usize,
    pub apa_error_count: usize,
    pub apa_warning_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleReadingsExport {
    pub filename: String,
    pub content_type: String,
    pub html: String,
    pub module_id: ModuleId,
    pub reading_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRadciteReviewReportRequest {
    pub document_id: DocumentId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadciteReviewReportExport {
    pub filename: String,
    pub content_type: String,
    pub json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RadciteReviewReport {
    filename: String,
    file_type: String,
    project_title: String,
    document_id: DocumentId,
    generated_at: String,
    statistics: RadciteReviewReportStatistics,
    details: Vec<RadciteReviewReportDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RadciteReviewReportStatistics {
    total_paragraphs: usize,
    paragraphs_with_citations: usize,
    paragraphs_needing_citations: usize,
    total_citations: usize,
    citation_coverage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RadciteReviewReportDetail {
    index: usize,
    page: Option<i32>,
    text_preview: String,
    citations: Vec<String>,
    needs_citation: bool,
    is_table: bool,
}

#[derive(Debug, Error)]
pub enum AnalyseDocxError {
    #[error("choose a DOCX file before running RADcite analysis")]
    EmptyPath,
    #[error("could not determine the DOCX filename")]
    MissingFilename,
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Ingestion(#[from] DocxIngestionError),
    #[error(transparent)]
    Storage(#[from] DocumentStorageError),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum AnalysePdfError {
    #[error("choose a PDF file before running RADcite analysis")]
    EmptyPath,
    #[error("could not determine the PDF filename")]
    MissingFilename,
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Ingestion(#[from] PdfIngestionError),
    #[error(transparent)]
    Storage(#[from] DocumentStorageError),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum RadcastAudioError {
    #[error("could not load RADcast project {0}")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Database(#[from] DbError),
    #[error(transparent)]
    Storage(#[from] RadcastStorageError),
    #[error("RADcast processing task failed")]
    ProcessingTask(#[from] tokio::task::JoinError),
    #[error("RADcast processing job was not found: {0}")]
    MissingJob(String),
}

#[derive(Debug, Error)]
pub enum RadciteProjectError {
    #[error("enter a project title before creating it")]
    EmptyTitle,
    #[error("cannot edit archived RADcite project {0}")]
    ArchivedProject(ProjectId),
    #[error("project structure must be either modules or weeks")]
    InvalidStructureMode,
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum ReviewActionError {
    #[error("enter citation text before adding a manual citation")]
    EmptyCitationText,
    #[error("could not load RADcite review document {0}")]
    MissingDocument(DocumentId),
    #[error("could not load project {0} for RADcite review document")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Database(#[from] DbError),
    #[error("could not serialise RADcite review report")]
    ReportSerialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum RadciteDocumentError {
    #[error("document number must be a positive integer")]
    InvalidDocumentNumber(i32),
    #[error("could not load RADcite document {0}")]
    MissingDocument(DocumentId),
    #[error("RADcite document {document_id} belongs to a different project")]
    ProjectMismatch {
        document_id: DocumentId,
        project_id: ProjectId,
    },
    #[error("cannot edit archived RADcite document {0}")]
    ArchivedDocument(DocumentId),
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum RadciteArchiveError {
    #[error("could not load RADcite archive project {0}")]
    MissingProject(ProjectId),
    #[error("could not load archived RADcite item {kind:?} {item_id}")]
    MissingItem {
        kind: RadciteArchiveItemKind,
        item_id: String,
    },
    #[error("invalid archived RADcite item id: {0}")]
    InvalidItemId(String),
    #[error("cannot restore a module reading while its module is archived")]
    ParentModuleArchived,
    #[error("could not load archived RADcite document {0}")]
    MissingDocument(DocumentId),
    #[error(transparent)]
    Database(#[from] DbError),
}

impl From<RadciteProjectLookupError> for RadciteArchiveError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

#[derive(Debug, Error)]
pub enum CourseReferenceError {
    #[error("enter reference text before adding a course reference")]
    EmptyReferenceText,
    #[error("{0}")]
    InvalidMerge(String),
    #[error("could not load course reference {0}")]
    MissingReference(ReferenceEntryId),
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum RadciteModuleError {
    #[error("enter a module title before adding it")]
    EmptyTitle,
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error("could not load RADcite module {0}")]
    MissingModule(ModuleId),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum ModuleReadingError {
    #[error("enter reading text before adding a module reading")]
    EmptyReadingText,
    #[error("choose compulsory or optional for the reading category")]
    InvalidCategory(String),
    #[error("could not load RADcite module {0}")]
    MissingModule(ModuleId),
    #[error("could not load module reading {0}")]
    MissingReading(ReferenceEntryId),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum ModuleReadingImportError {
    #[error("choose a DOCX, CSV, or PDF file before importing or previewing module readings")]
    EmptyPath,
    #[error(transparent)]
    Docx(#[from] DocxIngestionError),
    #[error(transparent)]
    Csv(#[from] CsvReadingImportError),
    #[error(transparent)]
    Pdf(#[from] PdfReadingExtractionError),
    #[error("could not load RADcite module {0}")]
    MissingModule(ModuleId),
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error("choose compulsory or optional for the reading category")]
    InvalidCategory(String),
    #[error("enter reading text before importing a module reading")]
    EmptyReadingText,
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum CourseReferenceExportError {
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error(
        "{count} course reference(s) need APA fixes before export; enable export with fixes pending to continue"
    )]
    ApaValidation { count: usize },
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum ModuleReadingExportError {
    #[error("could not load RADcite module {0}")]
    MissingModule(ModuleId),
    #[error(transparent)]
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
enum RadciteProjectLookupError {
    #[error("could not load RADcite project {0}")]
    MissingProject(ProjectId),
    #[error(transparent)]
    Database(#[from] DbError),
}

impl From<RadciteProjectLookupError> for AnalyseDocxError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

impl From<RadciteProjectLookupError> for AnalysePdfError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

impl From<RadciteProjectLookupError> for RadcastAudioError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

impl From<RadciteProjectLookupError> for ReviewActionError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

impl From<RadciteProjectLookupError> for CourseReferenceError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

impl From<RadciteProjectLookupError> for RadciteModuleError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

impl From<RadciteProjectLookupError> for CourseReferenceExportError {
    fn from(error: RadciteProjectLookupError) -> Self {
        match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                Self::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => Self::Database(error),
        }
    }
}

pub async fn list_radcite_projects(
    state: &DesktopState,
) -> Result<Vec<RadciteProjectSummary>, RadciteProjectError> {
    load_or_create_local_radcite_project(state).await?;

    let projects = SqliteProjectRepository::new(state.database_pool.clone())
        .list_projects()
        .await?;

    Ok(projects.into_iter().map(radcite_project_summary).collect())
}

pub async fn create_radcite_project(
    state: &DesktopState,
    request: CreateRadciteProjectRequest,
) -> Result<RadciteProjectSummary, RadciteProjectError> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err(RadciteProjectError::EmptyTitle);
    }

    let code = request
        .code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    let project = Project::new(code, title, UserId::new());

    SqliteProjectRepository::new(state.database_pool.clone())
        .insert_project(&project)
        .await?;

    Ok(radcite_project_summary(project))
}

pub async fn update_radcite_project(
    state: &DesktopState,
    request: UpdateRadciteProjectRequest,
) -> Result<RadciteProjectSummary, RadciteProjectError> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err(RadciteProjectError::EmptyTitle);
    }

    let project_repo = SqliteProjectRepository::new(state.database_pool.clone());
    let mut project = project_repo
        .load_project(request.project_id)
        .await?
        .ok_or(RadciteProjectError::MissingProject(request.project_id))?;
    if project.archived_at.is_some() {
        return Err(RadciteProjectError::ArchivedProject(project.id));
    }

    project.code = request
        .code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    project.title = title.to_string();
    project.description = request
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    project.structure_mode = match request.structure_mode.trim() {
        "modules" => "modules".to_string(),
        "weeks" => "weeks".to_string(),
        _ => return Err(RadciteProjectError::InvalidStructureMode),
    };

    project_repo.update_project(&project).await?;
    let updated = project_repo
        .load_project(project.id)
        .await?
        .ok_or(RadciteProjectError::MissingProject(project.id))?;

    Ok(radcite_project_summary(updated))
}

pub async fn archive_radcite_project(
    state: &DesktopState,
    request: ArchiveRadciteProjectRequest,
) -> Result<RadciteProjectSummary, RadciteProjectError> {
    let project_repo = SqliteProjectRepository::new(state.database_pool.clone());
    let project = project_repo
        .load_project(request.project_id)
        .await?
        .ok_or(RadciteProjectError::MissingProject(request.project_id))?;

    project_repo.archive_project(project.id).await?;
    let archived = project_repo
        .load_project(project.id)
        .await?
        .ok_or(RadciteProjectError::MissingProject(project.id))?;

    Ok(radcite_project_summary(archived))
}

pub async fn restore_radcite_project(
    state: &DesktopState,
    request: RestoreRadciteProjectRequest,
) -> Result<RadciteProjectSummary, RadciteProjectError> {
    let project_repo = SqliteProjectRepository::new(state.database_pool.clone());
    let project = project_repo
        .load_project(request.project_id)
        .await?
        .ok_or(RadciteProjectError::MissingProject(request.project_id))?;

    project_repo.restore_project(project.id).await?;
    let restored = project_repo
        .load_project(project.id)
        .await?
        .ok_or(RadciteProjectError::MissingProject(project.id))?;

    Ok(radcite_project_summary(restored))
}

pub async fn list_radcast_audio(
    state: &DesktopState,
    request: ListRadcastAudioRequest,
) -> Result<RadcastAudioListing, RadcastAudioError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    Ok(crate::radcast::list_audio(
        &state.paths.data_dir,
        project.id,
    )?)
}

pub async fn delete_radcast_audio(
    state: &DesktopState,
    request: DeleteRadcastAudioRequest,
) -> Result<(), RadcastAudioError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let data_dir = state.paths.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        crate::radcast::delete_audio(&data_dir, project.id, request)
    })
    .await?
    .map_err(Into::into)
}

pub async fn import_radcast_audio(
    state: &DesktopState,
    request: ImportRadcastAudioRequest,
) -> Result<RadcastAudioSource, RadcastAudioError> {
    import_radcast_audio_with_processor(state, request, AudioProcessor::default()).await
}

pub async fn import_radcast_audio_with_processor(
    state: &DesktopState,
    request: ImportRadcastAudioRequest,
    processor: AudioProcessor,
) -> Result<RadcastAudioSource, RadcastAudioError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let data_dir = state.paths.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        crate::radcast::import_audio(&data_dir, project.id, request, processor)
    })
    .await?
    .map_err(Into::into)
}

pub async fn process_radcast_audio(
    state: &DesktopState,
    request: ProcessRadcastAudioRequest,
) -> Result<RadcastAudioOutput, RadcastAudioError> {
    process_radcast_audio_with_processors(
        state,
        request,
        AudioProcessor::default(),
        CaptionProcessor::default(),
    )
    .await
}

pub async fn save_radcast_settings(
    state: &DesktopState,
    request: SaveRadcastSettingsRequest,
) -> Result<RadcastProjectSettings, RadcastAudioError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let data_dir = state.paths.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        crate::radcast::save_settings(&data_dir, project.id, request.settings)
    })
    .await?
    .map_err(Into::into)
}

pub async fn process_radcast_audio_with_processor(
    state: &DesktopState,
    request: ProcessRadcastAudioRequest,
    processor: AudioProcessor,
) -> Result<RadcastAudioOutput, RadcastAudioError> {
    process_radcast_audio_with_processors(state, request, processor, CaptionProcessor::default())
        .await
}

pub async fn process_radcast_audio_with_processors(
    state: &DesktopState,
    request: ProcessRadcastAudioRequest,
    processor: AudioProcessor,
    caption_processor: CaptionProcessor,
) -> Result<RadcastAudioOutput, RadcastAudioError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let data_dir = state.paths.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        crate::radcast::process_audio_with_processors(
            &data_dir,
            project.id,
            request,
            processor,
            caption_processor,
        )
    })
    .await?
    .map_err(Into::into)
}

pub async fn start_radcast_audio(
    state: &DesktopState,
    request: ProcessRadcastAudioRequest,
) -> Result<crate::RadcastJobStatus, RadcastAudioError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let job_id = Uuid::new_v4().to_string();
    let initial_status = crate::RadcastJobStatus::running(job_id.clone());
    state
        .radcast_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(job_id.clone(), initial_status.clone());
    state
        .radcast_cancel_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&job_id);

    let data_dir = state.paths.data_dir.clone();
    let jobs = state.radcast_jobs.clone();
    let cancel_requests = state.radcast_cancel_requests.clone();
    tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let result = crate::radcast::process_audio_with_processors_and_enhancement_with_progress_and_cancellation(
            &data_dir,
            project.id,
            request,
            AudioProcessor::default(),
            CaptionProcessor::default(),
            EnhancementProcessor::default(),
            |progress| {
                let mut jobs = jobs.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.update_progress(progress, started.elapsed().as_secs_f64());
                }
            },
            || {
                cancel_requests
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .contains(&job_id)
            },
        );
        cancel_requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&job_id);
        let mut jobs = jobs.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(job) = jobs.get_mut(&job_id) {
            job.elapsed_seconds = started.elapsed().as_secs_f64();
            match result {
                Ok(output) => {
                    job.state = crate::RadcastJobState::Completed;
                    job.percent = 100;
                    job.output = Some(output);
                }
                Err(crate::RadcastStorageError::Cancelled) => {
                    job.state = crate::RadcastJobState::Cancelled;
                    job.error = None;
                }
                Err(error) => {
                    job.state = crate::RadcastJobState::Failed;
                    job.error = Some(error.to_string());
                }
            }
        }
    });

    Ok(initial_status)
}

pub fn cancel_radcast_audio(
    state: &DesktopState,
    job_id: String,
) -> Result<crate::RadcastJobStatus, RadcastAudioError> {
    let status = state
        .radcast_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&job_id)
        .cloned()
        .ok_or_else(|| RadcastAudioError::MissingJob(job_id.clone()))?;
    if status.state == crate::RadcastJobState::Running {
        state
            .radcast_cancel_requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(job_id);
    }
    Ok(status)
}

pub fn get_radcast_audio_job(
    state: &DesktopState,
    job_id: String,
) -> Result<crate::RadcastJobStatus, RadcastAudioError> {
    state
        .radcast_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&job_id)
        .cloned()
        .ok_or(RadcastAudioError::MissingJob(job_id))
}

pub async fn process_radcast_audio_with_processors_and_enhancement(
    state: &DesktopState,
    request: ProcessRadcastAudioRequest,
    processor: AudioProcessor,
    caption_processor: CaptionProcessor,
    enhancement_processor: EnhancementProcessor,
) -> Result<RadcastAudioOutput, RadcastAudioError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let data_dir = state.paths.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        crate::radcast::process_audio_with_processors_and_enhancement(
            &data_dir,
            project.id,
            request,
            processor,
            caption_processor,
            enhancement_processor,
        )
    })
    .await?
    .map_err(Into::into)
}

pub async fn analyse_docx_path(
    state: &DesktopState,
    request: AnalyseDocxRequest,
) -> Result<AnalyseDocxResponse, AnalyseDocxError> {
    let analysed = analyse_docx(state, request).await?;
    let summary = build_summary(&analysed.paragraphs, &analysed.citations);

    Ok(AnalyseDocxResponse {
        project_id: analysed.project.id,
        project_title: analysed.project.title,
        document_id: analysed.document.id,
        original_filename: analysed.document.original_filename.clone(),
        source_path: analysed.document.source_path,
        source_file_type: document_file_type_label(analysed.document.file_type).to_string(),
        paragraph_count: summary.paragraph_count,
        citation_count: summary.citation_count,
        missing_citation_count: summary.missing_citation_count,
    })
}

pub async fn analyse_docx_for_review(
    state: &DesktopState,
    request: AnalyseDocxRequest,
) -> Result<AnalyseDocxReviewResponse, AnalyseDocxError> {
    let analysed = analyse_docx(state, request).await?;
    let references = load_course_reference_entries(state, analysed.project.id).await?;
    let paragraphs = build_review_paragraphs(
        analysed.paragraphs,
        analysed.citations,
        references.as_slice(),
    );
    let summary = build_review_summary(&paragraphs);

    Ok(AnalyseDocxReviewResponse {
        project_id: analysed.project.id,
        project_title: analysed.project.title,
        document_id: analysed.document.id,
        original_filename: analysed.document.original_filename.clone(),
        display_name: effective_document_display_name(&analysed.document),
        source_path: analysed.document.source_path,
        source_file_type: document_file_type_label(analysed.document.file_type).to_string(),
        doc_variant: document_variant_label(analysed.document.doc_variant).to_string(),
        doc_number: analysed.document.doc_number,
        exclude_from_references: analysed.document.exclude_from_references,
        summary,
        paragraphs,
    })
}

pub async fn analyse_pdf_for_review(
    state: &DesktopState,
    request: AnalysePdfRequest,
) -> Result<AnalyseDocxReviewResponse, AnalysePdfError> {
    let analysed = analyse_pdf(state, request).await?;
    let references = load_course_reference_entries(state, analysed.project.id).await?;
    let paragraphs = build_review_paragraphs(
        analysed.paragraphs,
        analysed.citations,
        references.as_slice(),
    );
    let summary = build_review_summary(&paragraphs);

    Ok(AnalyseDocxReviewResponse {
        project_id: analysed.project.id,
        project_title: analysed.project.title,
        document_id: analysed.document.id,
        original_filename: analysed.document.original_filename.clone(),
        display_name: effective_document_display_name(&analysed.document),
        source_path: analysed.document.source_path,
        source_file_type: document_file_type_label(analysed.document.file_type).to_string(),
        doc_variant: document_variant_label(analysed.document.doc_variant).to_string(),
        doc_number: analysed.document.doc_number,
        exclude_from_references: analysed.document.exclude_from_references,
        summary,
        paragraphs,
    })
}

pub async fn list_saved_radcite_reviews(
    state: &DesktopState,
    request: ListSavedReviewsRequest,
) -> Result<Vec<SavedRadciteReviewSummary>, ReviewActionError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let documents = SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .list_documents_for_project(project.id)
        .await?;

    Ok(documents
        .into_iter()
        .map(|document| SavedRadciteReviewSummary {
            document_id: document.document_id,
            project_id: document.project_id,
            original_filename: document.original_filename,
            display_name: document.display_name,
            source_path: document.source_path,
            source_file_type: document_file_type_label(document.file_type).to_string(),
            doc_variant: document_variant_label(document.doc_variant).to_string(),
            doc_number: document.doc_number,
            exclude_from_references: document.exclude_from_references,
            paragraph_count: document.paragraph_count as usize,
            citation_count: document.citation_count as usize,
            missing_citation_count: document.missing_citation_count as usize,
        })
        .collect())
}

pub async fn update_radcite_document(
    state: &DesktopState,
    request: UpdateRadciteDocumentRequest,
) -> Result<SavedRadciteReviewSummary, RadciteDocumentError> {
    if let Some(doc_number) = request.doc_number
        && doc_number < 1
    {
        return Err(RadciteDocumentError::InvalidDocumentNumber(doc_number));
    }

    let document_repo = SqliteCitationDocumentRepository::new(state.database_pool.clone());
    let mut analysis = document_repo
        .load_document_analysis(request.document_id)
        .await?
        .ok_or(RadciteDocumentError::MissingDocument(request.document_id))?;
    let project = SqliteProjectRepository::new(state.database_pool.clone())
        .load_project(analysis.document.project_id)
        .await?
        .ok_or(RadciteDocumentError::MissingProject(
            analysis.document.project_id,
        ))?;

    if let Some(requested_project_id) = request.project_id
        && requested_project_id != project.id
    {
        return Err(RadciteDocumentError::ProjectMismatch {
            document_id: request.document_id,
            project_id: requested_project_id,
        });
    }

    if analysis.document.archived_at.is_some() {
        return Err(RadciteDocumentError::ArchivedDocument(request.document_id));
    }

    analysis.document.notes = trimmed_optional(Some(request.display_name));
    analysis.document.doc_number = request.doc_number;
    analysis.document.doc_variant = request.doc_variant;
    analysis.document.exclude_from_references = request.exclude_from_references;
    analysis.document.updated_at = Utc::now();
    document_repo
        .update_document_metadata(&analysis.document)
        .await?;

    Ok(saved_review_summary_from_analysis(
        &analysis.document,
        &analysis.paragraphs,
        &analysis.citations,
    ))
}

pub async fn archive_radcite_document(
    state: &DesktopState,
    request: ArchiveRadciteDocumentRequest,
) -> Result<SavedRadciteReviewSummary, RadciteArchiveError> {
    let document_repo = SqliteCitationDocumentRepository::new(state.database_pool.clone());
    let analysis = document_repo
        .load_document_analysis(request.document_id)
        .await?
        .ok_or(RadciteArchiveError::MissingDocument(request.document_id))?;
    let project = SqliteProjectRepository::new(state.database_pool.clone())
        .load_project(analysis.document.project_id)
        .await?
        .ok_or(RadciteArchiveError::MissingProject(
            analysis.document.project_id,
        ))?;
    if let Some(requested_project_id) = request.project_id
        && requested_project_id != project.id
    {
        return Err(RadciteArchiveError::MissingDocument(request.document_id));
    }

    if analysis.document.archived_at.is_some() {
        return Err(RadciteArchiveError::MissingDocument(request.document_id));
    }

    document_repo.archive_document(analysis.document.id).await?;

    Ok(SavedRadciteReviewSummary {
        document_id: analysis.document.id,
        project_id: project.id,
        original_filename: analysis.document.original_filename.clone(),
        display_name: effective_document_display_name(&analysis.document),
        source_path: analysis.document.source_path,
        source_file_type: document_file_type_label(analysis.document.file_type).to_string(),
        doc_variant: document_variant_label(analysis.document.doc_variant).to_string(),
        doc_number: analysis.document.doc_number,
        exclude_from_references: analysis.document.exclude_from_references,
        paragraph_count: analysis.paragraphs.len(),
        citation_count: analysis.citations.len(),
        missing_citation_count: analysis
            .paragraphs
            .iter()
            .filter(|paragraph| paragraph.needs_citation)
            .count(),
    })
}

pub async fn list_radcite_archive(
    state: &DesktopState,
    request: ListRadciteArchiveRequest,
) -> Result<Vec<RadciteArchiveItem>, RadciteArchiveError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let module_repo = SqliteCourseModuleRepository::new(state.database_pool.clone());
    let reference_repo = SqliteReferenceEntryRepository::new(state.database_pool.clone());
    let document_repo = SqliteCitationDocumentRepository::new(state.database_pool.clone());
    let archived_modules = module_repo
        .list_archived_course_modules_for_project(project.id)
        .await?;
    let active_modules = module_repo
        .list_course_modules_for_project(project.id)
        .await?;
    let archived_module_ids = archived_modules
        .iter()
        .map(|module| module.id)
        .collect::<Vec<_>>();
    let module_titles = active_modules
        .into_iter()
        .chain(archived_modules.iter().cloned())
        .map(|module| (module.id, module.title.clone()))
        .collect::<Vec<_>>();
    let archived_documents = document_repo
        .list_archived_documents_for_project(project.id)
        .await?;
    let archived_references = reference_repo
        .list_archived_reference_entries_for_project(project.id, ReferenceEntryType::Reference)
        .await?;
    let archived_readings = reference_repo
        .list_archived_reference_entries_for_project(project.id, ReferenceEntryType::Reading)
        .await?;

    let mut items = Vec::new();
    items.extend(archived_documents.into_iter().filter_map(|document| {
        Some(RadciteArchiveItem {
            id: document.document_id.0.to_string(),
            kind: RadciteArchiveItemKind::Document,
            label: document.original_filename,
            detail: Some(format!(
                "{} paragraphs · {} citations",
                document.paragraph_count, document.citation_count
            )),
            archived_at: document.archived_at?.to_rfc3339(),
        })
    }));
    items.extend(archived_modules.into_iter().filter_map(|module| {
        Some(RadciteArchiveItem {
            id: module.id.0.to_string(),
            kind: RadciteArchiveItemKind::Module,
            label: module.title,
            detail: module.code,
            archived_at: module.archived_at?.to_rfc3339(),
        })
    }));
    items.extend(archived_references.into_iter().filter_map(|reference| {
        Some(RadciteArchiveItem {
            id: reference.id.0.to_string(),
            kind: RadciteArchiveItemKind::CourseReference,
            label: reference_label(&reference),
            detail: Some("Course reference".to_string()),
            archived_at: reference.archived_at?.to_rfc3339(),
        })
    }));
    items.extend(archived_readings.into_iter().filter_map(|reading| {
        let module_id = reading.module_id?;
        if archived_module_ids.contains(&module_id) {
            return None;
        }

        let module_title = module_titles
            .iter()
            .find(|(id, _)| *id == module_id)
            .map(|(_, title)| title.clone());
        Some(RadciteArchiveItem {
            id: reading.id.0.to_string(),
            kind: RadciteArchiveItemKind::ModuleReading,
            label: reference_label(&reading),
            detail: module_title.or_else(|| Some("Module reading".to_string())),
            archived_at: reading.archived_at?.to_rfc3339(),
        })
    }));

    items.sort_by(|left, right| {
        right
            .archived_at
            .cmp(&left.archived_at)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    Ok(items)
}

pub async fn restore_radcite_archive_item(
    state: &DesktopState,
    request: RestoreRadciteArchiveItemRequest,
) -> Result<Vec<RadciteArchiveItem>, RadciteArchiveError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let archive = list_radcite_archive(
        state,
        ListRadciteArchiveRequest {
            project_id: Some(project.id),
        },
    )
    .await?;
    if !archive
        .iter()
        .any(|item| item.kind == request.kind && item.id == request.item_id)
    {
        return Err(RadciteArchiveError::MissingItem {
            kind: request.kind,
            item_id: request.item_id,
        });
    }

    match request.kind {
        RadciteArchiveItemKind::Document => {
            let document_id = parse_document_id(&request.item_id)?;
            SqliteCitationDocumentRepository::new(state.database_pool.clone())
                .restore_document(document_id)
                .await?;
        }
        RadciteArchiveItemKind::Module => {
            let module_id = parse_module_id(&request.item_id)?;
            SqliteCourseModuleRepository::new(state.database_pool.clone())
                .restore_course_module(module_id)
                .await?;
        }
        RadciteArchiveItemKind::CourseReference | RadciteArchiveItemKind::ModuleReading => {
            let reference_id = parse_reference_entry_id(&request.item_id)?;
            SqliteReferenceEntryRepository::new(state.database_pool.clone())
                .restore_reference_entry(reference_id)
                .await?;
        }
    }

    list_radcite_archive(
        state,
        ListRadciteArchiveRequest {
            project_id: Some(project.id),
        },
    )
    .await
}

pub async fn load_saved_radcite_review(
    state: &DesktopState,
    document_id: DocumentId,
) -> Result<AnalyseDocxReviewResponse, ReviewActionError> {
    load_review_response(state, document_id).await
}

pub async fn export_radcite_review_report(
    state: &DesktopState,
    request: ExportRadciteReviewReportRequest,
) -> Result<RadciteReviewReportExport, ReviewActionError> {
    let review = load_review_response(state, request.document_id).await?;
    let cited_paragraph_count = review
        .paragraphs
        .iter()
        .filter(|paragraph| !paragraph.citations.is_empty())
        .count();
    let citation_count = review
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.citations.len())
        .sum::<usize>();
    let citation_coverage = if review.paragraphs.is_empty() {
        "0.0%".to_string()
    } else {
        format!(
            "{:.1}%",
            cited_paragraph_count as f64 / review.paragraphs.len() as f64 * 100.0
        )
    };
    let details = review
        .paragraphs
        .iter()
        .filter(|paragraph| paragraph.needs_citation || !paragraph.citations.is_empty())
        .map(|paragraph| RadciteReviewReportDetail {
            index: paragraph.order_index.max(0) as usize,
            page: paragraph.page,
            text_preview: review_report_text_preview(&paragraph.text),
            citations: paragraph
                .citations
                .iter()
                .map(|citation| citation.text.clone())
                .collect(),
            needs_citation: paragraph.needs_citation,
            is_table: paragraph.is_table,
        })
        .collect::<Vec<_>>();
    let report = RadciteReviewReport {
        filename: review.original_filename.clone(),
        file_type: review.source_file_type.clone(),
        project_title: review.project_title.clone(),
        document_id: review.document_id,
        generated_at: Utc::now().to_rfc3339(),
        statistics: RadciteReviewReportStatistics {
            total_paragraphs: review.paragraphs.len(),
            paragraphs_with_citations: cited_paragraph_count,
            paragraphs_needing_citations: review
                .paragraphs
                .iter()
                .filter(|paragraph| paragraph.needs_citation)
                .count(),
            total_citations: citation_count,
            citation_coverage,
        },
        details,
    };
    let json = serde_json::to_string_pretty(&report)?;
    let original_path = PathBuf::from(&review.original_filename);
    let source_stem = original_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("radcite-review");

    Ok(RadciteReviewReportExport {
        filename: format!("{}-citation-report.json", filename_slug(source_stem)),
        content_type: "application/json; charset=utf-8".to_string(),
        json,
    })
}

pub async fn list_course_references(
    state: &DesktopState,
    request: ListCourseReferencesRequest,
) -> Result<Vec<CourseReferenceSummary>, CourseReferenceError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let references = load_course_reference_entries(state, project.id).await?;

    Ok(references
        .into_iter()
        .map(course_reference_summary)
        .collect())
}

pub async fn add_course_reference(
    state: &DesktopState,
    request: AddCourseReferenceRequest,
) -> Result<CourseReferenceSummary, CourseReferenceError> {
    let apa_citation = request.apa_citation.trim();
    if apa_citation.is_empty() {
        return Err(CourseReferenceError::EmptyReferenceText);
    }

    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let reference_repo = SqliteReferenceEntryRepository::new(state.database_pool.clone());
    if let Some(existing_reference) =
        find_existing_course_reference(&reference_repo, project.id, apa_citation).await?
    {
        return Ok(course_reference_summary(existing_reference));
    }

    let mut reference = ReferenceEntry::new(project.id, ReferenceEntryType::Reference);
    reference.apa_citation = Some(apa_citation.to_string());
    reference.notes = request
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    apply_basic_apa_validation(&mut reference);

    reference_repo.insert_reference_entry(&reference).await?;

    Ok(course_reference_summary(reference))
}

pub async fn update_course_reference(
    state: &DesktopState,
    request: UpdateCourseReferenceRequest,
) -> Result<CourseReferenceSummary, CourseReferenceError> {
    let apa_citation = request.apa_citation.trim();
    if apa_citation.is_empty() {
        return Err(CourseReferenceError::EmptyReferenceText);
    }

    let mut reference = load_course_reference_or_error(state, request.reference_id).await?;
    reference.apa_citation = Some(apa_citation.to_string());
    reference.notes = trimmed_optional(request.notes);
    if let Some(citation_text) = request.citation_text {
        reference.citation_text = trimmed_optional(Some(citation_text));
    }
    if let Some(url) = request.url {
        reference.url = trimmed_optional(Some(url));
    }
    apply_basic_apa_validation(&mut reference);
    reference.updated_at = Utc::now();

    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .update_reference_entry(&reference)
        .await?;

    Ok(course_reference_summary(reference))
}

pub async fn archive_course_reference(
    state: &DesktopState,
    request: ArchiveCourseReferenceRequest,
) -> Result<CourseReferenceSummary, CourseReferenceError> {
    let reference = load_course_reference_or_error(state, request.reference_id).await?;
    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .archive_reference_entry(reference.id)
        .await?;

    Ok(course_reference_summary(reference))
}

pub async fn merge_course_references(
    state: &DesktopState,
    request: MergeCourseReferencesRequest,
) -> Result<CourseReferenceSummary, CourseReferenceError> {
    if request.merge_reference_ids.is_empty() {
        return Err(CourseReferenceError::InvalidMerge(
            "select at least one other course reference to merge".to_string(),
        ));
    }
    if request.merge_reference_ids.len() > 10 {
        return Err(CourseReferenceError::InvalidMerge(
            "choose no more than 10 duplicate references at a time".to_string(),
        ));
    }
    if request
        .merge_reference_ids
        .contains(&request.primary_reference_id)
    {
        return Err(CourseReferenceError::InvalidMerge(
            "the primary reference must be different from the references being merged".to_string(),
        ));
    }
    if request
        .merge_reference_ids
        .iter()
        .enumerate()
        .any(|(index, reference_id)| request.merge_reference_ids[..index].contains(reference_id))
    {
        return Err(CourseReferenceError::InvalidMerge(
            "a duplicate reference can only be selected once".to_string(),
        ));
    }

    let mut primary = load_course_reference_or_error(state, request.primary_reference_id).await?;
    let mut duplicates = Vec::with_capacity(request.merge_reference_ids.len());
    for reference_id in &request.merge_reference_ids {
        let duplicate = load_course_reference_or_error(state, *reference_id).await?;
        if duplicate.project_id != primary.project_id {
            return Err(CourseReferenceError::InvalidMerge(
                "course references must belong to the same project".to_string(),
            ));
        }
        duplicates.push(duplicate);
    }

    for duplicate in &duplicates {
        fill_missing_course_reference_metadata(&mut primary, duplicate);
    }
    apply_basic_apa_validation(&mut primary);
    primary.updated_at = Utc::now();

    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .merge_reference_entries(&primary, &request.merge_reference_ids)
        .await?;

    Ok(course_reference_summary(primary))
}

async fn find_existing_course_reference(
    reference_repo: &SqliteReferenceEntryRepository,
    project_id: ProjectId,
    apa_citation: &str,
) -> Result<Option<ReferenceEntry>, CourseReferenceError> {
    let import_key = normalised_reference_identity(apa_citation);
    let references = reference_repo
        .list_reference_entries_for_project(project_id, ReferenceEntryType::Reference)
        .await?;

    Ok(references.into_iter().find(|reference| {
        reference
            .apa_citation
            .as_deref()
            .or(reference.citation_text.as_deref())
            .map(normalised_reference_identity)
            .is_some_and(|existing_key| existing_key == import_key)
    }))
}

fn fill_missing_course_reference_metadata(
    primary: &mut ReferenceEntry,
    duplicate: &ReferenceEntry,
) {
    if primary.citation_text.is_none() {
        primary.citation_text = duplicate.citation_text.clone();
    }
    if primary.apa_citation.is_none() {
        primary.apa_citation = duplicate.apa_citation.clone();
    }
    if primary.title.is_none() {
        primary.title = duplicate.title.clone();
    }
    if primary.authors.is_empty() {
        primary.authors = duplicate.authors.clone();
    }
    if primary.publication_year.is_none() {
        primary.publication_year = duplicate.publication_year.clone();
    }
    if primary.source.is_none() {
        primary.source = duplicate.source.clone();
    }
    if primary.doi.is_none() {
        primary.doi = duplicate.doi.clone();
    }
    if primary.url.is_none() {
        primary.url = duplicate.url.clone();
    }
    if primary.notes.is_none() {
        primary.notes = duplicate.notes.clone();
    }
}

pub async fn list_radcite_modules(
    state: &DesktopState,
    request: ListRadciteModulesRequest,
) -> Result<Vec<CourseModuleSummary>, RadciteModuleError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let modules = SqliteCourseModuleRepository::new(state.database_pool.clone())
        .list_course_modules_for_project(project.id)
        .await?;

    Ok(modules.into_iter().map(course_module_summary).collect())
}

pub async fn add_radcite_module(
    state: &DesktopState,
    request: AddRadciteModuleRequest,
) -> Result<CourseModuleSummary, RadciteModuleError> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err(RadciteModuleError::EmptyTitle);
    }

    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let mut module = CourseModule::new(project.id, title, request.order_index);
    module.code = trimmed_optional(request.code);
    module.description = trimmed_optional(request.description);

    SqliteCourseModuleRepository::new(state.database_pool.clone())
        .insert_course_module(&module)
        .await?;

    Ok(course_module_summary(module))
}

pub async fn update_radcite_module(
    state: &DesktopState,
    request: UpdateRadciteModuleRequest,
) -> Result<CourseModuleSummary, RadciteModuleError> {
    let title = request.title.trim();
    if title.is_empty() {
        return Err(RadciteModuleError::EmptyTitle);
    }

    let mut module = load_radcite_module_or_error(state, request.module_id).await?;
    module.title = title.to_string();
    module.code = trimmed_optional(request.code);
    module.order_index = request.order_index;
    module.description = trimmed_optional(request.description);
    module.updated_at = Utc::now();

    SqliteCourseModuleRepository::new(state.database_pool.clone())
        .update_course_module(&module)
        .await?;

    Ok(course_module_summary(module))
}

pub async fn archive_radcite_module(
    state: &DesktopState,
    request: ArchiveRadciteModuleRequest,
) -> Result<CourseModuleSummary, RadciteModuleError> {
    let module = load_radcite_module_or_error(state, request.module_id).await?;
    SqliteCourseModuleRepository::new(state.database_pool.clone())
        .archive_course_module(module.id)
        .await?;

    Ok(course_module_summary(module))
}

pub async fn list_module_readings(
    state: &DesktopState,
    request: ListModuleReadingsRequest,
) -> Result<Vec<ModuleReadingSummary>, ModuleReadingError> {
    load_course_module_or_error(state, request.module_id).await?;

    let mut readings = SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .list_reference_entries_for_module(request.module_id, ReferenceEntryType::Reading)
        .await?;
    sort_module_reading_entries(&mut readings);

    Ok(readings
        .into_iter()
        .filter_map(module_reading_summary)
        .collect())
}

pub async fn add_module_reading(
    state: &DesktopState,
    request: AddModuleReadingRequest,
) -> Result<ModuleReadingSummary, ModuleReadingError> {
    let module = load_course_module_or_error(state, request.module_id).await?;
    let reading_category = parse_reading_category_request(&request.reading_category)?;
    let apa_citation = trimmed_optional(request.apa_citation);
    let citation_text = trimmed_optional(request.citation_text);

    if apa_citation.is_none() && citation_text.is_none() {
        return Err(ModuleReadingError::EmptyReadingText);
    }

    let reference_repo = SqliteReferenceEntryRepository::new(state.database_pool.clone());
    if let Some(mut existing_reading) = find_existing_module_reading(
        &reference_repo,
        module.id,
        apa_citation.as_deref(),
        citation_text.as_deref(),
        request.doi.as_deref(),
        request.url.as_deref(),
    )
    .await?
    {
        if reading_category == ReadingCategory::Compulsory
            && existing_reading.reading_category == Some(ReadingCategory::Optional)
        {
            existing_reading.reading_category = Some(ReadingCategory::Compulsory);
        }
        apply_basic_apa_validation(&mut existing_reading);
        reference_repo
            .update_reference_entry(&existing_reading)
            .await?;
        return module_reading_summary(existing_reading)
            .ok_or(ModuleReadingError::MissingModule(module.id));
    }

    let mut reading = ReferenceEntry::new(module.project_id, ReferenceEntryType::Reading);
    reading.module_id = Some(module.id);
    reading.reading_category = Some(reading_category);
    reading.lesson_code = trimmed_optional(request.lesson_code);
    reading.apa_citation = apa_citation;
    reading.citation_text = citation_text;
    reading.doi = trimmed_optional(request.doi);
    reading.url = trimmed_optional(request.url);
    reading.notes = trimmed_optional(request.notes);
    reading.reading_notes = trimmed_optional(request.reading_notes);
    reading.estimated_reading_time = trimmed_optional(request.estimated_reading_time);
    apply_basic_apa_validation(&mut reading);

    reference_repo.insert_reference_entry(&reading).await?;

    module_reading_summary(reading).ok_or(ModuleReadingError::MissingModule(module.id))
}

pub async fn update_module_reading(
    state: &DesktopState,
    request: UpdateModuleReadingRequest,
) -> Result<ModuleReadingSummary, ModuleReadingError> {
    let mut reading = load_module_reading_or_error(state, request.reading_id).await?;
    let module_id = reading
        .module_id
        .ok_or(ModuleReadingError::MissingReading(reading.id))?;
    load_course_module_or_error(state, module_id).await?;
    let reading_category = parse_reading_category_request(&request.reading_category)?;
    let apa_citation = trimmed_optional(request.apa_citation);
    let citation_text = trimmed_optional(request.citation_text);

    if apa_citation.is_none() && citation_text.is_none() {
        return Err(ModuleReadingError::EmptyReadingText);
    }

    reading.reading_category = Some(reading_category);
    reading.lesson_code = trimmed_optional(request.lesson_code);
    reading.apa_citation = apa_citation;
    reading.citation_text = citation_text;
    reading.doi = trimmed_optional(request.doi);
    reading.url = trimmed_optional(request.url);
    reading.notes = trimmed_optional(request.notes);
    reading.reading_notes = trimmed_optional(request.reading_notes);
    reading.estimated_reading_time = trimmed_optional(request.estimated_reading_time);
    apply_basic_apa_validation(&mut reading);
    reading.updated_at = Utc::now();

    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .update_reference_entry(&reading)
        .await?;

    module_reading_summary(reading).ok_or(ModuleReadingError::MissingReading(request.reading_id))
}

pub async fn archive_module_reading(
    state: &DesktopState,
    request: ArchiveModuleReadingRequest,
) -> Result<ModuleReadingSummary, ModuleReadingError> {
    let reading = load_module_reading_or_error(state, request.reading_id).await?;
    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .archive_reference_entry(reading.id)
        .await?;

    module_reading_summary(reading).ok_or(ModuleReadingError::MissingReading(request.reading_id))
}

pub async fn preview_module_readings_import(
    _state: &DesktopState,
    request: PreviewModuleReadingsImportRequest,
) -> Result<Vec<ModuleReadingImportCandidateSummary>, ModuleReadingImportError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(ModuleReadingImportError::EmptyPath);
    }

    let path = PathBuf::from(path);
    let original_filename = request
        .original_filename
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            path.file_name()
                .and_then(|filename| filename.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "module-readings.docx".to_string());

    let candidates = extract_docx_reading_candidates(DocxReadingExtractionRequest {
        path,
        original_filename,
    })?;

    Ok(candidates
        .into_iter()
        .map(module_reading_import_candidate_summary)
        .collect())
}

pub async fn preview_module_readings_csv_import(
    _state: &DesktopState,
    request: PreviewModuleReadingsCsvImportRequest,
) -> Result<Vec<ModuleReadingImportCandidateSummary>, ModuleReadingImportError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(ModuleReadingImportError::EmptyPath);
    }

    let path = PathBuf::from(path);
    let original_filename = request
        .original_filename
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            path.file_name()
                .and_then(|filename| filename.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "module-readings.csv".to_string());

    let candidates = extract_csv_reading_candidates(CsvReadingExtractionRequest {
        path,
        original_filename,
    })?;

    Ok(candidates
        .into_iter()
        .map(module_reading_import_candidate_summary)
        .collect())
}

pub async fn preview_module_readings_pdf_import(
    _state: &DesktopState,
    request: PreviewModuleReadingsPdfImportRequest,
) -> Result<ModuleReadingsPdfImportPreview, ModuleReadingImportError> {
    let paths = request
        .paths
        .into_iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    if paths.is_empty() {
        return Err(ModuleReadingImportError::EmptyPath);
    }

    let report = extract_pdf_reading_candidates_with_report(PdfReadingExtractionRequest { paths })?;

    Ok(ModuleReadingsPdfImportPreview {
        candidates: report
            .candidates
            .into_iter()
            .map(module_reading_import_candidate_summary)
            .collect(),
        failures: report
            .failures
            .into_iter()
            .map(|failure| ModuleReadingsPdfImportFailureSummary {
                path: failure.path.to_string_lossy().into_owned(),
                message: failure.message,
            })
            .collect(),
    })
}

pub async fn import_document_readings(
    state: &DesktopState,
    request: ImportDocumentReadingsRequest,
) -> Result<ImportDocumentReadingsResponse, ModuleReadingImportError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(ModuleReadingImportError::EmptyPath);
    }

    let path = PathBuf::from(path);
    let (candidates, failed_file_count) = match request.source_file_type {
        DocumentFileType::Docx => {
            let original_filename = path
                .file_name()
                .and_then(|filename| filename.to_str())
                .unwrap_or("module-readings.docx")
                .to_string();
            (
                extract_docx_reading_candidates(DocxReadingExtractionRequest {
                    path: path.clone(),
                    original_filename,
                })?,
                0,
            )
        }
        DocumentFileType::Pdf => {
            let report = extract_pdf_reading_candidates_with_report(PdfReadingExtractionRequest {
                paths: vec![path.clone()],
            })?;
            (report.candidates, report.failures.len())
        }
    };

    let candidate_count = candidates.len();
    let project = load_requested_or_local_radcite_project(state, request.project_id)
        .await
        .map_err(|error| match error {
            RadciteProjectLookupError::MissingProject(project_id) => {
                ModuleReadingImportError::MissingProject(project_id)
            }
            RadciteProjectLookupError::Database(error) => ModuleReadingImportError::Database(error),
        })?;
    let module_repo = SqliteCourseModuleRepository::new(state.database_pool.clone());
    let mut modules = module_repo
        .list_course_modules_for_project(project.id)
        .await?;
    let mut modules_to_insert = Vec::new();
    let mut created_module_count = 0;
    let mut unassigned_count = 0;
    let mut save_candidates = Vec::new();

    for candidate in candidates {
        let candidate_path = candidate
            .source_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.clone());
        let (path_module_order, path_module_title) = infer_module_from_path(&candidate_path);
        let module_order = candidate.module_order.or(path_module_order);
        let module_title = candidate.module_title.clone().or(path_module_title);
        let module = find_import_module(&modules, module_order, module_title.as_deref())
            .or_else(|| {
                (module_order.is_none() && module_title.is_none() && modules.len() == 1)
                    .then(|| modules.first().cloned())
                    .flatten()
            })
            .or_else(|| {
                let has_module_signal = module_order.is_some()
                    || module_title
                        .as_deref()
                        .is_some_and(|title| !title.trim().is_empty());
                if !has_module_signal {
                    return None;
                }

                let title = module_title
                    .as_deref()
                    .filter(|title| !title.trim().is_empty())
                    .map(str::trim)
                    .map(canonical_import_module_title)
                    .or_else(|| module_order.map(|order| format!("Module {order}")))?;
                let module = CourseModule::new(project.id, title, module_order);
                // The in-memory list is updated immediately so later candidates in the same
                // document resolve to the module just created.
                modules.push(module.clone());
                modules_to_insert.push(module.clone());
                created_module_count += 1;
                Some(module)
            });

        let Some(module) = module else {
            unassigned_count += 1;
            continue;
        };

        let source_filename = candidate.source_filename.clone().or_else(|| {
            candidate_path
                .file_name()
                .and_then(|filename| filename.to_str())
                .map(str::to_string)
        });
        let notes = source_filename
            .as_deref()
            .map(|filename| format!("Imported from {filename}"));

        save_candidates.push(SaveModuleReadingsImportCandidate {
            module_id: module.id,
            reading_category: reading_category_label(Some(candidate.reading_category)).to_string(),
            lesson_code: candidate.lesson_code,
            apa_citation: Some(candidate.apa_citation),
            citation_text: candidate.citation_text,
            doi: candidate.doi,
            url: candidate.url,
            notes,
            reading_notes: None,
            estimated_reading_time: None,
        });
    }

    for module in modules_to_insert {
        module_repo.insert_course_module(&module).await?;
    }

    let saved_count = save_module_readings_import(
        state,
        SaveModuleReadingsImportRequest {
            candidates: save_candidates,
        },
    )
    .await?
    .len();

    Ok(ImportDocumentReadingsResponse {
        candidate_count,
        saved_count,
        created_module_count,
        unassigned_count,
        failed_file_count,
    })
}

pub async fn save_module_readings_import(
    state: &DesktopState,
    request: SaveModuleReadingsImportRequest,
) -> Result<Vec<ModuleReadingSummary>, ModuleReadingImportError> {
    let mut saved_readings = Vec::new();
    let reference_repo = SqliteReferenceEntryRepository::new(state.database_pool.clone());

    for candidate in request.candidates {
        let module = load_course_module_for_import_or_error(state, candidate.module_id).await?;
        let reading_category = parse_reading_category_import_request(&candidate.reading_category)?;
        let apa_citation = trimmed_optional(candidate.apa_citation);
        let citation_text = trimmed_optional(candidate.citation_text);

        if apa_citation.is_none() && citation_text.is_none() {
            return Err(ModuleReadingImportError::EmptyReadingText);
        }

        if let Some(mut existing_reading) = find_existing_module_reading(
            &reference_repo,
            module.id,
            apa_citation.as_deref(),
            citation_text.as_deref(),
            candidate.doi.as_deref(),
            candidate.url.as_deref(),
        )
        .await?
        {
            if reading_category == ReadingCategory::Compulsory
                && existing_reading.reading_category == Some(ReadingCategory::Optional)
            {
                existing_reading.reading_category = Some(ReadingCategory::Compulsory);
            }
            apply_basic_apa_validation(&mut existing_reading);
            reference_repo
                .update_reference_entry(&existing_reading)
                .await?;
            saved_readings.push(
                module_reading_summary(existing_reading)
                    .ok_or(ModuleReadingImportError::MissingModule(module.id))?,
            );
            continue;
        }

        let mut reading = ReferenceEntry::new(module.project_id, ReferenceEntryType::Reading);
        reading.module_id = Some(module.id);
        reading.reading_category = Some(reading_category);
        reading.lesson_code = trimmed_optional(candidate.lesson_code);
        reading.apa_citation = apa_citation;
        reading.citation_text = citation_text;
        reading.doi = trimmed_optional(candidate.doi);
        reading.url = trimmed_optional(candidate.url);
        reading.notes = trimmed_optional(candidate.notes);
        reading.reading_notes = trimmed_optional(candidate.reading_notes);
        reading.estimated_reading_time = trimmed_optional(candidate.estimated_reading_time);
        apply_basic_apa_validation(&mut reading);

        reference_repo.insert_reference_entry(&reading).await?;

        saved_readings.push(
            module_reading_summary(reading)
                .ok_or(ModuleReadingImportError::MissingModule(module.id))?,
        );
    }

    Ok(saved_readings)
}

fn find_import_module(
    modules: &[CourseModule],
    module_order: Option<i32>,
    module_title: Option<&str>,
) -> Option<CourseModule> {
    module_order
        .and_then(|order| {
            modules
                .iter()
                .find(|module| module.order_index == Some(order))
                .cloned()
        })
        .or_else(|| {
            let title = module_title?.trim();
            (!title.is_empty()).then(|| {
                modules
                    .iter()
                    .find(|module| {
                        normalised_reference_identity(&module.title)
                            == normalised_reference_identity(title)
                    })
                    .cloned()
            })?
        })
}

fn infer_module_from_path(path: &std::path::Path) -> (Option<i32>, Option<String>) {
    let value = path.to_string_lossy();
    let Some(captures) = Regex::new(r"(?i)\b(module|week)[\s_-]*(\d{1,3})\b")
        .ok()
        .and_then(|regex| regex.captures(&value))
    else {
        return (None, None);
    };

    let order = captures
        .get(2)
        .and_then(|value| value.as_str().parse::<i32>().ok());
    let title = order.map(|order| {
        if captures
            .get(1)
            .is_some_and(|value| value.as_str().eq_ignore_ascii_case("week"))
        {
            format!("Week {order}")
        } else {
            format!("Module {order}")
        }
    });
    (order, title)
}

fn canonical_import_module_title(title: &str) -> String {
    let trimmed = title.trim();
    let Some(captures) = Regex::new(r"(?i)^(module|week)\s+(\d+)$")
        .ok()
        .and_then(|regex| regex.captures(trimmed))
    else {
        return trimmed.to_string();
    };

    let label = captures
        .get(1)
        .map(|value| value.as_str().to_ascii_lowercase())
        .unwrap_or_else(|| "module".to_string());
    let order = captures
        .get(2)
        .map(|value| value.as_str())
        .unwrap_or_default();
    let label = if label == "week" { "Week" } else { "Module" };
    format!("{label} {order}")
}

async fn find_existing_module_reading(
    reference_repo: &SqliteReferenceEntryRepository,
    module_id: ModuleId,
    apa_citation: Option<&str>,
    citation_text: Option<&str>,
    doi: Option<&str>,
    url: Option<&str>,
) -> Result<Option<ReferenceEntry>, DbError> {
    let Some(import_key) = module_reading_import_identity(apa_citation, citation_text, doi, url)
    else {
        return Ok(None);
    };

    let existing_readings = reference_repo
        .list_reference_entries_for_module(module_id, ReferenceEntryType::Reading)
        .await?;

    Ok(existing_readings.into_iter().find(|reading| {
        module_reading_entry_identity(reading)
            .as_ref()
            .is_some_and(|existing_key| existing_key == &import_key)
    }))
}

pub async fn export_course_references(
    state: &DesktopState,
    request: ExportCourseReferencesRequest,
) -> Result<CourseReferencesExport, CourseReferenceExportError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let references = load_course_reference_entries(state, project.id).await?;
    let reference_count = references.len();
    let apa_error_count = references
        .iter()
        .filter(|reference| reference.apa_validation_status == ApaValidationStatus::NeedsFix)
        .count();
    let apa_warning_count = references
        .iter()
        .filter(|reference| reference.apa_validation_status == ApaValidationStatus::Unknown)
        .count();
    if apa_error_count > 0 && !request.allow_incomplete {
        return Err(CourseReferenceExportError::ApaValidation {
            count: apa_error_count,
        });
    }
    let html = format_course_references_html(
        &references,
        request.for_ako_learn,
        request.use_library_links,
    );

    Ok(CourseReferencesExport {
        filename: format!(
            "{}-course-references.html",
            filename_slug(project.code.as_deref().unwrap_or(&project.title))
        ),
        content_type: "text/html; charset=utf-8".to_string(),
        html,
        reference_count,
        apa_error_count,
        apa_warning_count,
    })
}

pub async fn export_module_readings(
    state: &DesktopState,
    request: ExportModuleReadingsRequest,
) -> Result<ModuleReadingsExport, ModuleReadingExportError> {
    let module = SqliteCourseModuleRepository::new(state.database_pool.clone())
        .load_course_module(request.module_id)
        .await?
        .ok_or(ModuleReadingExportError::MissingModule(request.module_id))?;
    let project = SqliteProjectRepository::new(state.database_pool.clone())
        .load_project(module.project_id)
        .await?;
    let readings = SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .list_reference_entries_for_module(module.id, ReferenceEntryType::Reading)
        .await?;
    let mut readings =
        filter_references_by_document_exclusion(state, module.project_id, readings).await?;
    sort_module_reading_entries(&mut readings);
    let reading_count = readings.len();
    let html =
        format_module_readings_html(&readings, request.for_ako_learn, request.use_library_links);
    let project_label = project
        .as_ref()
        .map(|project| project.code.as_deref().unwrap_or(&project.title))
        .unwrap_or("radcite");
    let module_label = module.code.as_deref().unwrap_or(&module.title);

    Ok(ModuleReadingsExport {
        filename: format!(
            "{}-module-readings.html",
            filename_slug(&format!("{project_label}-{module_label}"))
        ),
        content_type: "text/html; charset=utf-8".to_string(),
        html,
        module_id: module.id,
        reading_count,
    })
}

pub async fn mark_paragraph_resolved_for_review(
    state: &DesktopState,
    request: UpdateParagraphReviewRequest,
) -> Result<AnalyseDocxReviewResponse, ReviewActionError> {
    SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .mark_paragraph_resolved(request.paragraph_id)
        .await?;

    load_review_response(state, request.document_id).await
}

pub async fn verify_paragraph_citations_for_review(
    state: &DesktopState,
    request: UpdateParagraphReviewRequest,
) -> Result<AnalyseDocxReviewResponse, ReviewActionError> {
    SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .verify_paragraph_citations(request.paragraph_id)
        .await?;

    load_review_response(state, request.document_id).await
}

pub async fn add_manual_citation_for_review(
    state: &DesktopState,
    request: AddManualCitationRequest,
) -> Result<AnalyseDocxReviewResponse, ReviewActionError> {
    let citation_text = request.citation_text.trim();
    if citation_text.is_empty() {
        return Err(ReviewActionError::EmptyCitationText);
    }

    SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .insert_manual_citation(request.paragraph_id, citation_text)
        .await?;

    load_review_response(state, request.document_id).await
}

pub async fn link_citation_to_reference_for_review(
    state: &DesktopState,
    request: LinkCitationReferenceRequest,
) -> Result<AnalyseDocxReviewResponse, ReviewActionError> {
    SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .link_citation_to_reference(request.citation_id, request.reference_entry_id)
        .await?;

    load_review_response(state, request.document_id).await
}

#[derive(Debug)]
struct DesktopAnalysedDocument {
    project: Project,
    document: Document,
    paragraphs: Vec<Paragraph>,
    citations: Vec<Citation>,
}

async fn analyse_docx(
    state: &DesktopState,
    request: AnalyseDocxRequest,
) -> Result<DesktopAnalysedDocument, AnalyseDocxError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(AnalyseDocxError::EmptyPath);
    }

    let path = PathBuf::from(path);
    let original_filename = request
        .original_filename
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            path.file_name()
                .and_then(|filename| filename.to_str())
                .map(str::to_string)
        })
        .ok_or(AnalyseDocxError::MissingFilename)?;

    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    validate_source(&path)?;

    let mut analysed = ingest_docx(DocxIngestionRequest {
        project_id: project.id,
        path: path.clone(),
        original_filename,
    })?;

    let managed_path = store_source(
        &state.paths.data_dir,
        project.id.0,
        analysed.document.id.0,
        &path,
        &analysed.document.original_filename,
    )?;
    analysed.document.source_path = Some(managed_path.to_string_lossy().into_owned());

    SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .insert_document_analysis(
            &analysed.document,
            &analysed.paragraphs,
            &analysed.citations,
        )
        .await?;

    Ok(DesktopAnalysedDocument {
        project,
        document: analysed.document,
        paragraphs: analysed.paragraphs,
        citations: analysed.citations,
    })
}

async fn analyse_pdf(
    state: &DesktopState,
    request: AnalysePdfRequest,
) -> Result<DesktopAnalysedDocument, AnalysePdfError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(AnalysePdfError::EmptyPath);
    }

    let path = PathBuf::from(path);
    let original_filename = request
        .original_filename
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            path.file_name()
                .and_then(|filename| filename.to_str())
                .map(str::to_string)
        })
        .ok_or(AnalysePdfError::MissingFilename)?;

    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    validate_source(&path)?;

    let mut analysed = ingest_pdf(PdfIngestionRequest {
        project_id: project.id,
        path: path.clone(),
        original_filename,
    })?;

    let managed_path = store_source(
        &state.paths.data_dir,
        project.id.0,
        analysed.document.id.0,
        &path,
        &analysed.document.original_filename,
    )?;
    analysed.document.source_path = Some(managed_path.to_string_lossy().into_owned());

    SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .insert_document_analysis(
            &analysed.document,
            &analysed.paragraphs,
            &analysed.citations,
        )
        .await?;

    Ok(DesktopAnalysedDocument {
        project,
        document: analysed.document,
        paragraphs: analysed.paragraphs,
        citations: analysed.citations,
    })
}

async fn load_review_response(
    state: &DesktopState,
    document_id: DocumentId,
) -> Result<AnalyseDocxReviewResponse, ReviewActionError> {
    let document_repo = SqliteCitationDocumentRepository::new(state.database_pool.clone());
    let analysis = document_repo
        .load_document_analysis(document_id)
        .await?
        .ok_or(ReviewActionError::MissingDocument(document_id))?;

    let project = SqliteProjectRepository::new(state.database_pool.clone())
        .load_project(analysis.document.project_id)
        .await?
        .ok_or(ReviewActionError::MissingProject(
            analysis.document.project_id,
        ))?;

    let references = load_course_reference_entries(state, project.id).await?;
    let paragraphs = build_review_paragraphs(
        analysis.paragraphs,
        analysis.citations,
        references.as_slice(),
    );
    let summary = build_review_summary(&paragraphs);

    Ok(AnalyseDocxReviewResponse {
        project_id: project.id,
        project_title: project.title,
        document_id: analysis.document.id,
        original_filename: analysis.document.original_filename.clone(),
        display_name: effective_document_display_name(&analysis.document),
        source_path: analysis.document.source_path,
        source_file_type: document_file_type_label(analysis.document.file_type).to_string(),
        doc_variant: document_variant_label(analysis.document.doc_variant).to_string(),
        doc_number: analysis.document.doc_number,
        exclude_from_references: analysis.document.exclude_from_references,
        summary,
        paragraphs,
    })
}

async fn load_or_create_local_radcite_project(state: &DesktopState) -> Result<Project, DbError> {
    let project_repo = SqliteProjectRepository::new(state.database_pool.clone());

    if let Some(project) = project_repo
        .load_project_by_code(LOCAL_RADCITE_PROJECT_CODE)
        .await?
    {
        return Ok(project);
    }

    let project = Project::new(
        LOCAL_RADCITE_PROJECT_CODE,
        LOCAL_RADCITE_PROJECT_TITLE,
        UserId::new(),
    );
    project_repo.insert_project(&project).await?;

    Ok(project)
}

async fn load_requested_or_local_radcite_project(
    state: &DesktopState,
    project_id: Option<ProjectId>,
) -> Result<Project, RadciteProjectLookupError> {
    let Some(project_id) = project_id else {
        return Ok(load_or_create_local_radcite_project(state).await?);
    };

    SqliteProjectRepository::new(state.database_pool.clone())
        .load_project(project_id)
        .await?
        .ok_or(RadciteProjectLookupError::MissingProject(project_id))
}

async fn filter_references_by_document_exclusion(
    state: &DesktopState,
    project_id: ProjectId,
    references: Vec<ReferenceEntry>,
) -> Result<Vec<ReferenceEntry>, DbError> {
    let excluded_document_ids = SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .list_documents_for_project(project_id)
        .await?
        .into_iter()
        .filter(|document| document.exclude_from_references)
        .map(|document| document.document_id)
        .collect::<HashSet<_>>();

    Ok(references
        .into_iter()
        .filter(|reference| {
            reference
                .document_id
                .is_none_or(|document_id| !excluded_document_ids.contains(&document_id))
        })
        .collect())
}

async fn load_course_reference_entries(
    state: &DesktopState,
    project_id: ProjectId,
) -> Result<Vec<ReferenceEntry>, DbError> {
    let references = SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .list_reference_entries_for_project(project_id, ReferenceEntryType::Reference)
        .await?;

    filter_references_by_document_exclusion(state, project_id, references).await
}

async fn load_course_reference_or_error(
    state: &DesktopState,
    reference_id: ReferenceEntryId,
) -> Result<ReferenceEntry, CourseReferenceError> {
    let reference = SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .load_reference_entry(reference_id)
        .await?
        .ok_or(CourseReferenceError::MissingReference(reference_id))?;

    if reference.reference_type != ReferenceEntryType::Reference {
        return Err(CourseReferenceError::MissingReference(reference_id));
    }

    Ok(reference)
}

fn radcite_project_summary(project: Project) -> RadciteProjectSummary {
    RadciteProjectSummary {
        id: project.id,
        code: project.code,
        title: project.title,
        description: project.description,
        structure_mode: project.structure_mode,
        archived_at: project.archived_at.map(|value| value.to_rfc3339()),
    }
}

fn course_reference_summary(reference: ReferenceEntry) -> CourseReferenceSummary {
    CourseReferenceSummary {
        id: reference.id,
        project_id: reference.project_id,
        reference_type: reference_type_label(reference.reference_type).to_string(),
        apa_citation: reference.apa_citation,
        citation_text: reference.citation_text,
        title: reference.title,
        authors: reference.authors,
        publication_year: reference.publication_year,
        source: reference.source,
        doi: reference.doi,
        url: reference.url,
        notes: reference.notes,
        validation_status: validation_status_label(reference.apa_validation_status).to_string(),
        validation_report: reference.apa_validation_report,
    }
}

fn course_module_summary(module: CourseModule) -> CourseModuleSummary {
    CourseModuleSummary {
        id: module.id,
        project_id: module.project_id,
        code: module.code,
        title: module.title,
        order_index: module.order_index,
        description: module.description,
    }
}

fn module_reading_summary(reading: ReferenceEntry) -> Option<ModuleReadingSummary> {
    Some(ModuleReadingSummary {
        id: reading.id,
        project_id: reading.project_id,
        module_id: reading.module_id?,
        reading_category: reading_category_label(reading.reading_category).to_string(),
        lesson_code: reading.lesson_code,
        apa_citation: reading.apa_citation,
        citation_text: reading.citation_text,
        title: reading.title,
        doi: reading.doi,
        url: reading.url,
        notes: reading.notes,
        reading_notes: reading.reading_notes,
        estimated_reading_time: reading.estimated_reading_time,
        validation_status: validation_status_label(reading.apa_validation_status).to_string(),
        validation_report: reading.apa_validation_report,
    })
}

fn sort_module_reading_entries(readings: &mut [ReferenceEntry]) {
    readings.sort_by(compare_module_readings);
}

fn compare_module_readings(left: &ReferenceEntry, right: &ReferenceEntry) -> Ordering {
    reading_category_rank(left.reading_category)
        .cmp(&reading_category_rank(right.reading_category))
        .then_with(|| {
            compare_lesson_codes(left.lesson_code.as_deref(), right.lesson_code.as_deref())
        })
        .then_with(|| {
            left.display_order
                .unwrap_or(i32::MAX)
                .cmp(&right.display_order.unwrap_or(i32::MAX))
        })
        .then_with(|| module_reading_sort_text(left).cmp(&module_reading_sort_text(right)))
        .then_with(|| left.id.0.to_string().cmp(&right.id.0.to_string()))
}

fn reading_category_rank(category: Option<ReadingCategory>) -> u8 {
    match category {
        Some(ReadingCategory::Compulsory) => 0,
        Some(ReadingCategory::Optional) => 1,
        None => 2,
    }
}

fn compare_lesson_codes(left: Option<&str>, right: Option<&str>) -> Ordering {
    lesson_code_sort_key(left).cmp(&lesson_code_sort_key(right))
}

fn lesson_code_sort_key(value: Option<&str>) -> (u8, Vec<LessonSortToken>, String) {
    let normalized = value.unwrap_or_default().trim().to_lowercase();
    if normalized.is_empty() {
        return (1, Vec::new(), String::new());
    }

    (0, lesson_sort_tokens(&normalized), normalized)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum LessonSortToken {
    Number(u64),
    Text(String),
}

fn lesson_sort_tokens(value: &str) -> Vec<LessonSortToken> {
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut reading_number: Option<bool> = None;

    for character in value.chars() {
        if !character.is_alphanumeric() {
            flush_lesson_sort_token(&mut tokens, &mut buffer, &mut reading_number);
            continue;
        }

        let is_number = character.is_ascii_digit();
        if reading_number.is_some_and(|current| current != is_number) {
            flush_lesson_sort_token(&mut tokens, &mut buffer, &mut reading_number);
        }

        reading_number = Some(is_number);
        buffer.push(character);
    }

    flush_lesson_sort_token(&mut tokens, &mut buffer, &mut reading_number);
    tokens
}

fn flush_lesson_sort_token(
    tokens: &mut Vec<LessonSortToken>,
    buffer: &mut String,
    reading_number: &mut Option<bool>,
) {
    if buffer.is_empty() {
        *reading_number = None;
        return;
    }

    if reading_number.unwrap_or(false) {
        tokens.push(LessonSortToken::Number(buffer.parse().unwrap_or(u64::MAX)));
    } else {
        tokens.push(LessonSortToken::Text(std::mem::take(buffer)));
        *reading_number = None;
        return;
    }

    buffer.clear();
    *reading_number = None;
}

fn module_reading_sort_text(reading: &ReferenceEntry) -> String {
    normalised_reference_identity(
        reading
            .apa_citation
            .as_deref()
            .or(reading.citation_text.as_deref())
            .or(reading.title.as_deref())
            .unwrap_or_default(),
    )
}

fn module_reading_import_candidate_summary(
    candidate: ReadingImportCandidate,
) -> ModuleReadingImportCandidateSummary {
    ModuleReadingImportCandidateSummary {
        source_path: candidate.source_path,
        source_filename: candidate.source_filename,
        module_order: candidate.module_order,
        module_title: candidate.module_title,
        reading_category: reading_category_label(Some(candidate.reading_category)).to_string(),
        lesson_code: candidate.lesson_code,
        apa_citation: candidate.apa_citation,
        citation_text: candidate.citation_text,
        doi: candidate.doi,
        url: candidate.url,
    }
}

fn document_file_type_label(file_type: DocumentFileType) -> &'static str {
    match file_type {
        DocumentFileType::Docx => "docx",
        DocumentFileType::Pdf => "pdf",
    }
}

fn document_variant_label(variant: DocumentVariant) -> &'static str {
    match variant {
        DocumentVariant::Content => "content",
        DocumentVariant::Rise => "rise",
        DocumentVariant::Other => "other",
    }
}

fn effective_document_display_name(document: &Document) -> String {
    document
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&document.original_filename)
        .to_string()
}

fn saved_review_summary_from_analysis(
    document: &Document,
    paragraphs: &[Paragraph],
    citations: &[Citation],
) -> SavedRadciteReviewSummary {
    SavedRadciteReviewSummary {
        document_id: document.id,
        project_id: document.project_id,
        original_filename: document.original_filename.clone(),
        display_name: effective_document_display_name(document),
        source_path: document.source_path.clone(),
        source_file_type: document_file_type_label(document.file_type).to_string(),
        doc_variant: document_variant_label(document.doc_variant).to_string(),
        doc_number: document.doc_number,
        exclude_from_references: document.exclude_from_references,
        paragraph_count: paragraphs.len(),
        citation_count: citations.len(),
        missing_citation_count: paragraphs
            .iter()
            .filter(|paragraph| paragraph.needs_citation)
            .count(),
    }
}

fn reference_type_label(reference_type: ReferenceEntryType) -> &'static str {
    match reference_type {
        ReferenceEntryType::Reference => "reference",
        ReferenceEntryType::Reading => "reading",
    }
}

fn reading_category_label(reading_category: Option<ReadingCategory>) -> &'static str {
    match reading_category {
        Some(ReadingCategory::Compulsory) | None => "compulsory",
        Some(ReadingCategory::Optional) => "optional",
    }
}

fn module_reading_import_identity(
    apa_citation: Option<&str>,
    citation_text: Option<&str>,
    doi: Option<&str>,
    url: Option<&str>,
) -> Option<String> {
    let reading_text = apa_citation.or(citation_text)?;
    Some(canonical_reading_identity(reading_text, doi, url))
}

fn module_reading_entry_identity(reading: &ReferenceEntry) -> Option<String> {
    module_reading_import_identity(
        reading.apa_citation.as_deref(),
        reading.citation_text.as_deref(),
        reading.doi.as_deref(),
        reading.url.as_deref(),
    )
}

fn canonical_reading_identity(value: &str, doi: Option<&str>, url: Option<&str>) -> String {
    if let Some(doi) = doi.map(canonical_reading_doi).filter(|doi| !doi.is_empty()) {
        return format!("doi:{doi}");
    }
    let citation = normalised_reading_identity(value);
    if !citation.is_empty() {
        return citation;
    }
    if let Some(url) = url.map(canonical_reading_url).filter(|url| !url.is_empty()) {
        return format!("url:{url}");
    }
    String::new()
}

fn canonical_reading_doi(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(['.', ',', ';', ')', ']'])
        .to_lowercase()
        .trim_start_matches("https://doi.org/")
        .trim_start_matches("http://doi.org/")
        .trim_start_matches("doi:")
        .to_string()
}

fn canonical_reading_url(value: &str) -> String {
    normalise_reference_url(value)
        .trim()
        .trim_end_matches(['.', ',', ';', ')', ']'])
        .to_lowercase()
        .trim_end_matches('/')
        .to_string()
}

fn normalised_reading_identity(value: &str) -> String {
    normalised_reference_identity(value)
}

fn normalised_reference_identity(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn parse_reading_category_request(value: &str) -> Result<ReadingCategory, ModuleReadingError> {
    match value.trim() {
        "compulsory" | "required" => Ok(ReadingCategory::Compulsory),
        "optional" => Ok(ReadingCategory::Optional),
        other => Err(ModuleReadingError::InvalidCategory(other.to_string())),
    }
}

fn parse_reading_category_import_request(
    value: &str,
) -> Result<ReadingCategory, ModuleReadingImportError> {
    match value.trim() {
        "compulsory" | "required" => Ok(ReadingCategory::Compulsory),
        "optional" => Ok(ReadingCategory::Optional),
        other => Err(ModuleReadingImportError::InvalidCategory(other.to_string())),
    }
}

async fn load_course_module_or_error(
    state: &DesktopState,
    module_id: ModuleId,
) -> Result<CourseModule, ModuleReadingError> {
    SqliteCourseModuleRepository::new(state.database_pool.clone())
        .load_course_module(module_id)
        .await?
        .ok_or(ModuleReadingError::MissingModule(module_id))
}

async fn load_course_module_for_import_or_error(
    state: &DesktopState,
    module_id: ModuleId,
) -> Result<CourseModule, ModuleReadingImportError> {
    SqliteCourseModuleRepository::new(state.database_pool.clone())
        .load_course_module(module_id)
        .await?
        .ok_or(ModuleReadingImportError::MissingModule(module_id))
}

async fn load_radcite_module_or_error(
    state: &DesktopState,
    module_id: ModuleId,
) -> Result<CourseModule, RadciteModuleError> {
    SqliteCourseModuleRepository::new(state.database_pool.clone())
        .load_course_module(module_id)
        .await?
        .ok_or(RadciteModuleError::MissingModule(module_id))
}

async fn load_module_reading_or_error(
    state: &DesktopState,
    reading_id: ReferenceEntryId,
) -> Result<ReferenceEntry, ModuleReadingError> {
    let reading = SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .load_reference_entry(reading_id)
        .await?
        .ok_or(ModuleReadingError::MissingReading(reading_id))?;

    if reading.reference_type != ReferenceEntryType::Reading || reading.module_id.is_none() {
        return Err(ModuleReadingError::MissingReading(reading_id));
    }

    Ok(reading)
}

fn parse_document_id(value: &str) -> Result<DocumentId, RadciteArchiveError> {
    Uuid::parse_str(value)
        .map(DocumentId)
        .map_err(|_| RadciteArchiveError::InvalidItemId(value.to_string()))
}

fn parse_module_id(value: &str) -> Result<ModuleId, RadciteArchiveError> {
    Uuid::parse_str(value)
        .map(ModuleId)
        .map_err(|_| RadciteArchiveError::InvalidItemId(value.to_string()))
}

fn parse_reference_entry_id(value: &str) -> Result<ReferenceEntryId, RadciteArchiveError> {
    Uuid::parse_str(value)
        .map(ReferenceEntryId)
        .map_err(|_| RadciteArchiveError::InvalidItemId(value.to_string()))
}

fn trimmed_optional(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn apply_basic_apa_validation(reference: &mut ReferenceEntry) {
    if let Some(url) = reference.url.as_deref() {
        reference.url = Some(normalise_reference_url(url));
    }

    let issues = basic_apa_validation_issues(
        reference
            .apa_citation
            .as_deref()
            .or(reference.citation_text.as_deref()),
    );
    reference.apa_validation_status = if issues.is_empty() {
        ApaValidationStatus::Valid
    } else {
        ApaValidationStatus::NeedsFix
    };
    reference.apa_validation_report = (!issues.is_empty()).then(|| issues.join("; "));
}

fn normalise_reference_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
    {
        format!("https://{}", &trimmed[7..])
    } else {
        trimmed.to_string()
    }
}

fn basic_apa_validation_issues(text: Option<&str>) -> Vec<&'static str> {
    let Some(text) = text.map(str::trim).filter(|value| !value.is_empty()) else {
        return vec!["No APA citation text recorded."];
    };

    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut issues = Vec::new();

    let lowered = normalized.to_lowercase();
    if text.contains('\n') {
        issues.push("Keep each APA reference on a single line without blank breaks.");
    }
    if lowered.starts_with("adapted from ")
        || lowered.starts_with("from ")
        || lowered.contains("adapted from")
    {
        issues.push("Remove narrative prefixes such as 'Adapted from ...'; APA references should start with the author name.");
    }
    if lowered.contains("examples are from") {
        issues.push("Remove explanatory sentences such as 'Examples are from ...'.");
    }

    let year_marker =
        Regex::new(r"(?i)\((?:(?:19|20)\d{2}[a-z]?|n\.d\.?)\)").expect("APA year marker regex");
    if !year_marker.is_match(&normalized) {
        issues.push("missing_year");
    }

    let author_format = Regex::new(r"[A-Za-z'’`\-]+,\s*[A-Z]").expect("APA author format regex");
    if !author_format.is_match(&normalized) {
        issues.push("Author names should follow Lastname, Initials.");
    }

    let year_punctuation = Regex::new(r"\)\.\s+").expect("APA year punctuation regex");
    if !year_punctuation.is_match(&normalized) {
        issues.push("Add a period after the year (e.g., (2024). Title...).");
    }

    let title_segment = Regex::new(r"\)\.\s+\S.{2,}").expect("APA title segment regex");
    if !title_segment.is_match(&normalized) {
        issues.push("Title segment missing after the year.");
    }

    if let Some(title) = normalized.split_once("). ").map(|(_, rest)| rest) {
        let title = title.split('.').next().unwrap_or_default().trim();
        if title.ends_with('/') {
            issues.push("Remove the trailing slash from the title.");
        }
    }

    issues
}

fn validation_status_label(status: ApaValidationStatus) -> &'static str {
    match status {
        ApaValidationStatus::Unknown => "unknown",
        ApaValidationStatus::Valid => "valid",
        ApaValidationStatus::NeedsFix => "needs_fix",
    }
}

fn format_course_references_html(
    references: &[ReferenceEntry],
    for_ako_learn: bool,
    use_library_links: bool,
) -> String {
    let mut lines = Vec::new();

    if !for_ako_learn {
        lines.push(r#"<p>{GENERICO:type="references"}</p>"#.to_string());
    }

    if references.is_empty() {
        lines.push("<p>No course references recorded yet.</p>".to_string());
    } else {
        lines.extend(references.iter().map(|reference| {
            format!(
                "<p>{}</p>",
                reference_export_html(reference, use_library_links)
            )
        }));
    }

    if !for_ako_learn {
        lines.push(r#"<p>{GENERICO:type="references_end"}</p>"#.to_string());
    }

    lines.join("\n")
}

fn format_module_readings_html(
    readings: &[ReferenceEntry],
    for_ako_learn: bool,
    use_library_links: bool,
) -> String {
    let html = format_module_readings_html_with_generico(readings, use_library_links);
    if for_ako_learn {
        apply_ako_module_readings_hanging_indent(&strip_generico_tokens(&html))
    } else {
        html
    }
}

fn format_module_readings_html_with_generico(
    readings: &[ReferenceEntry],
    use_library_links: bool,
) -> String {
    if readings.is_empty() {
        return concat!(
            r#"<p><span style="font-size: 0.9375rem;">"#,
            "No readings were detected for this module.",
            "</span></p>"
        )
        .to_string();
    }

    let compulsory_readings = readings
        .iter()
        .filter(|reading| reading_category_label(reading.reading_category) == "compulsory")
        .collect::<Vec<_>>();
    let optional_readings = readings
        .iter()
        .filter(|reading| reading_category_label(reading.reading_category) == "optional")
        .collect::<Vec<_>>();
    let mut parts = vec![
        "<p>The readings for this module are listed below.</p>".to_string(),
        r#"<p>{GENERICO:type="references"}</p>"#.to_string(),
    ];
    let mut generico_open = true;

    if !compulsory_readings.is_empty() {
        parts.push("<h4>Required readings</h4>".to_string());
        for (index, reading) in compulsory_readings.iter().enumerate() {
            let has_more_entries =
                index < compulsory_readings.len() - 1 || !optional_readings.is_empty();
            render_module_reading_entry(
                &mut parts,
                reading,
                has_more_entries,
                &mut generico_open,
                use_library_links,
            );
        }
    }

    if !optional_readings.is_empty() {
        parts.push(
            concat!(
                r#"<p><span style="font-size: 18px; font-weight: 700;">"#,
                "Optional readings",
                "</span></p>"
            )
            .to_string(),
        );
        for (index, reading) in optional_readings.iter().enumerate() {
            let has_more_entries = index < optional_readings.len() - 1;
            render_module_reading_entry(
                &mut parts,
                reading,
                has_more_entries,
                &mut generico_open,
                use_library_links,
            );
        }
    }

    if generico_open {
        parts.push(r#"<p>{GENERICO:type="references_end"}</p>"#.to_string());
    }

    parts.join("\n")
}

fn render_module_reading_entry(
    parts: &mut Vec<String>,
    reading: &ReferenceEntry,
    has_more_entries: bool,
    generico_open: &mut bool,
    use_library_links: bool,
) {
    let lesson_html = trimmed_str(reading.lesson_code.as_deref())
        .map(|lesson_code| format!("<strong>{}&nbsp;</strong>", escape_html(lesson_code)))
        .unwrap_or_default();

    parts.push(format!(
        r#"<p><span style="font-size: 0.9375rem;">{}{}</span></p>"#,
        lesson_html,
        reading_export_html(reading, use_library_links)
    ));

    let estimated_time_text = trimmed_str(reading.estimated_reading_time.as_deref());
    let notes_text = trimmed_str(reading.reading_notes.as_deref());
    if estimated_time_text.is_none() && notes_text.is_none() {
        return;
    }

    if *generico_open {
        parts.push(r#"<p>{GENERICO:type="references_end"}</p>"#.to_string());
        *generico_open = false;
    }

    if let Some(estimated_time_text) = estimated_time_text {
        parts.push(format!(
            r#"<p style="margin-left: 64px;"><strong>Estimated reading time: </strong>{}</p>"#,
            escape_html(estimated_time_text)
        ));
    }

    if let Some(notes_text) = notes_text {
        parts.push(format!(
            r#"<p style="margin-left: 64px; margin-bottom: 18px;">{}</p>"#,
            escape_html(notes_text)
        ));
    }

    if has_more_entries && !*generico_open {
        parts.push(r#"<p>{GENERICO:type="references"}</p>"#.to_string());
        *generico_open = true;
    }
}

fn reference_export_html(reference: &ReferenceEntry, use_library_links: bool) -> String {
    let source_text = reference_export_text(reference);
    let mut html = escape_html(&source_text);

    if let Some(visible_url) = reference_export_url(reference)
        && let Some(href) = reference_export_href(reference, use_library_links)
    {
        let escaped_visible_url = escape_html(&visible_url);
        let escaped_href = escape_html(&href);
        let url_link = format!(
            r#"<a href="{escaped_href}" target="_blank" rel="noopener noreferrer">{escaped_visible_url}</a>"#
        );
        if source_text.contains(&visible_url) {
            html = html.replacen(&escaped_visible_url, &url_link, 1);
        } else {
            html = format!("{html} {url_link}");
        }
    }

    html
}

fn reading_export_html(reading: &ReferenceEntry, use_library_links: bool) -> String {
    reference_export_html(reading, use_library_links)
}

fn reference_export_href(reference: &ReferenceEntry, use_library_links: bool) -> Option<String> {
    if use_library_links {
        build_uc_library_link(reference.doi.as_deref(), reference.url.as_deref())
            .or_else(|| reference_export_url(reference))
    } else {
        reference_export_url(reference)
    }
}

fn reference_export_url(reference: &ReferenceEntry) -> Option<String> {
    if let Some(url) = trimmed_str(reference.url.as_deref()) {
        return Some(url.to_string());
    }

    trimmed_str(reference.doi.as_deref()).map(doi_url)
}

fn doi_url(doi: &str) -> String {
    let doi = doi.trim();
    if doi.starts_with("http://") || doi.starts_with("https://") {
        doi.to_string()
    } else {
        format!("https://doi.org/{doi}")
    }
}

fn strip_generico_tokens(export_html: &str) -> String {
    export_html
        .replace(r#"<p>{GENERICO:type="references"}</p>"#, "")
        .replace(r#"<p>{GENERICO:type="references_end"}</p>"#, "")
}

fn apply_ako_module_readings_hanging_indent(export_html: &str) -> String {
    export_html.replace(
        r#"<p><span style="font-size: 0.9375rem;">"#,
        r#"<p style="margin-left: 64px; text-indent: -64px;"><span style="font-size: 0.9375rem;">"#,
    )
}

fn reference_export_text(reference: &ReferenceEntry) -> String {
    reference
        .apa_citation
        .as_deref()
        .or(reference.citation_text.as_deref())
        .or(reference.title.as_deref())
        .unwrap_or("Reference pending.")
        .trim()
        .to_string()
}

fn trimmed_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn escape_html(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '"' => "&quot;".chars().collect(),
            '\'' => "&#39;".chars().collect(),
            other => vec![other],
        })
        .collect()
}

fn filename_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_was_separator = false;
        } else if !last_was_separator && !slug.is_empty() {
            slug.push('-');
            last_was_separator = true;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "radcite".to_string()
    } else {
        slug.to_string()
    }
}

fn review_report_text_preview(value: &str) -> String {
    let mut characters = value.chars();
    let preview = characters.by_ref().take(100).collect::<String>();
    if characters.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}

fn build_summary(paragraphs: &[Paragraph], citations: &[Citation]) -> AnalyseDocxSummary {
    let cited_paragraph_count = paragraphs
        .iter()
        .filter(|paragraph| {
            citations
                .iter()
                .any(|citation| citation.paragraph_id == paragraph.id)
        })
        .count();
    let missing_citation_count = paragraphs
        .iter()
        .filter(|paragraph| paragraph.needs_citation)
        .count();
    let linked_citation_count = citations
        .iter()
        .filter(|citation| citation.reference_entry_id.is_some())
        .count();
    let unlinked_citation_count = citations.len() - linked_citation_count;

    AnalyseDocxSummary {
        paragraph_count: paragraphs.len(),
        citation_count: citations.len(),
        cited_paragraph_count,
        missing_citation_count,
        linked_citation_count,
        suggested_citation_count: 0,
        unlinked_citation_count,
    }
}

fn build_review_summary(paragraphs: &[ReviewParagraph]) -> AnalyseDocxSummary {
    let citation_count = paragraphs
        .iter()
        .map(|paragraph| paragraph.citations.len())
        .sum();
    let cited_paragraph_count = paragraphs
        .iter()
        .filter(|paragraph| !paragraph.citations.is_empty())
        .count();
    let missing_citation_count = paragraphs
        .iter()
        .filter(|paragraph| paragraph.needs_citation)
        .count();
    let linked_citation_count = paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.citations.iter())
        .filter(|citation| citation.reference_entry_id.is_some())
        .count();
    let suggested_citation_count = paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.citations.iter())
        .filter(|citation| {
            citation.reference_entry_id.is_none() && !citation.reference_suggestions.is_empty()
        })
        .count();
    let unlinked_citation_count = paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.citations.iter())
        .filter(|citation| citation.reference_entry_id.is_none())
        .count();

    AnalyseDocxSummary {
        paragraph_count: paragraphs.len(),
        citation_count,
        cited_paragraph_count,
        missing_citation_count,
        linked_citation_count,
        suggested_citation_count,
        unlinked_citation_count,
    }
}

fn build_review_paragraphs(
    paragraphs: Vec<Paragraph>,
    citations: Vec<Citation>,
    references: &[ReferenceEntry],
) -> Vec<ReviewParagraph> {
    paragraphs
        .into_iter()
        .map(|paragraph| {
            let paragraph_citations = citations
                .iter()
                .filter(|citation| citation.paragraph_id == paragraph.id)
                .map(|citation| ReviewCitation {
                    id: citation.id,
                    text: citation.citation_text.clone(),
                    start: citation.position_start,
                    end: citation.position_end,
                    verified: citation.verified,
                    reference_entry_id: citation.reference_entry_id,
                    reference_suggestions: reference_suggestions_for_citation(citation, references),
                })
                .collect();

            ReviewParagraph {
                id: paragraph.id,
                order_index: paragraph.order_index,
                page: paragraph.page,
                text: paragraph.text,
                formatted_text: paragraph.formatted_text,
                is_table: paragraph.is_table,
                needs_citation: paragraph.needs_citation,
                citations: paragraph_citations,
            }
        })
        .collect()
}

fn reference_suggestions_for_citation(
    citation: &Citation,
    references: &[ReferenceEntry],
) -> Vec<ReviewCitationReferenceSuggestion> {
    if citation.reference_entry_id.is_some() {
        return Vec::new();
    }

    let mut scored_suggestions: Vec<(i32, ReviewCitationReferenceSuggestion)> = references
        .iter()
        .filter_map(|reference| suggestion_for_reference(citation, reference))
        .collect();

    scored_suggestions.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.label.cmp(&right.label))
    });

    scored_suggestions
        .into_iter()
        .map(|(_, suggestion)| suggestion)
        .collect()
}

fn suggestion_for_reference(
    citation: &Citation,
    reference: &ReferenceEntry,
) -> Option<(i32, ReviewCitationReferenceSuggestion)> {
    let citation_year = extract_year(&citation.citation_text);
    let reference_search = reference_search_text(reference);
    let reference_year = reference
        .publication_year
        .as_deref()
        .map(str::to_string)
        .or_else(|| extract_year(&reference_search));
    let year_matches = citation_year.is_some() && citation_year == reference_year;
    let author_tokens = citation_author_tokens(&citation.citation_text);
    let author_matches = author_tokens
        .iter()
        .any(|token| reference_search.contains(token));
    let text_overlaps = citation_keyword_tokens(&citation.citation_text)
        .iter()
        .any(|token| reference_search.contains(token));

    let (score, confidence, reason) = if year_matches && author_matches {
        (100, "strong", "Author and year match")
    } else if year_matches && text_overlaps {
        (60, "possible", "Year and text overlap")
    } else if author_matches && citation_year.is_none() {
        (50, "possible", "Author match")
    } else {
        return None;
    };

    Some((
        score,
        ReviewCitationReferenceSuggestion {
            reference_entry_id: reference.id,
            label: reference_label(reference),
            confidence: confidence.to_string(),
            reason: reason.to_string(),
        },
    ))
}

fn extract_year(text: &str) -> Option<String> {
    let year = Regex::new(r"(?:19|20)\d{2}").expect("valid year regex");
    year.find(text).map(|hit| hit.as_str().to_string())
}

fn citation_author_tokens(citation_text: &str) -> Vec<String> {
    let without_years = Regex::new(r"(?:19|20)\d{2}[a-z]?")
        .expect("valid year regex")
        .replace_all(citation_text, "");

    Regex::new(r"[A-Za-z][A-Za-z\-']+")
        .expect("valid word regex")
        .find_iter(&without_years)
        .map(|hit| hit.as_str().trim_matches('\'').to_lowercase())
        .filter(|token| {
            token.len() > 1
                && !matches!(
                    token.as_str(),
                    "and" | "et" | "al" | "al." | "s" | "see" | "also"
                )
        })
        .collect()
}

fn citation_keyword_tokens(citation_text: &str) -> Vec<String> {
    citation_author_tokens(citation_text)
}

fn reference_search_text(reference: &ReferenceEntry) -> String {
    [
        reference.apa_citation.as_deref(),
        reference.citation_text.as_deref(),
        reference.title.as_deref(),
        reference.publication_year.as_deref(),
        reference.source.as_deref(),
        Some(reference.authors.join(" ")).as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase()
}

fn reference_label(reference: &ReferenceEntry) -> String {
    reference
        .apa_citation
        .as_deref()
        .or(reference.citation_text.as_deref())
        .or(reference.title.as_deref())
        .unwrap_or("Untitled reference")
        .to_string()
}
