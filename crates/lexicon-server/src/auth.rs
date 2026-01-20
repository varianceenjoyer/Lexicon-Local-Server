//! Authentication Middleware
//!
//! Provides optional API key authentication for the server.
//!
//! # Features
//!
//! - Optional API key authentication via `X-API-Key` header
//! - Multiple API keys supported
//! - Health check endpoint bypass
//! - Returns 401 Unauthorized for invalid/missing keys

use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::{debug, warn};

/// Authentication configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether authentication is enabled.
    pub enabled: bool,
    /// Valid API keys.
    pub api_keys: HashSet<String>,
    /// Paths that bypass authentication (e.g., health check).
    pub bypass_paths: HashSet<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        let mut bypass_paths = HashSet::new();
        bypass_paths.insert("/health".to_string());

        Self {
            enabled: false, // Disabled by default for local use
            api_keys: HashSet::new(),
            bypass_paths,
        }
    }
}

impl AuthConfig {
    /// Creates a new auth config with the given API key.
    #[must_use]
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        let mut api_keys = HashSet::new();
        api_keys.insert(api_key.into());

        let mut bypass_paths = HashSet::new();
        bypass_paths.insert("/health".to_string());

        Self {
            enabled: true,
            api_keys,
            bypass_paths,
        }
    }

    /// Creates a disabled auth config.
    #[must_use]
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Adds an API key.
    #[must_use]
    pub fn add_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_keys.insert(key.into());
        self.enabled = true;
        self
    }

    /// Adds multiple API keys.
    #[must_use]
    pub fn add_api_keys(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for key in keys {
            self.api_keys.insert(key.into());
        }
        if !self.api_keys.is_empty() {
            self.enabled = true;
        }
        self
    }

    /// Adds a path to bypass authentication.
    #[must_use]
    pub fn add_bypass_path(mut self, path: impl Into<String>) -> Self {
        self.bypass_paths.insert(path.into());
        self
    }

    /// Enables authentication.
    #[must_use]
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Disables authentication.
    #[must_use]
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Checks if a request should bypass authentication.
    pub fn should_bypass(&self, path: &str) -> bool {
        self.bypass_paths.contains(path) || self.bypass_paths.iter().any(|p| path.starts_with(p))
    }

    /// Validates an API key.
    pub fn validate_key(&self, key: &str) -> bool {
        self.api_keys.contains(key)
    }
}

/// Authentication state shared across requests.
pub struct Authenticator {
    config: AuthConfig,
}

impl Authenticator {
    /// Creates a new authenticator.
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// Returns true if authentication is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Checks if a request is authenticated.
    ///
    /// Returns true if:
    /// - Authentication is disabled
    /// - Path is in bypass list
    /// - Valid API key is provided
    pub fn is_authenticated(&self, path: &str, api_key: Option<&str>) -> bool {
        if !self.config.enabled {
            return true;
        }

        if self.config.should_bypass(path) {
            return true;
        }

        match api_key {
            Some(key) => self.config.validate_key(key),
            None => false,
        }
    }
}

/// Error response for authentication failures.
#[derive(Debug, Serialize)]
pub struct AuthError {
    pub error: String,
    pub message: String,
}

impl AuthError {
    fn unauthorized() -> Self {
        Self {
            error: "unauthorized".to_string(),
            message: "Invalid or missing API key".to_string(),
        }
    }

    fn missing_key() -> Self {
        Self {
            error: "unauthorized".to_string(),
            message: "API key required. Provide via X-API-Key header.".to_string(),
        }
    }
}

/// Authentication middleware.
///
/// Checks for valid API key in `X-API-Key` header.
/// Returns 401 Unauthorized if authentication fails.
pub async fn auth_middleware(
    axum::Extension(authenticator): axum::Extension<Arc<Authenticator>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();

    // Check for API key in header
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    if authenticator.is_authenticated(path, api_key) {
        debug!("Request authenticated for path: {}", path);
        next.run(request).await
    } else {
        let error = if api_key.is_some() {
            warn!("Invalid API key for path: {}", path);
            AuthError::unauthorized()
        } else {
            warn!("Missing API key for path: {}", path);
            AuthError::missing_key()
        };

        (
            StatusCode::UNAUTHORIZED,
            [(
                axum::http::header::WWW_AUTHENTICATE,
                HeaderValue::from_static("ApiKey"),
            )],
            Json(error),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.api_keys.is_empty());
        assert!(config.bypass_paths.contains("/health"));
    }

    #[test]
    fn test_auth_config_with_api_key() {
        let config = AuthConfig::with_api_key("test-key-123");
        assert!(config.enabled);
        assert!(config.api_keys.contains("test-key-123"));
    }

    #[test]
    fn test_auth_config_multiple_keys() {
        let config = AuthConfig::default()
            .add_api_key("key1")
            .add_api_key("key2")
            .add_api_key("key3");

        assert!(config.enabled);
        assert!(config.api_keys.len() == 3);
        assert!(config.validate_key("key1"));
        assert!(config.validate_key("key2"));
        assert!(config.validate_key("key3"));
        assert!(!config.validate_key("invalid"));
    }

    #[test]
    fn test_auth_config_bypass_paths() {
        let config = AuthConfig::with_api_key("key")
            .add_bypass_path("/public")
            .add_bypass_path("/api/v1/status");

        assert!(config.should_bypass("/health"));
        assert!(config.should_bypass("/public"));
        assert!(config.should_bypass("/public/file.txt"));
        assert!(config.should_bypass("/api/v1/status"));
        assert!(!config.should_bypass("/api/v1/models"));
    }

    #[test]
    fn test_authenticator_disabled() {
        let auth = Authenticator::new(AuthConfig::disabled());

        assert!(!auth.is_enabled());
        assert!(auth.is_authenticated("/api/v1/models", None));
        assert!(auth.is_authenticated("/api/v1/models", Some("any-key")));
    }

    #[test]
    fn test_authenticator_enabled() {
        let auth = Authenticator::new(AuthConfig::with_api_key("valid-key"));

        assert!(auth.is_enabled());
        // Health check bypassed
        assert!(auth.is_authenticated("/health", None));
        // Valid key
        assert!(auth.is_authenticated("/api/v1/models", Some("valid-key")));
        // Invalid key
        assert!(!auth.is_authenticated("/api/v1/models", Some("invalid-key")));
        // Missing key
        assert!(!auth.is_authenticated("/api/v1/models", None));
    }

    #[test]
    fn test_auth_config_builder_chain() {
        let config = AuthConfig::default()
            .add_api_key("key1")
            .add_api_keys(vec!["key2", "key3"])
            .add_bypass_path("/custom")
            .enable();

        assert!(config.enabled);
        assert!(config.api_keys.len() == 3);
        assert!(config.should_bypass("/custom"));
    }

    #[test]
    fn test_auth_error_serialization() {
        let error = AuthError::unauthorized();
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("unauthorized"));
        assert!(json.contains("Invalid or missing API key"));
    }
}
