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
fn readings_pdf_import_extracts_flate_encoded_text_streams() {
    let dir = test_dir("flate-stream");
    let pdf = dir.join("Module 8 Lesson 2.pdf");
    write_minimal_flate_pdf(
        &pdf,
        &[
            "Required readings",
            "Miller, P. (2024). Compressed PDF text extraction. Example Press.",
        ],
    );

    let candidates =
        extract_pdf_reading_candidates(PdfReadingExtractionRequest { paths: vec![pdf] })
            .expect("extract candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].module_order, Some(8));
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("Lesson 2"));
    assert_eq!(
        candidates[0].apa_citation,
        "Miller, P. (2024). Compressed PDF text extraction. Example Press."
    );
}

#[test]
fn readings_pdf_import_extracts_tj_array_text() {
    let dir = test_dir("tj-array");
    let pdf = dir.join("Module 9 Lesson 4.pdf");
    write_minimal_pdf_with_stream(
        &pdf,
        "[(Required readings) 120 (Miller, P. \\(2024\\). Array text extraction. Example Press.)] TJ",
    );

    let candidates =
        extract_pdf_reading_candidates(PdfReadingExtractionRequest { paths: vec![pdf] })
            .expect("extract candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].module_order, Some(9));
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("Lesson 4"));
    assert_eq!(
        candidates[0].apa_citation,
        "Miller, P. (2024). Array text extraction. Example Press."
    );
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

#[test]
#[ignore = "requires RADSUITE_REAL_SCORM_PDFS with semicolon-separated local PDF paths"]
fn readings_pdf_import_can_probe_real_scorm_pdfs() {
    let paths = std::env::var("RADSUITE_REAL_SCORM_PDFS")
        .expect("RADSUITE_REAL_SCORM_PDFS should contain semicolon-separated PDF paths")
        .split(';')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    let candidates = extract_pdf_reading_candidates(PdfReadingExtractionRequest { paths })
        .expect("extract real SCORM PDF candidates");

    eprintln!("detected {} reading candidates", candidates.len());
    for candidate in &candidates {
        eprintln!(
            "module={:?} lesson={:?} category={:?} citation={}",
            candidate.module_title,
            candidate.lesson_code,
            candidate.reading_category,
            candidate.apa_citation
        );
    }
}

fn write_minimal_pdf(path: &Path, lines: &[&str]) {
    let text = lines
        .iter()
        .map(|line| format!("({}) Tj", escape_pdf_text(line)))
        .collect::<Vec<_>>()
        .join("\n");
    write_minimal_pdf_with_stream(path, &text);
}

fn write_minimal_pdf_with_stream(path: &Path, text: &str) {
    let pdf = format!(
        "%PDF-1.4\n1 0 obj <<>> endobj\n2 0 obj << /Length {} >> stream\nBT\n{}\nET\nendstream\nendobj\ntrailer << /Root 1 0 R >>\n%%EOF\n",
        text.len() + 6,
        text
    );
    fs::write(path, pdf).expect("write pdf");
}

fn write_minimal_flate_pdf(path: &Path, lines: &[&str]) {
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write as _;

    let text = lines
        .iter()
        .map(|line| format!("({}) Tj", escape_pdf_text(line)))
        .collect::<Vec<_>>()
        .join("\n");
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(format!("BT\n{text}\nET").as_bytes())
        .expect("write flate data");
    let compressed = encoder.finish().expect("finish flate data");
    let mut pdf =
        b"%PDF-1.4\n1 0 obj <<>> endobj\n2 0 obj << /Filter /FlateDecode /Length ".to_vec();
    pdf.extend_from_slice(compressed.len().to_string().as_bytes());
    pdf.extend_from_slice(b" >> stream\n");
    pdf.extend_from_slice(&compressed);
    pdf.extend_from_slice(b"\nendstream\nendobj\ntrailer << /Root 1 0 R >>\n%%EOF\n");
    fs::write(path, pdf).expect("write flate pdf");
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
