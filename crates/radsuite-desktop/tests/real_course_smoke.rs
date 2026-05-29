use std::{
    collections::BTreeMap,
    env,
    error::Error,
    path::{Path, PathBuf},
};

use radsuite_core::ProjectId;
use radsuite_db::migrate;
use radsuite_desktop::{
    AddCourseReferenceRequest, AddRadciteModuleRequest, AnalyseDocxRequest, CourseModuleSummary,
    CreateRadciteProjectRequest, DesktopState, ExportCourseReferencesRequest,
    ExportModuleReadingsRequest, ListModuleReadingsRequest, ListSavedReviewsRequest,
    ModuleReadingImportCandidateSummary, PreviewModuleReadingsCsvImportRequest,
    SaveModuleReadingsImportCandidate, SaveModuleReadingsImportRequest, add_course_reference,
    add_radcite_module, analyse_docx_for_review, create_radcite_project, export_course_references,
    export_module_readings, list_module_readings, list_saved_radcite_reviews,
    preview_module_readings_csv_import, save_module_readings_import,
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn real_course_materials_can_exercise_project_csv_docx_flow_when_available()
-> Result<(), Box<dyn Error>> {
    let Some(course_root) = configured_course_output_root() else {
        eprintln!("skipping real-course smoke test: RADSUITE_REAL_COURSE_ROOT is not set");
        return Ok(());
    };

    let crju201_docx = required_file(
        &course_root,
        "Courses/CRJU201/Course content (cleaned transcripts)/CRJU201-26S1-W08-Crime_and_Justice-2026-04-30_course_content.docx",
    );
    let crju201_readings_csv = required_file(
        &course_root,
        "Courses/CRJU201/Extracted/Inventories/course_readings.csv",
    );
    let coms432_docx = required_file(
        &course_root,
        "Courses/COMS432-433/COMS432/AI Generated course content/COMS425-25S2-Lecture-s1-full_01_course_content.docx",
    );
    let mbis622_docx = required_file(
        &course_root,
        "Courses/MBIS622/mbis622-content-review-report-2026-05-26.docx",
    );
    for path in [
        &crju201_docx,
        &crju201_readings_csv,
        &coms432_docx,
        &mbis622_docx,
    ] {
        assert!(
            path.exists(),
            "missing real-course fixture {}",
            path.display()
        );
    }

    let state = desktop_state_with_migrated_pool().await?;
    let crju201 = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("CRJU201".to_string()),
            title: "Crime and Justice".to_string(),
        },
    )
    .await?;

    let crju201_review = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: Some(crju201.id),
            path: crju201_docx.to_string_lossy().into_owned(),
            original_filename: crju201_docx
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string),
        },
    )
    .await?;

    assert_eq!(crju201_review.project_id, crju201.id);
    assert!(crju201_review.summary.paragraph_count > 20);

    let csv_candidates = preview_module_readings_csv_import(
        &state,
        PreviewModuleReadingsCsvImportRequest {
            path: crju201_readings_csv.to_string_lossy().into_owned(),
            original_filename: Some("course_readings.csv".to_string()),
        },
    )
    .await?;

    assert_eq!(csv_candidates.len(), 6);
    assert_eq!(csv_candidates[0].module_order, Some(2));
    assert_eq!(
        csv_candidates[0].module_title.as_deref(),
        Some("Week 2 - Positivism")
    );
    assert!(csv_candidates.iter().any(|candidate| {
        candidate.module_order == Some(11)
            && candidate.apa_citation.contains("The Birth of the Prison")
    }));

    let modules_by_order =
        create_modules_from_csv_candidates(&state, crju201.id, &csv_candidates).await?;
    let saved_readings = save_module_readings_import(
        &state,
        SaveModuleReadingsImportRequest {
            candidates: csv_candidates
                .iter()
                .map(|candidate| {
                    let order = candidate
                        .module_order
                        .expect("real course candidate has module order");
                    let module = modules_by_order
                        .get(&order)
                        .expect("module exists for candidate order");

                    SaveModuleReadingsImportCandidate {
                        module_id: module.id,
                        reading_category: candidate.reading_category.clone(),
                        lesson_code: candidate.lesson_code.clone(),
                        apa_citation: Some(candidate.apa_citation.clone()),
                        citation_text: candidate.citation_text.clone(),
                        url: candidate.url.clone(),
                        notes: Some("Imported from CRJU201 course_readings.csv".to_string()),
                        reading_notes: None,
                        estimated_reading_time: None,
                    }
                })
                .collect(),
        },
    )
    .await?;

    assert_eq!(saved_readings.len(), csv_candidates.len());

    let week_2_module = modules_by_order.get(&2).expect("week 2 module exists");
    let week_2_readings = list_module_readings(
        &state,
        ListModuleReadingsRequest {
            module_id: week_2_module.id,
        },
    )
    .await?;
    assert_eq!(week_2_readings.len(), 3);

    for candidate in &csv_candidates {
        add_course_reference(
            &state,
            AddCourseReferenceRequest {
                project_id: Some(crju201.id),
                apa_citation: candidate.apa_citation.clone(),
                notes: Some("Imported from CRJU201 course_readings.csv".to_string()),
            },
        )
        .await?;
    }

    let references_export = export_course_references(
        &state,
        ExportCourseReferencesRequest {
            project_id: Some(crju201.id),
            for_ako_learn: false,
        },
    )
    .await?;
    assert_eq!(references_export.filename, "crju201-course-references.html");
    assert_eq!(references_export.reference_count, 6);
    assert!(
        references_export
            .html
            .contains("Biosocial Theories of Crime")
    );

    let readings_export = export_module_readings(
        &state,
        ExportModuleReadingsRequest {
            module_id: week_2_module.id,
            for_ako_learn: false,
        },
    )
    .await?;
    assert_eq!(readings_export.filename, "crju201-w02-module-readings.html");
    assert_eq!(readings_export.reading_count, 3);
    assert!(readings_export.html.contains("Biosocial Theories of Crime"));

    let coms432 = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("COMS432".to_string()),
            title: "Building Impactful Campaigns".to_string(),
        },
    )
    .await?;
    let coms432_review = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: Some(coms432.id),
            path: coms432_docx.to_string_lossy().into_owned(),
            original_filename: coms432_docx
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string),
        },
    )
    .await?;
    assert_eq!(coms432_review.project_id, coms432.id);
    assert!(coms432_review.summary.paragraph_count > 20);

    let mbis622 = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("MBIS622".to_string()),
            title: "IS Security and Risk Management".to_string(),
        },
    )
    .await?;
    let mbis622_review = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            project_id: Some(mbis622.id),
            path: mbis622_docx.to_string_lossy().into_owned(),
            original_filename: mbis622_docx
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string),
        },
    )
    .await?;
    assert_eq!(mbis622_review.project_id, mbis622.id);
    assert!(mbis622_review.summary.paragraph_count > 5);

    assert_eq!(
        list_saved_radcite_reviews(
            &state,
            ListSavedReviewsRequest {
                project_id: Some(crju201.id),
            },
        )
        .await?
        .len(),
        1
    );
    assert_eq!(
        list_saved_radcite_reviews(
            &state,
            ListSavedReviewsRequest {
                project_id: Some(coms432.id),
            },
        )
        .await?
        .len(),
        1
    );
    assert_eq!(
        list_saved_radcite_reviews(
            &state,
            ListSavedReviewsRequest {
                project_id: Some(mbis622.id),
            },
        )
        .await?
        .len(),
        1
    );

    Ok(())
}

