use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use radsuite_core::{
    DocumentVariant, ModuleId, ProjectId, ReadingCategory, ReferenceEntry, ReferenceEntryId,
    ReferenceEntryType,
};
use radsuite_db::{ReferenceEntryRepository, SqliteReferenceEntryRepository, migrate};
use radsuite_desktop::{
    AddCourseReferenceRequest, AddManualCitationRequest, AddModuleReadingRequest,
    AddRadciteModuleRequest, AnalyseDocxError, AnalyseDocxRequest, AnalysePdfRequest, AppPaths,
    ArchiveCourseReferenceRequest, ArchiveModuleReadingRequest, ArchiveRadciteDocumentRequest,
    ArchiveRadciteModuleRequest, ArchiveRadciteProjectRequest, AssignCourseReferenceModuleRequest,
    CourseReferenceError, CreateRadciteProjectRequest, DesktopState, ExportCourseReferencesRequest,
    ExportModuleReadingsRequest, ExportRadciteReviewReportRequest, ImportDocumentReadingsRequest,
    LinkCitationReferenceRequest, ListCourseReferencesRequest, ListModuleReadingsRequest,
    ListRadciteArchiveRequest, ListRadciteModulesRequest, ListSavedReviewsRequest,
    MergeCourseReferencesRequest, ModuleReadingError, ModuleReadingExportError,
    ModuleReadingImportError, PreviewModuleReadingsCsvImportRequest,
    PreviewModuleReadingsImportRequest, PreviewModuleReadingsPdfImportRequest,
    RadciteArchiveItemKind, RadciteDocumentError, RadciteModuleError, RadciteProjectError,
    RestoreRadciteArchiveItemRequest, RestoreRadciteProjectRequest,
    SaveModuleReadingsImportCandidate, SaveModuleReadingsImportRequest,
    UpdateCourseReferenceRequest, UpdateModuleReadingRequest, UpdateParagraphReviewRequest,
    UpdateRadciteDocumentRequest, UpdateRadciteModuleRequest, UpdateRadciteProjectRequest,
    add_course_reference, add_manual_citation_for_review, add_module_reading, add_radcite_module,
    analyse_docx_for_review, analyse_docx_path, analyse_pdf_for_review, archive_course_reference,
    archive_module_reading, archive_radcite_document, archive_radcite_module,
    archive_radcite_project, assign_course_reference_module, create_radcite_project,
    export_course_references, export_module_readings, export_radcite_review_report, get_app_status,
    import_document_readings, link_citation_to_reference_for_review, list_course_references,
    list_module_readings, list_radcite_archive, list_radcite_modules, list_radcite_projects,
    list_saved_radcite_reviews, load_saved_radcite_review, mark_paragraph_resolved_for_review,
    merge_course_references, preview_module_readings_csv_import, preview_module_readings_import,
    preview_module_readings_pdf_import, restore_radcite_archive_item, restore_radcite_project,
    save_module_readings_import, update_course_reference, update_module_reading,
    update_radcite_document, update_radcite_module, update_radcite_project,
    verify_paragraph_citations_for_review,
};
use serde_json::Value;
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
async fn local_radcite_projects_can_be_listed_and_created() {
    let state = desktop_state_with_migrated_pool().await;

    let initial_projects = list_radcite_projects(&state)
        .await
        .expect("list initial projects");

    assert_eq!(initial_projects.len(), 1);
    assert_eq!(initial_projects[0].code.as_deref(), Some("CRJU150"));
    assert_eq!(initial_projects[0].title, "RADcite Functional Testing");

    let created = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some(" CRJU201 ".to_string()),
            title: " Criminological Theory ".to_string(),
        },
    )
    .await
    .expect("create project");

    assert_eq!(created.code.as_deref(), Some("CRJU201"));
    assert_eq!(created.title, "Criminological Theory");

    let projects = list_radcite_projects(&state)
        .await
        .expect("list projects after create");

    assert_eq!(projects.len(), 2);
    assert!(projects.iter().any(|project| project.id == created.id));
}

#[tokio::test]
async fn local_radcite_projects_can_be_updated() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let project_id = projects[0].id;

    let updated = update_radcite_project(
        &state,
        UpdateRadciteProjectRequest {
            project_id,
            code: Some(" COMS432 ".to_string()),
            title: " Strategic Communication ".to_string(),
            description: Some(" Course foundations and applied practice ".to_string()),
            structure_mode: "weeks".to_string(),
        },
    )
    .await
    .expect("update project");

    assert_eq!(updated.id, project_id);
    assert_eq!(updated.code.as_deref(), Some("COMS432"));
    assert_eq!(updated.title, "Strategic Communication");
    assert_eq!(
        updated.description.as_deref(),
        Some("Course foundations and applied practice")
    );
    assert_eq!(updated.structure_mode, "weeks");

    let listed = list_radcite_projects(&state)
        .await
        .expect("list updated projects");
    let listed_project = listed
        .iter()
        .find(|project| project.id == project_id)
        .expect("updated project is listed");
    assert_eq!(listed_project.code.as_deref(), Some("COMS432"));
    assert_eq!(listed_project.title, "Strategic Communication");
    assert_eq!(
        listed_project.description.as_deref(),
        Some("Course foundations and applied practice")
    );
    assert_eq!(listed_project.structure_mode, "weeks");
}

#[tokio::test]
async fn archived_radcite_projects_cannot_be_updated() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let project_id = projects[0].id;

    archive_radcite_project(&state, ArchiveRadciteProjectRequest { project_id })
        .await
        .expect("archive project");

    let error = update_radcite_project(
        &state,
        UpdateRadciteProjectRequest {
            project_id,
            code: Some("COMS432".to_string()),
            title: "Strategic Communication".to_string(),
            description: None,
            structure_mode: "modules".to_string(),
        },
    )
    .await
    .expect_err("archived project should be read-only");

    assert!(matches!(error, RadciteProjectError::ArchivedProject(id) if id == project_id));
}

#[tokio::test]
async fn project_updates_reject_unknown_structure_modes() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let project_id = projects[0].id;

    let error = update_radcite_project(
        &state,
        UpdateRadciteProjectRequest {
            project_id,
            code: Some("CRJU150".to_string()),
            title: "Legal Method".to_string(),
            description: None,
            structure_mode: "terms".to_string(),
        },
    )
    .await
    .expect_err("unknown structure mode should fail");

    assert!(matches!(error, RadciteProjectError::InvalidStructureMode));
}

#[tokio::test]
async fn local_radcite_projects_can_be_archived_and_restored() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let project_id = projects[0].id;

    let archived = archive_radcite_project(&state, ArchiveRadciteProjectRequest { project_id })
        .await
        .expect("archive project");
    assert_eq!(archived.id, project_id);
    assert!(archived.archived_at.is_some());

    let listed = list_radcite_projects(&state)
        .await
        .expect("list archived project");
    assert_eq!(listed[0].archived_at, archived.archived_at);

    let restored = restore_radcite_project(&state, RestoreRadciteProjectRequest { project_id })
        .await
        .expect("restore project");
    assert_eq!(restored.id, project_id);
    assert!(restored.archived_at.is_none());

    let missing = archive_radcite_project(
        &state,
        ArchiveRadciteProjectRequest {
            project_id: ProjectId::new(),
        },
    )
    .await
    .expect_err("missing project should fail");
    assert!(matches!(missing, RadciteProjectError::MissingProject(_)));
}

#[tokio::test]
async fn radcite_commands_respect_selected_project_context() {
    let state = desktop_state_with_migrated_pool().await;
    let crju201 = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("CRJU201".to_string()),
            title: "Criminological Theory".to_string(),
        },
    )
    .await
    .expect("create CRJU201 project");
    let coms432 = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("COMS432".to_string()),
            title: "Strategic Communication".to_string(),
        },
    )
    .await
    .expect("create COMS432 project");
    let crju_path = write_minimal_docx("desktop-crju201-project.docx");
    let coms_path = write_minimal_docx("desktop-coms432-project.docx");

    let crju_doc = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: Some(crju201.id),
            path: crju_path.to_string_lossy().into_owned(),
            original_filename: Some("crju201.docx".to_string()),
        },
    )
    .await
    .expect("analyse CRJU201 docx");
    let coms_doc = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: Some(coms432.id),
            path: coms_path.to_string_lossy().into_owned(),
            original_filename: Some("coms432.docx".to_string()),
        },
    )
    .await
    .expect("analyse COMS432 docx");

    assert_eq!(crju_doc.project_id, crju201.id);
    assert_eq!(crju_doc.project_title, "Criminological Theory");
    assert_eq!(coms_doc.project_id, coms432.id);
    assert_eq!(coms_doc.project_title, "Strategic Communication");

    let crju_reviews = list_saved_radcite_reviews(
        &state,
        ListSavedReviewsRequest {
            project_id: Some(crju201.id),
        },
    )
    .await
    .expect("list CRJU201 saved reviews");
    let coms_reviews = list_saved_radcite_reviews(
        &state,
        ListSavedReviewsRequest {
            project_id: Some(coms432.id),
        },
    )
    .await
    .expect("list COMS432 saved reviews");

    assert_eq!(crju_reviews.len(), 1);
    assert_eq!(crju_reviews[0].document_id, crju_doc.document_id);
    assert_eq!(coms_reviews.len(), 1);
    assert_eq!(coms_reviews[0].document_id, coms_doc.document_id);

    let crju_reference = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: Some(crju201.id),
            apa_citation: "Smith, J. (2024). CRJU reference.".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add CRJU reference");
    let coms_reference = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: Some(coms432.id),
            apa_citation: "Taylor, R. (2024). COMS reference.".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add COMS reference");

    assert_eq!(
        list_course_references(
            &state,
            ListCourseReferencesRequest {
                project_id: Some(crju201.id),
            },
        )
        .await
        .expect("list CRJU references"),
        vec![crju_reference]
    );
    assert_eq!(
        list_course_references(
            &state,
            ListCourseReferencesRequest {
                project_id: Some(coms432.id),
            },
        )
        .await
        .expect("list COMS references"),
        vec![coms_reference]
    );

    let crju_module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: Some(crju201.id),
            title: "CRJU Module".to_string(),
            code: Some("M1".to_string()),
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add CRJU module");
    let coms_module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: Some(coms432.id),
            title: "COMS Module".to_string(),
            code: Some("M1".to_string()),
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add COMS module");

    assert_eq!(
        list_radcite_modules(
            &state,
            ListRadciteModulesRequest {
                project_id: Some(crju201.id),
            },
        )
        .await
        .expect("list CRJU modules"),
        vec![crju_module]
    );
    assert_eq!(
        list_radcite_modules(
            &state,
            ListRadciteModulesRequest {
                project_id: Some(coms432.id),
            },
        )
        .await
        .expect("list COMS modules"),
        vec![coms_module]
    );

    let crju_export = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: Some(crju201.id),
            for_ako_learn: false,
            allow_incomplete: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export CRJU references");

    assert_eq!(crju_export.filename, "crju201-course-references.html");
    assert_eq!(crju_export.reference_count, 1);
}

