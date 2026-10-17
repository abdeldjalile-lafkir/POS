use thiserror::Error;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("hash error: {0}")]
    Hash(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("{0}")]
    Other(String),
}

impl From<argon2::password_hash::Error> for StoreError {
    fn from(e: argon2::password_hash::Error) -> Self {
        StoreError::Hash(e.to_string())
    }
}
