use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    database::queries::UsageQueries,
    errors::{AppError, Result},
    handlers::AppState,
    models::ApiKeyLimits,
    services::rate_limiter::RateLimiter,
};

pub struct QuotaManager {
    rate_limiter: RateLimiter,
}

impl QuotaManager {
    pub fn new(rate_limiter: RateLimiter) -> Self {
        Self { rate_limiter }
    }

    pub async fn check_upload_quota(
        &self,
        state: &AppState,
        api_key_id: Uuid,
        owner_id: Uuid,
        file_size: u64,
        limits: &ApiKeyLimits,
    ) -> Result<QuotaCheckResult> {
        let mut violations = Vec::new();

        // Check file size limit
        if file_size > limits.max_image_size_bytes {
            violations.push(QuotaViolation {
                violation_type: QuotaViolationType::FileSizeExceeded,
                current_value: file_size,
                limit_value: limits.max_image_size_bytes,
                message: format!(
                    "File size {} bytes exceeds limit of {} bytes",
                    file_size, limits.max_image_size_bytes
                ),
            });
        }

        // Check daily upload quota
        let daily_uploads = self.rate_limiter
            .get_usage_counter(api_key_id, "daily_uploads")
            .await
            .unwrap_or(0);

        if daily_uploads >= limits.daily_limit as u64 {
            violations.push(QuotaViolation {
                violation_type: QuotaViolationType::DailyUploadsExceeded,
                current_value: daily_uploads,
                limit_value: limits.daily_limit as u64,
                message: format!(
                    "Daily upload limit of {} exceeded (current: {})",
                    limits.daily_limit, daily_uploads
                ),
            });
        }

        // Check monthly upload quota
        let monthly_uploads = self.rate_limiter
            .get_usage_counter(api_key_id, "monthly_uploads")
            .await
            .unwrap_or(0);

        if monthly_uploads >= limits.monthly_limit as u64 {
            violations.push(QuotaViolation {
                violation_type: QuotaViolationType::MonthlyUploadsExceeded,
                current_value: monthly_uploads,
                limit_value: limits.monthly_limit as u64,
                message: format!(
                    "Monthly upload limit of {} exceeded (current: {})",
                    limits.monthly_limit, monthly_uploads
                ),
            });
        }

        // Check total image count
        let image_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM images WHERE owner_id = $1 AND deleted_at IS NULL",
            owner_id
        )
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0) as u64;

        if image_count >= limits.max_images as u64 {
            violations.push(QuotaViolation {
                violation_type: QuotaViolationType::ImageCountExceeded,
                current_value: image_count,
                limit_value: limits.max_images as u64,
                message: format!(
                    "Maximum image count of {} exceeded (current: {})",
                    limits.max_images, image_count
                ),
            });
        }

        // Check total storage usage
        let storage_used = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(orig_size_bytes), 0) FROM images WHERE owner_id = $1 AND deleted_at IS NULL",
            owner_id
        )
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0) as u64;

        let max_storage = limits.max_image_size_bytes * limits.max_images as u64;
        
        if storage_used + file_size > max_storage {
            violations.push(QuotaViolation {
                violation_type: QuotaViolationType::StorageExceeded,
                current_value: storage_used + file_size,
                limit_value: max_storage,
                message: format!(
                    "Storage limit of {} bytes would be exceeded (current: {} + new file: {} = {})",
                    max_storage, storage_used, file_size, storage_used + file_size
                ),
            });
        }

        Ok(QuotaCheckResult {
            allowed: violations.is_empty(),
            violations,
            current_usage: QuotaUsage {
                daily_uploads,
                monthly_uploads,
                image_count,
                storage_used,
            },
        })
    }

    pub async fn record_upload(
        &self,
        state: &AppState,
        api_key_id: Uuid,
        file_size: u64,
    ) -> Result<()> {
        // Increment usage counters
        self.rate_limiter
            .increment_usage_counter(api_key_id, "daily_uploads", 1)
            .await?;

        self.rate_limiter
            .increment_usage_counter(api_key_id, "monthly_uploads", 1)
            .await?;

        self.rate_limiter
            .increment_usage_counter(api_key_id, "daily_bytes", file_size)
            .await?;

        self.rate_limiter
            .increment_usage_counter(api_key_id, "monthly_bytes", file_size)
            .await?;

        // Update database usage counters
        UsageQueries::update_usage(
            state.database.pool(),
            api_key_id,
            1, // requests
            0, // bytes served
            1, // uploads
        ).await?;

        Ok(())
    }

    pub async fn record_download(
        &self,
        state: &AppState,
        api_key_id: Uuid,
        bytes_served: u64,
    ) -> Result<()> {
        // Increment bandwidth usage
        self.rate_limiter
            .increment_usage_counter(api_key_id, "daily_bandwidth", bytes_served)
            .await?;

        self.rate_limiter
            .increment_usage_counter(api_key_id, "monthly_bandwidth", bytes_served)
            .await?;

        // Update database usage counters
        UsageQueries::update_usage(
            state.database.pool(),
            api_key_id,
            1, // requests
            bytes_served as i64,
            0, // uploads
        ).await?;

        Ok(())
    }

    pub async fn get_quota_status(
        &self,
        state: &AppState,
        api_key_id: Uuid,
        owner_id: Uuid,
        limits: &ApiKeyLimits,
    ) -> Result<QuotaStatus> {
        let daily_uploads = self.rate_limiter
            .get_usage_counter(api_key_id, "daily_uploads")
            .await
            .unwrap_or(0);

        let monthly_uploads = self.rate_limiter
            .get_usage_counter(api_key_id, "monthly_uploads")
            .await
            .unwrap_or(0);

        let daily_bandwidth = self.rate_limiter
            .get_usage_counter(api_key_id, "daily_bandwidth")
            .await
            .unwrap_or(0);

        let monthly_bandwidth = self.rate_limiter
            .get_usage_counter(api_key_id, "monthly_bandwidth")
            .await
            .unwrap_or(0);

        let image_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM images WHERE owner_id = $1 AND deleted_at IS NULL",
            owner_id
        )
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0) as u64;

        let storage_used = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(orig_size_bytes), 0) FROM images WHERE owner_id = $1 AND deleted_at IS NULL",
            owner_id
        )
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0) as u64;

        Ok(QuotaStatus {
            daily_uploads: QuotaMetric {
                used: daily_uploads,
                limit: limits.daily_limit as u64,
                percentage: (daily_uploads as f64 / limits.daily_limit as f64 * 100.0) as u32,
            },
            monthly_uploads: QuotaMetric {
                used: monthly_uploads,
                limit: limits.monthly_limit as u64,
                percentage: (monthly_uploads as f64 / limits.monthly_limit as f64 * 100.0) as u32,
            },
            image_count: QuotaMetric {
                used: image_count,
                limit: limits.max_images as u64,
                percentage: (image_count as f64 / limits.max_images as f64 * 100.0) as u32,
            },
            storage: QuotaMetric {
                used: storage_used,
                limit: limits.max_image_size_bytes * limits.max_images as u64,
                percentage: (storage_used as f64 / (limits.max_image_size_bytes * limits.max_images as u64) as f64 * 100.0) as u32,
            },
            daily_bandwidth: daily_bandwidth,
            monthly_bandwidth: monthly_bandwidth,
        })
    }
}

#[derive(Debug)]
pub struct QuotaCheckResult {
    pub allowed: bool,
    pub violations: Vec<QuotaViolation>,
    pub current_usage: QuotaUsage,
}

#[derive(Debug)]
pub struct QuotaViolation {
    pub violation_type: QuotaViolationType,
    pub current_value: u64,
    pub limit_value: u64,
    pub message: String,
}

#[derive(Debug)]
pub enum QuotaViolationType {
    FileSizeExceeded,
    DailyUploadsExceeded,
    MonthlyUploadsExceeded,
    ImageCountExceeded,
    StorageExceeded,
}

#[derive(Debug)]
pub struct QuotaUsage {
    pub daily_uploads: u64,
    pub monthly_uploads: u64,
    pub image_count: u64,
    pub storage_used: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct QuotaStatus {
    pub daily_uploads: QuotaMetric,
    pub monthly_uploads: QuotaMetric,
    pub image_count: QuotaMetric,
    pub storage: QuotaMetric,
    pub daily_bandwidth: u64,
    pub monthly_bandwidth: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct QuotaMetric {
    pub used: u64,
    pub limit: u64,
    pub percentage: u32,
}
