use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use radsuite_db::migrate;
use radsuite_desktop::{
    AddCourseReferenceRequest, AddManualCitationRequest, AnalyseDocxError, AnalyseDocxRequest,
    AppPaths, DesktopState, UpdateParagraphReviewRequest, add_course_reference,
    add_manual_citation_for_review, analyse_docx_for_review, analyse_docx_path, get_app_status,
    list_course_references, list_saved_radcite_reviews, load_saved_radcite_review,
    mark_paragraph_resolved_for_review, verify_paragraph_citations_for_review,
};
use sqlx::sqlite::SqlitePoolOptions;
use zip::{ZipWriter, write::SimpleFileOptions};

#[test]
fn app_paths_resolve_platform_data_directory_for_radsuite() {
    let paths = AppPaths::for_app("RADsuite").expect("resolve app paths");
    let data_dir = paths.data_dir.to_string_lossy();

    assert!(paths.data_dir.is_absolute());
    assert!(data_dir.to_lowercase().contains("radsuite"));
}

#[tokio::test]
async fn app_status_exposes_database_sync_and_engine_state() {
    let state = DesktopState::for_tests();
    let status = get_app_status(&state);

    assert_eq!(status.app_name, "RADsuite");
    assert!(status.database_ready);
    assert!(!status.sync_configured);
    assert_eq!(status.engines.len(), 4);
}

#[tokio::test]
async fn analyse_docx_path_persists_document_and_returns_summary() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-command-analysis.docx");

    let response = analyse_docx_path(
        &state,
        AnalyseDocxRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("lesson-3.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx");

    assert_eq!(response.original_filename, "lesson-3.docx");
    assert_eq!(response.paragraph_count, 2);
    assert_eq!(response.citation_count, 1);
    assert_eq!(response.missing_citation_count, 1);
    assert_eq!(response.project_title, "RADcite Functional Testing");
}

#[tokio::test]
async fn analyse_docx_for_review_returns_ordered_paragraphs_and_citations() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-review-analysis.docx");

    let response = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("review-source.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    assert_eq!(response.original_filename, "review-source.docx");
    assert_eq!(response.summary.paragraph_count, 2);
    assert_eq!(response.summary.citation_count, 1);
    assert_eq!(response.summary.cited_paragraph_count, 1);
    assert_eq!(response.summary.missing_citation_count, 1);
    assert_eq!(response.paragraphs.len(), 2);
    assert_eq!(response.paragraphs[0].order_index, 0);
    assert_eq!(response.paragraphs[0].citations.len(), 1);
    assert_eq!(response.paragraphs[0].citations[0].text, "Smith (2020)");
    assert!(!response.paragraphs[0].needs_citation);
    assert_eq!(response.paragraphs[1].order_index, 1);
    assert!(response.paragraphs[1].needs_citation);
}

#[tokio::test]
async fn radcite_review_actions_persist_and_return_refreshed_review() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-review-actions.docx");

    let response = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("review-actions.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    let cited_paragraph_id = response.paragraphs[0].id;
    let missing_paragraph_id = response.paragraphs[1].id;

    let verified = verify_paragraph_citations_for_review(
        &state,
        UpdateParagraphReviewRequest {
            document_id: response.document_id,
            paragraph_id: cited_paragraph_id,
        },
    )
    .await
    .expect("verify citations");

    assert!(
        verified.paragraphs[0]
            .citations
            .iter()
            .all(|citation| citation.verified)
    );

    let resolved = mark_paragraph_resolved_for_review(
        &state,
        UpdateParagraphReviewRequest {
            document_id: response.document_id,
            paragraph_id: missing_paragraph_id,
        },
    )
    .await
    .expect("mark resolved");

    assert!(!resolved.paragraphs[1].needs_citation);
    assert_eq!(resolved.summary.missing_citation_count, 0);

    let with_manual_citation = add_manual_citation_for_review(
        &state,
        AddManualCitationRequest {
            document_id: response.document_id,
            paragraph_id: missing_paragraph_id,
            citation_text: " Jones (2024) ".to_string(),
        },
    )
    .await
    .expect("add manual citation");

    assert_eq!(with_manual_citation.summary.citation_count, 2);
    assert!(
        with_manual_citation.paragraphs[1]
            .citations
            .iter()
            .any(|citation| {
                citation.text == "Jones (2024)"
                    && citation.start.is_none()
                    && citation.end.is_none()
                    && citation.verified
            })
    );
}