#[tokio::test]
async fn selected_project_commands_reject_missing_projects() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-missing-project.docx");
    let missing_project_id = ProjectId::new();

    let error = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: Some(missing_project_id),
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("missing-project.docx".to_string()),
        },
    )
    .await
    .expect_err("reject missing project");

    assert!(matches!(
        error,
        AnalyseDocxError::MissingProject(project_id) if project_id == missing_project_id
    ));
}

#[tokio::test]
async fn analyse_docx_path_persists_document_and_returns_summary() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-command-analysis.docx");

    let response = analyse_docx_path(
        &state,
        AnalyseDocxRequest {
            project_id: None,
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
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("review-source.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    assert_eq!(response.original_filename, "review-source.docx");
    assert_eq!(response.source_file_type, "docx");
    assert!(
        response.source_path.as_ref().is_some_and(|path| {
            Path::new(path).is_file() && path.ends_with("review-source.docx")
        })
    );
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
async fn review_report_export_contains_document_statistics_and_findings() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-review-report.docx");

    let review = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("review-source.docx".to_string()),
        },
    )
    .await
    .expect("analyse report source");

    let export = export_radcite_review_report(
        &state,
        ExportRadciteReviewReportRequest {
            document_id: review.document_id,
        },
    )
    .await
    .expect("export review report");

    assert_eq!(export.filename, "review-source-citation-report.json");
    assert_eq!(export.content_type, "application/json; charset=utf-8");

    let report: Value = serde_json::from_str(&export.json).expect("parse report JSON");
    assert_eq!(report["filename"], "review-source.docx");
    assert_eq!(report["file_type"], "docx");
    assert_eq!(report["project_title"], "RADcite Functional Testing");
    assert_eq!(report["statistics"]["total_paragraphs"], 2);
    assert_eq!(report["statistics"]["paragraphs_with_citations"], 1);
    assert_eq!(report["statistics"]["paragraphs_needing_citations"], 1);
    assert_eq!(report["statistics"]["total_citations"], 1);
    assert_eq!(report["statistics"]["citation_coverage"], "50.0%");
    assert_eq!(
        report["details"].as_array().expect("report details").len(),
        2
    );
    assert_eq!(report["details"][0]["citations"][0], "Smith (2020)");
    assert_eq!(report["details"][1]["needs_citation"], true);
}

#[tokio::test]
async fn analyse_pdf_for_review_persists_ordered_paragraphs_and_citations() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_readings_import_pdf(
        "desktop-review-analysis.pdf",
        &[
            "Smith (2020) explains worked examples.",
            "A 2021 survey reported that 64 percent of respondents changed their study habits.",
        ],
    );

    let response = analyse_pdf_for_review(
        &state,
        AnalysePdfRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("review-source.pdf".to_string()),
        },
    )
    .await
    .expect("analyse PDF for review");

    assert_eq!(response.original_filename, "review-source.pdf");
    assert_eq!(response.source_file_type, "pdf");
    assert!(
        response.source_path.as_ref().is_some_and(|path| {
            Path::new(path).is_file() && path.ends_with("review-source.pdf")
        })
    );
    assert_eq!(response.summary.paragraph_count, 2);
    assert_eq!(response.summary.citation_count, 1);
    assert_eq!(response.summary.cited_paragraph_count, 1);
    assert_eq!(response.summary.missing_citation_count, 1);
    assert_eq!(response.paragraphs.len(), 2);
    assert_eq!(response.paragraphs[0].citations[0].text, "Smith (2020)");

    let saved = list_saved_radcite_reviews(&state, ListSavedReviewsRequest { project_id: None })
        .await
        .expect("list saved PDF review");
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].original_filename, "review-source.pdf");
    assert_eq!(saved[0].source_file_type, "pdf");
    assert_eq!(saved[0].source_path, response.source_path);
}

#[tokio::test]
async fn source_copy_failure_does_not_persist_a_saved_review() {
    let state = desktop_state_with_migrated_pool().await;
    let root =
        std::env::temp_dir().join(format!("radsuite-invalid-source-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create invalid source directory");
    let path = root.join("not-a-file.docx");

    let error = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("not-a-file.docx".to_string()),
        },
    )
    .await
    .expect_err("directory source should fail before persistence");

    assert!(matches!(error, AnalyseDocxError::Storage(_)));
    assert!(
        list_saved_radcite_reviews(&state, ListSavedReviewsRequest::default())
            .await
            .expect("list saved reviews")
            .is_empty()
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn radcite_review_actions_persist_and_return_refreshed_review() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-review-actions.docx");

    let response = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
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
            project_id: None,
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

    let saved_reviews = list_saved_radcite_reviews(&state, ListSavedReviewsRequest::default())
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
async fn radcite_document_metadata_contract() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-document-metadata.docx");
    let response = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("document-metadata.docx".to_string()),
        },
    )
    .await
    .expect("analyse document");

    assert_eq!(response.display_name, "document-metadata.docx");
    assert_eq!(response.doc_variant, "content");
    assert_eq!(response.doc_number, None);
    assert!(!response.exclude_from_references);

    let invalid_number = update_radcite_document(
        &state,
        UpdateRadciteDocumentRequest {
            project_id: None,
            document_id: response.document_id,
            display_name: "Week 1".to_string(),
            doc_number: Some(0),
            doc_variant: DocumentVariant::Rise,
            exclude_from_references: true,
        },
    )
    .await
    .expect_err("reject non-positive document number");
    assert!(matches!(
        invalid_number,
        RadciteDocumentError::InvalidDocumentNumber(0)
    ));

    let other_project = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("OTHER".to_string()),
            title: "Other project".to_string(),
        },
    )
    .await
    .expect("create second project");
    let mismatched_project = update_radcite_document(
        &state,
        UpdateRadciteDocumentRequest {
            project_id: Some(other_project.id),
            document_id: response.document_id,
            display_name: "Week 1".to_string(),
            doc_number: Some(1),
            doc_variant: DocumentVariant::Rise,
            exclude_from_references: true,
        },
    )
    .await
    .expect_err("reject project mismatch");
    assert!(matches!(
        mismatched_project,
        RadciteDocumentError::ProjectMismatch { .. }
    ));

    let updated = update_radcite_document(
        &state,
        UpdateRadciteDocumentRequest {
            project_id: None,
            document_id: response.document_id,
            display_name: " Week 1 reading ".to_string(),
            doc_number: Some(3),
            doc_variant: DocumentVariant::Rise,
            exclude_from_references: true,
        },
    )
    .await
    .expect("update document metadata");
    assert_eq!(updated.display_name, "Week 1 reading");
    assert_eq!(updated.original_filename, "document-metadata.docx");
    assert_eq!(updated.doc_variant, "rise");
    assert_eq!(updated.doc_number, Some(3));
    assert!(updated.exclude_from_references);

    let loaded = load_saved_radcite_review(&state, response.document_id)
        .await
        .expect("load updated review");
    assert_eq!(loaded.display_name, "Week 1 reading");
    assert_eq!(loaded.doc_variant, "rise");
    assert_eq!(loaded.doc_number, Some(3));
    assert!(loaded.exclude_from_references);

    archive_radcite_document(
        &state,
        ArchiveRadciteDocumentRequest {
            project_id: None,
            document_id: response.document_id,
        },
    )
    .await
    .expect("archive document");
    let archived = update_radcite_document(
        &state,
        UpdateRadciteDocumentRequest {
            project_id: None,
            document_id: response.document_id,
            display_name: "Archived".to_string(),
            doc_number: None,
            doc_variant: DocumentVariant::Other,
            exclude_from_references: false,
        },
    )
    .await
    .expect_err("reject archived document update");
    assert!(matches!(
        archived,
        RadciteDocumentError::ArchivedDocument(document_id) if document_id == response.document_id
    ));
}

