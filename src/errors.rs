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
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    
    #[error("File processing error: {0}")]
    FileProcessing(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Not found")]
    NotFound,
    
    #[error("Forbidden")]
    Forbidden,
    
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
            AppError::Redis(ref e) => {
                tracing::error!("Redis error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Cache error")
            }
            AppError::Auth(ref msg) => (StatusCode::UNAUTHORIZED, msg.as_str()),
            AppError::Validation(ref msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            AppError::RateLimit => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            AppError::QuotaExceeded(ref msg) => (StatusCode::PAYMENT_REQUIRED, msg.as_str()),
            AppError::FileProcessing(ref msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.as_str()),
            AppError::Storage(ref msg) => {
                tracing::error!("Storage error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage error")
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Access forbidden"),
            AppError::Internal(ref e) => {
                tracing::error!("Internal error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
