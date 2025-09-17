use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Image {
    pub id: Uuid,
    pub user_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub width: i32,
    pub height: i32,
    pub sha256_hash: String,
    pub storage_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUploadRequest {
    pub filename: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageResponse {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub width: i32,
    pub height: i32,
    pub url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageTransformParams {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub quality: Option<u8>,
    pub format: Option<String>,
}