#[tokio::test]
async fn analysed_docx_reviews_reuse_the_local_radcite_project() {
    let state = desktop_state_with_migrated_pool().await;
    let first_path = write_minimal_docx("desktop-first-local-project.docx");
    let second_path = write_minimal_docx("desktop-second-local-project.docx");

    let first = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: first_path.to_string_lossy().into_owned(),
            original_filename: Some("first-local-project.docx".to_string()),
        },
    )
    .await
    .expect("analyse first docx for review");

    let second = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: second_path.to_string_lossy().into_owned(),
            original_filename: Some("second-local-project.docx".to_string()),
        },
    )
    .await
    .expect("analyse second docx for review");

    assert_eq!(first.project_id, second.project_id);
    assert_eq!(first.project_title, "RADcite Functional Testing");
    assert_eq!(second.project_title, "RADcite Functional Testing");

    let saved_reviews = list_saved_radcite_reviews(&state, ListSavedReviewsRequest::default())
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
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("reference-project.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    let added = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
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

    let references = list_course_references(&state, ListCourseReferencesRequest::default())
        .await
        .expect("list course references");

    assert_eq!(references.len(), 1);
    assert_eq!(references[0], added);

    let added_again = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: " smith, j. (2020).   worked examples in practice. learning press. "
                .to_string(),
            notes: Some("Duplicate note should not overwrite".to_string()),
        },
    )
    .await
    .expect("reuse duplicate course reference");

    assert_eq!(added_again, added);

    let references_after_duplicate =
        list_course_references(&state, ListCourseReferencesRequest::default())
            .await
            .expect("list course references after duplicate");

    assert_eq!(references_after_duplicate, vec![added]);
}

#[tokio::test]
async fn course_references_can_be_assigned_and_moved_between_modules() {
    let state = desktop_state_with_migrated_pool().await;
    let first_module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: Some("1".to_string()),
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add first module");
    let second_module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 2".to_string(),
            code: Some("2".to_string()),
            order_index: Some(2),
            description: None,
        },
    )
    .await
    .expect("add second module");

    let added = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2024). Module reference. Learning Press.".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add module reference");

    let added = assign_course_reference_module(
        &state,
        AssignCourseReferenceModuleRequest {
            reference_id: added.id,
            module_id: Some(first_module.id),
        },
    )
    .await
    .expect("assign course reference to first module");

    let added_json = serde_json::to_value(&added).expect("serialise assigned reference");
    assert_eq!(added_json["module_id"], first_module.id.0.to_string());

    let moved = assign_course_reference_module(
        &state,
        AssignCourseReferenceModuleRequest {
            reference_id: added.id,
            module_id: Some(second_module.id),
        },
    )
    .await
    .expect("move course reference");

    let moved_json = serde_json::to_value(&moved).expect("serialise moved reference");
    assert_eq!(moved_json["module_id"], second_module.id.0.to_string());
}

#[tokio::test]
async fn course_reference_export_can_use_uc_library_links() {
    let state = desktop_state_with_migrated_pool().await;
    let added = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add course reference");

    update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: added.id,
            apa_citation: added.apa_citation.clone().unwrap_or_default(),
            notes: None,
            citation_text: None,
            url: Some("https://example.org/article?id=42".to_string()),
        },
    )
    .await
    .expect("add reference source URL");

    let export = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: None,
            for_ako_learn: false,
            allow_incomplete: true,
            use_library_links: true,
        },
    )
    .await
    .expect("export course references with UC links");

    assert!(export.html.contains(
        r#"<a href="https://go.openathens.net/redirector/canterbury.ac.nz?url=https://example.org/article?id=42" target="_blank" rel="noopener noreferrer">https://example.org/article?id=42</a>"#
    ));
}

#[tokio::test]
async fn course_references_can_be_updated_and_archived() {
    let state = desktop_state_with_migrated_pool().await;

    let added = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: Some("Core course reference".to_string()),
        },
    )
    .await
    .expect("add course reference");

    let updated = update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: added.id,
            apa_citation: " Taylor, J. (2025). Updated reference. ".to_string(),
            notes: Some(" Updated note ".to_string()),
            citation_text: None,
            url: None,
        },
    )
    .await
    .expect("update course reference");

    assert_eq!(updated.id, added.id);
    assert_eq!(updated.project_id, added.project_id);
    assert_eq!(
        updated.apa_citation.as_deref(),
        Some("Taylor, J. (2025). Updated reference.")
    );
    assert_eq!(updated.notes.as_deref(), Some("Updated note"));
    assert_eq!(updated.reference_type, "reference");

    let references = list_course_references(&state, ListCourseReferencesRequest::default())
        .await
        .expect("list course references");

    assert_eq!(references, vec![updated.clone()]);

    let archived = archive_course_reference(
        &state,
        ArchiveCourseReferenceRequest {
            reference_id: added.id,
        },
    )
    .await
    .expect("archive course reference");

    assert_eq!(archived, updated);
    assert!(
        list_course_references(&state, ListCourseReferencesRequest::default())
            .await
            .expect("list course references after archive")
            .is_empty()
    );
}

#[tokio::test]
async fn course_reference_updates_can_apply_lookup_metadata_without_erasing_it() {
    let state = desktop_state_with_migrated_pool().await;

    let added = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples. Learning Press.".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add course reference");

    let enriched = update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: added.id,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: Some("Imported from Crossref search. DOI: 10.1234/example".to_string()),
            citation_text: Some("Smith, J. (2020). Worked examples in practice.".to_string()),
            url: Some("https://doi.org/10.1234/example".to_string()),
        },
    )
    .await
    .expect("apply lookup metadata");

    assert_eq!(
        enriched.citation_text.as_deref(),
        Some("Smith, J. (2020). Worked examples in practice.")
    );
    assert_eq!(
        enriched.url.as_deref(),
        Some("https://doi.org/10.1234/example")
    );

    let manually_edited = update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: added.id,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: Some("Reviewed by the course team".to_string()),
            citation_text: None,
            url: None,
        },
    )
    .await
    .expect("manually edit reference");

    assert_eq!(
        manually_edited.citation_text.as_deref(),
        Some("Smith, J. (2020). Worked examples in practice.")
    );
    assert_eq!(
        manually_edited.url.as_deref(),
        Some("https://doi.org/10.1234/example")
    );
    assert!(manually_edited.validation_report.is_none());

    let insecure_url = update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: added.id,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: None,
            citation_text: None,
            url: Some("http://doi.org/10.1234/example".to_string()),
        },
    )
    .await
    .expect("normalise insecure URL");

    assert_eq!(
        insecure_url.url.as_deref(),
        Some("https://doi.org/10.1234/example")
    );
}

#[tokio::test]
async fn course_references_get_apa_validation_status() {
    let state = desktop_state_with_migrated_pool().await;

    let valid = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2024). Example reference. Journal of Testing, 12(3), 1-10."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add valid reference");

    assert_eq!(valid.validation_status, "valid");
    assert!(valid.validation_report.is_none());

    let needs_fix = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith (2024)".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add incomplete reference");

    assert_eq!(needs_fix.validation_status, "needs_fix");
    let validation_report = needs_fix
        .validation_report
        .as_deref()
        .expect("validation report");
    assert!(validation_report.contains("Author names should follow Lastname, Initials."));
    assert!(validation_report.contains("Title segment missing after the year."));

    let fixed = update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: needs_fix.id,
            apa_citation: "Smith, J. (2024). Fixed reference. Journal of Testing.".to_string(),
            notes: None,
            citation_text: None,
            url: None,
        },
    )
    .await
    .expect("fix incomplete reference");

    assert_eq!(fixed.validation_status, "valid");
    assert!(fixed.validation_report.is_none());
}

#[tokio::test]
async fn course_reference_update_commands_validate_input() {
    let state = desktop_state_with_migrated_pool().await;
    let missing_reference_id = ReferenceEntryId::new();

    let empty_reference = update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: missing_reference_id,
            apa_citation: " ".to_string(),
            notes: None,
            citation_text: None,
            url: None,
        },
    )
    .await
    .expect_err("reject empty reference text");
    assert!(matches!(
        empty_reference,
        CourseReferenceError::EmptyReferenceText
    ));

    let missing_reference = update_course_reference(
        &state,
        UpdateCourseReferenceRequest {
            reference_id: missing_reference_id,
            apa_citation: "Smith, J. (2024). Missing reference.".to_string(),
            notes: None,
            citation_text: None,
            url: None,
        },
    )
    .await
    .expect_err("reject missing reference update");
    assert!(matches!(
        missing_reference,
        CourseReferenceError::MissingReference(reference_id) if reference_id == missing_reference_id
    ));

    let missing_archive = archive_course_reference(
        &state,
        ArchiveCourseReferenceRequest {
            reference_id: missing_reference_id,
        },
    )
    .await
    .expect_err("reject missing reference archive");
    assert!(matches!(
        missing_archive,
        CourseReferenceError::MissingReference(reference_id) if reference_id == missing_reference_id
    ));
}

#[tokio::test]
async fn course_references_can_be_merged_without_losing_citation_links() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-merge-reference.docx");

    let analysis = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("merge-reference.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");
    let citation_id = analysis.paragraphs[0].citations[0].id;

    let primary = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples. Learning Press.".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add primary course reference");
    let duplicate = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples. Learning Press, second entry."
                .to_string(),
            notes: Some("Imported from the module document".to_string()),
        },
    )
    .await
    .expect("add duplicate course reference");

    link_citation_to_reference_for_review(
        &state,
        LinkCitationReferenceRequest {
            document_id: analysis.document_id,
            citation_id,
            reference_entry_id: duplicate.id,
        },
    )
    .await
    .expect("link citation to duplicate");

    let merged = merge_course_references(
        &state,
        MergeCourseReferencesRequest {
            primary_reference_id: primary.id,
            merge_reference_ids: vec![duplicate.id],
        },
    )
    .await
    .expect("merge course references");

    assert_eq!(merged.id, primary.id);
    assert_eq!(
        merged.notes.as_deref(),
        Some("Imported from the module document")
    );

    let loaded = load_saved_radcite_review(&state, analysis.document_id)
        .await
        .expect("load saved review");
    assert_eq!(
        loaded.paragraphs[0].citations[0].reference_entry_id,
        Some(primary.id)
    );

    let references = list_course_references(&state, ListCourseReferencesRequest::default())
        .await
        .expect("list merged course references");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].id, primary.id);
}

