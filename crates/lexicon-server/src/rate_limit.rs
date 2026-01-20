//! Rate Limiting Middleware
//!
//! Provides per-IP rate limiting with configurable limits per endpoint category.
//!
//! # Features
//!
//! - Per-IP request tracking using token bucket algorithm
//! - Configurable limits for different endpoint types
//! - Rate limit headers in responses (`X-RateLimit-*`)
//! - Returns 429 Too Many Requests when exceeded

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limiting configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled.
    pub enabled: bool,
    /// Default requests per minute for general endpoints.
    pub default_rpm: u32,
    /// Requests per minute for heavy endpoints (transcribe, TTS synthesis).
    pub heavy_rpm: u32,
    /// Requests per minute for WebSocket connections.
    pub websocket_rpm: u32,
    /// Burst allowance (multiplier of per-minute rate for instantaneous burst).
    pub burst_multiplier: f32,
    /// Time window for rate calculations.
    pub window_secs: u64,
    /// Cleanup interval for expired entries (seconds).
    pub cleanup_interval_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_rpm: 100,        // 100 requests per minute for general endpoints
            heavy_rpm: 20,           // 20 requests per minute for transcribe/TTS
            websocket_rpm: 10,       // 10 WebSocket connections per minute
            burst_multiplier: 2.0,   // Allow 2x burst
            window_secs: 60,         // 1 minute window
            cleanup_interval_secs: 300, // Cleanup every 5 minutes
        }
    }
}

impl RateLimitConfig {
    /// Creates a permissive config for testing.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            enabled: true,
            default_rpm: 1000,
            heavy_rpm: 100,
            websocket_rpm: 50,
            burst_multiplier: 5.0,
            window_secs: 60,
            cleanup_interval_secs: 300,
        }
    }

    /// Creates a strict config for production.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            enabled: true,
            default_rpm: 60,
            heavy_rpm: 10,
            websocket_rpm: 5,
            burst_multiplier: 1.5,
            window_secs: 60,
            cleanup_interval_secs: 300,
        }
    }

    /// Disables rate limiting.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Sets the default requests per minute.
    #[must_use]
    pub fn with_default_rpm(mut self, rpm: u32) -> Self {
        self.default_rpm = rpm;
        self
    }

    /// Sets the heavy endpoint requests per minute.
    #[must_use]
    pub fn with_heavy_rpm(mut self, rpm: u32) -> Self {
        self.heavy_rpm = rpm;
        self
    }

    /// Sets the WebSocket connections per minute.
    #[must_use]
    pub fn with_websocket_rpm(mut self, rpm: u32) -> Self {
        self.websocket_rpm = rpm;
        self
    }

    /// Sets the burst multiplier.
    #[must_use]
    pub fn with_burst_multiplier(mut self, multiplier: f32) -> Self {
        self.burst_multiplier = multiplier;
        self
    }
}

/// Endpoint category for rate limiting purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndpointCategory {
    /// General API endpoints (models, sessions, health).
    General,
    /// Heavy processing endpoints (transcribe, download).
    Heavy,
    /// WebSocket connections.
    WebSocket,
}

impl EndpointCategory {
    /// Determines the category from the request path.
    fn from_path(path: &str) -> Self {
        if path.contains("/stream") {
            EndpointCategory::WebSocket
        } else if path.contains("/transcribe") || path.contains("/download") {
            EndpointCategory::Heavy
        } else {
            EndpointCategory::General
        }
    }

    /// Gets the rate limit for this category from config.
    fn rate_limit(&self, config: &RateLimitConfig) -> u32 {
        match self {
            EndpointCategory::General => config.default_rpm,
            EndpointCategory::Heavy => config.heavy_rpm,
            EndpointCategory::WebSocket => config.websocket_rpm,
        }
    }
}

/// Token bucket for rate limiting.
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of tokens.
    tokens: f64,
    /// Maximum tokens (burst capacity).
    max_tokens: f64,
    /// Tokens added per second.
    refill_rate: f64,
    /// Last update timestamp.
    last_update: Instant,
}

impl TokenBucket {
    /// Creates a new token bucket.
    fn new(rpm: u32, burst_multiplier: f32) -> Self {
        let tokens_per_second = rpm as f64 / 60.0;
        let max_tokens = (rpm as f32 * burst_multiplier) as f64;

        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate: tokens_per_second,
            last_update: Instant::now(),
        }
    }

    /// Attempts to consume a token. Returns true if successful.
    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refills tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_update = now;
    }

    /// Returns the number of remaining tokens (floored).
    fn remaining(&self) -> u32 {
        self.tokens.floor() as u32
    }

    /// Returns seconds until the next token is available.
    fn seconds_until_refill(&self) -> u64 {
        if self.tokens >= 1.0 {
            0
        } else {
            let needed = 1.0 - self.tokens;
            (needed / self.refill_rate).ceil() as u64
        }
    }

    /// Returns whether this bucket has expired (no activity for a while).
    fn is_expired(&self, max_age: Duration) -> bool {
        self.last_update.elapsed() > max_age
    }
}