async fn create_modules_from_csv_candidates(
    state: &DesktopState,
    project_id: ProjectId,
    candidates: &[ModuleReadingImportCandidateSummary],
) -> Result<BTreeMap<i32, CourseModuleSummary>, Box<dyn Error>> {
    let mut modules_by_order = BTreeMap::new();

    for candidate in candidates {
        let Some(order) = candidate.module_order else {
            continue;
        };
        if modules_by_order.contains_key(&order) {
            continue;
        }

        let title = candidate
            .module_title
            .clone()
            .unwrap_or_else(|| format!("Week {order}"));
        let module = add_radcite_module(
            state,
            AddRadciteModuleRequest {
                project_id: Some(project_id),
                title,
                code: Some(format!("W{order:02}")),
                order_index: Some(order),
                description: None,
            },
        )
        .await?;
        modules_by_order.insert(order, module);
    }

    Ok(modules_by_order)
}

async fn desktop_state_with_migrated_pool() -> Result<DesktopState, Box<dyn Error>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    migrate(&pool).await?;
    Ok(DesktopState::for_tests_with_pool(pool))
}

fn configured_course_output_root() -> Option<PathBuf> {
    env::var_os("RADSUITE_REAL_COURSE_ROOT").map(PathBuf::from)
}

fn required_file(root: &Path, relative_path: &str) -> PathBuf {
    root.join(relative_path)
}
