use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use radsuite_core::{ModuleId, ProjectId, ReferenceEntryId};
use radsuite_db::migrate;
use radsuite_desktop::{
    AddCourseReferenceRequest, AddManualCitationRequest, AddModuleReadingRequest,
    AddRadciteModuleRequest, AnalyseDocxError, AnalyseDocxRequest, AppPaths,
    ArchiveModuleReadingRequest, ArchiveRadciteModuleRequest, CreateRadciteProjectRequest,
    DesktopState, ExportCourseReferencesRequest, ExportModuleReadingsRequest,
    LinkCitationReferenceRequest, ListCourseReferencesRequest, ListModuleReadingsRequest,
    ListRadciteModulesRequest, ListSavedReviewsRequest, ModuleReadingError,
    ModuleReadingExportError, ModuleReadingImportError, PreviewModuleReadingsImportRequest,
    RadciteModuleError, SaveModuleReadingsImportCandidate, SaveModuleReadingsImportRequest,
    UpdateModuleReadingRequest, UpdateParagraphReviewRequest, UpdateRadciteModuleRequest,
    add_course_reference, add_manual_citation_for_review, add_module_reading, add_radcite_module,
    analyse_docx_for_review, analyse_docx_path, archive_module_reading, archive_radcite_module,
    create_radcite_project, export_course_references, export_module_readings, get_app_status,
    link_citation_to_reference_for_review, list_course_references, list_module_readings,
    list_radcite_modules, list_radcite_projects, list_saved_radcite_reviews,
    load_saved_radcite_review, mark_paragraph_resolved_for_review, preview_module_readings_import,
    save_module_readings_import, update_module_reading, update_radcite_module,
    verify_paragraph_citations_for_review,
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
    assert_eq!(reading.url.as_deref(), Some("https://example.com/reading"));
    assert_eq!(reading.notes.as_deref(), Some("Manual entry"));
    assert_eq!(reading.reading_notes.as_deref(), Some("Skim before class"));
    assert_eq!(
        reading.estimated_reading_time.as_deref(),
        Some("15 minutes")
    );
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
        "Smith, J. (2024). Worked examples. https://example.com/worked"
    );
    assert_eq!(
        candidates[0].citation_text.as_deref(),
        Some("1.2 Smith, J. (2024). Worked examples. https://example.com/worked")
    );
    assert_eq!(
        candidates[0].url.as_deref(),
        Some("https://example.com/worked")
    );
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
            url: Some(" https://example.com/updated ".to_string()),
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
    assert_eq!(
        updated_reading.url.as_deref(),
        Some("https://example.com/updated")
    );
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

    let missing_reading_id = ReferenceEntryId::new();
    let missing_reading = update_module_reading(
        &state,
        UpdateModuleReadingRequest {
            reading_id: missing_reading_id,
            reading_category: "compulsory".to_string(),
            lesson_code: None,
            apa_citation: Some("Smith, J. (2024). Reading.".to_string()),
            citation_text: None,
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
    assert!(export.html.contains("<h4>Compulsory readings</h4>"));
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
async fn module_readings_export_rejects_missing_module() {
    let state = desktop_state_with_migrated_pool().await;
    let missing_module_id = ModuleId::new();

    let error = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: missing_module_id,
            for_ako_learn: false,
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
      <w:r><w:t>1.2 Smith, J. (2024). Worked examples. https://example.com/worked</w:t></w:r>
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