#[tokio::test]
async fn module_readings_commands_add_and_list_local_modules_and_readings() {
    let state = desktop_state_with_migrated_pool().await;

    let first_module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: " Module 1 ".to_string(),
            code: Some(" M1 ".to_string()),
            order_index: Some(1),
            description: Some(" Foundations ".to_string()),
        },
    )
    .await
    .expect("add first module");
    let second_module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 2".to_string(),
            code: None,
            order_index: Some(2),
            description: None,
        },
    )
    .await
    .expect("add second module");

    let modules = list_radcite_modules(&state, ListRadciteModulesRequest::default())
        .await
        .expect("list modules");

    assert_eq!(modules, vec![first_module.clone(), second_module.clone()]);
    assert_eq!(first_module.title, "Module 1");
    assert_eq!(first_module.code.as_deref(), Some("M1"));
    assert_eq!(first_module.description.as_deref(), Some("Foundations"));

    let reading = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: first_module.id,
            reading_category: " optional ".to_string(),
            lesson_code: Some(" 1.2 ".to_string()),
            apa_citation: Some(" Smith, J. (2024). Optional reading. ".to_string()),
            citation_text: None,
            doi: Some(" 10.1234/manual.doi ".to_string()),
            url: Some(" https://example.com/reading ".to_string()),
            notes: Some(" Manual entry ".to_string()),
            reading_notes: Some(" Skim before class ".to_string()),
            estimated_reading_time: Some(" 15 minutes ".to_string()),
        },
    )
    .await
    .expect("add reading");
    add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: second_module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some("Jones, A. (2024). Other module reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add other module reading");

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: first_module.id,
        },
    )
    .await
    .expect("list module readings");

    assert_eq!(readings, vec![reading.clone()]);
    assert_eq!(reading.module_id, first_module.id);
    assert_eq!(reading.project_id, first_module.project_id);
    assert_eq!(reading.reading_category, "optional");
    assert_eq!(reading.lesson_code.as_deref(), Some("1.2"));
    assert_eq!(
        reading.apa_citation.as_deref(),
        Some("Smith, J. (2024). Optional reading.")
    );
    assert_eq!(reading.doi.as_deref(), Some("10.1234/manual.doi"));
    assert_eq!(reading.url.as_deref(), Some("https://example.com/reading"));
    assert_eq!(reading.validation_status, "valid");
    assert!(reading.validation_report.is_none());
    assert_eq!(reading.notes.as_deref(), Some("Manual entry"));
    assert_eq!(reading.reading_notes.as_deref(), Some("Skim before class"));
    assert_eq!(
        reading.estimated_reading_time.as_deref(),
        Some("15 minutes")
    );

    let required_reading = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: first_module.id,
            reading_category: " required ".to_string(),
            lesson_code: None,
            apa_citation: Some("Taylor, J. (2024). Required reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add required alias reading");

    assert_eq!(required_reading.reading_category, "compulsory");
}

#[tokio::test]
async fn manual_required_reading_upgrades_existing_optional_duplicate() {
    let state = desktop_state_with_migrated_pool().await;
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");
    let citation = "Rice, R. (2024). Learning through practice.";

    let optional = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "optional".to_string(),
            lesson_code: None,
            apa_citation: Some(citation.to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add optional reading");

    let required = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "required".to_string(),
            lesson_code: None,
            apa_citation: Some(citation.to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("upgrade optional reading");

    assert_eq!(required.id, optional.id);
    assert_eq!(required.reading_category, "compulsory");

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings");

    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].reading_category, "compulsory");
}

#[tokio::test]
async fn module_readings_are_listed_and_exported_in_natural_lesson_order() {
    let state = desktop_state_with_migrated_pool().await;
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 2".to_string(),
            code: Some("M2".to_string()),
            order_index: Some(2),
            description: None,
        },
    )
    .await
    .expect("add module");

    add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: Some("2.10".to_string()),
            apa_citation: Some("Taylor, R. (2024). Later lesson.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add later lesson reading");
    add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: Some("2.3".to_string()),
            apa_citation: Some("Smith, J. (2024). Earlier lesson.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add earlier lesson reading");
    add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some("Jones, A. (2024). Whole module reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add whole module reading");

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings");

    assert_eq!(
        readings
            .iter()
            .map(|reading| reading.lesson_code.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("2.3"), Some("2.10"), None]
    );

    let export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: module.id,
            for_ako_learn: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export module readings");

    let earlier_index = export.html.find("<strong>2.3&nbsp;</strong>").unwrap();
    let later_index = export.html.find("<strong>2.10&nbsp;</strong>").unwrap();
    let whole_module_index = export.html.find("Whole module reading").unwrap();

    assert!(earlier_index < later_index);
    assert!(later_index < whole_module_index);
}

#[tokio::test]
async fn module_readings_commands_validate_input() {
    let state = desktop_state_with_migrated_pool().await;

    let empty_title = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "  ".to_string(),
            code: None,
            order_index: None,
            description: None,
        },
    )
    .await
    .expect_err("reject empty module title");

    assert!(matches!(empty_title, RadciteModuleError::EmptyTitle));

    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");

    let empty_reading = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some(" ".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect_err("reject empty reading text");

    assert!(matches!(
        empty_reading,
        ModuleReadingError::EmptyReadingText
    ));

    let invalid_category = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "recommended".to_string(),
            lesson_code: None,
            apa_citation: Some("Smith, J. (2024). Reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect_err("reject invalid category");

    assert!(matches!(
        invalid_category,
        ModuleReadingError::InvalidCategory(value) if value == "recommended"
    ));

    let imported_required = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: " required ".to_string(),
                lesson_code: None,
                apa_citation: Some("Taylor, J. (2024). Imported required reading.".to_string()),
                citation_text: None,
                doi: None,
                url: None,
                notes: None,
                reading_notes: None,
                estimated_reading_time: None,
            }],
        },
    )
    .await
    .expect("import required alias reading");

    assert_eq!(imported_required[0].reading_category, "compulsory");

    let missing_module = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: ModuleId::new(),
        },
    )
    .await
    .expect_err("reject missing module");

    assert!(matches!(
        missing_module,
        ModuleReadingError::MissingModule(_)
    ));
}

#[tokio::test]
async fn module_readings_import_preview_extracts_candidates_without_persisting() {
    let state = desktop_state_with_migrated_pool().await;
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");
    let path = write_readings_import_docx("desktop-readings-import-preview.docx");

    let candidates = preview_module_readings_import(
        &state,
        PreviewModuleReadingsImportRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("module-readings.docx".to_string()),
        },
    )
    .await
    .expect("preview readings import");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].module_order, Some(1));
    assert_eq!(candidates[0].module_title.as_deref(), Some("Module 1"));
    assert_eq!(candidates[0].reading_category, "compulsory");
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("1.2"));
    assert_eq!(
        candidates[0].apa_citation,
        "Smith, J. (2024). Worked examples. https://doi.org/10.1234/worked"
    );
    assert_eq!(
        candidates[0].citation_text.as_deref(),
        Some("1.2 Smith, J. (2024). Worked examples. https://doi.org/10.1234/worked")
    );
    assert_eq!(
        candidates[0].url.as_deref(),
        Some("https://doi.org/10.1234/worked")
    );
    assert_eq!(candidates[0].doi.as_deref(), Some("10.1234/worked"));
    assert_eq!(candidates[1].reading_category, "optional");
    assert_eq!(
        candidates[1].apa_citation,
        "Taylor, R. (2023). Optional primer."
    );

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings");

    assert!(readings.is_empty());
}

#[tokio::test]
async fn document_readings_import_creates_modules_and_is_idempotent() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_readings_import_docx("desktop-document-readings-import.docx");
    let request = ImportDocumentReadingsRequest {
        project_id: None,
        path: path.to_string_lossy().into_owned(),
        source_file_type: radsuite_core::DocumentFileType::Docx,
    };

    let first_import = import_document_readings(&state, request.clone())
        .await
        .expect("import document readings");

    assert_eq!(first_import.candidate_count, 2);
    assert_eq!(first_import.saved_count, 2);
    assert_eq!(first_import.created_module_count, 1);
    assert_eq!(first_import.unassigned_count, 0);
    assert_eq!(first_import.failed_file_count, 0);

    let modules = list_radcite_modules(&state, ListRadciteModulesRequest { project_id: None })
        .await
        .expect("list imported modules");
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].title, "Module 1");
    assert_eq!(modules[0].order_index, Some(1));

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: modules[0].id,
        },
    )
    .await
    .expect("list imported readings");
    assert_eq!(readings.len(), 2);
    assert_eq!(readings[0].reading_category, "compulsory");
    assert_eq!(readings[1].reading_category, "optional");

    let second_import = import_document_readings(&state, request)
        .await
        .expect("re-import document readings");
    assert_eq!(second_import.candidate_count, 2);
    assert_eq!(second_import.saved_count, 2);
    assert_eq!(second_import.created_module_count, 0);
    assert_eq!(second_import.unassigned_count, 0);

    let modules_after_reimport =
        list_radcite_modules(&state, ListRadciteModulesRequest { project_id: None })
            .await
            .expect("list modules after re-import");
    assert_eq!(modules_after_reimport.len(), 1);

    let readings_after_reimport = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: modules_after_reimport[0].id,
        },
    )
    .await
    .expect("list readings after re-import");
    assert_eq!(readings_after_reimport.len(), 2);
}

