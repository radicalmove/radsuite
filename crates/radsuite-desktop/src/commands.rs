use std::path::PathBuf;

use chrono::Utc;
use radsuite_cite::{
    CsvReadingExtractionRequest, CsvReadingImportError, DocxIngestionError, DocxIngestionRequest,
    DocxReadingExtractionRequest, ReadingImportCandidate, extract_csv_reading_candidates,
    extract_docx_reading_candidates, ingest_docx,
};
use radsuite_core::{
    Citation, CitationId, CourseModule, Document, DocumentId, ModuleId, Paragraph, ParagraphId,
    Project, ProjectId, ReadingCategory, ReferenceEntry, ReferenceEntryId, ReferenceEntryType,
    UserId,
};
use radsuite_db::{
    CitationDocumentRepository, CourseModuleRepository, DbError, ProjectRepository,
    ReferenceEntryRepository, SqliteCitationDocumentRepository, SqliteCourseModuleRepository,
    SqliteProjectRepository, SqliteReferenceEntryRepository,
};
use radsuite_engines::EngineStatus;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DesktopState;

const LOCAL_RADCITE_PROJECT_CODE: &str = "CRJU150";
const LOCAL_RADCITE_PROJECT_TITLE: &str = "RADcite Functional Testing";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    pub app_name: String,
    pub database_ready: bool,
    pub sync_configured: bool,
    pub engines: Vec<EngineStatus>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRadciteProjectRequest {
    pub code: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyseDocxRequest {
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
    pub paragraph_count: usize,
    pub citation_count: usize,
    pub missing_citation_count: usize,
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
    pub url: Option<String>,
    pub notes: Option<String>,
    pub reading_notes: Option<String>,
    pub estimated_reading_time: Option<String>,
    pub validation_status: String,
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
pub struct ModuleReadingImportCandidateSummary {
    pub module_order: Option<i32>,
    pub module_title: Option<String>,
    pub reading_category: String,
    pub lesson_code: Option<String>,
    pub apa_citation: String,
    pub citation_text: Option<String>,
    pub url: Option<String>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportModuleReadingsRequest {
    pub module_id: ModuleId,
    pub for_ako_learn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseReferencesExport {
    pub filename: String,
    pub content_type: String,
    pub html: String,
    pub reference_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleReadingsExport {
    pub filename: String,
    pub content_type: String,
    pub html: String,
    pub module_id: ModuleId,
    pub reading_count: usize,
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
    Database(#[from] DbError),
}

#[derive(Debug, Error)]
pub enum RadciteProjectError {
    #[error("enter a project title before creating it")]
    EmptyTitle,
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
}

#[derive(Debug, Error)]
pub enum CourseReferenceError {
    #[error("enter reference text before adding a course reference")]
    EmptyReferenceText,
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
    #[error("choose a DOCX or CSV file before previewing module readings")]
    EmptyPath,
    #[error(transparent)]
    Docx(#[from] DocxIngestionError),
    #[error(transparent)]
    Csv(#[from] CsvReadingImportError),
    #[error("could not load RADcite module {0}")]
    MissingModule(ModuleId),
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
        original_filename: analysed.document.original_filename,
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
        original_filename: analysed.document.original_filename,
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
            paragraph_count: document.paragraph_count as usize,
            citation_count: document.citation_count as usize,
            missing_citation_count: document.missing_citation_count as usize,
        })
        .collect())
}

pub async fn load_saved_radcite_review(
    state: &DesktopState,
    document_id: DocumentId,
) -> Result<AnalyseDocxReviewResponse, ReviewActionError> {
    load_review_response(state, document_id).await
}

pub async fn list_course_references(
    state: &DesktopState,
    request: ListCourseReferencesRequest,
) -> Result<Vec<CourseReferenceSummary>, CourseReferenceError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let references = SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .list_reference_entries_for_project(project.id, ReferenceEntryType::Reference)
        .await?;

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
    let mut reference = ReferenceEntry::new(project.id, ReferenceEntryType::Reference);
    reference.apa_citation = Some(apa_citation.to_string());
    reference.notes = request
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .insert_reference_entry(&reference)
        .await?;

    Ok(course_reference_summary(reference))
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

    let readings = SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .list_reference_entries_for_module(request.module_id, ReferenceEntryType::Reading)
        .await?;

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

    let mut reading = ReferenceEntry::new(module.project_id, ReferenceEntryType::Reading);
    reading.module_id = Some(module.id);
    reading.reading_category = Some(reading_category);
    reading.lesson_code = trimmed_optional(request.lesson_code);
    reading.apa_citation = apa_citation;
    reading.citation_text = citation_text;
    reading.url = trimmed_optional(request.url);
    reading.notes = trimmed_optional(request.notes);
    reading.reading_notes = trimmed_optional(request.reading_notes);
    reading.estimated_reading_time = trimmed_optional(request.estimated_reading_time);

    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .insert_reference_entry(&reading)
        .await?;

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
    reading.url = trimmed_optional(request.url);
    reading.notes = trimmed_optional(request.notes);
    reading.reading_notes = trimmed_optional(request.reading_notes);
    reading.estimated_reading_time = trimmed_optional(request.estimated_reading_time);
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

        let mut reading = ReferenceEntry::new(module.project_id, ReferenceEntryType::Reading);
        reading.module_id = Some(module.id);
        reading.reading_category = Some(reading_category);
        reading.lesson_code = trimmed_optional(candidate.lesson_code);
        reading.apa_citation = apa_citation;
        reading.citation_text = citation_text;
        reading.url = trimmed_optional(candidate.url);
        reading.notes = trimmed_optional(candidate.notes);
        reading.reading_notes = trimmed_optional(candidate.reading_notes);
        reading.estimated_reading_time = trimmed_optional(candidate.estimated_reading_time);

        reference_repo.insert_reference_entry(&reading).await?;

        saved_readings.push(
            module_reading_summary(reading)
                .ok_or(ModuleReadingImportError::MissingModule(module.id))?,
        );
    }

    Ok(saved_readings)
}

pub async fn export_course_references(
    state: &DesktopState,
    request: ExportCourseReferencesRequest,
) -> Result<CourseReferencesExport, CourseReferenceExportError> {
    let project = load_requested_or_local_radcite_project(state, request.project_id).await?;
    let references = load_course_reference_entries(state, project.id).await?;
    let reference_count = references.len();
    let html = format_course_references_html(&references, request.for_ako_learn);

    Ok(CourseReferencesExport {
        filename: format!(
            "{}-course-references.html",
            filename_slug(project.code.as_deref().unwrap_or(&project.title))
        ),
        content_type: "text/html; charset=utf-8".to_string(),
        html,
        reference_count,
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
    let reading_count = readings.len();
    let html = format_module_readings_html(&readings, request.for_ako_learn);
    let project_label = project
        .as_ref()
        .and_then(|project| project.code.as_deref())
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

    let analysed = ingest_docx(DocxIngestionRequest {
        project_id: project.id,
        path,
        original_filename,
    })?;

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
        original_filename: analysis.document.original_filename,
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

async fn load_course_reference_entries(
    state: &DesktopState,
    project_id: ProjectId,
) -> Result<Vec<ReferenceEntry>, DbError> {
    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .list_reference_entries_for_project(project_id, ReferenceEntryType::Reference)
        .await
}

fn radcite_project_summary(project: Project) -> RadciteProjectSummary {
    RadciteProjectSummary {
        id: project.id,
        code: project.code,
        title: project.title,
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
        url: reading.url,
        notes: reading.notes,
        reading_notes: reading.reading_notes,
        estimated_reading_time: reading.estimated_reading_time,
        validation_status: validation_status_label(reading.apa_validation_status).to_string(),
    })
}

fn module_reading_import_candidate_summary(
    candidate: ReadingImportCandidate,
) -> ModuleReadingImportCandidateSummary {
    ModuleReadingImportCandidateSummary {
        module_order: candidate.module_order,
        module_title: candidate.module_title,
        reading_category: reading_category_label(Some(candidate.reading_category)).to_string(),
        lesson_code: candidate.lesson_code,
        apa_citation: candidate.apa_citation,
        citation_text: candidate.citation_text,
        url: candidate.url,
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

fn parse_reading_category_request(value: &str) -> Result<ReadingCategory, ModuleReadingError> {
    match value.trim() {
        "compulsory" => Ok(ReadingCategory::Compulsory),
        "optional" => Ok(ReadingCategory::Optional),
        other => Err(ModuleReadingError::InvalidCategory(other.to_string())),
    }
}

fn parse_reading_category_import_request(
    value: &str,
) -> Result<ReadingCategory, ModuleReadingImportError> {
    match value.trim() {
        "compulsory" => Ok(ReadingCategory::Compulsory),
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

fn trimmed_optional(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn validation_status_label(status: radsuite_core::ApaValidationStatus) -> &'static str {
    match status {
        radsuite_core::ApaValidationStatus::Unknown => "unknown",
        radsuite_core::ApaValidationStatus::Valid => "valid",
        radsuite_core::ApaValidationStatus::NeedsFix => "needs_fix",
    }
}

fn format_course_references_html(references: &[ReferenceEntry], for_ako_learn: bool) -> String {
    let mut lines = Vec::new();

    if !for_ako_learn {
        lines.push(r#"<p>{GENERICO:type="references"}</p>"#.to_string());
    }

    if references.is_empty() {
        lines.push("<p>No course references recorded yet.</p>".to_string());
    } else {
        lines.extend(
            references.iter().map(|reference| {
                format!("<p>{}</p>", escape_html(&reference_export_text(reference)))
            }),
        );
    }

    if !for_ako_learn {
        lines.push(r#"<p>{GENERICO:type="references_end"}</p>"#.to_string());
    }

    lines.join("\n")
}

fn format_module_readings_html(readings: &[ReferenceEntry], for_ako_learn: bool) -> String {
    let html = format_module_readings_html_with_generico(readings);
    if for_ako_learn {
        apply_ako_module_readings_hanging_indent(&strip_generico_tokens(&html))
    } else {
        html
    }
}

fn format_module_readings_html_with_generico(readings: &[ReferenceEntry]) -> String {
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
        "<p>These are the readings located in the module content, provided here for your convenience/change text.</p>".to_string(),
        r#"<p>{GENERICO:type="references"}</p>"#.to_string(),
    ];
    let mut generico_open = true;

    if !compulsory_readings.is_empty() {
        parts.push("<h4>Compulsory readings</h4>".to_string());
        for (index, reading) in compulsory_readings.iter().enumerate() {
            let has_more_entries =
                index < compulsory_readings.len() - 1 || !optional_readings.is_empty();
            render_module_reading_entry(&mut parts, reading, has_more_entries, &mut generico_open);
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
            render_module_reading_entry(&mut parts, reading, has_more_entries, &mut generico_open);
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
) {
    let lesson_html = trimmed_str(reading.lesson_code.as_deref())
        .map(|lesson_code| format!("<strong>{}&nbsp;</strong>", escape_html(lesson_code)))
        .unwrap_or_default();

    parts.push(format!(
        r#"<p><span style="font-size: 0.9375rem;">{}{}</span></p>"#,
        lesson_html,
        reading_export_html(reading)
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

fn reading_export_html(reading: &ReferenceEntry) -> String {
    let source_text = reference_export_text(reading);
    let mut html = escape_html(&source_text);

    if let Some(url) = trimmed_str(reading.url.as_deref()) {
        let escaped_url = escape_html(url);
        let url_link = format!(
            r#"<a href="{escaped_url}" target="_blank" rel="noopener noreferrer">{escaped_url}</a>"#
        );
        if source_text.contains(url) {
            html = html.replacen(&escaped_url, &url_link, 1);
        } else {
            html = format!("{html} {url_link}");
        }
    }

    html
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
