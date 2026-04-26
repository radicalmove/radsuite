use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Uuid(#[from] uuid::Error),

    #[error(transparent)]
    Chrono(#[from] chrono::ParseError),

    #[error("unknown project role: {0}")]
    UnknownRole(String),

    #[error("unknown document file type: {0}")]
    UnknownDocumentFileType(String),

    #[error("unknown document variant: {0}")]
    UnknownDocumentVariant(String),

    #[error("unknown reference entry type: {0}")]
    UnknownReferenceEntryType(String),

    #[error("unknown APA validation status: {0}")]
    UnknownApaValidationStatus(String),
}
