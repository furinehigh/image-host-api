use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::time::Instant;

use crate::{handlers::AppState, services::metrics::RequestTimer};

pub async fn metrics_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().path().to_string();
    
    // Create timer that will record duration on drop
    let _timer = RequestTimer::new(format!("{} {}", method, uri));
    
    let response = next.run(request).await;
    
    // Record metrics
    let status = response.status();
    if status.is_server_error() {
        state.metrics.record_error("server_error");
    } else if status.is_client_error() {
        state.metrics.record_error("client_error");
    }
    
    response
}
