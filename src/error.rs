use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Image processing error: {0}")]
    ImageProcessing(#[from] image::ImageError),
    
    #[error("Invalid file format")]
    InvalidFileFormat,
    
    #[error("File too large")]
    FileTooLarge,
    
    #[error("Image dimensions too large")]
    ImageTooLarge,
    
    #[error("Unauthorized")]
    Unauthorized,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Quota exceeded")]
    QuotaExceeded,
    
    #[error("File not found")]
    FileNotFound,
    
    #[error("Virus detected")]
    VirusDetected,
    
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AppError::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Cache error"),
            AppError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO error"),
            AppError::ImageProcessing(_) => (StatusCode::BAD_REQUEST, "Image processing failed"),
            AppError::InvalidFileFormat => (StatusCode::BAD_REQUEST, "Invalid file format"),
            AppError::FileTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "File too large"),
            AppError::ImageTooLarge => (StatusCode::BAD_REQUEST, "Image dimensions too large"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            AppError::QuotaExceeded => (StatusCode::FORBIDDEN, "Quota exceeded"),
            AppError::FileNotFound => (StatusCode::NOT_FOUND, "File not found"),
            AppError::VirusDetected => (StatusCode::BAD_REQUEST, "Virus detected"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        let body = Json(json!({
            "error": error_message,
            "details": self.to_string()
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
