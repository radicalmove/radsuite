use async_trait::async_trait;
use chrono::{DateTime, Utc};
use radsuite_core::{
    ApiProjectSummary, AssetId, Citation, CitationId, Document, DocumentFileType, DocumentId,
    DocumentVariant, Paragraph, ParagraphId, Project, ProjectId, ProjectRole, UserId,
};
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
    async fn load_project(&self, project_id: ProjectId) -> Result<Option<Project>, DbError>;
    async fn load_project_by_code(&self, code: &str) -> Result<Option<Project>, DbError>;
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

    async fn load_project(&self, project_id: ProjectId) -> Result<Option<Project>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT id, owner_id, code, title, created_at, updated_at
            FROM projects
            WHERE id = ?1
            "#,
        )
        .bind(project_id.0.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(project_from_row).transpose()
    }

    async fn load_project_by_code(&self, code: &str) -> Result<Option<Project>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT id, owner_id, code, title, created_at, updated_at
            FROM projects
            WHERE code = ?1
            ORDER BY created_at, id
            LIMIT 1
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(project_from_row).transpose()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationDocumentSummary {
    pub document_id: DocumentId,
    pub project_id: ProjectId,
    pub original_filename: String,
    pub file_type: DocumentFileType,
    pub paragraph_count: i64,
    pub citation_count: i64,
    pub missing_citation_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationDocumentAnalysis {
    pub document: Document,
    pub paragraphs: Vec<Paragraph>,
    pub citations: Vec<Citation>,
}

#[async_trait]
pub trait CitationDocumentRepository {
    async fn insert_document_analysis(
        &self,
        document: &Document,
        paragraphs: &[Paragraph],
        citations: &[Citation],
    ) -> Result<(), DbError>;

    async fn list_documents_for_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<CitationDocumentSummary>, DbError>;

    async fn list_saved_documents(&self) -> Result<Vec<CitationDocumentSummary>, DbError>;

    async fn load_document_analysis(
        &self,
        document_id: DocumentId,
    ) -> Result<Option<CitationDocumentAnalysis>, DbError>;

    async fn mark_paragraph_resolved(&self, paragraph_id: ParagraphId) -> Result<(), DbError>;

    async fn verify_paragraph_citations(&self, paragraph_id: ParagraphId) -> Result<(), DbError>;

    async fn insert_manual_citation(
        &self,
        paragraph_id: ParagraphId,
        citation_text: &str,
    ) -> Result<Citation, DbError>;
}

#[derive(Debug, Clone)]
pub struct SqliteCitationDocumentRepository {
    pool: SqlitePool,
}

impl SqliteCitationDocumentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CitationDocumentRepository for SqliteCitationDocumentRepository {
    async fn insert_document_analysis(
        &self,
        document: &Document,
        paragraphs: &[Paragraph],
        citations: &[Citation],
    ) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO documents
                (id, project_id, asset_id, original_filename, file_type, doc_variant, doc_number,
                 notes, exclude_from_references, archived_at, uploaded_at, created_at, updated_at)
            VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
        )
        .bind(document.id.0.to_string())
        .bind(document.project_id.0.to_string())
        .bind(document.asset_id.map(|id| id.0.to_string()))
        .bind(&document.original_filename)
        .bind(document_file_type_as_str(document.file_type))
        .bind(document_variant_as_str(document.doc_variant))
        .bind(document.doc_number)
        .bind(document.notes.as_deref())
        .bind(document.exclude_from_references)
        .bind(document.archived_at.map(|value| value.to_rfc3339()))
        .bind(document.uploaded_at.to_rfc3339())
        .bind(document.created_at.to_rfc3339())
        .bind(document.updated_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        for paragraph in paragraphs {
            sqlx::query(
                r#"
                INSERT INTO paragraphs
                    (id, document_id, order_index, page, text, formatted_text, is_table,
                     needs_citation, created_at, updated_at)
                VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )
            .bind(paragraph.id.0.to_string())
            .bind(paragraph.document_id.0.to_string())
            .bind(paragraph.order_index)
            .bind(paragraph.page)
            .bind(&paragraph.text)
            .bind(paragraph.formatted_text.as_deref())
            .bind(paragraph.is_table)
            .bind(paragraph.needs_citation)
            .bind(paragraph.created_at.to_rfc3339())
            .bind(paragraph.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        for citation in citations {
            sqlx::query(
                r#"
                INSERT INTO paragraph_citations
                    (id, paragraph_id, reference_entry_id, citation_text, position_start,
                     position_end, verified, created_at, updated_at)
                VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
            )
            .bind(citation.id.0.to_string())
            .bind(citation.paragraph_id.0.to_string())
            .bind(citation.reference_entry_id.map(|id| id.0.to_string()))
            .bind(&citation.citation_text)
            .bind(citation.position_start)
            .bind(citation.position_end)
            .bind(citation.verified)
            .bind(citation.created_at.to_rfc3339())
            .bind(citation.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_documents_for_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<CitationDocumentSummary>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT
                d.id,
                d.project_id,
                d.original_filename,
                d.file_type,
                COUNT(DISTINCT p.id) AS paragraph_count,
                COUNT(DISTINCT pc.id) AS citation_count,
                COUNT(DISTINCT CASE WHEN p.needs_citation = 1 THEN p.id END) AS missing_citation_count
            FROM documents d
            LEFT JOIN paragraphs p ON p.document_id = d.id
            LEFT JOIN paragraph_citations pc ON pc.paragraph_id = p.id
            WHERE d.project_id = ?1
              AND d.archived_at IS NULL
            GROUP BY d.id, d.project_id, d.original_filename, d.file_type, d.uploaded_at
            ORDER BY d.uploaded_at DESC, d.original_filename COLLATE NOCASE
            "#,
        )
        .bind(project_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let document_id: String = row.try_get("id")?;
                let row_project_id: String = row.try_get("project_id")?;
                let file_type: String = row.try_get("file_type")?;

                Ok(CitationDocumentSummary {
                    document_id: DocumentId(Uuid::parse_str(&document_id)?),
                    project_id: ProjectId(Uuid::parse_str(&row_project_id)?),
                    original_filename: row.try_get("original_filename")?,
                    file_type: parse_document_file_type(&file_type)?,
                    paragraph_count: row.try_get("paragraph_count")?,
                    citation_count: row.try_get("citation_count")?,
                    missing_citation_count: row.try_get("missing_citation_count")?,
                })
            })
            .collect()
    }

    async fn list_saved_documents(&self) -> Result<Vec<CitationDocumentSummary>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT
                d.id,
                d.project_id,
                d.original_filename,
                d.file_type,
                COUNT(DISTINCT p.id) AS paragraph_count,
                COUNT(DISTINCT pc.id) AS citation_count,
                COUNT(DISTINCT CASE WHEN p.needs_citation = 1 THEN p.id END) AS missing_citation_count
            FROM documents d
            LEFT JOIN paragraphs p ON p.document_id = d.id
            LEFT JOIN paragraph_citations pc ON pc.paragraph_id = p.id
            WHERE d.archived_at IS NULL
            GROUP BY d.id, d.project_id, d.original_filename, d.file_type, d.uploaded_at
            ORDER BY d.uploaded_at DESC, d.original_filename COLLATE NOCASE
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(citation_summary_from_row).collect()
    }

    async fn load_document_analysis(
        &self,
        document_id: DocumentId,
    ) -> Result<Option<CitationDocumentAnalysis>, DbError> {
        let Some(document_row) = sqlx::query(
            r#"
            SELECT
                id, project_id, asset_id, original_filename, file_type, doc_variant, doc_number,
                notes, exclude_from_references, archived_at, uploaded_at, created_at, updated_at
            FROM documents
            WHERE id = ?1
            "#,
        )
        .bind(document_id.0.to_string())
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        let document = document_from_row(&document_row)?;

        let paragraph_rows = sqlx::query(
            r#"
            SELECT
                id, document_id, order_index, page, text, formatted_text, is_table,
                needs_citation, created_at, updated_at
            FROM paragraphs
            WHERE document_id = ?1
            ORDER BY order_index, id
            "#,
        )
        .bind(document_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        let paragraphs = paragraph_rows
            .iter()
            .map(paragraph_from_row)
            .collect::<Result<Vec<_>, _>>()?;

        let citation_rows = sqlx::query(
            r#"
            SELECT
                pc.id, pc.paragraph_id, pc.reference_entry_id, pc.citation_text,
                pc.position_start, pc.position_end, pc.verified, pc.created_at, pc.updated_at
            FROM paragraph_citations pc
            INNER JOIN paragraphs p ON p.id = pc.paragraph_id
            WHERE p.document_id = ?1
            ORDER BY p.order_index, pc.position_start, pc.citation_text
            "#,
        )
        .bind(document_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        let citations = citation_rows
            .iter()
            .map(citation_from_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(CitationDocumentAnalysis {
            document,
            paragraphs,
            citations,
        }))
    }

    async fn mark_paragraph_resolved(&self, paragraph_id: ParagraphId) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE paragraphs
            SET needs_citation = 0,
                updated_at = ?1
            WHERE id = ?2
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(paragraph_id.0.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn verify_paragraph_citations(&self, paragraph_id: ParagraphId) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE paragraph_citations
            SET verified = 1,
                updated_at = ?1
            WHERE paragraph_id = ?2
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(paragraph_id.0.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_manual_citation(
        &self,
        paragraph_id: ParagraphId,
        citation_text: &str,
    ) -> Result<Citation, DbError> {
        let now = Utc::now();
        let citation = Citation {
            id: CitationId::new(),
            paragraph_id,
            reference_entry_id: None,
            citation_text: citation_text.trim().to_string(),
            position_start: None,
            position_end: None,
            verified: true,
            created_at: now,
            updated_at: now,
        };

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO paragraph_citations
                (id, paragraph_id, reference_entry_id, citation_text, position_start,
                 position_end, verified, created_at, updated_at)
            VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(citation.id.0.to_string())
        .bind(citation.paragraph_id.0.to_string())
        .bind(citation.reference_entry_id.map(|id| id.0.to_string()))
        .bind(&citation.citation_text)
        .bind(citation.position_start)
        .bind(citation.position_end)
        .bind(citation.verified)
        .bind(citation.created_at.to_rfc3339())
        .bind(citation.updated_at.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE paragraphs
            SET needs_citation = 0,
                updated_at = ?1
            WHERE id = ?2
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(paragraph_id.0.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(citation)
    }
}

fn citation_summary_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<CitationDocumentSummary, DbError> {
    let document_id: String = row.try_get("id")?;
    let row_project_id: String = row.try_get("project_id")?;
    let file_type: String = row.try_get("file_type")?;

    Ok(CitationDocumentSummary {
        document_id: DocumentId(Uuid::parse_str(&document_id)?),
        project_id: ProjectId(Uuid::parse_str(&row_project_id)?),
        original_filename: row.try_get("original_filename")?,
        file_type: parse_document_file_type(&file_type)?,
        paragraph_count: row.try_get("paragraph_count")?,
        citation_count: row.try_get("citation_count")?,
        missing_citation_count: row.try_get("missing_citation_count")?,
    })
}

fn project_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Project, DbError> {
    let id: String = row.try_get("id")?;
    let owner_id: String = row.try_get("owner_id")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;

    Ok(Project {
        id: ProjectId(Uuid::parse_str(&id)?),
        owner_id: UserId(Uuid::parse_str(&owner_id)?),
        code: row.try_get("code")?,
        title: row.try_get("title")?,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn document_file_type_as_str(value: DocumentFileType) -> &'static str {
    match value {
        DocumentFileType::Pdf => "pdf",
        DocumentFileType::Docx => "docx",
    }
}

fn parse_document_file_type(value: &str) -> Result<DocumentFileType, DbError> {
    match value {
        "pdf" => Ok(DocumentFileType::Pdf),
        "docx" => Ok(DocumentFileType::Docx),
        other => Err(DbError::UnknownDocumentFileType(other.to_string())),
    }
}

fn document_variant_as_str(value: DocumentVariant) -> &'static str {
    match value {
        DocumentVariant::Content => "content",
        DocumentVariant::Rise => "rise",
        DocumentVariant::Other => "other",
    }
}

fn parse_document_variant(value: &str) -> Result<DocumentVariant, DbError> {
    match value {
        "content" => Ok(DocumentVariant::Content),
        "rise" => Ok(DocumentVariant::Rise),
        "other" => Ok(DocumentVariant::Other),
        other => Err(DbError::UnknownDocumentVariant(other.to_string())),
    }
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, DbError> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

fn parse_optional_datetime(value: Option<String>) -> Result<Option<DateTime<Utc>>, DbError> {
    value.as_deref().map(parse_datetime).transpose()
}

fn document_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Document, DbError> {
    let id: String = row.try_get("id")?;
    let project_id: String = row.try_get("project_id")?;
    let asset_id: Option<String> = row.try_get("asset_id")?;
    let file_type: String = row.try_get("file_type")?;
    let doc_variant: String = row.try_get("doc_variant")?;
    let uploaded_at: String = row.try_get("uploaded_at")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;

    Ok(Document {
        id: DocumentId(Uuid::parse_str(&id)?),
        project_id: ProjectId(Uuid::parse_str(&project_id)?),
        asset_id: asset_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()?
            .map(AssetId),
        original_filename: row.try_get("original_filename")?,
        file_type: parse_document_file_type(&file_type)?,
        doc_variant: parse_document_variant(&doc_variant)?,
        doc_number: row.try_get("doc_number")?,
        notes: row.try_get("notes")?,
        exclude_from_references: row.try_get("exclude_from_references")?,
        archived_at: parse_optional_datetime(row.try_get("archived_at")?)?,
        uploaded_at: parse_datetime(&uploaded_at)?,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn paragraph_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Paragraph, DbError> {
    let id: String = row.try_get("id")?;
    let document_id: String = row.try_get("document_id")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;

    Ok(Paragraph {
        id: ParagraphId(Uuid::parse_str(&id)?),
        document_id: DocumentId(Uuid::parse_str(&document_id)?),
        order_index: row.try_get("order_index")?,
        page: row.try_get("page")?,
        text: row.try_get("text")?,
        formatted_text: row.try_get("formatted_text")?,
        is_table: row.try_get("is_table")?,
        needs_citation: row.try_get("needs_citation")?,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}

fn citation_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Citation, DbError> {
    let id: String = row.try_get("id")?;
    let paragraph_id: String = row.try_get("paragraph_id")?;
    let reference_entry_id: Option<String> = row.try_get("reference_entry_id")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;

    Ok(Citation {
        id: CitationId(Uuid::parse_str(&id)?),
        paragraph_id: ParagraphId(Uuid::parse_str(&paragraph_id)?),
        reference_entry_id: reference_entry_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()?
            .map(radsuite_core::ReferenceEntryId),
        citation_text: row.try_get("citation_text")?,
        position_start: row.try_get("position_start")?,
        position_end: row.try_get("position_end")?,
        verified: row.try_get("verified")?,
        created_at: parse_datetime(&created_at)?,
        updated_at: parse_datetime(&updated_at)?,
    })
}
