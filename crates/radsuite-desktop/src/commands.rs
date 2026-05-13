use std::path::PathBuf;

use radsuite_cite::{DocxIngestionError, DocxIngestionRequest, ingest_docx};
use radsuite_core::{
    Citation, CitationId, Document, DocumentId, Paragraph, ParagraphId, Project, ProjectId,
    ReferenceEntry, ReferenceEntryId, ReferenceEntryType, UserId,
};
use radsuite_db::{
    CitationDocumentRepository, DbError, ProjectRepository, ReferenceEntryRepository,
    SqliteCitationDocumentRepository, SqliteProjectRepository, SqliteReferenceEntryRepository,
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
pub struct AnalyseDocxRequest {
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
    pub apa_citation: String,
    pub notes: Option<String>,
}

#[derive(Debug, Error)]
pub enum AnalyseDocxError {
    #[error("choose a DOCX file before running RADcite analysis")]
    EmptyPath,
    #[error("could not determine the DOCX filename")]
    MissingFilename,
    #[error(transparent)]
    Ingestion(#[from] DocxIngestionError),
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
    #[error(transparent)]
    Database(#[from] DbError),
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
    let summary = build_summary(&analysed.paragraphs, &analysed.citations);
    let references = load_course_reference_entries(state, analysed.project.id).await?;
    let paragraphs = build_review_paragraphs(
        analysed.paragraphs,
        analysed.citations,
        references.as_slice(),
    );

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
) -> Result<Vec<SavedRadciteReviewSummary>, ReviewActionError> {
    let documents = SqliteCitationDocumentRepository::new(state.database_pool.clone())
        .list_saved_documents()
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
) -> Result<Vec<CourseReferenceSummary>, CourseReferenceError> {
    let project = load_or_create_local_radcite_project(state).await?;
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

    let project = load_or_create_local_radcite_project(state).await?;
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

    let project = load_or_create_local_radcite_project(state).await?;

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

    let summary = build_summary(&analysis.paragraphs, &analysis.citations);
    let references = load_course_reference_entries(state, project.id).await?;
    let paragraphs = build_review_paragraphs(
        analysis.paragraphs,
        analysis.citations,
        references.as_slice(),
    );

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

async fn load_course_reference_entries(
    state: &DesktopState,
    project_id: ProjectId,
) -> Result<Vec<ReferenceEntry>, DbError> {
    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .list_reference_entries_for_project(project_id, ReferenceEntryType::Reference)
        .await
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

fn reference_type_label(reference_type: ReferenceEntryType) -> &'static str {
    match reference_type {
        ReferenceEntryType::Reference => "reference",
        ReferenceEntryType::Reading => "reading",
    }
}

fn validation_status_label(status: radsuite_core::ApaValidationStatus) -> &'static str {
    match status {
        radsuite_core::ApaValidationStatus::Unknown => "unknown",
        radsuite_core::ApaValidationStatus::Valid => "valid",
        radsuite_core::ApaValidationStatus::NeedsFix => "needs_fix",
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

    AnalyseDocxSummary {
        paragraph_count: paragraphs.len(),
        citation_count: citations.len(),
        cited_paragraph_count,
        missing_citation_count,
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
