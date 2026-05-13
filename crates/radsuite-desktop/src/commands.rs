use std::path::PathBuf;

use radsuite_cite::{DocxIngestionError, DocxIngestionRequest, ingest_docx};
use radsuite_core::{
    Citation, CitationId, Document, DocumentId, Paragraph, ParagraphId, Project, ProjectId, UserId,
};
use radsuite_db::{
    CitationDocumentRepository, DbError, ProjectRepository, SqliteCitationDocumentRepository,
    SqliteProjectRepository,
};
use radsuite_engines::EngineStatus;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DesktopState;

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
    let paragraphs = build_review_paragraphs(analysed.paragraphs, analysed.citations);

    Ok(AnalyseDocxReviewResponse {
        project_id: analysed.project.id,
        project_title: analysed.project.title,
        document_id: analysed.document.id,
        original_filename: analysed.document.original_filename,
        summary,
        paragraphs,
    })
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

    let project = Project::new("RADCITE-DEMO", "RADcite Functional Testing", UserId::new());
    SqliteProjectRepository::new(state.database_pool.clone())
        .insert_project(&project)
        .await?;

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
