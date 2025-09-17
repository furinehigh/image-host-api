use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    errors::AppError,
    handlers::AppState,
    middleware::auth::{AuthenticatedApiKey, AuthenticatedUser},
    models::ApiKeyLimits,
    services::rate_limiter::RateLimiter,
};

pub async fn rate_limit_middleware(
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

    // Check rate limits in order of strictness (minute -> hour -> day)
    let rate_checks = [
        ("minute", limits.rate_limits.requests_per_minute, limits.rate_limits.requests_per_minute),
        ("hour", limits.rate_limits.requests_per_hour, limits.rate_limits.requests_per_hour / 60), // refill rate per minute
        ("day", limits.rate_limits.requests_per_day, limits.rate_limits.requests_per_day / (24 * 60)), // refill rate per minute
    ];

    for (limit_type, capacity, refill_rate) in rate_checks {
        match rate_limiter.check_rate_limit(api_key.id, limit_type, capacity, refill_rate).await {
            Ok(result) => {
                if !result.allowed {
                    let mut headers = HeaderMap::new();
                    headers.insert("X-RateLimit-Limit", capacity.to_string().parse().unwrap());
                    headers.insert("X-RateLimit-Remaining", result.remaining_tokens.to_string().parse().unwrap());
                    headers.insert("X-RateLimit-Reset", result.reset_time.to_string().parse().unwrap());
                    headers.insert("Retry-After", "60".parse().unwrap()); // Retry after 1 minute

                    return Err((
                        StatusCode::TOO_MANY_REQUESTS,
                        headers,
                        Json(json!({
                            "error": "Rate limit exceeded",
                            "limit_type": limit_type,
                            "limit": capacity,
                            "remaining": result.remaining_tokens,
                            "reset_time": result.reset_time,
                            "retry_after": 60
                        })),
                    ).into_response());
                }

                // Add rate limit headers to successful responses
                let response = next.run(request).await;
                let mut response = response;
                let headers = response.headers_mut();
                
                headers.insert("X-RateLimit-Limit", capacity.to_string().parse().unwrap());
                headers.insert("X-RateLimit-Remaining", result.remaining_tokens.to_string().parse().unwrap());
                headers.insert("X-RateLimit-Reset", result.reset_time.to_string().parse().unwrap());

                return Ok(response);
            }
            Err(e) => {
                // Log error but don't block request on rate limiter failure
                tracing::warn!("Rate limiter check failed for API key {}: {}", api_key.id, e);
            }
        }
    }

    // If all rate limit checks pass or fail, continue with request
    Ok(next.run(request).await)
}

pub async fn admin_rate_limit_middleware(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Admin endpoints have more generous rate limits
    let rate_limiter = RateLimiter::new(state.redis.clone());

    // Admin rate limits (higher than regular API keys)
    let admin_limits = [
        ("minute", 200u32, 200u32),   // 200 requests per minute
        ("hour", 5000u32, 5000u32),   // 5000 requests per hour
        ("day", 50000u32, 50000u32),  // 50000 requests per day
    ];

    for (limit_type, capacity, refill_rate) in admin_limits {
        let key = format!("admin_{}", user.id);
        match rate_limiter.check_rate_limit(
            uuid::Uuid::parse_str(&key).unwrap_or(user.id),
            limit_type,
            capacity,
            refill_rate,
        ).await {
            Ok(result) => {
                if !result.allowed {
                    let mut headers = HeaderMap::new();
                    headers.insert("X-RateLimit-Limit", capacity.to_string().parse().unwrap());
                    headers.insert("X-RateLimit-Remaining", result.remaining_tokens.to_string().parse().unwrap());
                    headers.insert("X-RateLimit-Reset", result.reset_time.to_string().parse().unwrap());

                    return Err((
                        StatusCode::TOO_MANY_REQUESTS,
                        headers,
                        Json(json!({
                            "error": "Admin rate limit exceeded",
                            "limit_type": limit_type,
                            "limit": capacity,
                            "remaining": result.remaining_tokens,
                            "reset_time": result.reset_time
                        })),
                    ).into_response());
                }
            }
            Err(e) => {
                tracing::warn!("Admin rate limiter check failed for user {}: {}", user.id, e);
            }
        }
    }

    Ok(next.run(request).await)
}

// Middleware for endpoints that don't require authentication but still need basic rate limiting
pub async fn public_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let rate_limiter = RateLimiter::new(state.redis.clone());
    
    // Use IP address for rate limiting public endpoints
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|hv| hv.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|hv| hv.to_str().ok())
        })
        .unwrap_or("unknown")
        .to_string();

    // Generate a UUID from the IP for consistency with rate limiter
    let ip_uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, client_ip.as_bytes());

    // Public endpoints have strict rate limits
    let public_limits = [
        ("minute", 10u32, 10u32),   // 10 requests per minute per IP
        ("hour", 100u32, 100u32),   // 100 requests per hour per IP
    ];

    for (limit_type, capacity, refill_rate) in public_limits {
        match rate_limiter.check_rate_limit(ip_uuid, limit_type, capacity, refill_rate).await {
            Ok(result) => {
                if !result.allowed {
                    return Err((
                        StatusCode::TOO_MANY_REQUESTS,
                        Json(json!({
                            "error": "Rate limit exceeded for public endpoint",
                            "limit_type": limit_type,
                            "limit": capacity,
                            "remaining": result.remaining_tokens,
                            "reset_time": result.reset_time
                        })),
                    ).into_response());
                }
            }
            Err(e) => {
                tracing::warn!("Public rate limiter check failed for IP {}: {}", client_ip, e);
            }
        }
    }

    Ok(next.run(request).await)
}
