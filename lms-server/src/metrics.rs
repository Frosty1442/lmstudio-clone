//! Prometheus metrics endpoint for observability.
//!
//! Exposes metrics in Prometheus text format at `/metrics`.
//!
//! # Metrics Exported
//!
//! - `lms_requests_total` - Total HTTP requests by endpoint and status
//! - `lms_request_duration_seconds` - Request latency histogram
//! - `lms_models_loaded` - Number of currently loaded models
//! - `lms_active_connections` - Current active connections
//! - `lms_tokens_generated_total` - Total tokens generated
//! - `lms_inference_duration_seconds` - Inference latency histogram

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Metrics collector for the server
pub struct Metrics {
    /// Total requests by endpoint and status
    requests: RwLock<HashMap<String, HashMap<u16, u64>>>,
    /// Request durations for histogram calculation
    durations: RwLock<Vec<Duration>>,
    /// Number of loaded models
    loaded_models: AtomicU64,
    /// Active connections
    active_connections: AtomicU64,
    /// Total tokens generated
    tokens_generated: AtomicU64,
    /// Inference durations
    inference_durations: RwLock<Vec<Duration>>,
    /// Server start time
    start_time: Instant,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            durations: RwLock::new(Vec::new()),
            loaded_models: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            tokens_generated: AtomicU64::new(0),
            inference_durations: RwLock::new(Vec::new()),
            start_time: Instant::now(),
        }
    }

    /// Record a request
    pub fn record_request(&self, endpoint: &str, status: u16, duration: Duration) {
        // Record request count
        let mut requests = self.requests.write().unwrap();
        requests
            .entry(endpoint.to_string())
            .or_default()
            .entry(status)
            .and_modify(|c| *c += 1)
            .or_insert(1);

        // Record duration (keep last 10000 for histogram)
        let mut durations = self.durations.write().unwrap();
        if durations.len() >= 10000 {
            durations.remove(0);
        }
        durations.push(duration);
    }

    /// Record inference metrics
    pub fn record_inference(&self, duration: Duration, tokens: u64) {
        self.tokens_generated.fetch_add(tokens, Ordering::Relaxed);

        let mut durations = self.inference_durations.write().unwrap();
        if durations.len() >= 10000 {
            durations.remove(0);
        }
        durations.push(duration);
    }

    /// Set loaded models count
    pub fn set_loaded_models(&self, count: u64) {
        self.loaded_models.store(count, Ordering::Relaxed);
    }

    /// Increment active connections
    pub fn inc_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn dec_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Generate Prometheus format metrics
    pub fn export(&self) -> String {
        let mut output = String::new();

        // Server uptime
        let uptime = self.start_time.elapsed().as_secs();
        output.push_str("# HELP lms_uptime_seconds Server uptime in seconds\n");
        output.push_str("# TYPE lms_uptime_seconds gauge\n");
        output.push_str(&format!("lms_uptime_seconds {}\n\n", uptime));

        // Request counts
        output.push_str("# HELP lms_requests_total Total HTTP requests\n");
        output.push_str("# TYPE lms_requests_total counter\n");
        let requests = self.requests.read().unwrap();
        for (endpoint, statuses) in requests.iter() {
            for (status, count) in statuses.iter() {
                output.push_str(&format!(
                    "lms_requests_total{{endpoint=\"{}\",status=\"{}\"}} {}\n",
                    endpoint, status, count
                ));
            }
        }
        output.push('\n');

        // Request duration histogram
        output.push_str("# HELP lms_request_duration_seconds Request latency histogram\n");
        output.push_str("# TYPE lms_request_duration_seconds histogram\n");
        let durations = self.durations.read().unwrap();
        if !durations.is_empty() {
            let buckets = [0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
            let mut counts: Vec<u64> = vec![0; buckets.len()];
            let mut sum = 0.0;

            for d in durations.iter() {
                let secs = d.as_secs_f64();
                sum += secs;
                for (i, &bucket) in buckets.iter().enumerate() {
                    if secs <= bucket {
                        counts[i] += 1;
                    }
                }
            }

            for (i, &bucket) in buckets.iter().enumerate() {
                let cumulative: u64 = counts[..=i].iter().sum();
                output.push_str(&format!(
                    "lms_request_duration_seconds_bucket{{le=\"{}\"}} {}\n",
                    bucket, cumulative
                ));
            }
            output.push_str(&format!(
                "lms_request_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
                durations.len()
            ));
            output.push_str(&format!(
                "lms_request_duration_seconds_sum {}\n",
                sum
            ));
            output.push_str(&format!(
                "lms_request_duration_seconds_count {}\n",
                durations.len()
            ));
        }
        output.push('\n');

        // Loaded models
        output.push_str("# HELP lms_models_loaded Number of currently loaded models\n");
        output.push_str("# TYPE lms_models_loaded gauge\n");
        output.push_str(&format!(
            "lms_models_loaded {}\n\n",
            self.loaded_models.load(Ordering::Relaxed)
        ));

        // Active connections
        output.push_str("# HELP lms_active_connections Current active connections\n");
        output.push_str("# TYPE lms_active_connections gauge\n");
        output.push_str(&format!(
            "lms_active_connections {}\n\n",
            self.active_connections.load(Ordering::Relaxed)
        ));

        // Tokens generated
        output.push_str("# HELP lms_tokens_generated_total Total tokens generated\n");
        output.push_str("# TYPE lms_tokens_generated_total counter\n");
        output.push_str(&format!(
            "lms_tokens_generated_total {}\n\n",
            self.tokens_generated.load(Ordering::Relaxed)
        ));

        // Inference duration histogram
        output.push_str("# HELP lms_inference_duration_seconds Inference latency histogram\n");
        output.push_str("# TYPE lms_inference_duration_seconds histogram\n");
        let inf_durations = self.inference_durations.read().unwrap();
        if !inf_durations.is_empty() {
            let buckets = [0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0];
            let mut counts: Vec<u64> = vec![0; buckets.len()];
            let mut sum = 0.0;

            for d in inf_durations.iter() {
                let secs = d.as_secs_f64();
                sum += secs;
                for (i, &bucket) in buckets.iter().enumerate() {
                    if secs <= bucket {
                        counts[i] += 1;
                    }
                }
            }

            for (i, &bucket) in buckets.iter().enumerate() {
                let cumulative: u64 = counts[..=i].iter().sum();
                output.push_str(&format!(
                    "lms_inference_duration_seconds_bucket{{le=\"{}\"}} {}\n",
                    bucket, cumulative
                ));
            }
            output.push_str(&format!(
                "lms_inference_duration_seconds_bucket{{le=\"+Inf\"}} {}\n",
                inf_durations.len()
            ));
            output.push_str(&format!(
                "lms_inference_duration_seconds_sum {}\n",
                sum
            ));
            output.push_str(&format!(
                "lms_inference_duration_seconds_count {}\n",
                inf_durations.len()
            ));
        }

        output
    }
}

