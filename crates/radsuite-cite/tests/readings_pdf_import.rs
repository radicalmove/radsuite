use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_cite::{
    PdfReadingExtractionRequest, extract_pdf_reading_candidates,
    extract_pdf_reading_candidates_with_report,
};
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
    assert_eq!(
        candidates[0].source_filename.as_deref(),
        Some("COMS432 Module 6 Microlearning 3.pdf")
    );
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
        candidates[0].doi.as_deref(),
        Some("10.1080/1553118X.2022.2137674")
    );
    assert_eq!(
        candidates[1].lesson_code.as_deref(),
        Some("Microlearning 4")
    );
    assert_eq!(candidates[1].reading_category, ReadingCategory::Optional);
}

#[test]
fn readings_pdf_import_expands_directories_recursively() {
    let dir = test_dir("directory-expansion");
    let nested = dir.join("Module 10").join("assets");
    fs::create_dir_all(&nested).expect("create nested folder");
    let pdf = nested.join("Module 10 Lesson 2.pdf");
    write_minimal_pdf(
        &pdf,
        &[
            "Required readings",
            "Parker, S. (2024). Folder imports for SCORM readings. Example Press.",
        ],
    );
    fs::write(nested.join("ignore.txt"), "not a pdf").expect("write ignored file");

    let candidates =
        extract_pdf_reading_candidates(PdfReadingExtractionRequest { paths: vec![dir] })
            .expect("extract candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].source_filename.as_deref(),
        Some("Module 10 Lesson 2.pdf")
    );
    assert_eq!(candidates[0].module_order, Some(10));
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("Lesson 2"));
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
fn readings_pdf_import_deduplicates_by_doi_and_keeps_required_category() {
    let dir = test_dir("required-doi-precedence");
    let optional_pdf = dir.join("Module 2 Microlearning 1.pdf");
    let required_pdf = dir.join("Module 2 Microlearning 2.pdf");
    write_minimal_pdf(
        &optional_pdf,
        &[
            "Optional readings",
            "Rice, L. (2024). Communication and culture. Academic Press. https://doi.org/10.1234/rice",
        ],
    );
    write_minimal_pdf(
        &required_pdf,
        &[
            "Required readings",
            "Rice, L. (2024). Communication and culture: A revised edition. Academic Press. https://doi.org/10.1234/rice",
        ],
    );

    let candidates = extract_pdf_reading_candidates(PdfReadingExtractionRequest {
        paths: vec![optional_pdf, required_pdf],
    })
    .expect("extract candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].reading_category, ReadingCategory::Compulsory);
    assert!(candidates[0].apa_citation.contains("revised edition"));
    assert_eq!(candidates[0].doi.as_deref(), Some("10.1234/rice"));
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
fn readings_pdf_import_extracts_composite_font_text_with_tounicode_map() {
    let dir = test_dir("composite-font");
    let pdf = dir.join("Module 12 Lesson 1.pdf");
    write_composite_font_pdf(
        &pdf,
        &[
            "Required readings",
            "Smith, J. (2024). Composite font reading extraction. Example Press. https://example.com/reading",
        ],
    );

    let candidates =
        extract_pdf_reading_candidates(PdfReadingExtractionRequest { paths: vec![pdf] })
            .expect("extract candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].module_order, Some(12));
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("Lesson 1"));
    assert_eq!(
        candidates[0].apa_citation,
        "Smith, J. (2024). Composite font reading extraction. Example Press. https://example.com/reading"
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
fn readings_pdf_import_reports_unreadable_files_without_aborting_batch() {
    let dir = test_dir("partial-failure");
    let good_pdf = dir.join("Module 11 Lesson 1.pdf");
    let missing_pdf = dir.join("Module 11 Lesson 2.pdf");
    write_minimal_pdf(
        &good_pdf,
        &[
            "Required readings",
            "Turner, A. (2024). Partial PDF batch imports. Example Press.",
        ],
    );

    let report = extract_pdf_reading_candidates_with_report(PdfReadingExtractionRequest {
        paths: vec![good_pdf, missing_pdf.clone()],
    })
    .expect("extract candidates with report");

    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].module_order, Some(11));
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].path, missing_pdf);
    assert!(
        report.failures[0]
            .message
            .contains("failed to read PDF file")
    );
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

fn write_composite_font_pdf(path: &Path, lines: &[&str]) {
    let mut characters = std::collections::BTreeMap::new();
    let mut next_code = 1u16;
    for line in lines {
        for character in line.chars() {
            characters.entry(character).or_insert_with(|| {
                let code = next_code;
                next_code += 1;
                code
            });
        }
    }

    let content = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let encoded = line
                .chars()
                .map(|character| format!("{:04X}", characters[&character]))
                .collect::<String>();
            if index == 0 {
                format!("72 720 Td <{encoded}> Tj")
            } else {
                format!("0 -20 Td <{encoded}> Tj")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!("BT\n/F1 12 Tf\n{content}\nET");

    let cmap_entries = characters
        .values()
        .map(|code| {
            let character = characters
                .iter()
                .find_map(|(character, value)| (value == code).then_some(*character))
                .expect("mapped character");
            format!("<{code:04X}> <{:04X}>", character as u32)
        })
        .collect::<Vec<_>>();
    let cmap = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n{} beginbfchar\n{}\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend",
        cmap_entries.len(),
        cmap_entries.join("\n")
    );

    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        stream_object(&content),
        "<< /Type /Font /Subtype /Type0 /BaseFont /TestComposite /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>".to_string(),
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestComposite /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 1000 >>".to_string(),
        "<< /Type /FontDescriptor /FontName /TestComposite /Flags 4 /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 >>".to_string(),
        stream_object(&cmap),
    ];

    fs::write(path, build_pdf(objects)).expect("write composite font pdf");
}

fn stream_object(content: &str) -> String {
    format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        content
    )
}

fn build_pdf(objects: Vec<String>) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];

    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    pdf
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