#[tokio::test]
async fn document_pdf_readings_import_infers_module_from_scorm_filename() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_readings_import_pdf(
        "desktop-module-6-microlearning-1-import.pdf",
        &[
            "Required readings",
            "Goldberg, M. H., & Gustafson, A. (2023). Strategic campaigns. International Journal of Strategic Communication, 17(1), 1-20.",
        ],
    );

    let result = import_document_readings(
        &state,
        ImportDocumentReadingsRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            source_file_type: radsuite_core::DocumentFileType::Pdf,
        },
    )
    .await
    .expect("import PDF readings");

    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.saved_count, 1);
    assert_eq!(result.created_module_count, 1);
    assert_eq!(result.unassigned_count, 0);
    assert_eq!(result.failed_file_count, 0);

    let modules = list_radcite_modules(&state, ListRadciteModulesRequest { project_id: None })
        .await
        .expect("list PDF import module");
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].title, "Module 6");
    assert_eq!(modules[0].order_index, Some(6));
}

#[tokio::test]
async fn module_readings_csv_import_preview_extracts_candidates_for_selected_module_save() {
    let state = desktop_state_with_migrated_pool().await;
    let project = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("CRJU201".to_string()),
            title: "Criminological Theory".to_string(),
        },
    )
    .await
    .expect("create project");
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: Some(project.id),
            title: "Week 2 - Positivism".to_string(),
            code: Some("W02".to_string()),
            order_index: Some(2),
            description: None,
        },
    )
    .await
    .expect("add module");
    let path = write_readings_import_csv("desktop-readings-import-preview.csv");

    let candidates = preview_module_readings_csv_import(
        &state,
        PreviewModuleReadingsCsvImportRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("course_readings.csv".to_string()),
        },
    )
    .await
    .expect("preview csv readings import");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].module_order, Some(2));
    assert_eq!(
        candidates[0].module_title.as_deref(),
        Some("Week 2 - Positivism")
    );
    assert_eq!(candidates[0].reading_category, "compulsory");
    assert_eq!(candidates[0].lesson_code.as_deref(), Some("02"));
    assert_eq!(
        candidates[0].apa_citation,
        "\"Biosocial Theories of Crime\" in Miller, M., Schreck, C. & Tewksbury, R. (2015). Criminological Theory: A Brief Introduction (4th ed.). Pearson."
    );
    assert_eq!(candidates[0].citation_text, None);

    let saved = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: candidates[0].reading_category.clone(),
                lesson_code: candidates[0].lesson_code.clone(),
                apa_citation: Some(candidates[0].apa_citation.clone()),
                citation_text: candidates[0].citation_text.clone(),
                doi: candidates[0].doi.clone(),
                url: candidates[0].url.clone(),
                notes: Some("Imported from CSV".to_string()),
                reading_notes: None,
                estimated_reading_time: None,
            }],
        },
    )
    .await
    .expect("save csv reading import");

    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].project_id, project.id);
    assert_eq!(saved[0].module_id, module.id);
    assert_eq!(saved[0].lesson_code.as_deref(), Some("02"));
    assert_eq!(
        saved[0].apa_citation.as_deref(),
        Some(
            "\"Biosocial Theories of Crime\" in Miller, M., Schreck, C. & Tewksbury, R. (2015). Criminological Theory: A Brief Introduction (4th ed.). Pearson."
        )
    );

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings after csv import");

    assert_eq!(readings, saved);
}

#[tokio::test]
async fn module_readings_pdf_import_preview_extracts_candidates_from_multiple_pdfs() {
    let state = desktop_state_with_migrated_pool().await;
    let first_path = write_readings_import_pdf(
        "desktop-module-6-microlearning-1.pdf",
        &[
            "Required readings",
            "Goldberg, M. H., & Gustafson, A. (2023). Strategic campaigns. International Journal of Strategic Communication, 17(1), 1-20.",
        ],
    );
    let second_path = write_readings_import_pdf(
        "desktop-module-6-microlearning-2.pdf",
        &[
            "Optional readings",
            "Taylor, R. (2023). Optional primer. Teaching Press.",
        ],
    );

    let preview = preview_module_readings_pdf_import(
        &state,
        PreviewModuleReadingsPdfImportRequest {
            paths: vec![
                first_path.to_string_lossy().into_owned(),
                second_path.to_string_lossy().into_owned(),
            ],
        },
    )
    .await
    .expect("preview pdf readings import");

    assert!(preview.failures.is_empty());
    let candidates = preview.candidates;
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].source_filename.as_deref(),
        Some("radsuite-desktop-module-6-microlearning-1.pdf")
    );
    assert_eq!(candidates[0].module_order, Some(6));
    assert_eq!(candidates[0].module_title.as_deref(), Some("module 6"));
    assert_eq!(
        candidates[0].lesson_code.as_deref(),
        Some("microlearning 1")
    );
    assert_eq!(candidates[0].reading_category, "compulsory");
    assert_eq!(candidates[1].reading_category, "optional");
}

#[tokio::test]
async fn module_readings_pdf_import_preview_reports_unreadable_files_without_losing_candidates() {
    let state = desktop_state_with_migrated_pool().await;
    let good_path = write_readings_import_pdf(
        "desktop-module-12-lesson-1.pdf",
        &[
            "Required readings",
            "Turner, A. (2024). Desktop partial PDF imports. Example Press.",
        ],
    );
    let missing_path = good_path.with_file_name("missing-module-12-lesson-2.pdf");

    let preview = preview_module_readings_pdf_import(
        &state,
        PreviewModuleReadingsPdfImportRequest {
            paths: vec![
                good_path.to_string_lossy().into_owned(),
                missing_path.to_string_lossy().into_owned(),
            ],
        },
    )
    .await
    .expect("preview pdf readings import with partial failure");

    assert_eq!(preview.candidates.len(), 1);
    assert_eq!(preview.candidates[0].module_order, Some(12));
    assert_eq!(preview.failures.len(), 1);
    assert_eq!(
        preview.failures[0].path,
        missing_path.to_string_lossy().into_owned()
    );
    assert!(
        preview.failures[0]
            .message
            .contains("failed to read PDF file")
    );
}

#[tokio::test]
async fn module_readings_pdf_import_rejects_empty_selection() {
    let state = desktop_state_with_migrated_pool().await;

    let error = preview_module_readings_pdf_import(
        &state,
        PreviewModuleReadingsPdfImportRequest {
            paths: vec!["  ".to_string()],
        },
    )
    .await
    .expect_err("empty PDF selection");

    assert!(matches!(error, ModuleReadingImportError::EmptyPath));
}

#[tokio::test]
async fn module_readings_import_save_persists_selected_candidates() {
    let state = desktop_state_with_migrated_pool().await;
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");

    let saved = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: " optional ".to_string(),
                lesson_code: Some(" 1.2 ".to_string()),
                apa_citation: Some(" Smith, J. (2024). Worked examples. ".to_string()),
                citation_text: None,
                doi: Some(" 10.1234/worked ".to_string()),
                url: Some(" https://example.com/worked ".to_string()),
                notes: Some(" Imported from DOCX ".to_string()),
                reading_notes: Some(" Read before class ".to_string()),
                estimated_reading_time: Some(" 20 minutes ".to_string()),
            }],
        },
    )
    .await
    .expect("save readings import");

    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].module_id, module.id);
    assert_eq!(saved[0].project_id, module.project_id);
    assert_eq!(saved[0].reading_category, "optional");
    assert_eq!(saved[0].lesson_code.as_deref(), Some("1.2"));
    assert_eq!(
        saved[0].apa_citation.as_deref(),
        Some("Smith, J. (2024). Worked examples.")
    );
    assert_eq!(saved[0].url.as_deref(), Some("https://example.com/worked"));
    assert_eq!(saved[0].doi.as_deref(), Some("10.1234/worked"));
    assert_eq!(saved[0].notes.as_deref(), Some("Imported from DOCX"));
    assert_eq!(saved[0].reading_notes.as_deref(), Some("Read before class"));
    assert_eq!(
        saved[0].estimated_reading_time.as_deref(),
        Some("20 minutes")
    );

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings");

    assert_eq!(readings, saved);
}

#[tokio::test]
async fn module_readings_import_save_reuses_existing_duplicate_candidates() {
    let state = desktop_state_with_migrated_pool().await;
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");

    let first_saved = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: "compulsory".to_string(),
                lesson_code: Some("1.2".to_string()),
                apa_citation: Some("Smith, J. (2024). Worked examples.".to_string()),
                citation_text: None,
                doi: None,
                url: Some("https://example.com/worked".to_string()),
                notes: Some("Imported from DOCX".to_string()),
                reading_notes: Some("Read before class".to_string()),
                estimated_reading_time: Some("20 minutes".to_string()),
            }],
        },
    )
    .await
    .expect("save first readings import");

    let saved_again = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: " compulsory ".to_string(),
                lesson_code: Some(" 9.9 ".to_string()),
                apa_citation: Some("  smith, j. (2024).   worked examples. ".to_string()),
                citation_text: None,
                doi: None,
                url: Some("https://example.com/duplicate".to_string()),
                notes: Some("Imported again".to_string()),
                reading_notes: Some("Duplicate note".to_string()),
                estimated_reading_time: Some("30 minutes".to_string()),
            }],
        },
    )
    .await
    .expect("save duplicate readings import");

    assert_eq!(saved_again, first_saved);

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings");

    assert_eq!(readings, first_saved);
}

#[tokio::test]
async fn module_readings_import_required_upgrades_existing_optional_reading() {
    let state = desktop_state_with_migrated_pool().await;
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");
    let citation = "Rice, R. (2024). Learning through practice.";

    let optional = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: "optional".to_string(),
                lesson_code: None,
                apa_citation: Some(citation.to_string()),
                citation_text: None,
                doi: None,
                url: None,
                notes: None,
                reading_notes: None,
                estimated_reading_time: None,
            }],
        },
    )
    .await
    .expect("save optional reading");

    let required = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: "required".to_string(),
                lesson_code: None,
                apa_citation: Some(citation.to_string()),
                citation_text: None,
                doi: None,
                url: None,
                notes: None,
                reading_notes: None,
                estimated_reading_time: None,
            }],
        },
    )
    .await
    .expect("upgrade optional reading");

    assert_eq!(required[0].id, optional[0].id);
    assert_eq!(required[0].reading_category, "compulsory");

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings");

    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].reading_category, "compulsory");
}

