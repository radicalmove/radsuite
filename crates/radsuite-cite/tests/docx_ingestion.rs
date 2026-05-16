use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use radsuite_cite::{
    DocxIngestionError, DocxIngestionRequest, DocxReadingExtractionRequest,
    extract_docx_reading_candidates, ingest_docx,
};
use radsuite_core::{ProjectId, ReadingCategory};
use zip::{ZipWriter, write::SimpleFileOptions};

#[test]
fn docx_ingestion_extracts_and_analyses_paragraphs() {
    let path = write_minimal_docx("docx-ingestion-extracts.docx");
    let project_id = ProjectId::new();

    let analysed = ingest_docx(DocxIngestionRequest {
        project_id,
        path: path.clone(),
        original_filename: "lesson-1.docx".to_string(),
    })
    .expect("ingest docx");

    assert_eq!(analysed.document.project_id, project_id);
    assert_eq!(analysed.document.original_filename, "lesson-1.docx");
    assert_eq!(analysed.paragraphs.len(), 4);

    assert_eq!(analysed.paragraphs[0].order_index, 0);
    assert_eq!(
        analysed.paragraphs[0].text,
        "Smith (2020) explains worked examples."
    );
    assert!(!analysed.paragraphs[0].needs_citation);

    assert_eq!(
        analysed.paragraphs[1].text,
        "A 2021 survey reported that 64 percent of respondents changed their study habits."
    );
    assert!(analysed.paragraphs[1].needs_citation);

    assert_eq!(
        analysed.paragraphs[2].text,
        "Read the article Library link https://example.edu/read"
    );
    assert_eq!(analysed.paragraphs[3].text, "Week 1 Required reading");
    assert!(analysed.paragraphs[3].is_table);

    assert_eq!(analysed.citations.len(), 1);
    assert_eq!(
        analysed.citations[0].paragraph_id,
        analysed.paragraphs[0].id
    );
    assert_eq!(analysed.citations[0].citation_text, "Smith (2020)");
}

#[test]
fn docx_ingestion_rejects_non_docx_files() {
    let path = std::env::temp_dir().join("radsuite-not-a-docx.txt");
    std::fs::write(&path, "not a docx").expect("write text fixture");

    let error = ingest_docx(DocxIngestionRequest {
        project_id: ProjectId::new(),
        path: path.clone(),
        original_filename: "not-a-docx.txt".to_string(),
    })
    .expect_err("reject non-docx");

    assert!(matches!(
        error,
        DocxIngestionError::UnsupportedExtension { path: rejected } if rejected == path
    ));
}

#[test]
fn docx_ingestion_reports_missing_document_xml() {
    let path = write_docx_without_document_xml("docx-ingestion-missing-document.docx");

    let error = ingest_docx(DocxIngestionRequest {
        project_id: ProjectId::new(),
        path,
        original_filename: "broken.docx".to_string(),
    })
    .expect_err("report missing document XML");

    assert!(matches!(error, DocxIngestionError::MissingDocumentXml));
}

#[test]
fn docx_ingestion_decodes_xml_entities() {
    let path = write_docx_with_document_xml(
        "docx-ingestion-xml-entities.docx",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>Smith &amp; Jones (2020) describe A &lt; B.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#,
    );

    let analysed = ingest_docx(DocxIngestionRequest {
        project_id: ProjectId::new(),
        path,
        original_filename: "entities.docx".to_string(),
    })
    .expect("ingest docx");

    assert_eq!(
        analysed.paragraphs[0].text,
        "Smith & Jones (2020) describe A < B."
    );
    assert_eq!(analysed.citations[0].citation_text, "Smith & Jones (2020)");
}

