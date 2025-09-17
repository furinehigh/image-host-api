use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    database::queries::UsageQueries,
    errors::AppError,
    handlers::AppState,
    middleware::auth::AuthenticatedApiKey,
    models::ApiKeyLimits,
    services::rate_limiter::RateLimiter,
};

pub async fn quota_middleware(
    State(state): State<AppState>,
    api_key: AuthenticatedApiKey,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Parse API key limits
    let limits: ApiKeyLimits = serde_json::from_value(api_key.limits.clone())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse API key limits"})),
            ).into_response()
        })?;

    let rate_limiter = RateLimiter::new(state.redis.clone());

    // Check daily quota
    let daily_usage = rate_limiter
        .get_usage_counter(api_key.id, "daily")
        .await
        .unwrap_or(0);

    if daily_usage >= limits.daily_limit as u64 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Daily quota exceeded",
                "current_usage": daily_usage,
                "limit": limits.daily_limit,
                "reset_time": get_next_reset_time("daily")
            })),
        ).into_response());
    }

    // Check monthly quota
    let monthly_usage = rate_limiter
        .get_usage_counter(api_key.id, "monthly")
        .await
        .unwrap_or(0);

    if monthly_usage >= limits.monthly_limit as u64 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Monthly quota exceeded",
                "current_usage": monthly_usage,
                "limit": limits.monthly_limit,
                "reset_time": get_next_reset_time("monthly")
            })),
        ).into_response());
    }

    // Check image count quota (from database)
    let image_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM images WHERE owner_id = $1 AND deleted_at IS NULL",
        api_key.owner_id
    )
    .fetch_one(state.database.pool())
    .await
    .unwrap_or(0);

    if image_count >= limits.max_images as i64 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Maximum image count exceeded",
                "current_count": image_count,
                "limit": limits.max_images
            })),
        ).into_response());
    }

    // Check storage quota
    let storage_used = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(orig_size_bytes), 0) FROM images WHERE owner_id = $1 AND deleted_at IS NULL",
        api_key.owner_id
    )
    .fetch_one(state.database.pool())
    .await
    .unwrap_or(0);

    // Calculate max storage (for now, use 10x max_image_size_bytes as total storage limit)
    let max_storage = limits.max_image_size_bytes * 10;

    if storage_used as u64 >= max_storage {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "Storage quota exceeded",
                "current_usage_bytes": storage_used,
                "limit_bytes": max_storage
            })),
        ).into_response());
    }

    // Add quota headers to response
    let response = next.run(request).await;
    
    // Add quota information to response headers
    let mut response = response;
    let headers = response.headers_mut();
    
    headers.insert("X-Quota-Daily-Used", daily_usage.to_string().parse().unwrap());
    headers.insert("X-Quota-Daily-Limit", limits.daily_limit.to_string().parse().unwrap());
    headers.insert("X-Quota-Monthly-Used", monthly_usage.to_string().parse().unwrap());
    headers.insert("X-Quota-Monthly-Limit", limits.monthly_limit.to_string().parse().unwrap());
    headers.insert("X-Quota-Images-Used", image_count.to_string().parse().unwrap());
    headers.insert("X-Quota-Images-Limit", limits.max_images.to_string().parse().unwrap());
    headers.insert("X-Quota-Storage-Used", storage_used.to_string().parse().unwrap());
    headers.insert("X-Quota-Storage-Limit", max_storage.to_string().parse().unwrap());

    Ok(response)
}

fn get_next_reset_time(period: &str) -> u64 {
    let now = chrono::Utc::now();
    match period {
        "daily" => {
            let tomorrow = now.date_naive().succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap();
            tomorrow.and_utc().timestamp() as u64
        }
        "monthly" => {
            let next_month = if now.month() == 12 {
                chrono::NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap()
            } else {
                chrono::NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap()
            };
            next_month.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() as u64
        }
        _ => (now + chrono::Duration::hours(1)).timestamp() as u64,
    }
}
