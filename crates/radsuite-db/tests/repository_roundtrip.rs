use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use radsuite_cite::{DocxIngestionRequest, ingest_docx};
use radsuite_core::{
    ApaValidationStatus, Citation, CourseModule, Document, DocumentFileType, Paragraph, Project,
    ProjectRole, ReadingCategory, ReferenceEntry, ReferenceEntryType, UserId,
};
use radsuite_db::{
    CitationDocumentRepository, CourseModuleRepository, ProjectRepository,
    ReferenceEntryRepository, SqliteCitationDocumentRepository, SqliteCourseModuleRepository,
    SqliteProjectRepository, SqliteReferenceEntryRepository, migrate,
};
use sqlx::sqlite::SqlitePoolOptions;
use zip::{ZipWriter, write::SimpleFileOptions};

#[tokio::test]
async fn project_can_be_inserted_and_listed_for_owner() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let repo = SqliteProjectRepository::new(pool);
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);

    repo.insert_project(&project).await.expect("insert project");
    let rows = repo
        .list_projects_for_user(owner_id)
        .await
        .expect("list projects");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "Legal Method");
    assert_eq!(rows[0].role, ProjectRole::Owner);

    let loaded = repo
        .load_project(project.id)
        .await
        .expect("load project")
        .expect("project exists");

    assert_eq!(loaded, project);
}

#[tokio::test]
async fn project_can_be_loaded_by_code() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let repo = SqliteProjectRepository::new(pool);
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);

    repo.insert_project(&project).await.expect("insert project");

    let loaded = repo
        .load_project_by_code("CRJU150")
        .await
        .expect("load project by code")
        .expect("project exists");

    assert_eq!(loaded, project);
    assert!(
        repo.load_project_by_code("UNKNOWN")
            .await
            .expect("load missing project by code")
            .is_none()
    );
}

#[tokio::test]
async fn radcite_document_can_be_inserted_and_loaded() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let project_repo = SqliteProjectRepository::new(pool.clone());
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);
    project_repo
        .insert_project(&project)
        .await
        .expect("insert project");

    let document_repo = SqliteCitationDocumentRepository::new(pool);
    let document = Document::new(project.id, "lesson-1.docx", DocumentFileType::Docx);
    let mut cited = Paragraph::new(
        document.id,
        0,
        "Research shows that worked examples reduce cognitive load (Smith, 2020).",
    );
    cited.page = Some(1);
    let mut missing = Paragraph::new(
        document.id,
        1,
        "A 2021 survey reported that 64 percent of respondents changed their study habits.",
    );
    missing.page = Some(2);
    missing.needs_citation = true;
    let citation = Citation::new(cited.id, "(Smith, 2020)", 58, 71);

    document_repo
        .insert_document_analysis(
            &document,
            &[cited.clone(), missing.clone()],
            std::slice::from_ref(&citation),
        )
        .await
        .expect("insert document analysis");

    let summaries = document_repo
        .list_documents_for_project(project.id)
        .await
        .expect("list documents");

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].document_id, document.id);
    assert_eq!(summaries[0].original_filename, "lesson-1.docx");
    assert_eq!(summaries[0].paragraph_count, 2);
    assert_eq!(summaries[0].citation_count, 1);
    assert_eq!(summaries[0].missing_citation_count, 1);

    let loaded = document_repo
        .load_document_analysis(document.id)
        .await
        .expect("load document")
        .expect("document exists");

    assert_eq!(loaded.document.id, document.id);
    assert_eq!(loaded.paragraphs, vec![cited, missing]);
    assert_eq!(loaded.citations, vec![citation]);
}

#[tokio::test]
async fn reference_entries_can_be_inserted_and_listed_for_project() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let project_repo = SqliteProjectRepository::new(pool.clone());
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);
    project_repo
        .insert_project(&project)
        .await
        .expect("insert project");

    let reference_repo = SqliteReferenceEntryRepository::new(pool);
    let mut reference = ReferenceEntry::new(project.id, ReferenceEntryType::Reference);
    reference.apa_citation =
        Some("Smith, J. (2020). Worked examples in practice. Learning Press.".to_string());
    reference.authors = vec!["Smith, J.".to_string()];
    reference.publication_year = Some("2020".to_string());
    reference.title = Some("Worked examples in practice".to_string());
    reference.source = Some("Learning Press".to_string());
    reference.display_order = Some(1);

    reference_repo
        .insert_reference_entry(&reference)
        .await
        .expect("insert reference entry");

    let references = reference_repo
        .list_reference_entries_for_project(project.id, ReferenceEntryType::Reference)
        .await
        .expect("list reference entries");

    assert_eq!(references, vec![reference]);
    assert_eq!(
        references[0].apa_validation_status,
        ApaValidationStatus::Unknown
    );
}

