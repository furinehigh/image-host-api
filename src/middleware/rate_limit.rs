use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
};
use std::time::Duration;
use std::net::IpAddr;
use crate::{
    handlers::AppState,
    error::AppError,
    middleware::auth::{get_authenticated_user, AuthenticatedUser},
};

#[derive(Clone)]
pub struct RateLimitLayer {
    requests_per_window: u32,
    window_duration: Duration,
}

impl RateLimitLayer {
    pub fn new() -> Self {
        Self {
            requests_per_window: 100, // Default: 100 requests per hour
            window_duration: Duration::from_secs(3600), // 1 hour
        }
    }

    pub fn with_limits(requests: u32, window_seconds: u64) -> Self {
        Self {
            requests_per_window: requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Determine rate limit key based on authentication status
    let rate_limit_key = if let Ok(user) = get_authenticated_user(&request) {
        // Authenticated users get per-user rate limiting
        format!("user:{}", user.user_id)
    } else {
        // Anonymous users get per-IP rate limiting
        let ip = extract_client_ip(&headers, &request)
            .unwrap_or_else(|| "unknown".to_string());
        format!("ip:{}", ip)
    };

    // Check rate limit
    let allowed = state.redis.check_rate_limit(
        &rate_limit_key,
        state.config.rate_limit_requests,
        Duration::from_secs(state.config.rate_limit_window),
    ).await.unwrap_or(false);

    if !allowed {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Add rate limit info to response headers
    let mut response = next.run(request).await;
    
    // Get current count for headers
    if let Ok(current_count) = get_current_rate_limit_count(&state, &rate_limit_key).await {
        let headers = response.headers_mut();
        headers.insert("X-RateLimit-Limit", state.config.rate_limit_requests.into());
        headers.insert("X-RateLimit-Remaining", (state.config.rate_limit_requests.saturating_sub(current_count)).into());
        headers.insert("X-RateLimit-Reset", (chrono::Utc::now().timestamp() + state.config.rate_limit_window as i64).into());
    }

    Ok(response)
}

fn extract_client_ip(headers: &HeaderMap, request: &Request) -> Option<String> {
    // Try various headers for real IP (in order of preference)
    let ip_headers = [
        "CF-Connecting-IP",      // Cloudflare
        "X-Real-IP",             // Nginx
        "X-Forwarded-For",       // Standard proxy header
        "X-Client-IP",           // Apache
        "X-Cluster-Client-IP",   // Cluster
    ];

    for header_name in &ip_headers {
        if let Some(header_value) = headers.get(*header_name) {
            if let Ok(ip_str) = header_value.to_str() {
                // X-Forwarded-For can contain multiple IPs, take the first one
                let ip = ip_str.split(',').next().unwrap_or(ip_str).trim();
                if let Ok(_) = ip.parse::<IpAddr>() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    // Fallback to connection remote addr
    None // In a real implementation, you'd extract this from the connection
}

async fn get_current_rate_limit_count(state: &AppState, key: &str) -> Result<u32, AppError> {
    let mut conn = state.redis.get_connection()?;
    let count: u32 = redis::Commands::get(&mut conn, key).unwrap_or(0);
    Ok(count)
}

// Specialized rate limiters for different endpoints
pub struct EndpointRateLimiter;

impl EndpointRateLimiter {
    // Upload endpoint - more restrictive
    pub async fn check_upload_limit(state: &AppState, user_id: &str) -> Result<bool, AppError> {
        let key = format!("upload:user:{}", user_id);
        let allowed = state.redis.check_rate_limit(
            &key,
            10, // 10 uploads per hour
            Duration::from_secs(3600),
        ).await?;
        Ok(allowed)
    }

    // Transform endpoint - moderate limits
    pub async fn check_transform_limit(state: &AppState, user_id: &str) -> Result<bool, AppError> {
        let key = format!("transform:user:{}", user_id);
        let allowed = state.redis.check_rate_limit(
            &key,
            100, // 100 transforms per hour
            Duration::from_secs(3600),
        ).await?;
        Ok(allowed)
    }

    // Auth endpoints - prevent brute force
    pub async fn check_auth_limit(state: &AppState, ip: &str) -> Result<bool, AppError> {
        let key = format!("auth:ip:{}", ip);
        let allowed = state.redis.check_rate_limit(
            &key,
            5, // 5 auth attempts per 15 minutes
            Duration::from_secs(900),
        ).await?;
        Ok(allowed)
    }
}

// Sliding window rate limiter for more precise control
pub struct SlidingWindowRateLimiter {
    redis: crate::services::redis::RedisService,
}

impl SlidingWindowRateLimiter {
    pub fn new(redis: crate::services::redis::RedisService) -> Self {
        Self { redis }
    }

    pub async fn check_limit(
        &self,
        key: &str,
        limit: u32,
        window_seconds: u64,
    ) -> Result<bool, AppError> {
        let mut conn = self.redis.get_connection()?;
        let now = chrono::Utc::now().timestamp();
        let window_start = now - window_seconds as i64;

        // Use Redis sorted set for sliding window
        let script = r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local limit = tonumber(ARGV[3])
            
            -- Remove old entries
            redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)
            
            -- Count current entries
            local current = redis.call('ZCARD', key)
            
            if current < limit then
                -- Add current request
                redis.call('ZADD', key, now, now)
                redis.call('EXPIRE', key, ARGV[4])
                return 1
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(key)
            .arg(now)
            .arg(window_start)
            .arg(limit)
            .arg(window_seconds)
            .invoke(&mut conn)?;

        Ok(result == 1)
    }
}
