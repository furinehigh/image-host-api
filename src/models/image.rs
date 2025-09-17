use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Image {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub sha256: String,
    pub mime: String,
    pub orig_size_bytes: i64,
    pub width: i32,
    pub height: i32,
    pub storage_path: String,
    pub variants: serde_json::Value,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub filename: Option<String>,
    pub resize: Option<Vec<u32>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub visibility: Option<Visibility>,
}

#[derive(Debug, Deserialize)]
pub enum Visibility {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "private")]
    Private,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageVariant {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub size_bytes: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageVariants {
    pub original: ImageVariant,
    pub webp: Option<ImageVariant>,
    pub avif: Option<ImageVariant>,
    pub thumbnails: Vec<ImageVariant>,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: Uuid,
    pub url: String,
    pub variants: ImageVariants,
    pub metadata: ImageMetadata,
}

#[derive(Debug, Serialize)]
pub struct ImageMetadata {
    pub width: i32,
    pub height: i32,
    pub size_bytes: i64,
    pub mime: String,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
}
