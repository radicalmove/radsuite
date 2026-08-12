use radsuite_db::migrate;
use sqlx::{Row, sqlite::SqlitePoolOptions};

#[tokio::test]
async fn project_archive_migration_preserves_existing_projects_and_children() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect");

    sqlx::raw_sql(include_str!("../migrations/0001_foundation.sql"))
        .execute(&pool)
        .await
        .expect("apply foundation schema");
    sqlx::raw_sql(include_str!(
        "../migrations/0002_course_modules_readings.sql"
    ))
    .execute(&pool)
    .await
    .expect("apply course schema");

    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, is_active, is_admin, created_at, updated_at) VALUES ('owner', 'owner@example.test', 'Owner', '', 1, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("insert legacy owner");
    sqlx::query(
        "INSERT INTO projects (id, owner_id, code, title, created_at, updated_at) VALUES ('project', 'owner', 'CRJU201', 'Criminological Theory', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("insert legacy project");
    sqlx::query(
        "INSERT INTO course_modules (id, project_id, title, created_at, updated_at) VALUES ('module', 'project', 'Module 1', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("insert legacy module");

    sqlx::query("CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY NOT NULL, description TEXT NOT NULL, installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, success BOOLEAN NOT NULL, checksum BLOB NOT NULL, execution_time BIGINT NOT NULL)")
        .execute(&pool)
        .await
        .expect("create migration ledger");
    let migrator = sqlx::migrate!("./migrations");
    for migration in migrator.iter().take(2) {
        sqlx::query("INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (?1, ?2, 1, ?3, 0)")
            .bind(migration.version)
            .bind(migration.description.as_ref())
            .bind(migration.checksum.as_ref())
            .execute(&pool)
            .await
            .expect("record legacy migration");
    }

    migrate(&pool).await.expect("apply archive migration");

    let project = sqlx::query(
        "SELECT archived_at, description, structure_mode FROM projects WHERE id = 'project'",
    )
    .fetch_one(&pool)
    .await
    .expect("load migrated project");
    let archived_at: Option<String> = project.try_get("archived_at").expect("archive column");
    assert!(archived_at.is_none());
    let description: Option<String> = project.try_get("description").expect("description column");
    assert!(description.is_none());
    let structure_mode: String = project
        .try_get("structure_mode")
        .expect("structure mode column");
    assert_eq!(structure_mode, "modules");

    let module_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM course_modules WHERE project_id = 'project' AND id = 'module'",
    )
    .fetch_one(&pool)
    .await
    .expect("count preserved modules");
    assert_eq!(module_count, 1);

    let source_path_column_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('documents') WHERE name = 'source_path'",
    )
    .fetch_one(&pool)
    .await
    .expect("check source path column");
    assert_eq!(source_path_column_count, 1);

    let module_id_column_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('documents') WHERE name = 'module_id'",
    )
    .fetch_one(&pool)
    .await
    .expect("check document module column");
    assert_eq!(module_id_column_count, 1);
}
