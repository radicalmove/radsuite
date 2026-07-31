use std::path::PathBuf;

use radsuite_core::{DocumentFileType, ProjectId};
use thiserror::Error;

use crate::{
    AnalysedDocument,
    docx::{ExtractedParagraph, analyse_extracted_paragraphs},
    readings_pdf::{PdfReadingExtractionError, extract_pdf_text_lines},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfIngestionRequest {
    pub project_id: ProjectId,
    pub path: PathBuf,
    pub original_filename: String,
}

#[derive(Debug, Error)]
pub enum PdfIngestionError {
    #[error("expected a .pdf file: {path}")]
    UnsupportedExtension { path: PathBuf },
    #[error(transparent)]
    Extraction(#[from] PdfReadingExtractionError),
}

pub fn ingest_pdf(request: PdfIngestionRequest) -> Result<AnalysedDocument, PdfIngestionError> {
    if !has_pdf_extension(&request.path) {
        return Err(PdfIngestionError::UnsupportedExtension { path: request.path });
    }

    let extracted_paragraphs = extract_pdf_text_lines(&request.path)?
        .into_iter()
        .map(|text| ExtractedParagraph {
            text,
            is_table: false,
        });

    Ok(analyse_extracted_paragraphs(
        request.project_id,
        request.original_filename,
        DocumentFileType::Pdf,
        extracted_paragraphs,
    ))
}

fn has_pdf_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}
