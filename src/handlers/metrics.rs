use axum::response::{Response, IntoResponse};
use axum::http::{StatusCode, header};
use prometheus::{Encoder, TextEncoder, gather};

pub async fn metrics() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(output) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, encoder.format_type())
            .body(output)
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("Failed to encode metrics".to_string())
            .unwrap(),
    }
}
