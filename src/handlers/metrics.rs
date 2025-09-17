use axum::{response::Response, http::StatusCode};
use metrics_exporter_prometheus::PrometheusHandle;

pub async fn metrics_handler(handle: PrometheusHandle) -> Result<Response<String>, StatusCode> {
    match handle.render() {
        Ok(metrics) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/plain; version=0.0.4")
                .body(metrics)
                .unwrap())
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
