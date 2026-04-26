use radsuite_core::{
    Citation, Document, DocumentFileType, Paragraph, Project, ProjectRole, UserId,
};
use radsuite_db::{
    CitationDocumentRepository, ProjectRepository, SqliteCitationDocumentRepository,
    SqliteProjectRepository, migrate,
};
use sqlx::sqlite::SqlitePoolOptions;

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
