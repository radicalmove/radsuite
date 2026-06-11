use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_cite::{PdfReadingExtractionRequest, extract_pdf_reading_candidates};
use radsuite_core::ReadingCategory;

#[test]
fn readings_pdf_import_extracts_microlearning_readings_from_multiple_pdfs() {
    let dir = test_dir("microlearning");
    let required_pdf = dir.join("COMS432 Module 6 Microlearning 3.pdf");
    let optional_pdf = dir.join("COMS432 Module 6 Microlearning 4.pdf");
    write_minimal_pdf(
        &required_pdf,
        &[
            "Required readings",
            "Goldberg, M. H., & Gustafson, A. (2023). A framework for understanding campaigns. Journal of Strategic Communication, 17(1), 1-20. https://doi.org/10.1080/1553118X.2022.2137674",
        ],
    );
    write_minimal_pdf(
        &optional_pdf,
        &[
            "Optional readings",
            "Taylor, R. (2023). Optional primer. Teaching Press.",
        ],
    );

    let candidates = extract_pdf_reading_candidates(PdfReadingExtractionRequest {
        paths: vec![required_pdf, optional_pdf],
    })
    .expect("extract candidates");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].module_order, Some(6));
    assert_eq!(candidates[0].module_title.as_deref(), Some("Module 6"));
    assert_eq!(
        candidates[0].lesson_code.as_deref(),
        Some("Microlearning 3")
    );
    assert_eq!(candidates[0].reading_category, ReadingCategory::Compulsory);
    assert_eq!(
        candidates[0].url.as_deref(),
        Some("https://doi.org/10.1080/1553118X.2022.2137674")
    );
    assert_eq!(
        candidates[1].lesson_code.as_deref(),
        Some("Microlearning 4")
    );
    assert_eq!(candidates[1].reading_category, ReadingCategory::Optional);
}

#[test]
fn readings_pdf_import_uses_course_headings_when_filename_is_generic() {
    let dir = test_dir("course-heading");
    let pdf = dir.join("story_content.pdf");
    write_minimal_pdf(
        &pdf,
        &[
            "Module 7: Media literacy",
            "Required readings",
            "Nguyen, T. (2024). Critical media literacy in practice. Learning Press.",
        ],
    );

    let candidates =
        extract_pdf_reading_candidates(PdfReadingExtractionRequest { paths: vec![pdf] })
            .expect("extract candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].module_order, Some(7));
    assert_eq!(
        candidates[0].module_title.as_deref(),
        Some("Module 7: Media literacy")
    );
}

#[test]
fn readings_pdf_import_required_supersedes_optional_duplicate() {
    let dir = test_dir("required-precedence");
    let optional_pdf = dir.join("Module 2 Microlearning 1.pdf");
    let required_pdf = dir.join("Module 2 Microlearning 2.pdf");
    let reading = "Rice, L. (2024). Communication and culture. Academic Press.";
    write_minimal_pdf(&optional_pdf, &["Optional readings", reading]);
    write_minimal_pdf(&required_pdf, &["Required readings", reading]);

    let candidates = extract_pdf_reading_candidates(PdfReadingExtractionRequest {
        paths: vec![optional_pdf, required_pdf],
    })
    .expect("extract candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].reading_category, ReadingCategory::Compulsory);
}

#[test]
fn readings_pdf_import_rejects_non_pdf_paths() {
    let dir = test_dir("non-pdf");
    let path = dir.join("readings.docx");
    fs::write(&path, b"not a pdf").expect("write file");

    let error = extract_pdf_reading_candidates(PdfReadingExtractionRequest { paths: vec![path] })
        .expect_err("unsupported extension");

    assert!(error.to_string().contains("expected a .pdf file"));
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
    fs::write(path, pdf).expect("write pdf");
}

fn test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "radsuite-readings-pdf-import-{name}-{nanos}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create test dir");
    path
}

fn escape_pdf_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