#[tokio::test]
async fn saved_radcite_review_can_be_listed_and_loaded() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-saved-review.docx");

    let response = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("saved-review.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");
    let missing_paragraph_id = response.paragraphs[1].id;

    add_manual_citation_for_review(
        &state,
        AddManualCitationRequest {
            document_id: response.document_id,
            paragraph_id: missing_paragraph_id,
            citation_text: "Jones (2024)".to_string(),
        },
    )
    .await
    .expect("add manual citation");

    let saved_reviews = list_saved_radcite_reviews(&state)
        .await
        .expect("list saved reviews");

    assert_eq!(saved_reviews.len(), 1);
    assert_eq!(saved_reviews[0].document_id, response.document_id);
    assert_eq!(saved_reviews[0].original_filename, "saved-review.docx");
    assert_eq!(saved_reviews[0].paragraph_count, 2);
    assert_eq!(saved_reviews[0].citation_count, 2);
    assert_eq!(saved_reviews[0].missing_citation_count, 0);

    let loaded = load_saved_radcite_review(&state, response.document_id)
        .await
        .expect("load saved review");

    assert_eq!(loaded.document_id, response.document_id);
    assert_eq!(loaded.original_filename, "saved-review.docx");
    assert_eq!(loaded.summary.citation_count, 2);
    assert_eq!(loaded.summary.missing_citation_count, 0);
    assert!(loaded.paragraphs[1].citations.iter().any(|citation| {
        citation.text == "Jones (2024)" && citation.verified && citation.start.is_none()
    }));
}

#[tokio::test]
async fn analysed_docx_reviews_reuse_the_local_radcite_project() {
    let state = desktop_state_with_migrated_pool().await;
    let first_path = write_minimal_docx("desktop-first-local-project.docx");
    let second_path = write_minimal_docx("desktop-second-local-project.docx");

    let first = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            path: first_path.to_string_lossy().into_owned(),
            original_filename: Some("first-local-project.docx".to_string()),
        },
    )
    .await
    .expect("analyse first docx for review");

    let second = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            path: second_path.to_string_lossy().into_owned(),
            original_filename: Some("second-local-project.docx".to_string()),
        },
    )
    .await
    .expect("analyse second docx for review");

    assert_eq!(first.project_id, second.project_id);
    assert_eq!(first.project_title, "RADcite Functional Testing");
    assert_eq!(second.project_title, "RADcite Functional Testing");

    let saved_reviews = list_saved_radcite_reviews(&state)
        .await
        .expect("list saved reviews");

    assert_eq!(saved_reviews.len(), 2);
    assert!(
        saved_reviews
            .iter()
            .all(|review| review.project_id == first.project_id)
    );
}

#[tokio::test]
async fn local_course_references_are_added_to_the_radcite_project() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-reference-project.docx");

    let analysis = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("reference-project.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    let added = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: Some("Core course reference".to_string()),
        },
    )
    .await
    .expect("add course reference");

    assert_eq!(added.project_id, analysis.project_id);
    assert_eq!(
        added.apa_citation.as_deref(),
        Some("Smith, J. (2020). Worked examples in practice. Learning Press.")
    );
    assert_eq!(added.notes.as_deref(), Some("Core course reference"));
    assert_eq!(added.reference_type, "reference");

    let references = list_course_references(&state)
        .await
        .expect("list course references");

    assert_eq!(references.len(), 1);
    assert_eq!(references[0], added);
}

#[tokio::test]
async fn analyse_docx_path_rejects_empty_path() {
    let state = desktop_state_with_migrated_pool().await;

    let error = analyse_docx_path(
        &state,
        AnalyseDocxRequest {
            path: "  ".to_string(),
            original_filename: None,
        },
    )
    .await
    .expect_err("reject empty path");

    assert!(matches!(error, AnalyseDocxError::EmptyPath));
}

async fn desktop_state_with_migrated_pool() -> DesktopState {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    migrate(&pool).await.expect("migrate");
    DesktopState::for_tests_with_pool(pool)
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
