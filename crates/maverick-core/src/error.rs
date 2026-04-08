use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::Database(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Config(_) => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Io(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Serialization(_) => (axum::http::StatusCode::BAD_REQUEST, self.to_string()),
        };

        axum::Json(serde_json::json!({
            "error": message,
            "status": status.as_u16()
        }))
        .into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;