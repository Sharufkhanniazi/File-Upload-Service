use axum::{Json, 
    http::StatusCode, 
    response::IntoResponse
};
use serde_json::json;
use thiserror::Error;

/// Application-level error type.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("Unsupported media type: {0}")]
    UnSupportedMediaType(String),

    #[error("Multipart error: {0}")]
    MultipartError(String),

    #[error("File processing error: {0}")]
    FileProcessingError(String),

    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
}


/// Convert `AppError` into an HTTP response.
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        // Map application errors to HTTP status codes and messages
        let (status, error_message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, msg),
            AppError::MultipartError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::FileProcessingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::UnSupportedMediaType(msg) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, msg),
            AppError::DatabaseError(err) => {
                tracing::error!("Database Error: {:}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
        };

        // Return standardized JSON error response
        let body = Json(json!({"error": error_message}));
        (status, body).into_response()
    }
}