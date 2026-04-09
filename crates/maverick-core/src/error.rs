use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum DomainError {
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("{entity} already exists: {id}")]
    AlreadyExists { entity: &'static str, id: String },

    #[error("validation failed for {field}: {reason}")]
    Validation { field: &'static str, reason: String },

    #[error("invalid state for {entity}: {reason}")]
    InvalidState {
        entity: &'static str,
        reason: String,
    },
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Event bus error: {0}")]
    Event(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::Domain(DomainError::NotFound { .. }) => {
                (axum::http::StatusCode::NOT_FOUND, self.to_string())
            }
            AppError::Domain(DomainError::AlreadyExists { .. }) => {
                (axum::http::StatusCode::CONFLICT, self.to_string())
            }
            AppError::Domain(DomainError::Validation { .. }) => {
                (axum::http::StatusCode::BAD_REQUEST, self.to_string())
            }
            AppError::Domain(DomainError::InvalidState { .. }) => {
                (axum::http::StatusCode::CONFLICT, self.to_string())
            }
            AppError::Database(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
            AppError::Config(_) => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Event(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
            AppError::Io(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
            AppError::Serialization(_) => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
            AppError::ConstraintViolation(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
        };

        (
            status,
            axum::Json(serde_json::json!({
                "error": message,
                "status": status.as_u16()
            })),
        )
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