/// Per-IP rate limiter state.
#[derive(Debug, Clone)]
struct IpState {
    /// Buckets per endpoint category.
    buckets: HashMap<EndpointCategory, TokenBucket>,
}

impl IpState {
    fn new() -> Self {
        Self {
            buckets: HashMap::new(),
        }
    }

    /// Gets or creates a bucket for the category.
    fn get_bucket(&mut self, category: EndpointCategory, config: &RateLimitConfig) -> &mut TokenBucket {
        self.buckets.entry(category).or_insert_with(|| {
            TokenBucket::new(category.rate_limit(config), config.burst_multiplier)
        })
    }
}

/// Rate limiter state shared across requests.
#[derive(Debug)]
pub struct RateLimiter {
    /// Configuration.
    config: RateLimitConfig,
    /// Per-IP state.
    state: RwLock<HashMap<IpAddr, IpState>>,
    /// Last cleanup time.
    last_cleanup: RwLock<Instant>,
}

impl RateLimiter {
    /// Creates a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            state: RwLock::new(HashMap::new()),
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Checks if a request should be rate limited.
    ///
    /// Returns `Ok(RateLimitInfo)` if allowed, `Err(RateLimitInfo)` if exceeded.
    pub async fn check(&self, ip: IpAddr, category: EndpointCategory) -> Result<RateLimitInfo, RateLimitInfo> {
        if !self.config.enabled {
            return Ok(RateLimitInfo::unlimited());
        }

        // Periodic cleanup
        self.maybe_cleanup().await;

        let mut state = self.state.write().await;
        let ip_state = state.entry(ip).or_insert_with(IpState::new);
        let bucket = ip_state.get_bucket(category, &self.config);

        let limit = category.rate_limit(&self.config);
        let remaining = bucket.remaining();
        let reset_secs = bucket.seconds_until_refill();

        if bucket.try_consume() {
            debug!(
                "Rate limit check passed for {:?} from {}: {}/{} remaining",
                category, ip, remaining, limit
            );
            Ok(RateLimitInfo {
                limit,
                remaining: bucket.remaining(),
                reset_secs,
            })
        } else {
            warn!(
                "Rate limit exceeded for {:?} from {}: 0/{} remaining, reset in {}s",
                category, ip, limit, reset_secs
            );
            Err(RateLimitInfo {
                limit,
                remaining: 0,
                reset_secs,
            })
        }
    }

    /// Performs cleanup of expired entries if needed.
    async fn maybe_cleanup(&self) {
        let should_cleanup = {
            let last = self.last_cleanup.read().await;
            last.elapsed().as_secs() >= self.config.cleanup_interval_secs
        };

        if should_cleanup {
            let mut state = self.state.write().await;
            let max_age = Duration::from_secs(self.config.window_secs * 10);

            state.retain(|_ip, ip_state| {
                ip_state.buckets.retain(|_cat, bucket| !bucket.is_expired(max_age));
                !ip_state.buckets.is_empty()
            });

            *self.last_cleanup.write().await = Instant::now();
            debug!("Rate limiter cleanup complete, {} IPs tracked", state.len());
        }
    }
}

/// Rate limit information for headers.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitInfo {
    /// Maximum requests allowed in the window.
    pub limit: u32,
    /// Remaining requests in the current window.
    pub remaining: u32,
    /// Seconds until the limit resets.
    pub reset_secs: u64,
}

impl RateLimitInfo {
    /// Creates info for unlimited access.
    fn unlimited() -> Self {
        Self {
            limit: u32::MAX,
            remaining: u32::MAX,
            reset_secs: 0,
        }
    }
}

/// Rate limit exceeded error response.
#[derive(Serialize)]
pub struct RateLimitExceededResponse {
    pub error: String,
    pub code: String,
    pub retry_after_secs: u64,
}

/// Axum middleware for rate limiting.
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    axum::Extension(limiter): axum::Extension<Arc<RateLimiter>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let ip = addr.ip();
    let path = request.uri().path();
    let category = EndpointCategory::from_path(path);

    match limiter.check(ip, category).await {
        Ok(info) => {
            // Request allowed - add headers and continue
            let mut response = next.run(request).await;
            add_rate_limit_headers(response.headers_mut(), &info);
            response
        }
        Err(info) => {
            // Rate limit exceeded
            rate_limit_exceeded_response(&info)
        }
    }
}

/// Alternative middleware that extracts IP from X-Forwarded-For header.
pub async fn rate_limit_middleware_forwarded(
    axum::Extension(limiter): axum::Extension<Arc<RateLimiter>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Try to get IP from X-Forwarded-For, fall back to direct connection
    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or_else(|| {
            // Fallback to localhost if no IP can be determined
            "127.0.0.1".parse().unwrap()
        });

    let path = request.uri().path();
    let category = EndpointCategory::from_path(path);

    match limiter.check(ip, category).await {
        Ok(info) => {
            let mut response = next.run(request).await;
            add_rate_limit_headers(response.headers_mut(), &info);
            response
        }
        Err(info) => rate_limit_exceeded_response(&info),
    }
}

