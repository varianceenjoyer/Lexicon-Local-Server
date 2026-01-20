//! Integration tests for the Lexicon HTTP server.
//!
//! These tests verify the server's REST API endpoints and WebSocket handlers.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;

/// Helper to create a test server router with unique temp directory.
async fn test_app() -> axum::Router {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Use atomic counter to create unique directories for each test
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();

    let temp_dir = std::env::temp_dir()
        .join(format!("lexicon-server-test-{test_id}-{thread_id:?}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .without_rate_limit(); // Disable rate limiting for tests

    let server = Server::new(server_config).expect("Failed to create server");

    // Initialize the database for storage tests
    server.state().init_database().await.expect("Failed to init database");

    server.router()
}

// =============================================================================
// Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_health_check() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
}

// =============================================================================
// Models API Tests
// =============================================================================

#[tokio::test]
async fn test_list_models() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.is_array());
}

#[tokio::test]
async fn test_model_info_not_downloaded() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/nonexistent-model")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // The API returns 200 with model info for any model ID, indicating whether it's downloaded
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["id"], "nonexistent-model");
    assert_eq!(json["downloaded"], false);
}

// =============================================================================
// Sessions API Tests
// =============================================================================

#[tokio::test]
async fn test_list_sessions_empty() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return OK even if database not initialized (service unavailable)
    // or empty array if database is initialized
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn test_get_session_not_found() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return NOT_FOUND or SERVICE_UNAVAILABLE (if db not init)
    assert!(
        response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn test_get_session_invalid_id() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/not-a-number")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_export_session_invalid_format() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/1/export?format=invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// =============================================================================
// Transcription API Tests
// =============================================================================

#[tokio::test]
async fn test_transcribe_missing_audio() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transcribe")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error for missing required 'audio' field
    // Axum returns 422 UNPROCESSABLE_ENTITY for JSON parse errors (missing required fields)
    // Or SERVICE_UNAVAILABLE if STT feature is not enabled
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_transcribe_invalid_json() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transcribe")
                .header("content-type", "application/json")
                .body(Body::from("not json"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 Unprocessable Entity for JSON parse errors
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

// =============================================================================
// CORS Tests
// =============================================================================

#[tokio::test]
async fn test_cors_preflight() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/models")
                .header("origin", "http://localhost:3000")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // CORS preflight should succeed
    assert!(response.status().is_success() || response.status() == StatusCode::NO_CONTENT);
}

// =============================================================================
// Error Response Format Tests
// =============================================================================

#[tokio::test]
async fn test_error_response_format() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/invalid-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Error responses should have standard format
    assert!(json.get("error").is_some() || json.get("message").is_some());
}

// =============================================================================
// Route Not Found Tests
// =============================================================================

#[tokio::test]
async fn test_unknown_route() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/unknown-endpoint")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// =============================================================================
// Server Configuration Tests
// =============================================================================

#[test]
fn test_server_config_default() {
    use lexicon_server::ServerConfig;

    let config = ServerConfig::default();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8765);
    assert!(config.cors_permissive);
    assert!(config.static_dir.is_none());
}

#[test]
fn test_server_config_builder() {
    use lexicon_server::ServerConfig;
    use std::path::PathBuf;

    let config = ServerConfig::new("0.0.0.0", 9000)
        .with_static_dir(PathBuf::from("/tmp/static"))
        .with_strict_cors();

    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 9000);
    assert!(!config.cors_permissive);
    assert_eq!(config.static_dir, Some(PathBuf::from("/tmp/static")));
}

#[test]
fn test_server_creation() {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-server-creation-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .without_rate_limit();

    let server = Server::new(server_config);
    assert!(server.is_ok());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// =============================================================================
// HTTP Method Tests
// =============================================================================

#[tokio::test]
async fn test_health_check_head() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // HEAD should return same status as GET
    assert!(response.status().is_success() || response.status() == StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_models_post_not_allowed() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // POST to /models should be method not allowed
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_sessions_delete_invalid_id() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/sessions/not-a-number")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// =============================================================================
// Content-Type Tests
// =============================================================================

#[tokio::test]
async fn test_health_returns_json() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    assert!(content_type.contains("application/json"));
}

#[tokio::test]
async fn test_models_returns_json() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    assert!(content_type.contains("application/json"));
}

// =============================================================================
// Export Format Tests
// =============================================================================

#[tokio::test]
async fn test_export_srt_format() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/1/export?format=srt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Even if session doesn't exist, we should get NOT_FOUND, not BAD_REQUEST
    assert!(
        response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn test_export_unsupported_format() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/1/export?format=txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 'txt' is not a supported format (only 'srt' and 'json' are supported)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_export_json_format() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/1/export?format=json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
    );
}

// =============================================================================
// Query Parameter Tests
// =============================================================================

#[tokio::test]
async fn test_export_missing_format() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions/1/export")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should default to SRT or return an error, not crash
    assert!(
        response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::SERVICE_UNAVAILABLE
            || response.status() == StatusCode::BAD_REQUEST
    );
}

// =============================================================================
// App State Tests
// =============================================================================

#[tokio::test]
async fn test_app_state_access() {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-state-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .without_rate_limit();

    let server = Server::new(server_config).expect("Failed to create server");

    // Verify we can access state
    let state = server.state();
    assert_eq!(state.config.data_dir, temp_dir);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