#[tokio::test]
async fn module_readings_import_deduplicates_by_doi_and_upgrades_required_category() {
    let state = desktop_state_with_migrated_pool().await;
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");

    let optional = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: "optional".to_string(),
                lesson_code: None,
                apa_citation: Some(
                    "Rice, R. (2024). Communication and culture. Academic Press.".to_string(),
                ),
                citation_text: None,
                doi: Some("10.1234/rice".to_string()),
                url: Some("https://doi.org/10.1234/rice".to_string()),
                notes: None,
                reading_notes: None,
                estimated_reading_time: None,
            }],
        },
    )
    .await
    .expect("save optional reading");

    let required = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: module.id,
                reading_category: "required".to_string(),
                lesson_code: None,
                apa_citation: Some(
                    "Rice, R. (2024). Communication and culture: A revised edition. Academic Press."
                        .to_string(),
                ),
                citation_text: None,
                doi: Some("10.1234/rice".to_string()),
                url: Some("https://doi.org/10.1234/rice".to_string()),
                notes: None,
                reading_notes: None,
                estimated_reading_time: None,
            }],
        },
    )
    .await
    .expect("save required reading");

    assert_eq!(required[0].id, optional[0].id);
    assert_eq!(required[0].reading_category, "compulsory");

    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list module readings");

    assert_eq!(readings.len(), 1);
}

#[tokio::test]
async fn module_readings_import_save_validates_missing_module() {
    let state = desktop_state_with_migrated_pool().await;
    let missing_module_id = ModuleId::new();

    let error = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: vec![SaveModuleReadingsImportCandidate {
                module_id: missing_module_id,
                reading_category: "compulsory".to_string(),
                lesson_code: None,
                apa_citation: Some("Smith, J. (2024). Worked examples.".to_string()),
                citation_text: None,
                doi: None,
                url: None,
                notes: None,
                reading_notes: None,
                estimated_reading_time: None,
            }],
        },
    )
    .await
    .expect_err("reject missing module");

    assert!(matches!(
        error,
        ModuleReadingImportError::MissingModule(module_id) if module_id == missing_module_id
    ));
}

#[tokio::test]
async fn module_readings_commands_update_and_archive_modules_and_readings() {
    let state = desktop_state_with_migrated_pool().await;

    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: Some("M1".to_string()),
            order_index: Some(1),
            description: Some("Foundations".to_string()),
        },
    )
    .await
    .expect("add module");

    let updated_module = update_radcite_module(
        &state,
        UpdateRadciteModuleRequest {
            module_id: module.id,
            title: " Module 1 updated ".to_string(),
            code: Some(" MOD1 ".to_string()),
            order_index: Some(3),
            description: Some(" Updated description ".to_string()),
        },
    )
    .await
    .expect("update module");

    assert_eq!(updated_module.id, module.id);
    assert_eq!(updated_module.title, "Module 1 updated");
    assert_eq!(updated_module.code.as_deref(), Some("MOD1"));
    assert_eq!(updated_module.order_index, Some(3));
    assert_eq!(
        updated_module.description.as_deref(),
        Some("Updated description")
    );
    assert_eq!(
        list_radcite_modules(&state, ListRadciteModulesRequest::default())
            .await
            .expect("list modules"),
        vec![updated_module.clone()]
    );

    let reading = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: Some("1.1".to_string()),
            apa_citation: Some("Smith, J. (2024). Module reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add reading");

    let updated_reading = update_module_reading(
        &state,
        UpdateModuleReadingRequest {
            reading_id: reading.id,
            reading_category: " optional ".to_string(),
            lesson_code: Some(" 1.2 ".to_string()),
            apa_citation: Some(" Taylor, J. (2025). Updated reading. ".to_string()),
            citation_text: None,
            doi: Some(" 10.1234/updated.doi ".to_string()),
            url: Some(" http://example.com/updated ".to_string()),
            notes: Some(" Staff note ".to_string()),
            reading_notes: Some(" Student note ".to_string()),
            estimated_reading_time: Some(" 20 minutes ".to_string()),
        },
    )
    .await
    .expect("update reading");

    assert_eq!(updated_reading.id, reading.id);
    assert_eq!(updated_reading.module_id, module.id);
    assert_eq!(updated_reading.reading_category, "optional");
    assert_eq!(updated_reading.lesson_code.as_deref(), Some("1.2"));
    assert_eq!(
        updated_reading.apa_citation.as_deref(),
        Some("Taylor, J. (2025). Updated reading.")
    );
    assert_eq!(updated_reading.doi.as_deref(), Some("10.1234/updated.doi"));
    assert_eq!(
        updated_reading.url.as_deref(),
        Some("https://example.com/updated")
    );
    assert_eq!(updated_reading.validation_status, "valid");
    assert!(updated_reading.validation_report.is_none());
    assert_eq!(updated_reading.notes.as_deref(), Some("Staff note"));
    assert_eq!(
        updated_reading.reading_notes.as_deref(),
        Some("Student note")
    );
    assert_eq!(
        updated_reading.estimated_reading_time.as_deref(),
        Some("20 minutes")
    );

    archive_module_reading(
        &state,
        ArchiveModuleReadingRequest {
            reading_id: reading.id,
        },
    )
    .await
    .expect("archive reading");
    let readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("list readings after archive");
    assert!(readings.is_empty());

    let child_reading = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some("Jones, A. (2024). Child reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add child reading");

    archive_radcite_module(
        &state,
        ArchiveRadciteModuleRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("archive module");

    assert!(
        list_radcite_modules(&state, ListRadciteModulesRequest::default())
            .await
            .expect("list modules")
            .is_empty()
    );
    let missing_module = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: child_reading.module_id,
        },
    )
    .await
    .expect_err("module should be archived");
    assert!(matches!(
        missing_module,
        ModuleReadingError::MissingModule(module_id) if module_id == module.id
    ));
}

