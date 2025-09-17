use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub key_hash: String,
    pub owner_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub config_json: Option<serde_json::Value>,
    pub limits_json: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub daily_limit: Option<u32>,
    pub monthly_limit: Option<u32>,
    pub max_images: Option<u32>,
    pub max_image_size_bytes: Option<u64>,
    pub allowed_origins: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyLimits {
    pub daily_limit: u32,
    pub monthly_limit: u32,
    pub max_images: u32,
    pub max_image_size_bytes: u64,
    pub allowed_origins: Vec<String>,
    pub rate_limits: RateLimits,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: Option<String>, // Only returned on creation
    pub created_at: DateTime<Utc>,
    pub limits: ApiKeyLimits,
}
