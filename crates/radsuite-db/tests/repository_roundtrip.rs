use radsuite_core::{Project, ProjectRole, UserId};
use radsuite_db::{ProjectRepository, SqliteProjectRepository, migrate};
use sqlx::SqlitePool;

#[tokio::test]
async fn project_can_be_inserted_and_listed_for_owner() {
    let pool = SqlitePool::connect("sqlite::memory:")
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
