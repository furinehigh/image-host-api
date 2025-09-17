use figment::{Figment, providers::{Env, Toml}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub max_upload_size: usize,
    pub storage: StorageConfig,
    pub rate_limiting: RateLimitConfig,
    pub image_processing: ImageProcessingConfig,
    pub cloudflare: Option<CloudflareConfig>,
    pub virus_scan: VirusScanConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub storage_type: StorageType,
    pub local_path: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum StorageType {
    Local,
    S3,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    pub default_requests_per_minute: u32,
    pub default_requests_per_hour: u32,
    pub default_requests_per_day: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageProcessingConfig {
    pub use_vips: bool,
    pub vips_path: String,
    pub thumbnail_sizes: Vec<u32>,
    pub quality_webp: u8,
    pub quality_avif: u8,
    pub max_workers: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudflareConfig {
    pub api_token: String,
    pub zone_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VirusScanConfig {
    pub enabled: bool,
    pub clamav_path: Option<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config: Config = Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("IMAGE_SERVER_"))
            .extract()?;

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost/image_hosting".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            jwt_secret: "your-secret-key".to_string(),
            port: 3000,
            max_upload_size: 20 * 1024 * 1024, // 20MB
            storage: StorageConfig {
                storage_type: StorageType::Local,
                local_path: Some("/mnt/images".to_string()),
                s3_bucket: None,
                s3_region: None,
                s3_endpoint: None,
                s3_access_key: None,
                s3_secret_key: None,
            },
            rate_limiting: RateLimitConfig {
                default_requests_per_minute: 60,
                default_requests_per_hour: 1000,
                default_requests_per_day: 10000,
            },
            image_processing: ImageProcessingConfig {
                use_vips: true,
                vips_path: "vips".to_string(),
                thumbnail_sizes: vec![64, 128, 256, 512, 1024],
                quality_webp: 80,
                quality_avif: 70,
                max_workers: 4,
            },
            cloudflare: None,
            virus_scan: VirusScanConfig {
                enabled: false,
                clamav_path: Some("clamscan".to_string()),
            },
        }
    }
}
