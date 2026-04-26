use async_trait::async_trait;
use chrono::Utc;
use radsuite_core::{ApiProjectSummary, Project, ProjectId, ProjectRole, UserId};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::DbError;

#[async_trait]
pub trait ProjectRepository {
    async fn insert_project(&self, project: &Project) -> Result<(), DbError>;
    async fn list_projects_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ApiProjectSummary>, DbError>;
}

#[derive(Debug, Clone)]
pub struct SqliteProjectRepository {
    pool: SqlitePool,
}

impl SqliteProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepository for SqliteProjectRepository {
    async fn insert_project(&self, project: &Project) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let owner_id = project.owner_id.0.to_string();
        let owner_email = format!("local-{}@radsuite.invalid", project.owner_id.0);

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO users
                (id, email, display_name, password_hash, is_active, is_admin, created_at, updated_at)
            VALUES
                (?1, ?2, 'Local owner', '', 1, 0, ?3, ?3)
            "#,
        )
        .bind(&owner_id)
        .bind(owner_email)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO projects
                (id, owner_id, code, title, created_at, updated_at)
            VALUES
                (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(project.id.0.to_string())
        .bind(&owner_id)
        .bind(project.code.as_deref())
        .bind(&project.title)
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO project_members
                (project_id, user_id, role, created_at)
            VALUES
                (?1, ?2, 'owner', ?3)
            "#,
        )
        .bind(project.id.0.to_string())
        .bind(owner_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_projects_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ApiProjectSummary>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.code, p.title, pm.role
            FROM projects p
            INNER JOIN project_members pm ON pm.project_id = p.id
            WHERE pm.user_id = ?1
            ORDER BY p.title COLLATE NOCASE
            "#,
        )
        .bind(user_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let project_id: String = row.try_get("id")?;
                let role: String = row.try_get("role")?;
                Ok(ApiProjectSummary {
                    id: ProjectId(Uuid::parse_str(&project_id)?),
                    code: row.try_get("code")?,
                    title: row.try_get("title")?,
                    role: parse_role(&role)?,
                })
            })
            .collect()
    }
}

fn parse_role(value: &str) -> Result<ProjectRole, DbError> {
    match value {
        "owner" => Ok(ProjectRole::Owner),
        "editor" => Ok(ProjectRole::Editor),
        "viewer" => Ok(ProjectRole::Viewer),
        other => Err(DbError::UnknownRole(other.to_string())),
    }
}