/// Global metrics instance
static METRICS: std::sync::OnceLock<Arc<Metrics>> = std::sync::OnceLock::new();

/// Initialize metrics
pub fn init_metrics() -> Arc<Metrics> {
    METRICS
        .get_or_init(|| Arc::new(Metrics::new()))
        .clone()
}

/// Get metrics instance
pub fn get_metrics() -> Option<Arc<Metrics>> {
    METRICS.get().cloned()
}

/// Metrics middleware to track request counts and durations
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let metrics = match get_metrics() {
        Some(m) => m,
        None => return next.run(request).await,
    };

    let path = request.uri().path().to_string();
    let start = Instant::now();

    metrics.inc_connections();
    let response = next.run(request).await;
    metrics.dec_connections();

    let duration = start.elapsed();
    let status = response.status().as_u16();

    // Normalize path for metrics (replace IDs with placeholders)
    let normalized_path = normalize_path(&path);
    metrics.record_request(&normalized_path, status, duration);

    response
}

/// Normalize path by replacing UUIDs and IDs with placeholders
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let mut result = Vec::new();

    for segment in segments {
        // Check if segment looks like a UUID
        if segment.len() == 36 && segment.chars().filter(|c| *c == '-').count() == 4 {
            result.push(":id");
        } else if segment.chars().all(|c| c.is_ascii_hexdigit()) && segment.len() > 8 {
            result.push(":id");
        } else {
            result.push(segment);
        }
    }

    result.join("/")
}

/// Handler for /metrics endpoint
pub async fn metrics_handler() -> impl IntoResponse {
    match get_metrics() {
        Some(metrics) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            metrics.export(),
        ),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            "Metrics not initialized".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        assert_eq!(metrics.loaded_models.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.active_connections.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.tokens_generated.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_request() {
        let metrics = Metrics::new();
        metrics.record_request("/v1/chat/completions", 200, Duration::from_millis(100));
        metrics.record_request("/v1/chat/completions", 200, Duration::from_millis(50));
        metrics.record_request("/v1/chat/completions", 500, Duration::from_millis(10));

        let requests = metrics.requests.read().unwrap();
        let endpoint_stats = requests.get("/v1/chat/completions").unwrap();
        assert_eq!(*endpoint_stats.get(&200).unwrap(), 2);
        assert_eq!(*endpoint_stats.get(&500).unwrap(), 1);
    }

    #[test]
    fn test_record_inference() {
        let metrics = Metrics::new();
        metrics.record_inference(Duration::from_secs(1), 100);
        metrics.record_inference(Duration::from_secs(2), 150);

        assert_eq!(metrics.tokens_generated.load(Ordering::Relaxed), 250);
    }

    #[test]
    fn test_set_loaded_models() {
        let metrics = Metrics::new();
        metrics.set_loaded_models(3);
        assert_eq!(metrics.loaded_models.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_connections() {
        let metrics = Metrics::new();
        metrics.inc_connections();
        metrics.inc_connections();
        assert_eq!(metrics.active_connections.load(Ordering::Relaxed), 2);
        metrics.dec_connections();
        assert_eq!(metrics.active_connections.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_export_format() {
        let metrics = Metrics::new();
        metrics.set_loaded_models(2);
        metrics.record_request("/v1/models", 200, Duration::from_millis(10));

        let output = metrics.export();
        assert!(output.contains("# HELP lms_uptime_seconds"));
        assert!(output.contains("# TYPE lms_uptime_seconds gauge"));
        assert!(output.contains("lms_models_loaded 2"));
        assert!(output.contains("lms_requests_total"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path("/v1/workspaces/123e4567-e89b-12d3-a456-426614174000"),
            "/v1/workspaces/:id"
        );
        assert_eq!(normalize_path("/v1/models"), "/v1/models");
        assert_eq!(
            normalize_path("/v1/workspaces/abc123def456/documents"),
            "/v1/workspaces/:id/documents"
        );
    }
}
