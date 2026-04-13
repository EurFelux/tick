use thiserror::Error;

#[derive(Debug, Error)]
pub enum TickError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    InvalidArgument(String),

    #[error("{0}")]
    NotInitialized(String),

    #[error("{0}")]
    Conflict(String),

    #[error(transparent)]
    Db(#[from] rusqlite::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl TickError {
    pub fn exit_code(&self) -> i32 {
        match self {
            TickError::Internal(_) => 1,
            TickError::NotFound(_) => 2,
            TickError::InvalidArgument(_) => 3,
            TickError::Db(_) => 4,
            TickError::NotInitialized(_) => 5,
            TickError::Conflict(_) => 6,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            TickError::Internal(_) => "INTERNAL_ERROR",
            TickError::NotFound(_) => "NOT_FOUND",
            TickError::InvalidArgument(_) => "INVALID_ARGUMENT",
            TickError::Db(_) => "DB_ERROR",
            TickError::NotInitialized(_) => "NOT_INITIALIZED",
            TickError::Conflict(_) => "CONFLICT",
        }
    }
}

pub type Result<T> = std::result::Result<T, TickError>;
