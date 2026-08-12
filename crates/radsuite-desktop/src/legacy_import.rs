use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use radsuite_core::{
    DocumentFileType, DocumentId, DocumentVariant, ModuleId, ParagraphId, Project, ReadingCategory,
    ReferenceEntryId, ReferenceEntryType, UserId,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{Row, Sqlite, Transaction};
use thiserror::Error;

use crate::DesktopState;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LegacyRadciteImportRequest {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LegacyRadciteImportResult {
    pub source_path: String,
    pub projects_imported: usize,
    pub modules_imported: usize,
    pub documents_imported: usize,
    pub paragraphs_imported: usize,
    pub references_imported: usize,
    pub readings_imported: usize,
    pub citations_imported: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum LegacyRadciteImportError {
    #[error("choose the old RADcite SQLite database before importing")]
    EmptyPath,
    #[error("the selected legacy RADcite database does not exist: {0}")]
    MissingFile(PathBuf),
    #[error("could not open the legacy RADcite database")]
    Open(#[source] sqlx::Error),
    #[error("the legacy RADcite database is missing the {0} table")]
    MissingTable(String),
    #[error("could not read the legacy RADcite database")]
    Read(#[source] sqlx::Error),
    #[error("could not save the imported RADcite data")]
    Database(#[source] sqlx::Error),
}

#[derive(Debug)]
struct LegacyCourse {
    id: i64,
    code: Option<String>,
    title: String,
    description: Option<String>,
    structure_mode: Option<String>,
}

#[derive(Debug)]
struct LegacyModule {
    id: i64,
    course_id: i64,
    code: Option<String>,
    title: String,
    order_index: Option<i32>,
    description: Option<String>,
}

#[derive(Debug)]
struct LegacyDocument {
    id: i64,
    course_id: i64,
    module_id: Option<i64>,
    original_filename: String,
    stored_filename: Option<String>,
    file_type: String,
    doc_variant: Option<String>,
    doc_number: Option<i32>,
    notes: Option<String>,
    exclude_from_references: bool,
}

#[derive(Debug)]
struct LegacyParagraph {
    id: i64,
    document_id: i64,
    order_index: i32,
    page: Option<i32>,
    text: String,
    formatted_text: Option<String>,
    is_table: bool,
    needs_citation: bool,
}

#[derive(Debug)]
struct LegacyReference {
    id: i64,
    module_id: Option<i64>,
    document_id: Option<i64>,
    paragraph_id: Option<i64>,
    reference_type: String,
    display_order: Option<i32>,
    lesson_code: Option<String>,
    reading_category: Option<String>,
    citation_text: Option<String>,
    apa_citation: Option<String>,
    title: Option<String>,
    authors_json: Option<String>,
    publication_year: Option<String>,
    source: Option<String>,
    doi: Option<String>,
    url: Option<String>,
    notes: Option<String>,
    reading_notes: Option<String>,
    estimated_reading_time: Option<String>,
    apa_validation_status: Option<String>,
    apa_validation_report: Option<String>,
}

#[derive(Debug)]
struct LegacyCitation {
    paragraph_id: i64,
    reference_entry_id: Option<i64>,
    citation_text: String,
    position_start: Option<i32>,
    position_end: Option<i32>,
    verified: bool,
}

pub async fn import_legacy_radcite_database(
    state: &DesktopState,
    request: LegacyRadciteImportRequest,
) -> Result<LegacyRadciteImportResult, LegacyRadciteImportError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(LegacyRadciteImportError::EmptyPath);
    }

    let source_path = PathBuf::from(path);
    if !source_path.is_file() {
        return Err(LegacyRadciteImportError::MissingFile(source_path));
    }

    let source_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&source_path)
                .read_only(true),
        )
        .await
        .map_err(LegacyRadciteImportError::Open)?;

    let tables = load_table_names(&source_pool).await?;
    for required_table in ["courses", "modules", "documents", "reference_entries"] {
        if !tables.contains(required_table) {
            return Err(LegacyRadciteImportError::MissingTable(
                required_table.to_string(),
            ));
        }
    }

    let courses = load_courses(&source_pool).await?;
    let modules = load_modules(&source_pool).await?;
    let documents = load_documents(&source_pool).await?;
    let paragraphs = if tables.contains("paragraphs") {
        load_paragraphs(&source_pool).await?
    } else {
        Vec::new()
    };
    let references = load_references(&source_pool).await?;
    let citations = if tables.contains("paragraph_citations") {
        load_citations(&source_pool).await?
    } else {
        Vec::new()
    };
    source_pool.close().await;

    let mut transaction = state
        .database_pool
        .begin()
        .await
        .map_err(LegacyRadciteImportError::Database)?;
    let mut result = LegacyRadciteImportResult {
        source_path: source_path.to_string_lossy().into_owned(),
        projects_imported: 0,
        modules_imported: 0,
        documents_imported: 0,
        paragraphs_imported: 0,
        references_imported: 0,
        readings_imported: 0,
        citations_imported: 0,
        warnings: Vec::new(),
    };

    let mut project_ids = HashMap::new();
    for course in courses {
        let mut project = Project::new(
            course.code.clone().unwrap_or_default(),
            course.title,
            UserId::new(),
        );
        project.structure_mode = normalise_structure_mode(course.structure_mode.as_deref());
        let project_id = project.id;
        insert_project(&mut transaction, &project)
            .await
            .map_err(LegacyRadciteImportError::Database)?;
        project_ids.insert(course.id, project_id);
        result.projects_imported += 1;

        if let Some(description) = course.description.filter(|value| !value.trim().is_empty()) {
            sqlx::query("UPDATE projects SET description = ?2 WHERE id = ?1")
                .bind(project_id.to_string())
                .bind(description)
                .execute(&mut *transaction)
                .await
                .map_err(LegacyRadciteImportError::Database)?;
        }
    }

    let mut module_ids = HashMap::new();
    let mut module_projects = HashMap::new();
    for module in modules {
        let Some(project_id) = project_ids.get(&module.course_id).copied() else {
            push_warning(
                &mut result.warnings,
                format!(
                    "Skipped module {} because its course was not imported",
                    module.id
                ),
            );
            continue;
        };

        let module_id = ModuleId::new();
        sqlx::query(
            "INSERT INTO course_modules (id, project_id, code, title, order_index, description, archived_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?7)",
        )
        .bind(module_id.to_string())
        .bind(project_id.to_string())
        .bind(module.code.as_deref())
        .bind(module.title)
        .bind(module.order_index)
        .bind(module.description.as_deref())
        .bind(now_string())
        .execute(&mut *transaction)
        .await
        .map_err(LegacyRadciteImportError::Database)?;
        module_ids.insert(module.id, module_id);
        module_projects.insert(module.id, project_id);
        result.modules_imported += 1;
    }

    let mut document_ids = HashMap::new();
    let mut document_projects = HashMap::new();
    for document in documents {
        let Some(project_id) = project_ids.get(&document.course_id).copied() else {
            push_warning(
                &mut result.warnings,
                format!(
                    "Skipped document {} because its course was not imported",
                    document.original_filename
                ),
            );
            continue;
        };
        document_projects.insert(document.id, project_id);

        let Some(file_type) = parse_file_type(&document.file_type) else {
            push_warning(
                &mut result.warnings,
                format!(
                    "Skipped document {} because its file type is unsupported: {}",
                    document.original_filename, document.file_type
                ),
            );
            continue;
        };
        let document_id = DocumentId::new();
        let source_path = existing_absolute_path(document.stored_filename.as_deref());
        sqlx::query(
            "INSERT INTO documents (id, project_id, module_id, asset_id, source_path, original_filename, file_type, doc_variant, doc_number, notes, exclude_from_references, archived_at, uploaded_at, created_at, updated_at) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, ?11, ?11)",
        )
        .bind(document_id.to_string())
        .bind(project_id.to_string())
        .bind(document.module_id.and_then(|id| module_ids.get(&id).map(ToString::to_string)))
        .bind(source_path.as_deref())
        .bind(document.original_filename)
        .bind(file_type_as_str(file_type))
        .bind(document_variant_as_str(parse_document_variant(
            document.doc_variant.as_deref(),
        )))
        .bind(document.doc_number)
        .bind(document.notes.as_deref())
        .bind(document.exclude_from_references)
        .bind(now_string())
        .execute(&mut *transaction)
        .await
        .map_err(LegacyRadciteImportError::Database)?;
        document_ids.insert(document.id, document_id);
        result.documents_imported += 1;
    }

    let mut paragraph_ids = HashMap::new();
    let mut paragraph_projects = HashMap::new();
    for paragraph in paragraphs {
        let Some(document_id) = document_ids.get(&paragraph.document_id).copied() else {
            push_warning(
                &mut result.warnings,
                format!(
                    "Skipped paragraph {} because its document was not imported",
                    paragraph.id
                ),
            );
            continue;
        };
        let project_id = document_projects
            .get(&paragraph.document_id)
            .copied()
            .expect("document project map contains imported document");
        let paragraph_id = ParagraphId::new();
        sqlx::query(
            "INSERT INTO paragraphs (id, document_id, order_index, page, text, formatted_text, is_table, needs_citation, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        )
        .bind(paragraph_id.to_string())
        .bind(document_id.to_string())
        .bind(paragraph.order_index)
        .bind(paragraph.page)
        .bind(paragraph.text)
        .bind(paragraph.formatted_text.as_deref())
        .bind(paragraph.is_table)
        .bind(paragraph.needs_citation)
        .bind(now_string())
        .execute(&mut *transaction)
        .await
        .map_err(LegacyRadciteImportError::Database)?;
        paragraph_ids.insert(paragraph.id, paragraph_id);
        paragraph_projects.insert(paragraph.id, project_id);
        result.paragraphs_imported += 1;
    }

    let mut reference_ids = HashMap::new();
    for reference in references {
        let project_id = reference
            .module_id
            .and_then(|id| module_projects.get(&id).copied())
            .or_else(|| {
                reference
                    .document_id
                    .and_then(|id| document_projects.get(&id).copied())
            })
            .or_else(|| {
                reference
                    .paragraph_id
                    .and_then(|id| paragraph_projects.get(&id).copied())
            });
        let Some(project_id) = project_id else {
            push_warning(
                &mut result.warnings,
                format!(
                    "Skipped reference {} because its course could not be identified",
                    reference.id
                ),
            );
            continue;
        };
        let Some(reference_type) = parse_reference_type(&reference.reference_type) else {
            push_warning(
                &mut result.warnings,
                format!(
                    "Skipped reference {} because its type is unsupported",
                    reference.id
                ),
            );
            continue;
        };
        let reference_id = ReferenceEntryId::new();
        sqlx::query(
            "INSERT INTO reference_entries (id, project_id, module_id, document_id, paragraph_id, reference_type, display_order, lesson_code, reading_category, citation_text, apa_citation, title, authors_json, publication_year, source, doi, url, notes, reading_notes, estimated_reading_time, apa_validation_status, apa_validation_report, archived_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, NULL, ?23, ?23)",
        )
        .bind(reference_id.to_string())
        .bind(project_id.to_string())
        .bind(reference.module_id.and_then(|id| module_ids.get(&id).map(ToString::to_string)))
        .bind(reference.document_id.and_then(|id| document_ids.get(&id).map(ToString::to_string)))
        .bind(reference.paragraph_id.and_then(|id| paragraph_ids.get(&id).map(ToString::to_string)))
        .bind(reference_type_as_str(reference_type))
        .bind(reference.display_order)
        .bind(reference.lesson_code.as_deref())
        .bind(reference.reading_category.as_deref().and_then(parse_reading_category).map(reading_category_as_str))
        .bind(reference.citation_text.as_deref())
        .bind(reference.apa_citation.as_deref())
        .bind(reference.title.as_deref())
        .bind(normalise_authors_json(reference.authors_json.as_deref()))
        .bind(reference.publication_year.as_deref())
        .bind(reference.source.as_deref())
        .bind(reference.doi.as_deref())
        .bind(reference.url.as_deref())
        .bind(reference.notes.as_deref())
        .bind(reference.reading_notes.as_deref())
        .bind(reference.estimated_reading_time.as_deref())
        .bind(apa_status_as_str(reference.apa_validation_status.as_deref()))
        .bind(reference.apa_validation_report.as_deref())
        .bind(now_string())
        .execute(&mut *transaction)
        .await
        .map_err(LegacyRadciteImportError::Database)?;
        reference_ids.insert(reference.id, reference_id);
        result.references_imported += 1;
        if reference_type == ReferenceEntryType::Reading {
            result.readings_imported += 1;
        }
    }

    for citation in citations {
        let Some(paragraph_id) = paragraph_ids.get(&citation.paragraph_id).copied() else {
            continue;
        };
        let citation_id = radsuite_core::CitationId::new();
        sqlx::query(
            "INSERT INTO paragraph_citations (id, paragraph_id, reference_entry_id, citation_text, position_start, position_end, verified, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        )
        .bind(citation_id.to_string())
        .bind(paragraph_id.to_string())
        .bind(citation.reference_entry_id.and_then(|id| reference_ids.get(&id).map(ToString::to_string)))
        .bind(citation.citation_text)
        .bind(citation.position_start)
        .bind(citation.position_end)
        .bind(citation.verified)
        .bind(now_string())
        .execute(&mut *transaction)
        .await
        .map_err(LegacyRadciteImportError::Database)?;
        result.citations_imported += 1;
    }

    transaction
        .commit()
        .await
        .map_err(LegacyRadciteImportError::Database)?;
    Ok(result)
}

async fn load_table_names(pool: &SqlitePool) -> Result<HashSet<String>, LegacyRadciteImportError> {
    let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table'")
        .fetch_all(pool)
        .await
        .map_err(LegacyRadciteImportError::Read)?;
    rows.into_iter()
        .map(|row| row.try_get::<String, _>("name"))
        .collect::<Result<HashSet<_>, _>>()
        .map_err(LegacyRadciteImportError::Read)
}

async fn load_courses(pool: &SqlitePool) -> Result<Vec<LegacyCourse>, LegacyRadciteImportError> {
    let rows =
        sqlx::query("SELECT id, code, title, description, structure_mode FROM courses ORDER BY id")
            .fetch_all(pool)
            .await
            .map_err(LegacyRadciteImportError::Read)?;
    rows.into_iter()
        .map(|row| {
            Ok(LegacyCourse {
                id: row.try_get("id").map_err(LegacyRadciteImportError::Read)?,
                code: row
                    .try_get("code")
                    .map_err(LegacyRadciteImportError::Read)?,
                title: row
                    .try_get("title")
                    .map_err(LegacyRadciteImportError::Read)?,
                description: row
                    .try_get("description")
                    .map_err(LegacyRadciteImportError::Read)?,
                structure_mode: row
                    .try_get("structure_mode")
                    .map_err(LegacyRadciteImportError::Read)?,
            })
        })
        .collect()
}

async fn load_modules(pool: &SqlitePool) -> Result<Vec<LegacyModule>, LegacyRadciteImportError> {
    let rows = sqlx::query(
        "SELECT id, course_id, code, title, order_index, description FROM modules ORDER BY course_id, order_index, id",
    )
    .fetch_all(pool)
    .await
    .map_err(LegacyRadciteImportError::Read)?;
    rows.into_iter()
        .map(|row| {
            Ok(LegacyModule {
                id: row.try_get("id").map_err(LegacyRadciteImportError::Read)?,
                course_id: row
                    .try_get("course_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                code: row
                    .try_get("code")
                    .map_err(LegacyRadciteImportError::Read)?,
                title: row
                    .try_get("title")
                    .map_err(LegacyRadciteImportError::Read)?,
                order_index: row
                    .try_get("order_index")
                    .map_err(LegacyRadciteImportError::Read)?,
                description: row
                    .try_get("description")
                    .map_err(LegacyRadciteImportError::Read)?,
            })
        })
        .collect()
}

async fn load_documents(
    pool: &SqlitePool,
) -> Result<Vec<LegacyDocument>, LegacyRadciteImportError> {
    let rows = sqlx::query(
        "SELECT id, course_id, module_id, original_filename, stored_filename, file_type, doc_variant, doc_number, notes, exclude_from_references FROM documents ORDER BY course_id, id",
    )
    .fetch_all(pool)
    .await
    .map_err(LegacyRadciteImportError::Read)?;
    rows.into_iter()
        .map(|row| {
            Ok(LegacyDocument {
                id: row.try_get("id").map_err(LegacyRadciteImportError::Read)?,
                course_id: row
                    .try_get("course_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                module_id: row
                    .try_get("module_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                original_filename: row
                    .try_get("original_filename")
                    .map_err(LegacyRadciteImportError::Read)?,
                stored_filename: row
                    .try_get("stored_filename")
                    .map_err(LegacyRadciteImportError::Read)?,
                file_type: row
                    .try_get("file_type")
                    .map_err(LegacyRadciteImportError::Read)?,
                doc_variant: row
                    .try_get("doc_variant")
                    .map_err(LegacyRadciteImportError::Read)?,
                doc_number: row
                    .try_get("doc_number")
                    .map_err(LegacyRadciteImportError::Read)?,
                notes: row
                    .try_get("notes")
                    .map_err(LegacyRadciteImportError::Read)?,
                exclude_from_references: row
                    .try_get("exclude_from_references")
                    .map_err(LegacyRadciteImportError::Read)?,
            })
        })
        .collect()
}

async fn load_paragraphs(
    pool: &SqlitePool,
) -> Result<Vec<LegacyParagraph>, LegacyRadciteImportError> {
    let rows = sqlx::query(
        "SELECT id, document_id, order_index, page, text, formatted_text, is_table, needs_citation FROM paragraphs ORDER BY document_id, order_index, id",
    )
    .fetch_all(pool)
    .await
    .map_err(LegacyRadciteImportError::Read)?;
    rows.into_iter()
        .map(|row| {
            Ok(LegacyParagraph {
                id: row.try_get("id").map_err(LegacyRadciteImportError::Read)?,
                document_id: row
                    .try_get("document_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                order_index: row
                    .try_get("order_index")
                    .map_err(LegacyRadciteImportError::Read)?,
                page: row
                    .try_get("page")
                    .map_err(LegacyRadciteImportError::Read)?,
                text: row
                    .try_get("text")
                    .map_err(LegacyRadciteImportError::Read)?,
                formatted_text: row
                    .try_get("formatted_text")
                    .map_err(LegacyRadciteImportError::Read)?,
                is_table: row
                    .try_get("is_table")
                    .map_err(LegacyRadciteImportError::Read)?,
                needs_citation: row
                    .try_get("needs_citation")
                    .map_err(LegacyRadciteImportError::Read)?,
            })
        })
        .collect()
}

async fn load_references(
    pool: &SqlitePool,
) -> Result<Vec<LegacyReference>, LegacyRadciteImportError> {
    let rows = sqlx::query(
        "SELECT id, module_id, document_id, paragraph_id, reference_type, display_order, lesson_code, reading_category, citation_text, apa_citation, title, authors_json, publication_year, source, doi, url, notes, reading_notes, estimated_reading_time, apa_validation_status, apa_validation_report FROM reference_entries ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(LegacyRadciteImportError::Read)?;
    rows.into_iter()
        .map(|row| {
            Ok(LegacyReference {
                id: row.try_get("id").map_err(LegacyRadciteImportError::Read)?,
                module_id: row
                    .try_get("module_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                document_id: row
                    .try_get("document_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                paragraph_id: row
                    .try_get("paragraph_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                reference_type: row
                    .try_get("reference_type")
                    .map_err(LegacyRadciteImportError::Read)?,
                display_order: row
                    .try_get("display_order")
                    .map_err(LegacyRadciteImportError::Read)?,
                lesson_code: row
                    .try_get("lesson_code")
                    .map_err(LegacyRadciteImportError::Read)?,
                reading_category: row
                    .try_get("reading_category")
                    .map_err(LegacyRadciteImportError::Read)?,
                citation_text: row
                    .try_get("citation_text")
                    .map_err(LegacyRadciteImportError::Read)?,
                apa_citation: row
                    .try_get("apa_citation")
                    .map_err(LegacyRadciteImportError::Read)?,
                title: row
                    .try_get("title")
                    .map_err(LegacyRadciteImportError::Read)?,
                authors_json: row
                    .try_get("authors_json")
                    .map_err(LegacyRadciteImportError::Read)?,
                publication_year: row
                    .try_get("publication_year")
                    .map_err(LegacyRadciteImportError::Read)?,
                source: row
                    .try_get("source")
                    .map_err(LegacyRadciteImportError::Read)?,
                doi: row.try_get("doi").map_err(LegacyRadciteImportError::Read)?,
                url: row.try_get("url").map_err(LegacyRadciteImportError::Read)?,
                notes: row
                    .try_get("notes")
                    .map_err(LegacyRadciteImportError::Read)?,
                reading_notes: row
                    .try_get("reading_notes")
                    .map_err(LegacyRadciteImportError::Read)?,
                estimated_reading_time: row
                    .try_get("estimated_reading_time")
                    .map_err(LegacyRadciteImportError::Read)?,
                apa_validation_status: row
                    .try_get("apa_validation_status")
                    .map_err(LegacyRadciteImportError::Read)?,
                apa_validation_report: row
                    .try_get("apa_validation_report")
                    .map_err(LegacyRadciteImportError::Read)?,
            })
        })
        .collect()
}

async fn load_citations(
    pool: &SqlitePool,
) -> Result<Vec<LegacyCitation>, LegacyRadciteImportError> {
    let rows = sqlx::query(
        "SELECT paragraph_id, reference_entry_id, citation_text, position_start, position_end, verified FROM paragraph_citations ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(LegacyRadciteImportError::Read)?;
    rows.into_iter()
        .map(|row| {
            Ok(LegacyCitation {
                paragraph_id: row
                    .try_get("paragraph_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                reference_entry_id: row
                    .try_get("reference_entry_id")
                    .map_err(LegacyRadciteImportError::Read)?,
                citation_text: row
                    .try_get("citation_text")
                    .map_err(LegacyRadciteImportError::Read)?,
                position_start: row
                    .try_get("position_start")
                    .map_err(LegacyRadciteImportError::Read)?,
                position_end: row
                    .try_get("position_end")
                    .map_err(LegacyRadciteImportError::Read)?,
                verified: row
                    .try_get("verified")
                    .map_err(LegacyRadciteImportError::Read)?,
            })
        })
        .collect()
}

async fn insert_project(
    transaction: &mut Transaction<'_, Sqlite>,
    project: &Project,
) -> Result<(), sqlx::Error> {
    let now = project.created_at.to_rfc3339();
    let owner_id = project.owner_id.to_string();
    let project_id = project.id.to_string();
    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, is_active, is_admin, created_at, updated_at) VALUES (?1, ?2, ?3, '', 1, 0, ?4, ?4)",
    )
    .bind(&owner_id)
    .bind(format!("legacy-owner-{}@radsuite.invalid", project.id))
    .bind("Imported RADcite owner")
    .bind(&now)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO projects (id, owner_id, code, title, description, structure_mode, archived_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?7)",
    )
    .bind(&project_id)
    .bind(&owner_id)
    .bind(project.code.as_deref())
    .bind(&project.title)
    .bind(project.description.as_deref())
    .bind(&project.structure_mode)
    .bind(&now)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO project_members (project_id, user_id, role, created_at) VALUES (?1, ?2, 'owner', ?3)",
    )
    .bind(project_id)
    .bind(owner_id)
    .bind(now)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn now_string() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn push_warning(warnings: &mut Vec<String>, message: String) {
    if warnings.len() < 50 {
        warnings.push(message);
    }
}

fn parse_file_type(value: &str) -> Option<DocumentFileType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "pdf" | ".pdf" | "application/pdf" => Some(DocumentFileType::Pdf),
        "docx"
        | ".docx"
        | "word"
        | "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            Some(DocumentFileType::Docx)
        }
        _ => None,
    }
}

fn parse_document_variant(value: Option<&str>) -> DocumentVariant {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "rise" => DocumentVariant::Rise,
        "other" => DocumentVariant::Other,
        _ => DocumentVariant::Content,
    }
}

fn normalise_structure_mode(value: Option<&str>) -> String {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "weeks" => "weeks".to_string(),
        _ => "modules".to_string(),
    }
}

fn parse_reference_type(value: &str) -> Option<ReferenceEntryType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "reference" => Some(ReferenceEntryType::Reference),
        "reading" => Some(ReferenceEntryType::Reading),
        _ => None,
    }
}

fn parse_reading_category(value: &str) -> Option<ReadingCategory> {
    match value.trim().to_ascii_lowercase().as_str() {
        "compulsory" | "required" => Some(ReadingCategory::Compulsory),
        "optional" => Some(ReadingCategory::Optional),
        _ => None,
    }
}

fn parse_apa_status(value: Option<&str>) -> &'static str {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "valid" => "valid",
        "needs_fix" | "needs-fix" => "needs_fix",
        _ => "unknown",
    }
}

fn existing_absolute_path(value: Option<&str>) -> Option<String> {
    let path = Path::new(value?.trim());
    (path.is_absolute() && path.is_file()).then(|| path.to_string_lossy().into_owned())
}

fn normalise_authors_json(value: Option<&str>) -> String {
    value
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .map(|authors| serde_json::to_string(&authors).expect("serialise parsed authors"))
        .unwrap_or_else(|| "[]".to_string())
}

fn file_type_as_str(value: DocumentFileType) -> &'static str {
    match value {
        DocumentFileType::Pdf => "pdf",
        DocumentFileType::Docx => "docx",
    }
}

fn document_variant_as_str(value: DocumentVariant) -> &'static str {
    match value {
        DocumentVariant::Content => "content",
        DocumentVariant::Rise => "rise",
        DocumentVariant::Other => "other",
    }
}

fn reference_type_as_str(value: ReferenceEntryType) -> &'static str {
    match value {
        ReferenceEntryType::Reference => "reference",
        ReferenceEntryType::Reading => "reading",
    }
}

fn reading_category_as_str(value: ReadingCategory) -> &'static str {
    match value {
        ReadingCategory::Compulsory => "compulsory",
        ReadingCategory::Optional => "optional",
    }
}

fn apa_status_as_str(value: Option<&str>) -> &'static str {
    parse_apa_status(value)
}
