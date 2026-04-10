use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("domain constraint: {0}")]
    Domain(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("infrastructure: {0}")]
    Infrastructure(String),

    #[error("circuit open: {0}")]
    CircuitOpen(String),
}

pub type AppResult<T> = Result<T, AppError>;
