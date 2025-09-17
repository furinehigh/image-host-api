use anyhow::Result;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub port: u16,
    pub max_file_size: usize,
    pub upload_dir: String,
    pub allowed_mime_types: Vec<String>,
    pub max_image_dimension: u32,
    pub jwt_secret: String,
    pub rate_limit_requests: u32,
    pub rate_limit_window: u64,
    pub default_quota_bytes: i64,
    pub virus_scan_enabled: bool,
    pub virus_scan_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost/image_hosting".to_string()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB
                .parse()?,
            upload_dir: env::var("UPLOAD_DIR")
                .unwrap_or_else(|_| "./uploads".to_string()),
            allowed_mime_types: env::var("ALLOWED_MIME_TYPES")
                .unwrap_or_else(|_| "image/jpeg,image/png,image/webp,image/gif".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            max_image_dimension: env::var("MAX_IMAGE_DIMENSION")
                .unwrap_or_else(|_| "4096".to_string())
                .parse()?,
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key".to_string()),
            rate_limit_requests: env::var("RATE_LIMIT_REQUESTS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
            rate_limit_window: env::var("RATE_LIMIT_WINDOW")
                .unwrap_or_else(|_| "3600".to_string()) // 1 hour
                .parse()?,
            default_quota_bytes: env::var("DEFAULT_QUOTA_BYTES")
                .unwrap_or_else(|_| "1073741824".to_string()) // 1GB
                .parse()?,
            virus_scan_enabled: env::var("VIRUS_SCAN_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()?,
            virus_scan_url: env::var("VIRUS_SCAN_URL").ok(),
        })
    }
}
