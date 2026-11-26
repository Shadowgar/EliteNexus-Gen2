//! Error type of the galaxy database.

/// Errors from [`crate::GalaxyDb`].
#[derive(Debug, thiserror::Error)]
pub enum GalaxyError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the galaxy database writer thread has stopped")]
    WriterGone,
    #[error("galaxy writes failed before the persistence check: {0}")]
    WriteFailed(String),
    #[error("galaxy WAL checkpoint incomplete: {0}")]
    Checkpoint(String),
    #[error("galaxy schema {found} is newer than supported schema {supported}; use a compatible build or a separate profile")]
    NewerSchema { found: i64, supported: i64 },
    #[error("{0}")]
    Other(String),
}

pub type Result<T, E = GalaxyError> = std::result::Result<T, E>;