#[test]
fn docx_reading_import_extracts_compulsory_and_optional_candidates() {
    let path = write_docx_with_document_xml(
        "docx-reading-import-candidates.docx",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Compulsory readings</w:t></w:r></w:p>
    <w:p><w:r><w:t>1.2 Smith, J. (2024). Worked examples in practice. https://example.com/worked</w:t></w:r></w:p>
    <w:p><w:r><w:t>This paragraph explains the weekly activity and is not a reference.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Optional readings</w:t></w:r></w:p>
    <w:p><w:r><w:t>Taylor, R. (2023). Optional primer. Teaching Press.</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
    );

    let candidates = extract_docx_reading_candidates(DocxReadingExtractionRequest {
        path,
        original_filename: "module-1-readings.docx".to_string(),
    })
    .expect("extract reading candidates");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].reading_category, ReadingCategory::Compulsory);
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("1.2"));
    assert_eq!(
        candidates[0].apa_citation,
        "Smith, J. (2024). Worked examples in practice. https://example.com/worked"
    );
    assert_eq!(
        candidates[0].url.as_deref(),
        Some("https://example.com/worked")
    );

    assert_eq!(candidates[1].reading_category, ReadingCategory::Optional);
    assert_eq!(candidates[1].lesson_code, None);
    assert_eq!(
        candidates[1].apa_citation,
        "Taylor, R. (2023). Optional primer. Teaching Press."
    );
    assert_eq!(candidates[1].url, None);
}

#[test]
fn docx_reading_import_extracts_composite_module_lesson_markers() {
    let path = write_docx_with_document_xml(
        "docx-reading-import-composite-lesson.docx",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Module 2 lesson 3 Smith, J. (2024). Composite lesson markers.</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
    );

    let candidates = extract_docx_reading_candidates(DocxReadingExtractionRequest {
        path,
        original_filename: "module-2-lesson-3.docx".to_string(),
    })
    .expect("extract reading candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("2.3"));
    assert_eq!(
        candidates[0].apa_citation,
        "Smith, J. (2024). Composite lesson markers."
    );
}

fn write_minimal_docx(filename: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("radsuite-{filename}"));
    let file = File::create(&path).expect("create docx fixture");
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    start_file(&mut zip, "[Content_Types].xml", options);
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )
    .expect("write content types");

    start_file(&mut zip, "_rels/.rels", options);
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdOfficeDocument" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )
    .expect("write package relationships");

    start_file(&mut zip, "word/_rels/document.xml.rels", options);
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.edu/read" TargetMode="External"/>
</Relationships>"#,
    )
    .expect("write document relationships");

    start_file(&mut zip, "word/document.xml", options);
    zip.write_all(document_xml().as_bytes())
        .expect("write document xml");

    zip.finish().expect("finish docx");
    path
}

fn write_docx_with_document_xml(filename: &str, document_xml: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("radsuite-{filename}"));
    let file = File::create(&path).expect("create docx fixture");
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    start_file(&mut zip, "[Content_Types].xml", options);
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )
    .expect("write content types");

    start_file(&mut zip, "word/document.xml", options);
    zip.write_all(document_xml.as_bytes())
        .expect("write document XML");

    zip.finish().expect("finish docx");
    path
}

fn write_docx_without_document_xml(filename: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("radsuite-{filename}"));
    let file = File::create(&path).expect("create broken docx fixture");
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    start_file(&mut zip, "[Content_Types].xml", options);
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#,
    )
    .expect("write content types");

    zip.finish().expect("finish broken docx");
    path
}

fn start_file(zip: &mut ZipWriter<File>, path: &str, options: SimpleFileOptions) {
    zip.start_file(Path::new(path).to_string_lossy(), options)
        .expect("start zip file");
}

fn document_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:r><w:t>Smith (2020) explains worked examples.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>A 2021 survey reported that 64 percent of respondents changed their study habits.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Read the article </w:t></w:r>
      <w:hyperlink r:id="rId1">
        <w:r><w:t>Library link</w:t></w:r>
      </w:hyperlink>
    </w:p>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Week 1</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>Required reading</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#
}