#[tokio::test]
async fn module_readings_can_be_inserted_and_listed_for_module() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let project_repo = SqliteProjectRepository::new(pool.clone());
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);
    project_repo
        .insert_project(&project)
        .await
        .expect("insert project");

    let module_repo = SqliteCourseModuleRepository::new(pool.clone());
    let mut first_module = CourseModule::new(project.id, "Module 1", Some(1));
    first_module.code = Some("M1".to_string());
    first_module.description = Some("Introductory material".to_string());
    let second_module = CourseModule::new(project.id, "Module 2", Some(2));

    module_repo
        .insert_course_module(&first_module)
        .await
        .expect("insert first module");
    module_repo
        .insert_course_module(&second_module)
        .await
        .expect("insert second module");

    let modules = module_repo
        .list_course_modules_for_project(project.id)
        .await
        .expect("list modules");

    assert_eq!(modules, vec![first_module.clone(), second_module.clone()]);

    let reference_repo = SqliteReferenceEntryRepository::new(pool);
    let mut reading = ReferenceEntry::new(project.id, ReferenceEntryType::Reading);
    reading.module_id = Some(first_module.id);
    reading.reading_category = Some(ReadingCategory::Optional);
    reading.lesson_code = Some("1.2".to_string());
    reading.apa_citation =
        Some("Smith, J. (2024). Optional module reading. Learning Journal.".to_string());
    reading.url = Some("https://example.com/reading".to_string());
    reading.reading_notes = Some("Skim before class".to_string());
    reading.estimated_reading_time = Some("15 minutes".to_string());
    reading.display_order = Some(1);

    let mut other_module_reading = ReferenceEntry::new(project.id, ReferenceEntryType::Reading);
    other_module_reading.module_id = Some(second_module.id);
    other_module_reading.apa_citation = Some("Other, A. (2024). Another reading.".to_string());

    reference_repo
        .insert_reference_entry(&reading)
        .await
        .expect("insert reading");
    reference_repo
        .insert_reference_entry(&other_module_reading)
        .await
        .expect("insert other reading");

    let readings = reference_repo
        .list_reference_entries_for_module(first_module.id, ReferenceEntryType::Reading)
        .await
        .expect("list readings");

    assert_eq!(readings, vec![reading]);
}

#[tokio::test]
async fn paragraph_citation_can_be_linked_to_reference_entry() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let project_repo = SqliteProjectRepository::new(pool.clone());
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);
    project_repo
        .insert_project(&project)
        .await
        .expect("insert project");

    let reference_repo = SqliteReferenceEntryRepository::new(pool.clone());
    let mut reference = ReferenceEntry::new(project.id, ReferenceEntryType::Reference);
    reference.apa_citation =
        Some("Smith, J. (2020). Worked examples in practice. Learning Press.".to_string());
    reference_repo
        .insert_reference_entry(&reference)
        .await
        .expect("insert reference entry");

    let document_repo = SqliteCitationDocumentRepository::new(pool);
    let document = Document::new(project.id, "lesson-1.docx", DocumentFileType::Docx);
    let cited = Paragraph::new(document.id, 0, "Smith (2020) explains worked examples.");
    let citation = Citation::new(cited.id, "Smith (2020)", 0, 12);

    document_repo
        .insert_document_analysis(
            &document,
            std::slice::from_ref(&cited),
            std::slice::from_ref(&citation),
        )
        .await
        .expect("insert document analysis");

    document_repo
        .link_citation_to_reference(citation.id, reference.id)
        .await
        .expect("link citation to reference");

    let loaded = document_repo
        .load_document_analysis(document.id)
        .await
        .expect("load document")
        .expect("document exists");

    assert_eq!(loaded.citations[0].reference_entry_id, Some(reference.id));
    assert!(!loaded.citations[0].verified);
}

#[tokio::test]
async fn radcite_review_actions_are_persisted() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let project_repo = SqliteProjectRepository::new(pool.clone());
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);
    project_repo
        .insert_project(&project)
        .await
        .expect("insert project");

    let document_repo = SqliteCitationDocumentRepository::new(pool);
    let document = Document::new(project.id, "lesson-1.docx", DocumentFileType::Docx);
    let cited = Paragraph::new(
        document.id,
        0,
        "Research shows that worked examples reduce cognitive load (Smith, 2020).",
    );
    let mut missing = Paragraph::new(
        document.id,
        1,
        "A 2021 survey reported that 64 percent of respondents changed their study habits.",
    );
    missing.needs_citation = true;
    let citation = Citation::new(cited.id, "(Smith, 2020)", 58, 71);

    document_repo
        .insert_document_analysis(
            &document,
            &[cited.clone(), missing.clone()],
            std::slice::from_ref(&citation),
        )
        .await
        .expect("insert document analysis");

    document_repo
        .verify_paragraph_citations(cited.id)
        .await
        .expect("verify paragraph citations");
    document_repo
        .mark_paragraph_resolved(missing.id)
        .await
        .expect("mark paragraph resolved");
    let manual_citation = document_repo
        .insert_manual_citation(missing.id, "Jones (2024)")
        .await
        .expect("insert manual citation");

    assert_eq!(manual_citation.paragraph_id, missing.id);
    assert_eq!(manual_citation.citation_text, "Jones (2024)");
    assert_eq!(manual_citation.position_start, None);
    assert_eq!(manual_citation.position_end, None);
    assert!(manual_citation.verified);

    let summaries = document_repo
        .list_documents_for_project(project.id)
        .await
        .expect("list documents");

    assert_eq!(summaries[0].citation_count, 2);
    assert_eq!(summaries[0].missing_citation_count, 0);

    let loaded = document_repo
        .load_document_analysis(document.id)
        .await
        .expect("load document")
        .expect("document exists");

    assert!(!loaded.paragraphs[1].needs_citation);
    assert!(
        loaded
            .citations
            .iter()
            .any(|item| item.id == citation.id && item.verified)
    );
    assert!(
        loaded
            .citations
            .iter()
            .any(|item| item.id == manual_citation.id)
    );
}

