use metrics::{counter, histogram, gauge};
use std::time::Instant;

pub struct MetricsService;

impl MetricsService {
    pub fn new() -> Self {
        Self
    }

    pub fn record_upload(&self) {
        counter!("uploads_total").increment(1);
    }

    pub fn record_download(&self) {
        counter!("downloads_total").increment(1);
    }

    pub fn record_request_duration(&self, duration: std::time::Duration, endpoint: &str) {
        histogram!("request_duration_seconds", "endpoint" => endpoint.to_string())
            .record(duration.as_secs_f64());
    }

    pub fn record_bytes_processed(&self, bytes: u64, operation: &str) {
        counter!("bytes_processed_total", "operation" => operation.to_string())
            .increment(bytes);
    }

    pub fn record_error(&self, error_type: &str) {
        counter!("errors_total", "type" => error_type.to_string()).increment(1);
    }

    pub fn set_active_connections(&self, count: i64) {
        gauge!("active_connections").set(count as f64);
    }

    pub fn set_queue_size(&self, size: i64) {
        gauge!("processing_queue_size").set(size as f64);
    }

    pub fn record_cache_hit(&self) {
        counter!("cache_hits_total").increment(1);
    }

    pub fn record_cache_miss(&self) {
        counter!("cache_misses_total").increment(1);
    }
}

pub struct RequestTimer {
    start: Instant,
    endpoint: String,
}

impl RequestTimer {
    pub fn new(endpoint: String) -> Self {
        Self {
            start: Instant::now(),
            endpoint,
        }
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        histogram!("request_duration_seconds", "endpoint" => self.endpoint.clone())
            .record(duration.as_secs_f64());
    }
}
