use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AssetId, CitationId, DocumentId, ParagraphId, ProjectId, ReferenceEntryId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub owner_id: UserId,
    pub code: Option<String>,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub fn new(code: impl Into<String>, title: impl Into<String>, owner_id: UserId) -> Self {
        let now = Utc::now();
        let code = code.into();
        Self {
            id: ProjectId::new(),
            owner_id,
            code: (!code.trim().is_empty()).then_some(code),
            title: title.into(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFileType {
    Pdf,
    Docx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentVariant {
    Content,
    Rise,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub project_id: ProjectId,
    pub asset_id: Option<AssetId>,
    pub original_filename: String,
    pub file_type: DocumentFileType,
    pub doc_variant: DocumentVariant,
    pub doc_number: Option<i32>,
    pub notes: Option<String>,
    pub exclude_from_references: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub uploaded_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(
        project_id: ProjectId,
        original_filename: impl Into<String>,
        file_type: DocumentFileType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: DocumentId::new(),
            project_id,
            asset_id: None,
            original_filename: original_filename.into(),
            file_type,
            doc_variant: DocumentVariant::Content,
            doc_number: None,
            notes: None,
            exclude_from_references: false,
            archived_at: None,
            uploaded_at: now,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub id: ParagraphId,
    pub document_id: DocumentId,
    pub order_index: i32,
    pub page: Option<i32>,
    pub text: String,
    pub formatted_text: Option<String>,
    pub is_table: bool,
    pub needs_citation: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Paragraph {
    pub fn new(document_id: DocumentId, order_index: i32, text: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: ParagraphId::new(),
            document_id,
            order_index,
            page: None,
            text: text.into(),
            formatted_text: None,
            is_table: false,
            needs_citation: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    pub id: CitationId,
    pub paragraph_id: ParagraphId,
    pub reference_entry_id: Option<ReferenceEntryId>,
    pub citation_text: String,
    pub position_start: Option<i32>,
    pub position_end: Option<i32>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Citation {
    pub fn new(
        paragraph_id: ParagraphId,
        citation_text: impl Into<String>,
        position_start: i32,
        position_end: i32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: CitationId::new(),
            paragraph_id,
            reference_entry_id: None,
            citation_text: citation_text.into(),
            position_start: Some(position_start),
            position_end: Some(position_end),
            verified: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceEntryType {
    Reference,
    Reading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApaValidationStatus {
    Unknown,
    Valid,
    NeedsFix,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEntry {
    pub id: ReferenceEntryId,
    pub project_id: ProjectId,
    pub document_id: Option<DocumentId>,
    pub paragraph_id: Option<ParagraphId>,
    pub reference_type: ReferenceEntryType,
    pub display_order: Option<i32>,
    pub citation_text: Option<String>,
    pub apa_citation: Option<String>,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub publication_year: Option<String>,
    pub source: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub apa_validation_status: ApaValidationStatus,
    pub apa_validation_report: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ReferenceEntry {
    pub fn new(project_id: ProjectId, reference_type: ReferenceEntryType) -> Self {
        let now = Utc::now();
        Self {
            id: ReferenceEntryId::new(),
            project_id,
            document_id: None,
            paragraph_id: None,
            reference_type,
            display_order: None,
            citation_text: None,
            apa_citation: None,
            title: None,
            authors: Vec::new(),
            publication_year: None,
            source: None,
            doi: None,
            url: None,
            notes: None,
            apa_validation_status: ApaValidationStatus::Unknown,
            apa_validation_report: None,
            archived_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}
