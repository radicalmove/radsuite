use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Uuid(#[from] uuid::Error),

    #[error("unknown project role: {0}")]
    UnknownRole(String),
}
