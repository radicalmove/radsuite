use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_cite::{PdfIngestionError, PdfIngestionRequest, ingest_pdf};
use radsuite_core::{DocumentFileType, ProjectId};

#[test]
fn pdf_ingestion_builds_ordered_citation_review_content() {
    let path = test_path("document-review.pdf");
    write_minimal_pdf(
        &path,
        &[
            "Smith (2020) explains worked examples.",
            "A 2021 survey reported that 64 percent of respondents changed their study habits.",
        ],
    );

    let analysed = ingest_pdf(PdfIngestionRequest {
        project_id: ProjectId::new(),
        path: path.clone(),
        original_filename: "review-source.pdf".to_string(),
    })
    .expect("ingest PDF");

    assert_eq!(analysed.document.file_type, DocumentFileType::Pdf);
    assert_eq!(analysed.document.original_filename, "review-source.pdf");
    assert_eq!(analysed.paragraphs.len(), 2);
    assert_eq!(analysed.paragraphs[0].order_index, 0);
    assert_eq!(
        analysed.paragraphs[0].text,
        "Smith (2020) explains worked examples."
    );
    assert_eq!(analysed.citations.len(), 1);
    assert_eq!(analysed.citations[0].citation_text, "Smith (2020)");
    assert!(!analysed.paragraphs[0].needs_citation);
    assert!(analysed.paragraphs[1].needs_citation);

    remove_file(path);
}

#[test]
fn pdf_ingestion_rejects_non_pdf_paths() {
    let path = test_path("review-source.docx");
    fs::write(&path, b"not a PDF").expect("write invalid fixture");

    let error = ingest_pdf(PdfIngestionRequest {
        project_id: ProjectId::new(),
        path: path.clone(),
        original_filename: "review-source.docx".to_string(),
    })
    .expect_err("reject non-PDF");

    assert!(matches!(
        error,
        PdfIngestionError::UnsupportedExtension { .. }
    ));
    remove_file(path);
}

fn write_minimal_pdf(path: &Path, lines: &[&str]) {
    let text = lines
        .iter()
        .map(|line| format!("({}) Tj", escape_pdf_text(line)))
        .collect::<Vec<_>>()
        .join("\n");
    let pdf = format!(
        "%PDF-1.4\n1 0 obj <<>> endobj\n2 0 obj << /Length {} >> stream\nBT\n{}\nET\nendstream\nendobj\ntrailer << /Root 1 0 R >>\n%%EOF\n",
        text.len() + 6,
        text
    );
    fs::write(path, pdf).expect("write PDF fixture");
}

fn escape_pdf_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn test_path(filename: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!("radsuite-pdf-ingestion-{suffix}-{filename}"))
}

fn remove_file(path: PathBuf) {
    let _ = fs::remove_file(path);
}