#[tokio::test]
async fn module_readings_update_commands_validate_input() {
    let state = desktop_state_with_migrated_pool().await;

    let missing_module_id = ModuleId::new();
    let empty_title = update_radcite_module(
        &state,
        UpdateRadciteModuleRequest {
            module_id: missing_module_id,
            title: " ".to_string(),
            code: None,
            order_index: None,
            description: None,
        },
    )
    .await
    .expect_err("reject empty title");
    assert!(matches!(empty_title, RadciteModuleError::EmptyTitle));

    let missing_module = update_radcite_module(
        &state,
        UpdateRadciteModuleRequest {
            module_id: missing_module_id,
            title: "Missing".to_string(),
            code: None,
            order_index: None,
            description: None,
        },
    )
    .await
    .expect_err("reject missing module");
    assert!(matches!(
        missing_module,
        RadciteModuleError::MissingModule(module_id) if module_id == missing_module_id
    ));

    let missing_archive = archive_radcite_module(
        &state,
        ArchiveRadciteModuleRequest {
            module_id: missing_module_id,
        },
    )
    .await
    .expect_err("reject missing module archive");
    assert!(matches!(
        missing_archive,
        RadciteModuleError::MissingModule(module_id) if module_id == missing_module_id
    ));

    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Module 1".to_string(),
            code: None,
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");
    let reading = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some("Smith, J. (2024). Reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add reading");

    let empty_reading = update_module_reading(
        &state,
        UpdateModuleReadingRequest {
            reading_id: reading.id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some(" ".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect_err("reject empty reading text");
    assert!(matches!(
        empty_reading,
        ModuleReadingError::EmptyReadingText
    ));

    let invalid_category = update_module_reading(
        &state,
        UpdateModuleReadingRequest {
            reading_id: reading.id,
            reading_category: "recommended".to_string(),
            lesson_code: None,
            apa_citation: Some("Smith, J. (2024). Reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect_err("reject invalid category");
    assert!(matches!(
        invalid_category,
        ModuleReadingError::InvalidCategory(value) if value == "recommended"
    ));

    let updated_required = update_module_reading(
        &state,
        UpdateModuleReadingRequest {
            reading_id: reading.id,
            reading_category: " required ".to_string(),
            lesson_code: None,
            apa_citation: Some("Taylor, J. (2024). Updated required reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("update required alias reading");

    assert_eq!(updated_required.reading_category, "compulsory");

    let missing_reading_id = ReferenceEntryId::new();
    let missing_reading = update_module_reading(
        &state,
        UpdateModuleReadingRequest {
            reading_id: missing_reading_id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some("Smith, J. (2024). Reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect_err("reject missing reading");
    assert!(matches!(
        missing_reading,
        ModuleReadingError::MissingReading(reading_id) if reading_id == missing_reading_id
    ));

    let missing_archive = archive_module_reading(
        &state,
        ArchiveModuleReadingRequest {
            reading_id: missing_reading_id,
        },
    )
    .await
    .expect_err("reject missing reading archive");
    assert!(matches!(
        missing_archive,
        ModuleReadingError::MissingReading(reading_id) if reading_id == missing_reading_id
    ));
}

#[tokio::test]
async fn paragraph_citations_can_be_linked_to_course_references() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-linked-reference.docx");

    let analysis = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("linked-reference.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");
    let citation_id = analysis.paragraphs[0].citations[0].id;

    let reference = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add course reference");

    let linked = link_citation_to_reference_for_review(
        &state,
        LinkCitationReferenceRequest {
            document_id: analysis.document_id,
            citation_id,
            reference_entry_id: reference.id,
        },
    )
    .await
    .expect("link citation to reference");

    assert_eq!(
        linked.paragraphs[0].citations[0].reference_entry_id,
        Some(reference.id)
    );
    assert!(!linked.paragraphs[0].citations[0].verified);

    let loaded = load_saved_radcite_review(&state, analysis.document_id)
        .await
        .expect("load saved review");

    assert_eq!(
        loaded.paragraphs[0].citations[0].reference_entry_id,
        Some(reference.id)
    );
}

#[tokio::test]
async fn reference_suggestions_include_strong_course_reference_matches() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-reference-suggestions.docx");

    let reference = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add course reference");

    let analysis = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("reference-suggestions.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    let suggestions = &analysis.paragraphs[0].citations[0].reference_suggestions;

    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].reference_entry_id, reference.id);
    assert_eq!(suggestions[0].confidence, "strong");
    assert_eq!(suggestions[0].reason, "Author and year match");
    assert_eq!(
        suggestions[0].label,
        "Smith, J. (2020). Worked examples in practice. Learning Press."
    );
}

#[tokio::test]
async fn review_queue_summary_tracks_linked_suggested_and_unlinked_citations() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-review-queue.docx");

    let reference = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add course reference");

    let analysis = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("review-queue.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    assert_eq!(analysis.summary.citation_count, 1);
    assert_eq!(analysis.summary.linked_citation_count, 0);
    assert_eq!(analysis.summary.suggested_citation_count, 1);
    assert_eq!(analysis.summary.unlinked_citation_count, 1);

    let citation_id = analysis.paragraphs[0].citations[0].id;
    let linked = link_citation_to_reference_for_review(
        &state,
        LinkCitationReferenceRequest {
            document_id: analysis.document_id,
            citation_id,
            reference_entry_id: reference.id,
        },
    )
    .await
    .expect("link citation to reference");

    assert_eq!(linked.summary.citation_count, 1);
    assert_eq!(linked.summary.linked_citation_count, 1);
    assert_eq!(linked.summary.suggested_citation_count, 0);
    assert_eq!(linked.summary.unlinked_citation_count, 0);
}

#[tokio::test]
async fn reference_suggestions_are_empty_when_course_references_do_not_match() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-reference-suggestions-empty.docx");

    add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Jones, A. (2024). Assessment rubrics in practice. Teaching Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add course reference");

    let analysis = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("reference-suggestions-empty.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    assert!(
        analysis.paragraphs[0].citations[0]
            .reference_suggestions
            .is_empty()
    );
}

#[tokio::test]
async fn radcite_excluded_document_filtering() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-excluded-document.docx");
    let analysis = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("excluded-document.docx".to_string()),
        },
    )
    .await
    .expect("analyse document");

    let reference_repo = SqliteReferenceEntryRepository::new(state.database_pool.clone());
    let mut linked_reference =
        ReferenceEntry::new(analysis.project_id, ReferenceEntryType::Reference);
    linked_reference.document_id = Some(analysis.document_id);
    linked_reference.apa_citation = Some(
        "Smith, J. (2020). Linked reference from an excluded document. Learning Press.".to_string(),
    );
    reference_repo
        .insert_reference_entry(&linked_reference)
        .await
        .expect("insert linked course reference");

    let mut unlinked_reference =
        ReferenceEntry::new(analysis.project_id, ReferenceEntryType::Reference);
    unlinked_reference.apa_citation =
        Some("Jones, A. (2024). Unlinked course reference. Teaching Press.".to_string());
    reference_repo
        .insert_reference_entry(&unlinked_reference)
        .await
        .expect("insert unlinked course reference");

    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Excluded document module".to_string(),
            code: Some("M1".to_string()),
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");
    let mut linked_reading = ReferenceEntry::new(analysis.project_id, ReferenceEntryType::Reading);
    linked_reading.module_id = Some(module.id);
    linked_reading.document_id = Some(analysis.document_id);
    linked_reading.reading_category = Some(ReadingCategory::Compulsory);
    linked_reading.apa_citation = Some(
        "Smith, J. (2020). Linked reading from an excluded document. Learning Press.".to_string(),
    );
    reference_repo
        .insert_reference_entry(&linked_reading)
        .await
        .expect("insert linked module reading");

    let mut unlinked_reading =
        ReferenceEntry::new(analysis.project_id, ReferenceEntryType::Reading);
    unlinked_reading.module_id = Some(module.id);
    unlinked_reading.reading_category = Some(ReadingCategory::Optional);
    unlinked_reading.apa_citation =
        Some("Jones, A. (2024). Unlinked module reading. Teaching Press.".to_string());
    reference_repo
        .insert_reference_entry(&unlinked_reading)
        .await
        .expect("insert unlinked module reading");

    update_radcite_document(
        &state,
        UpdateRadciteDocumentRequest {
            project_id: None,
            document_id: analysis.document_id,
            display_name: String::new(),
            doc_number: None,
            doc_variant: DocumentVariant::Content,
            exclude_from_references: true,
        },
    )
    .await
    .expect("exclude document");

    let references = list_course_references(&state, ListCourseReferencesRequest::default())
        .await
        .expect("list filtered course references");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].id, unlinked_reference.id);

    let loaded = load_saved_radcite_review(&state, analysis.document_id)
        .await
        .expect("load filtered review");
    assert!(!loaded.paragraphs.iter().any(|paragraph| {
        paragraph
            .citations
            .iter()
            .flat_map(|citation| citation.reference_suggestions.iter())
            .any(|suggestion| suggestion.reference_entry_id == linked_reference.id)
    }));

    let course_export = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: None,
            for_ako_learn: false,
            allow_incomplete: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export filtered course references");
    assert_eq!(course_export.reference_count, 1);
    assert!(
        !course_export
            .html
            .contains("Linked reference from an excluded document")
    );
    assert!(course_export.html.contains("Unlinked course reference"));

    let module_export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: module.id,
            for_ako_learn: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export filtered module readings");
    assert_eq!(module_export.reading_count, 1);
    assert!(
        !module_export
            .html
            .contains("Linked reading from an excluded document")
    );
    assert!(module_export.html.contains("Unlinked module reading"));
}

#[tokio::test]
async fn course_references_can_be_exported_as_html() {
    let state = desktop_state_with_migrated_pool().await;

    add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples & practice. Learning Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add first reference");
    add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Jones, A. (2024). Assessment rubrics <revised>. Teaching Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add second reference");

    let export = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: None,
            for_ako_learn: false,
            allow_incomplete: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export course references");

    assert_eq!(export.reference_count, 2);
    assert_eq!(export.content_type, "text/html; charset=utf-8");
    assert!(export.filename.ends_with("course-references.html"));
    assert!(export.html.contains(r#"{GENERICO:type="references"}"#));
    assert!(export.html.contains("Worked examples &amp; practice."));
    assert!(export.html.contains("Assessment rubrics &lt;revised&gt;."));
}

#[tokio::test]
async fn course_reference_export_blocks_apa_fixes_unless_overridden() {
    let state = desktop_state_with_migrated_pool().await;

    add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith (2024)".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add reference needing APA fixes");

    let error = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: None,
            for_ako_learn: false,
            allow_incomplete: false,
            use_library_links: false,
        },
    )
    .await
    .expect_err("block export while APA fixes are pending");
    assert!(error.to_string().contains("APA fixes"));

    let export = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: None,
            for_ako_learn: false,
            allow_incomplete: true,
            use_library_links: false,
        },
    )
    .await
    .expect("allow export with explicit override");

    assert_eq!(export.reference_count, 1);
    assert_eq!(export.apa_error_count, 1);
    assert_eq!(export.apa_warning_count, 0);
}

#[tokio::test]
async fn course_reference_export_can_omit_generico_tags() {
    let state = desktop_state_with_migrated_pool().await;

    add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press."
                .to_string(),
            notes: None,
        },
    )
    .await
    .expect("add course reference");

    let export = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: None,
            for_ako_learn: true,
            allow_incomplete: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export course references");

    assert_eq!(export.reference_count, 1);
    assert!(!export.html.contains("GENERICO"));
    assert!(export.html.contains("Smith, J. (2020)."));
}