/// Adds rate limit headers to the response.
fn add_rate_limit_headers(headers: &mut axum::http::HeaderMap, info: &RateLimitInfo) {
    // Only add headers for limited responses
    if info.limit != u32::MAX {
        if let Ok(v) = HeaderValue::from_str(&info.limit.to_string()) {
            headers.insert("X-RateLimit-Limit", v);
        }
        if let Ok(v) = HeaderValue::from_str(&info.remaining.to_string()) {
            headers.insert("X-RateLimit-Remaining", v);
        }
        if let Ok(v) = HeaderValue::from_str(&info.reset_secs.to_string()) {
            headers.insert("X-RateLimit-Reset", v);
        }
    }
}

/// Creates a 429 response for rate limit exceeded.
fn rate_limit_exceeded_response(info: &RateLimitInfo) -> Response {
    let body = RateLimitExceededResponse {
        error: "Rate limit exceeded".to_string(),
        code: "RATE_LIMIT_EXCEEDED".to_string(),
        retry_after_secs: info.reset_secs,
    };

    let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();

    // Add retry-after header
    if let Ok(v) = HeaderValue::from_str(&info.reset_secs.to_string()) {
        response.headers_mut().insert("Retry-After", v);
    }

    add_rate_limit_headers(response.headers_mut(), info);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_token_bucket_basic() {
        let mut bucket = TokenBucket::new(60, 1.0); // 1 per second, no burst

        // Should have 60 tokens initially (burst = rpm * multiplier)
        assert!(bucket.tokens >= 59.0);

        // Consume should succeed
        assert!(bucket.try_consume());
        assert!(bucket.remaining() >= 58);
    }

    #[test]
    fn test_token_bucket_exhaustion() {
        let mut bucket = TokenBucket::new(2, 1.0); // 2 per minute, no burst

        // Should have 2 tokens initially
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());

        // Third should fail
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_endpoint_category_detection() {
        assert_eq!(
            EndpointCategory::from_path("/api/v1/health"),
            EndpointCategory::General
        );
        assert_eq!(
            EndpointCategory::from_path("/api/v1/transcribe"),
            EndpointCategory::Heavy
        );
        assert_eq!(
            EndpointCategory::from_path("/api/v1/models/download"),
            EndpointCategory::Heavy
        );
        assert_eq!(
            EndpointCategory::from_path("/api/v1/stt/stream"),
            EndpointCategory::WebSocket
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_requests() {
        let config = RateLimitConfig {
            enabled: true,
            default_rpm: 100,
            heavy_rpm: 20,
            websocket_rpm: 10,
            burst_multiplier: 1.0,
            window_secs: 60,
            cleanup_interval_secs: 300,
        };

        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First request should succeed
        let result = limiter.check(ip, EndpointCategory::General).await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.limit, 100);
        assert!(info.remaining >= 98);
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_excess() {
        let config = RateLimitConfig {
            enabled: true,
            default_rpm: 2,
            heavy_rpm: 1,
            websocket_rpm: 1,
            burst_multiplier: 1.0,
            window_secs: 60,
            cleanup_interval_secs: 300,
        };

        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First two should succeed
        assert!(limiter.check(ip, EndpointCategory::General).await.is_ok());
        assert!(limiter.check(ip, EndpointCategory::General).await.is_ok());

        // Third should fail
        let result = limiter.check(ip, EndpointCategory::General).await;
        assert!(result.is_err());

        let info = result.unwrap_err();
        assert_eq!(info.remaining, 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = RateLimitConfig::disabled();
        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // All requests should succeed when disabled
        for _ in 0..100 {
            assert!(limiter.check(ip, EndpointCategory::General).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_per_ip() {
        let config = RateLimitConfig {
            enabled: true,
            default_rpm: 2,
            heavy_rpm: 1,
            websocket_rpm: 1,
            burst_multiplier: 1.0,
            window_secs: 60,
            cleanup_interval_secs: 300,
        };

        let limiter = RateLimiter::new(config);
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        // Exhaust IP1
        assert!(limiter.check(ip1, EndpointCategory::General).await.is_ok());
        assert!(limiter.check(ip1, EndpointCategory::General).await.is_ok());
        assert!(limiter.check(ip1, EndpointCategory::General).await.is_err());

        // IP2 should still work
        assert!(limiter.check(ip2, EndpointCategory::General).await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_per_category() {
        let config = RateLimitConfig {
            enabled: true,
            default_rpm: 10,
            heavy_rpm: 2,
            websocket_rpm: 1,
            burst_multiplier: 1.0,
            window_secs: 60,
            cleanup_interval_secs: 300,
        };

        let limiter = RateLimiter::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Exhaust heavy endpoint limit
        assert!(limiter.check(ip, EndpointCategory::Heavy).await.is_ok());
        assert!(limiter.check(ip, EndpointCategory::Heavy).await.is_ok());
        assert!(limiter.check(ip, EndpointCategory::Heavy).await.is_err());

        // General endpoints should still work
        assert!(limiter.check(ip, EndpointCategory::General).await.is_ok());
    }
}