#[tokio::test]
async fn saved_radcite_documents_can_be_listed_across_projects() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let project_repo = SqliteProjectRepository::new(pool.clone());
    let owner_id = UserId::new();
    let first_project = Project::new("CRJU150", "Legal Method", owner_id);
    let second_project = Project::new("HLTH430", "Motivating Change", owner_id);
    project_repo
        .insert_project(&first_project)
        .await
        .expect("insert first project");
    project_repo
        .insert_project(&second_project)
        .await
        .expect("insert second project");

    let document_repo = SqliteCitationDocumentRepository::new(pool);
    let first_document = Document::new(first_project.id, "lesson-1.docx", DocumentFileType::Docx);
    let second_document = Document::new(second_project.id, "lesson-2.docx", DocumentFileType::Docx);
    let mut needs_citation = Paragraph::new(
        first_document.id,
        0,
        "A 2021 survey reported that 64 percent of respondents changed their study habits.",
    );
    needs_citation.needs_citation = true;
    let cited = Paragraph::new(
        second_document.id,
        0,
        "Smith (2020) explains worked examples.",
    );
    let citation = Citation::new(cited.id, "Smith (2020)", 0, 12);

    document_repo
        .insert_document_analysis(&first_document, &[needs_citation], &[])
        .await
        .expect("insert first document analysis");
    document_repo
        .insert_document_analysis(&second_document, &[cited], &[citation])
        .await
        .expect("insert second document analysis");

    let documents = document_repo
        .list_saved_documents()
        .await
        .expect("list saved documents");

    assert_eq!(documents.len(), 2);
    assert!(documents.iter().any(|item| {
        item.document_id == first_document.id
            && item.project_id == first_project.id
            && item.original_filename == "lesson-1.docx"
            && item.paragraph_count == 1
            && item.citation_count == 0
            && item.missing_citation_count == 1
    }));
    assert!(documents.iter().any(|item| {
        item.document_id == second_document.id
            && item.project_id == second_project.id
            && item.original_filename == "lesson-2.docx"
            && item.paragraph_count == 1
            && item.citation_count == 1
            && item.missing_citation_count == 0
    }));
}

#[tokio::test]
async fn radcite_docx_ingestion_result_can_be_persisted() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");

    let project_repo = SqliteProjectRepository::new(pool.clone());
    let owner_id = UserId::new();
    let project = Project::new("CRJU150", "Legal Method", owner_id);
    project_repo
        .insert_project(&project)
        .await
        .expect("insert project");

    let analysed = ingest_docx(DocxIngestionRequest {
        project_id: project.id,
        path: write_minimal_docx("db-docx-ingestion.docx"),
        original_filename: "lesson-2.docx".to_string(),
    })
    .expect("ingest docx");

    let document_repo = SqliteCitationDocumentRepository::new(pool);
    document_repo
        .insert_document_analysis(
            &analysed.document,
            &analysed.paragraphs,
            &analysed.citations,
        )
        .await
        .expect("insert ingested document analysis");

    let summaries = document_repo
        .list_documents_for_project(project.id)
        .await
        .expect("list documents");

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].document_id, analysed.document.id);
    assert_eq!(summaries[0].original_filename, "lesson-2.docx");
    assert_eq!(summaries[0].paragraph_count, 2);
    assert_eq!(summaries[0].citation_count, 1);
    assert_eq!(summaries[0].missing_citation_count, 1);

    let loaded = document_repo
        .load_document_analysis(analysed.document.id)
        .await
        .expect("load document")
        .expect("document exists");

    assert_eq!(loaded.document, analysed.document);
    assert_eq!(loaded.paragraphs, analysed.paragraphs);
    assert_eq!(loaded.citations, analysed.citations);
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

    start_file(&mut zip, "word/document.xml", options);
    zip.write_all(document_xml().as_bytes())
        .expect("write document XML");

    zip.finish().expect("finish docx");
    path
}

fn start_file(zip: &mut ZipWriter<File>, path: &str, options: SimpleFileOptions) {
    zip.start_file(Path::new(path).to_string_lossy(), options)
        .expect("start zip file");
}

fn document_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>Smith (2020) explains worked examples.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>A 2021 survey reported that 64 percent of respondents changed their study habits.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#
}
