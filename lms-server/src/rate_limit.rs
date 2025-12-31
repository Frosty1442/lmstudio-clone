//! Rate limiting middleware for the LMS server.
//!
//! Provides configurable rate limiting per client IP address.
//!
//! Configuration via environment variables:
//! - `LMS_RATE_LIMIT`: Requests per minute (default: 60)
//! - `LMS_RATE_LIMIT_BURST`: Burst size (default: 10)

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub requests_per_minute: u32,
    /// Burst allowance
    pub burst_size: u32,
    /// Window duration
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            window: Duration::from_secs(60),
        }
    }
}

impl RateLimitConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let requests_per_minute = std::env::var("LMS_RATE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let burst_size = std::env::var("LMS_RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Self {
            requests_per_minute,
            burst_size,
            window: Duration::from_secs(60),
        }
    }
}

/// Token bucket for a single client
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_update: Instant,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
}

impl TokenBucket {
    fn new(config: &RateLimitConfig) -> Self {
        let max_tokens = config.burst_size as f64;
        let refill_rate = config.requests_per_minute as f64 / 60.0;

        Self {
            tokens: max_tokens,
            last_update: Instant::now(),
            max_tokens,
            refill_rate,
        }
    }

    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        // Refill tokens
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_update = now;

        // Try to consume a token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn tokens_remaining(&self) -> u32 {
        self.tokens as u32
    }

    fn retry_after(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            let needed = 1.0 - self.tokens;
            Duration::from_secs_f64(needed / self.refill_rate)
        }
    }
}

/// Global rate limiter state
pub struct RateLimiter {
    buckets: RwLock<HashMap<IpAddr, TokenBucket>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Check if a request from the given IP is allowed
    pub fn check(&self, ip: IpAddr) -> RateLimitResult {
        let mut buckets = self.buckets.write().unwrap();

        let bucket = buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(&self.config));

        if bucket.try_consume() {
            RateLimitResult::Allowed {
                remaining: bucket.tokens_remaining(),
                limit: self.config.requests_per_minute,
            }
        } else {
            RateLimitResult::Limited {
                retry_after: bucket.retry_after(),
                limit: self.config.requests_per_minute,
            }
        }
    }

    /// Clean up old entries (call periodically)
    pub fn cleanup(&self) {
        let mut buckets = self.buckets.write().unwrap();
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes

        buckets.retain(|_, bucket| bucket.last_update > cutoff);
    }
}

/// Result of a rate limit check
pub enum RateLimitResult {
    Allowed { remaining: u32, limit: u32 },
    Limited { retry_after: Duration, limit: u32 },
}

/// Global rate limiter instance
static RATE_LIMITER: std::sync::OnceLock<Arc<RateLimiter>> = std::sync::OnceLock::new();

/// Initialize the rate limiter
pub fn init_rate_limiter() -> bool {
    let config = RateLimitConfig::from_env();
    let enabled = config.requests_per_minute > 0;

    if enabled {
        RATE_LIMITER.get_or_init(|| Arc::new(RateLimiter::new(config)));
    }

    enabled
}

/// Check if rate limiting is enabled
pub fn is_rate_limiting_enabled() -> bool {
    RATE_LIMITER.get().is_some()
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    // Skip if rate limiting is not enabled
    let limiter = match RATE_LIMITER.get() {
        Some(l) => l,
        None => return next.run(request).await,
    };

    // Get client IP
    let ip = extract_client_ip(&request);

    // Check rate limit
    match limiter.check(ip) {
        RateLimitResult::Allowed { remaining, limit } => {
            let mut response = next.run(request).await;

            // Add rate limit headers
            let headers = response.headers_mut();
            headers.insert("X-RateLimit-Limit", limit.into());
            headers.insert("X-RateLimit-Remaining", remaining.into());

            response
        }
        RateLimitResult::Limited { retry_after, limit } => {
            let retry_secs = retry_after.as_secs().max(1);

            (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    ("X-RateLimit-Limit", limit.to_string()),
                    ("X-RateLimit-Remaining", "0".to_string()),
                    ("Retry-After", retry_secs.to_string()),
                ],
                Json(serde_json::json!({
                    "error": {
                        "message": format!("Rate limit exceeded. Retry after {} seconds.", retry_secs),
                        "type": "rate_limit_exceeded",
                        "code": 429
                    }
                })),
            )
                .into_response()
        }
    }
}

/// Extract client IP from request
fn extract_client_ip(request: &Request) -> IpAddr {
    // Try X-Forwarded-For header first (for reverse proxies)
    if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(first_ip) = s.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return ip;
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("X-Real-IP") {
        if let Ok(s) = real_ip.to_str() {
            if let Ok(ip) = s.parse() {
                return ip;
            }
        }
    }

    // Default to localhost
    "127.0.0.1".parse().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 5,
            window: Duration::from_secs(60),
        };

        let mut bucket = TokenBucket::new(&config);

        // Should allow burst_size requests immediately
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }

        // 6th request should be denied
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_rate_limiter() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 3,
            window: Duration::from_secs(60),
        };

        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // First 3 requests should be allowed
        for _ in 0..3 {
            assert!(matches!(limiter.check(ip), RateLimitResult::Allowed { .. }));
        }

        // 4th request should be limited
        assert!(matches!(limiter.check(ip), RateLimitResult::Limited { .. }));
    }

    #[test]
    fn test_config_from_env() {
        // Default config when no env vars set
        let config = RateLimitConfig::from_env();
        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.burst_size, 10);
    }
}
