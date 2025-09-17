use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::ApiKeyService,
    database::queries::{ApiKeyQueries, UsageQueries},
    errors::{AppError, Result},
    handlers::AppState,
    models::{ApiKeyLimits, ApiKeyResponse, CreateApiKeyRequest, RateLimits, UsageQuery},
    middleware::auth::AuthenticatedUser,
    services::{quota_manager::QuotaManager, rate_limiter::RateLimiter},
};

pub async fn create_api_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<serde_json::Value>> {
    // Only admins can create API keys for now
    if !user.is_admin {
        return Err(AppError::Forbidden);
    }

    // Generate new API key
    let api_key = ApiKeyService::generate_api_key();
    let key_hash = ApiKeyService::hash_api_key(&api_key);

    // Create limits with defaults
    let limits = ApiKeyLimits {
        daily_limit: request.daily_limit.unwrap_or(state.config.rate_limiting.default_requests_per_day),
        monthly_limit: request.monthly_limit.unwrap_or(state.config.rate_limiting.default_requests_per_day * 30),
        max_images: request.max_images.unwrap_or(10000),
        max_image_size_bytes: request.max_image_size_bytes.unwrap_or(state.config.max_upload_size as u64),
        allowed_origins: request.allowed_origins.unwrap_or_default(),
        rate_limits: RateLimits {
            requests_per_minute: state.config.rate_limiting.default_requests_per_minute,
            requests_per_hour: state.config.rate_limiting.default_requests_per_hour,
            requests_per_day: state.config.rate_limiting.default_requests_per_day,
        },
    };

    // Save to database
    let db_api_key = ApiKeyQueries::create_api_key(
        state.database.pool(),
        &key_hash,
        user.id,
        &request.name,
        &limits,
    ).await?;

    let response = ApiKeyResponse {
        id: db_api_key.id,
        name: db_api_key.name,
        key: Some(api_key), // Only return the key on creation
        created_at: db_api_key.created_at,
        limits,
    };

    Ok(Json(json!({
        "message": "API key created successfully",
        "data": response
    })))
}

pub async fn get_api_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Find the API key
    let api_key = sqlx::query!(
        "SELECT id, name, owner_id, created_at, limits_json FROM api_keys WHERE id = $1 AND revoked_at IS NULL",
        key_id
    )
    .fetch_optional(state.database.pool())
    .await?
    .ok_or(AppError::NotFound)?;

    // Check permissions - users can only see their own keys, admins can see all
    if !user.is_admin && api_key.owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    let limits: ApiKeyLimits = serde_json::from_value(api_key.limits_json)?;

    let response = ApiKeyResponse {
        id: api_key.id,
        name: api_key.name,
        key: None, // Never return the actual key after creation
        created_at: api_key.created_at,
        limits,
    };

    Ok(Json(json!({
        "data": response
    })))
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Find the API key
    let api_key = sqlx::query!(
        "SELECT owner_id FROM api_keys WHERE id = $1 AND revoked_at IS NULL",
        key_id
    )
    .fetch_optional(state.database.pool())
    .await?
    .ok_or(AppError::NotFound)?;

    // Check permissions
    if !user.is_admin && api_key.owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    // Revoke the key
    ApiKeyQueries::revoke_api_key(state.database.pool(), key_id).await?;

    Ok(Json(json!({
        "message": "API key revoked successfully"
    })))
}

pub async fn get_usage(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<UsageQuery>,
) -> Result<Json<serde_json::Value>> {
    // Non-admin users can only see their own usage
    let api_key_id = if user.is_admin {
        query.key
    } else {
        // Get user's API keys
        let user_keys = sqlx::query!(
            "SELECT id FROM api_keys WHERE owner_id = $1 AND revoked_at IS NULL",
            user.id
        )
        .fetch_all(state.database.pool())
        .await?;

        if user_keys.is_empty() {
            return Ok(Json(json!({
                "data": {
                    "total_requests": 0,
                    "total_bytes_served": 0,
                    "total_uploads": 0,
                    "daily_breakdown": []
                }
            })));
        }

        // For now, just use the first key. In a real system, you'd aggregate across all user keys
        Some(user_keys[0].id)
    };

    let usage = UsageQueries::get_usage_stats(
        state.database.pool(),
        api_key_id,
        query.from,
        query.to,
    ).await?;

    Ok(Json(json!({
        "data": usage
    })))
}

pub async fn get_quota_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Find the API key
    let api_key = sqlx::query!(
        "SELECT id, owner_id, limits_json FROM api_keys WHERE id = $1 AND revoked_at IS NULL",
        key_id
    )
    .fetch_optional(state.database.pool())
    .await?
    .ok_or(AppError::NotFound)?;

    // Check permissions
    if !user.is_admin && api_key.owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    let limits: ApiKeyLimits = serde_json::from_value(api_key.limits_json)?;
    let rate_limiter = RateLimiter::new(state.redis.clone());
    let quota_manager = QuotaManager::new(rate_limiter);

    let quota_status = quota_manager
        .get_quota_status(&state, api_key.id, api_key.owner_id, &limits)
        .await?;

    Ok(Json(json!({
        "data": quota_status
    })))
}

pub async fn reset_quota(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(key_id): Path<Uuid>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>> {
    // Only admins can reset quotas
    if !user.is_admin {
        return Err(AppError::Forbidden);
    }

    let quota_type = request
        .get("quota_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("quota_type is required".to_string()))?;

    let rate_limiter = RateLimiter::new(state.redis.clone());

    match quota_type {
        "daily" => {
            rate_limiter.reset_rate_limit(key_id, "daily").await?;
            // Also reset usage counters
            let key = format!("usage:{}:daily_uploads", key_id);
            let mut conn = rate_limiter.redis.connection_manager().clone();
            redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
        }
        "monthly" => {
            rate_limiter.reset_rate_limit(key_id, "monthly").await?;
            let key = format!("usage:{}:monthly_uploads", key_id);
            let mut conn = rate_limiter.redis.connection_manager().clone();
            redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
        }
        "all" => {
            rate_limiter.reset_rate_limit(key_id, "daily").await?;
            rate_limiter.reset_rate_limit(key_id, "monthly").await?;
            rate_limiter.reset_rate_limit(key_id, "minute").await?;
            rate_limiter.reset_rate_limit(key_id, "hour").await?;
            
            // Reset all usage counters
            let patterns = ["daily_uploads", "monthly_uploads", "daily_bandwidth", "monthly_bandwidth"];
            let mut conn = rate_limiter.redis.connection_manager().clone();
            for pattern in patterns {
                let key = format!("usage:{}:{}", key_id, pattern);
                redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
            }
        }
        _ => return Err(AppError::Validation("Invalid quota_type".to_string())),
    }

    Ok(Json(json!({
        "message": format!("Quota {} reset successfully", quota_type)
    })))
}
