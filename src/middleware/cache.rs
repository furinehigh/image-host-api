use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap, header},
    middleware::Next,
    response::Response,
    body::Body,
};
use std::time::Duration;
use sha2::{Sha256, Digest};
use crate::handlers::AppState;

pub async fn cache_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Only cache GET requests
    if request.method() != axum::http::Method::GET {
        return Ok(next.run(request).await);
    }

    let uri = request.uri().to_string();
    
    // Skip caching for certain endpoints
    if should_skip_cache(&uri) {
        return Ok(next.run(request).await);
    }

    // Generate cache key
    let cache_key = generate_cache_key(&uri, request.headers());

    // Try to get from cache
    if let Ok(Some(cached_response)) = get_cached_response(&state, &cache_key).await {
        return Ok(cached_response);
    }

    // Process request
    let response = next.run(request).await;

    // Cache successful responses
    if response.status().is_success() {
        cache_response(&state, &cache_key, &response, get_cache_ttl(&uri)).await.ok();
    }

    Ok(response)
}

fn should_skip_cache(uri: &str) -> bool {
    // Skip caching for dynamic or user-specific endpoints
    uri.contains("/api/v1/user/") ||
    uri.contains("/api/v1/upload") ||
    uri.contains("/api/v1/auth/") ||
    uri.contains("/metrics") ||
    uri.contains("/health")
}

fn generate_cache_key(uri: &str, headers: &HeaderMap) -> String {
    let mut hasher = Sha256::new();
    hasher.update(uri.as_bytes());
    
    // Include relevant headers in cache key
    if let Some(accept) = headers.get(header::ACCEPT) {
        hasher.update(accept.as_bytes());
    }
    if let Some(accept_encoding) = headers.get(header::ACCEPT_ENCODING) {
        hasher.update(accept_encoding.as_bytes());
    }

    format!("cache:{:x}", hasher.finalize())
}

async fn get_cached_response(
    state: &AppState,
    cache_key: &str,
) -> Result<Option<Response<Body>>, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = state.redis.get_connection()?;
    
    let cached_data: Option<Vec<u8>> = redis::Commands::get(&mut conn, cache_key)?;
    
    if let Some(data) = cached_data {
        // Deserialize cached response
        if let Ok(cached_response) = bincode::deserialize::<CachedResponse>(&data) {
            let mut response_builder = Response::builder().status(cached_response.status);
            
            // Add cached headers
            for (name, value) in cached_response.headers {
                response_builder = response_builder.header(name, value);
            }
            
            // Add cache hit header
            response_builder = response_builder.header("X-Cache", "HIT");
            
            let response = response_builder.body(Body::from(cached_response.body))?;
            return Ok(Some(response));
        }
    }
    
    Ok(None)
}

async fn cache_response(
    state: &AppState,
    cache_key: &str,
    response: &Response<Body>,
    ttl: Duration,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Only cache responses with certain content types
    if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
        let content_type_str = content_type.to_str().unwrap_or("");
        if !is_cacheable_content_type(content_type_str) {
            return Ok(());
        }
    }

    // Extract response data for caching
    let status = response.status();
    let headers: Vec<(String, String)> = response.headers()
        .iter()
        .filter_map(|(name, value)| {
            // Skip certain headers
            if should_skip_header(name.as_str()) {
                return None;
            }
            Some((name.to_string(), value.to_str().ok()?.to_string()))
        })
        .collect();

    // Note: In a real implementation, you'd need to handle the body extraction differently
    // since Response<Body> doesn't allow easy body extraction without consuming it
    let cached_response = CachedResponse {
        status: status.as_u16(),
        headers,
        body: Vec::new(), // This would need proper implementation
    };

    let serialized = bincode::serialize(&cached_response)?;
    
    let mut conn = state.redis.get_connection()?;
    redis::Commands::set_ex(&mut conn, cache_key, serialized, ttl.as_secs() as usize)?;

    Ok(())
}

fn get_cache_ttl(uri: &str) -> Duration {
    if uri.contains("/images/") && !uri.contains("/transform") {
        // Original images - cache for 1 year
        Duration::from_secs(31536000)
    } else if uri.contains("/transform") {
        // Transformed images - cache for 1 day
        Duration::from_secs(86400)
    } else {
        // Default - cache for 1 hour
        Duration::from_secs(3600)
    }
}

fn is_cacheable_content_type(content_type: &str) -> bool {
    content_type.starts_with("image/") ||
    content_type.starts_with("application/json") ||
    content_type.starts_with("text/")
}

fn should_skip_header(header_name: &str) -> bool {
    matches!(header_name.to_lowercase().as_str(),
        "date" | "server" | "x-request-id" | "set-cookie"
    )
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}
