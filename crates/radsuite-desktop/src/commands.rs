use std::path::PathBuf;

use radsuite_cite::{DocxIngestionError, DocxIngestionRequest, ingest_docx};
use radsuite_core::{DocumentId, Project, ProjectId, UserId};
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

    let missing_citation_count = analysed
        .paragraphs
        .iter()
        .filter(|paragraph| paragraph.needs_citation)
        .count();

    Ok(AnalyseDocxResponse {
        project_id: project.id,
        project_title: project.title,
        document_id: analysed.document.id,
        original_filename: analysed.document.original_filename,
        paragraph_count: analysed.paragraphs.len(),
        citation_count: analysed.citations.len(),
        missing_citation_count,
    })
}