#[tokio::test]
async fn module_readings_can_be_exported_as_html() {
    let state = desktop_state_with_migrated_pool().await;

    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Engaging people in conversations about change".to_string(),
            code: Some("Module 1".to_string()),
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add module");

    add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: Some("1.2".to_string()),
            apa_citation: Some(
                "Gregory, A. (2022). Strategic public relations leadership & planning.".to_string(),
            ),
            citation_text: None,
            doi: None,
            url: Some("https://doi.org/10.4324/9781003185253".to_string()),
            notes: None,
            reading_notes: Some("Read Chapter 10 for macro/micro planning.".to_string()),
            estimated_reading_time: Some("50 minutes".to_string()),
        },
    )
    .await
    .expect("add compulsory reading");
    add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "optional".to_string(),
            lesson_code: Some("1.3".to_string()),
            apa_citation: Some(
                "Taylor, J. (2023). Optional ethics <primer>. Teaching Press.".to_string(),
            ),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add optional reading");

    let export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: module.id,
            for_ako_learn: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export module readings");

    assert_eq!(export.module_id, module.id);
    assert_eq!(export.reading_count, 2);
    assert_eq!(export.content_type, "text/html; charset=utf-8");
    assert!(export.filename.ends_with("module-readings.html"));
    assert!(export.html.contains(r#"{GENERICO:type="references"}"#));
    assert!(export.html.contains(r#"{GENERICO:type="references_end"}"#));
    assert!(
        export
            .html
            .contains("<p>The readings for this module are listed below.</p>")
    );
    assert!(!export.html.contains("change text"));
    assert!(export.html.contains("<h4>Required readings</h4>"));
    assert!(export.html.contains("Optional readings"));
    assert!(export.html.contains("<strong>1.2&nbsp;</strong>"));
    assert!(export.html.contains("leadership &amp; planning."));
    assert!(export.html.contains("Optional ethics &lt;primer&gt;."));
    assert!(export.html.contains(
        r#"<a href="https://doi.org/10.4324/9781003185253" target="_blank" rel="noopener noreferrer">https://doi.org/10.4324/9781003185253</a>"#
    ));
    assert!(
        export
            .html
            .contains("<strong>Estimated reading time: </strong>50 minutes")
    );
    assert!(
        export
            .html
            .contains("Read Chapter 10 for macro/micro planning.")
    );
    assert!(
        export.html.find(r#"{GENERICO:type="references_end"}"#)
            < export.html.find("Estimated reading time:")
    );
}

#[tokio::test]
async fn module_readings_export_links_stored_doi_when_url_is_blank() {
    let state = desktop_state_with_migrated_pool().await;

    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Evidence-informed practice".to_string(),
            code: Some("Module 2".to_string()),
            order_index: Some(2),
            description: None,
        },
    )
    .await
    .expect("add module");

    let mut reading = ReferenceEntry::new(module.project_id, ReferenceEntryType::Reading);
    reading.module_id = Some(module.id);
    reading.reading_category = Some(ReadingCategory::Compulsory);
    reading.apa_citation =
        Some("Smith, J. (2024). DOI-only readings in practice. Learning Journal.".to_string());
    reading.doi = Some("10.1234/example.doi".to_string());
    SqliteReferenceEntryRepository::new(state.database_pool.clone())
        .insert_reference_entry(&reading)
        .await
        .expect("insert DOI-only reading");

    let export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: module.id,
            for_ako_learn: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export module readings");

    assert!(export.html.contains(
        r#"<a href="https://doi.org/10.1234/example.doi" target="_blank" rel="noopener noreferrer">https://doi.org/10.1234/example.doi</a>"#
    ));

    let library_export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: module.id,
            for_ako_learn: false,
            use_library_links: true,
        },
    )
    .await
    .expect("export module readings with UC links");

    assert!(library_export.html.contains(
        r#"<a href="https://go.openathens.net/redirector/canterbury.ac.nz?url=https://doi.org/10.1234/example.doi" target="_blank" rel="noopener noreferrer">https://doi.org/10.1234/example.doi</a>"#
    ));
}

#[tokio::test]
async fn module_readings_export_can_emit_ako_html() {
    let state = desktop_state_with_migrated_pool().await;

    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Motivating change behaviour".to_string(),
            code: Some("Module 2".to_string()),
            order_index: Some(2),
            description: None,
        },
    )
    .await
    .expect("add module");
    add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "compulsory".to_string(),
            lesson_code: Some("2.1".to_string()),
            apa_citation: Some(
                "Miller, W. R., & Rollnick, S. (2023). Motivational interviewing.".to_string(),
            ),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add reading");

    let export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: module.id,
            for_ako_learn: true,
            use_library_links: false,
        },
    )
    .await
    .expect("export module readings");

    assert_eq!(export.reading_count, 1);
    assert!(!export.html.contains("GENERICO"));
    assert!(export.html.contains(
        r#"<p style="margin-left: 64px; text-indent: -64px;"><span style="font-size: 0.9375rem;">"#
    ));
    assert!(export.html.contains("Miller, W. R."));
}

#[tokio::test]
async fn module_readings_export_filename_uses_project_title_when_code_is_missing() {
    let state = desktop_state_with_migrated_pool().await;
    let project = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: None,
            title: "Strategic Communication".to_string(),
        },
    )
    .await
    .expect("create project without code");
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: Some(project.id),
            title: "Module 6".to_string(),
            code: None,
            order_index: Some(6),
            description: None,
        },
    )
    .await
    .expect("add module");

    let export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: module.id,
            for_ako_learn: false,
            use_library_links: false,
        },
    )
    .await
    .expect("export module readings");

    assert_eq!(
        export.filename,
        "strategic-communication-module-6-module-readings.html"
    );
}

#[tokio::test]
async fn module_readings_export_rejects_missing_module() {
    let state = desktop_state_with_migrated_pool().await;
    let missing_module_id = ModuleId::new();

    let error = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: missing_module_id,
            for_ako_learn: false,
            use_library_links: false,
        },
    )
    .await
    .expect_err("reject missing module");

    assert!(matches!(
        error,
        ModuleReadingExportError::MissingModule(module_id) if module_id == missing_module_id
    ));
}

#[tokio::test]
async fn analyse_docx_path_rejects_empty_path() {
    let state = desktop_state_with_migrated_pool().await;

    let error = analyse_docx_path(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: "  ".to_string(),
            original_filename: None,
        },
    )
    .await
    .expect_err("reject empty path");

    assert!(matches!(error, AnalyseDocxError::EmptyPath));
}

#[tokio::test]
async fn radcite_archive_lists_and_restores_project_items() {
    let state = desktop_state_with_migrated_pool().await;
    let document = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: None,
            path: write_minimal_docx("desktop-archive.docx")
                .to_string_lossy()
                .into_owned(),
            original_filename: Some("desktop-archive.docx".to_string()),
        },
    )
    .await
    .expect("analyse archive document");
    let reference = add_course_reference(
        &state,
        AddCourseReferenceRequest {
            project_id: None,
            apa_citation: "Smith, J. (2024). Archived reference.".to_string(),
            notes: None,
        },
    )
    .await
    .expect("add archive reference");
    let module = add_radcite_module(
        &state,
        AddRadciteModuleRequest {
            project_id: None,
            title: "Archived module".to_string(),
            code: Some("M1".to_string()),
            order_index: Some(1),
            description: None,
        },
    )
    .await
    .expect("add archive module");
    let reading = add_module_reading(
        &state,
        AddModuleReadingRequest {
            module_id: module.id,
            reading_category: "required".to_string(),
            lesson_code: Some("1.1".to_string()),
            apa_citation: Some("Taylor, R. (2023). Archived reading.".to_string()),
            citation_text: None,
            doi: None,
            url: None,
            notes: None,
            reading_notes: None,
            estimated_reading_time: None,
        },
    )
    .await
    .expect("add archive reading");

    archive_radcite_document(
        &state,
        ArchiveRadciteDocumentRequest {
            project_id: None,
            document_id: document.document_id,
        },
    )
    .await
    .expect("archive document");
    archive_course_reference(
        &state,
        ArchiveCourseReferenceRequest {
            reference_id: reference.id,
        },
    )
    .await
    .expect("archive reference");
    archive_module_reading(
        &state,
        ArchiveModuleReadingRequest {
            reading_id: reading.id,
        },
    )
    .await
    .expect("archive reading");
    archive_radcite_module(
        &state,
        ArchiveRadciteModuleRequest {
            module_id: module.id,
        },
    )
    .await
    .expect("archive module");

    let archive = list_radcite_archive(&state, ListRadciteArchiveRequest::default())
        .await
        .expect("list archive");
    assert!(archive.iter().any(|item| {
        item.kind == RadciteArchiveItemKind::Document && item.id == document.document_id.to_string()
    }));
    assert!(archive.iter().any(|item| {
        item.kind == RadciteArchiveItemKind::CourseReference && item.id == reference.id.to_string()
    }));
    assert!(archive.iter().any(|item| {
        item.kind == RadciteArchiveItemKind::Module && item.id == module.id.to_string()
    }));
    assert!(
        !archive
            .iter()
            .any(|item| item.kind == RadciteArchiveItemKind::ModuleReading)
    );

    restore_radcite_archive_item(
        &state,
        RestoreRadciteArchiveItemRequest {
            project_id: None,
            kind: RadciteArchiveItemKind::Document,
            item_id: document.document_id.to_string(),
        },
    )
    .await
    .expect("restore document");
    restore_radcite_archive_item(
        &state,
        RestoreRadciteArchiveItemRequest {
            project_id: None,
            kind: RadciteArchiveItemKind::CourseReference,
            item_id: reference.id.to_string(),
        },
    )
    .await
    .expect("restore reference");
    restore_radcite_archive_item(
        &state,
        RestoreRadciteArchiveItemRequest {
            project_id: None,
            kind: RadciteArchiveItemKind::Module,
            item_id: module.id.to_string(),
        },
    )
    .await
    .expect("restore module and child reading");

    assert!(
        list_radcite_archive(&state, ListRadciteArchiveRequest::default())
            .await
            .expect("list restored archive")
            .is_empty()
    );
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
    write_docx_with_document_xml(filename, document_xml())
}

fn write_readings_import_docx(filename: &str) -> PathBuf {
    write_docx_with_document_xml(filename, readings_import_document_xml())
}

fn write_readings_import_csv(filename: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("radsuite-{filename}"));
    std::fs::write(
        &path,
        concat!(
            "section_seq,section_title,week,citation,talis_article_id\n",
            "5,Week 2 - Positivism,02,\"\"\"Biosocial Theories of Crime\"\" in Miller, M., Schreck, C. & Tewksbury, R. (2015). Criminological Theory: A Brief Introduction (4th ed.). Pearson.\",26922\n",
            "8,Week 5 - The Rise of Critical Criminology,05,\"\"\"Marxist, Postmodern and Green Criminology\"\" in Bernard, T.J., Snipes, J.B., Gerould, A.L., & Vold, G.B. (2019). Vold's Theoretical Criminology. (8th ed.). Oxford University Press. Pages 293-301\",25805\n",
        ),
    )
    .expect("write csv fixture");
    path
}

fn write_readings_import_pdf(filename: &str, lines: &[&str]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("radsuite-{filename}"));
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
    std::fs::write(&path, pdf).expect("write pdf fixture");
    path
}

fn escape_pdf_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
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

fn readings_import_document_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>Module 1</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Compulsory readings</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>1.2 Smith, J. (2024). Worked examples. https://doi.org/10.1234/worked</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Optional readings</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Taylor, R. (2023). Optional primer.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>This ordinary teaching note should not be imported.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#
}
