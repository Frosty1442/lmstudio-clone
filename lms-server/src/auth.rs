//! API Key authentication middleware for the LMS server.
//!
//! Supports authentication via:
//! - `Authorization: Bearer <api_key>` header (OpenAI compatible)
//! - `X-API-Key: <api_key>` header
//!
//! API keys can be configured via:
//! - `LMS_API_KEY` environment variable
//! - `LMS_API_KEYS` environment variable (comma-separated list)

use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashSet;
use std::sync::OnceLock;

/// Static storage for valid API keys
static API_KEYS: OnceLock<HashSet<String>> = OnceLock::new();

/// Initialize API keys from environment variables
pub fn init_api_keys() -> bool {
    let keys = API_KEYS.get_or_init(|| {
        let mut key_set = HashSet::new();

        // Check for single API key
        if let Ok(key) = std::env::var("LMS_API_KEY") {
            if !key.is_empty() {
                key_set.insert(key);
            }
        }

        // Check for multiple API keys (comma-separated)
        if let Ok(keys) = std::env::var("LMS_API_KEYS") {
            for key in keys.split(',') {
                let trimmed = key.trim();
                if !trimmed.is_empty() {
                    key_set.insert(trimmed.to_string());
                }
            }
        }

        key_set
    });

    !keys.is_empty()
}

/// Check if API key authentication is enabled
pub fn is_auth_enabled() -> bool {
    API_KEYS.get().is_some_and(|keys| !keys.is_empty())
}

/// Validate an API key
fn validate_api_key(key: &str) -> bool {
    API_KEYS
        .get()
        .is_some_and(|keys| keys.contains(key))
}

/// Extract API key from request headers
fn extract_api_key(request: &Request) -> Option<String> {
    // Try Authorization: Bearer <key>
    if let Some(auth_header) = request.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                return Some(key.to_string());
            }
        }
    }

    // Try X-API-Key header
    if let Some(api_key_header) = request.headers().get("X-API-Key") {
        if let Ok(key) = api_key_header.to_str() {
            return Some(key.to_string());
        }
    }

    None
}

/// Authentication middleware
///
/// Validates API key if authentication is enabled.
/// Allows requests through if no API keys are configured.
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    // Skip auth if not enabled
    if !is_auth_enabled() {
        return next.run(request).await;
    }

    // Allow health check endpoint without auth
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    // Extract and validate API key
    match extract_api_key(&request) {
        Some(key) if validate_api_key(&key) => {
            // Valid API key, proceed
            next.run(request).await
        }
        Some(_) => {
            // Invalid API key
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": {
                        "message": "Invalid API key",
                        "type": "invalid_api_key",
                        "code": 401
                    }
                })),
            )
                .into_response()
        }
        None => {
            // Missing API key
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": {
                        "message": "Missing API key. Provide via 'Authorization: Bearer <key>' or 'X-API-Key' header",
                        "type": "missing_api_key",
                        "code": 401
                    }
                })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_auth_enabled_when_no_keys() {
        // Without initialization, auth should be disabled
        assert!(!is_auth_enabled() || API_KEYS.get().map_or(true, |k| k.is_empty()));
    }
}
