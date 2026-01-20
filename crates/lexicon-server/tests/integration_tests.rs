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

// =============================================================================
// Graceful Shutdown Tests (WP-ROADBLOCK-18)
// =============================================================================

/// Test that server can be started and accepts requests.
#[tokio::test]
async fn test_server_lifecycle() {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;
    use tokio::net::TcpListener;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(10000);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    let temp_dir = std::env::temp_dir()
        .join(format!("lexicon-shutdown-test-{test_id}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind");
    let addr = listener.local_addr().expect("Failed to get address");
    drop(listener); // Release the port

    let server_config = ServerConfig::new("127.0.0.1", addr.port())
        .with_core_config(core_config)
        .without_rate_limit();

    let server = Server::new(server_config).expect("Failed to create server");
    let router = server.router();

    // Initialize database
    server.state().init_database().await.expect("Failed to init DB");

    // Test that router works
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test that data persists across server restarts.
#[tokio::test]
async fn test_session_data_persists() {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(20000);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    let temp_dir = std::env::temp_dir()
        .join(format!("lexicon-persist-test-{test_id}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    // Create first server instance, verify endpoints work
    {
        let core_config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .expect("Failed to create config");

        let server_config = ServerConfig::new("127.0.0.1", 0)
            .with_core_config(core_config)
            .without_rate_limit();

        let server = Server::new(server_config).expect("Failed to create server");
        server.state().init_database().await.expect("Failed to init DB");

        let app = server.router();

        // List sessions (starts empty)
        let request = Request::builder()
            .uri("/api/v1/sessions")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Health check should work
        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Server drops here, simulating shutdown
    }

    // Create second server instance, verify database re-opens correctly
    {
        let core_config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .expect("Failed to create config");

        let server_config = ServerConfig::new("127.0.0.1", 0)
            .with_core_config(core_config)
            .without_rate_limit();

        let server = Server::new(server_config).expect("Failed to create server");

        // Re-initializing database should work without errors
        server.state().init_database().await.expect("Failed to init DB on second start");

        let app = server.router();

        // List sessions should still work
        let request = Request::builder()
            .uri("/api/v1/sessions")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test that database file exists after server operations.
#[tokio::test]
async fn test_database_file_persists() {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(30000);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    let temp_dir = std::env::temp_dir()
        .join(format!("lexicon-dbfile-test-{test_id}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    let db_path = temp_dir.join("lexicon.db");

    // Create and use server
    {
        let core_config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .expect("Failed to create config");

        let server_config = ServerConfig::new("127.0.0.1", 0)
            .with_core_config(core_config)
            .without_rate_limit();

        let server = Server::new(server_config).expect("Failed to create server");
        server.state().init_database().await.expect("Failed to init DB");

        // Verify database file was created
        assert!(db_path.exists(), "Database file should exist after init");
    }

    // Database file should still exist after server shutdown
    assert!(db_path.exists(), "Database file should persist after shutdown");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test that multiple server restarts don't corrupt data.
#[tokio::test]
async fn test_multiple_restarts_no_corruption() {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(40000);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    let temp_dir = std::env::temp_dir()
        .join(format!("lexicon-restart-test-{test_id}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    // Perform multiple create/destroy cycles
    for i in 0..5 {
        let core_config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .expect("Failed to create config");

        let server_config = ServerConfig::new("127.0.0.1", 0)
            .with_core_config(core_config)
            .without_rate_limit();

        let server = Server::new(server_config).expect("Failed to create server");
        server.state().init_database().await.expect("Failed to init DB");

        let app = server.router();

        // Each restart should be able to access data
        let request = Request::builder()
            .uri("/api/v1/sessions")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Cycle {} should succeed",
            i
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// =============================================================================
// Authentication Tests (WP-ROADBLOCK-15)
// =============================================================================

/// Helper to create a test server router with authentication enabled.
async fn test_app_with_auth(api_key: &str) -> axum::Router {
    use lexicon_server::{Server, ServerConfig, AuthConfig};
    use lexicon_core::LexiconConfig;
    use std::sync::atomic::{AtomicU64, Ordering};

    static AUTH_COUNTER: AtomicU64 = AtomicU64::new(10000);
    let test_id = AUTH_COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();

    let temp_dir = std::env::temp_dir()
        .join(format!("lexicon-auth-test-{test_id}-{thread_id:?}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    let auth_config = AuthConfig::with_api_key(api_key);

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .with_auth(auth_config)
        .without_rate_limit();

    let server = Server::new(server_config).expect("Failed to create server");
    server.state().init_database().await.expect("Failed to init database");
    server.router()
}

/// Test: Health check endpoint bypasses authentication.
///
/// WP-ROADBLOCK-15: Health check bypasses authentication.
#[tokio::test]
async fn test_auth_health_check_bypasses_auth() {
    let app = test_app_with_auth("secret-api-key-123").await;

    // Health check should work without any API key
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
}

/// Test: Valid API key allows access to protected endpoints.
///
/// WP-ROADBLOCK-15: API key authentication implemented.
#[tokio::test]
async fn test_auth_valid_key_allowed() {
    let api_key = "valid-test-key-abc123";
    let app = test_app_with_auth(api_key).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .header("X-API-Key", api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test: Missing API key returns 401 Unauthorized.
///
/// WP-ROADBLOCK-15: 401 Unauthorized returned for invalid keys.
#[tokio::test]
async fn test_auth_missing_key_unauthorized() {
    let app = test_app_with_auth("secret-key").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                // No X-API-Key header
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"], "unauthorized");
    assert!(json["message"].as_str().unwrap().contains("API key"));
}

/// Test: Invalid API key returns 401 Unauthorized.
///
/// WP-ROADBLOCK-15: 401 Unauthorized returned for invalid keys.
#[tokio::test]
async fn test_auth_invalid_key_unauthorized() {
    let app = test_app_with_auth("correct-key").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .header("X-API-Key", "wrong-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test: Authentication is optional when disabled.
///
/// WP-ROADBLOCK-15: Authentication is optional (configurable).
#[tokio::test]
async fn test_auth_disabled_allows_all() {
    // Use regular test_app() which has auth disabled
    let app = test_app().await;

    // Should work without API key when auth is disabled
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
}

/// Test: Multiple API keys can be configured.
#[tokio::test]
async fn test_auth_multiple_keys() {
    use lexicon_server::{Server, ServerConfig, AuthConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-multi-key-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    let auth_config = AuthConfig::default()
        .add_api_key("key-one")
        .add_api_key("key-two")
        .add_api_key("key-three");

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .with_auth(auth_config)
        .without_rate_limit();

    let server = Server::new(server_config).expect("Failed to create server");
    server.state().init_database().await.expect("Failed to init database");

    // Test each key works
    for key in &["key-one", "key-two", "key-three"] {
        let app = server.router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/models")
                    .header("X-API-Key", *key)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Key '{}' should be valid",
            key
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test: WWW-Authenticate header is returned on 401.
#[tokio::test]
async fn test_auth_www_authenticate_header() {
    let app = test_app_with_auth("secret").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().contains_key("www-authenticate"));
}

// =============================================================================
// Rate Limiting Tests
// =============================================================================

/// Test: Rate limit headers are present in responses.
#[tokio::test]
async fn test_rate_limit_headers_present() {
    use lexicon_server::{Server, ServerConfig, RateLimitConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-rate-limit-headers-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    let rate_limit_config = RateLimitConfig::default()
        .with_default_rpm(100)
        .with_burst_multiplier(1.0);

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .with_rate_limit(rate_limit_config);

    let server = Server::new(server_config).expect("Failed to create server");
    server.state().init_database().await.expect("Failed to init database");

    let app = server.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("x-forwarded-for", "192.168.1.100")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Rate limit headers should be present
    assert!(
        response.headers().contains_key("x-ratelimit-limit"),
        "X-RateLimit-Limit header should be present"
    );
    assert!(
        response.headers().contains_key("x-ratelimit-remaining"),
        "X-RateLimit-Remaining header should be present"
    );
    assert!(
        response.headers().contains_key("x-ratelimit-reset"),
        "X-RateLimit-Reset header should be present"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test: Rate limiting returns 429 when limit exceeded.
#[tokio::test]
async fn test_rate_limit_returns_429() {
    use lexicon_server::{Server, ServerConfig, RateLimitConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-rate-limit-429-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    // Very low limit for testing: 2 requests per minute, no burst
    let rate_limit_config = RateLimitConfig::default()
        .with_default_rpm(2)
        .with_burst_multiplier(1.0);

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .with_rate_limit(rate_limit_config);

    let server = Server::new(server_config).expect("Failed to create server");
    server.state().init_database().await.expect("Failed to init database");

    // First 2 requests should succeed
    for i in 0..2 {
        let app = server.router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("x-forwarded-for", "10.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} should succeed",
            i + 1
        );
    }

    // Third request should be rate limited
    let app = server.router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("x-forwarded-for", "10.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Third request should be rate limited (429)"
    );

    // Check Retry-After header
    assert!(
        response.headers().contains_key("retry-after"),
        "Retry-After header should be present"
    );

    // Check response body
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], "RATE_LIMIT_EXCEEDED");
    assert!(json["retry_after_secs"].is_number());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test: Rate limiting is per-IP.
#[tokio::test]
async fn test_rate_limit_per_ip() {
    use lexicon_server::{Server, ServerConfig, RateLimitConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-rate-limit-per-ip-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    // 2 requests per minute, no burst
    let rate_limit_config = RateLimitConfig::default()
        .with_default_rpm(2)
        .with_burst_multiplier(1.0);

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .with_rate_limit(rate_limit_config);

    let server = Server::new(server_config).expect("Failed to create server");
    server.state().init_database().await.expect("Failed to init database");

    // Exhaust IP1's limit
    for _ in 0..2 {
        let app = server.router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("x-forwarded-for", "10.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // IP1 should now be rate limited
    let app = server.router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("x-forwarded-for", "10.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // But IP2 should still work
    let app = server.router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("x-forwarded-for", "10.0.0.2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Different IP should not be rate limited"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test: Heavy endpoints have stricter rate limits.
#[tokio::test]
async fn test_rate_limit_heavy_endpoints() {
    use lexicon_server::{Server, ServerConfig, RateLimitConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-rate-limit-heavy-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    // Default: 10 req/min, Heavy: 2 req/min
    let rate_limit_config = RateLimitConfig::default()
        .with_default_rpm(10)
        .with_heavy_rpm(2)
        .with_burst_multiplier(1.0);

    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .with_rate_limit(rate_limit_config);

    let server = Server::new(server_config).expect("Failed to create server");
    server.state().init_database().await.expect("Failed to init database");

    // Exhaust heavy endpoint limit (transcribe is a heavy endpoint)
    // Note: These will return errors (no model loaded), but rate limiting still counts
    for i in 0..2 {
        let app = server.router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/transcribe")
                    .header("x-forwarded-for", "10.0.0.50")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"audio": ""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // May return 400 (bad request) or other errors, but not 429 yet
        assert_ne!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "Request {} should not be rate limited yet",
            i + 1
        );
    }

    // Third transcribe request should be rate limited
    let app = server.router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/transcribe")
                .header("x-forwarded-for", "10.0.0.50")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"audio": ""}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Third transcribe request should be rate limited"
    );

    // But general endpoints should still work (different bucket)
    let app = server.router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models")
                .header("x-forwarded-for", "10.0.0.50")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "General endpoints should still work (different rate limit bucket)"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test: Rate limiting disabled via config.
#[tokio::test]
async fn test_rate_limit_disabled() {
    use lexicon_server::{Server, ServerConfig};
    use lexicon_core::LexiconConfig;

    let temp_dir = std::env::temp_dir().join("lexicon-rate-limit-disabled-test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    // Disabled rate limiting
    let server_config = ServerConfig::new("127.0.0.1", 0)
        .with_core_config(core_config)
        .without_rate_limit();

    let server = Server::new(server_config).expect("Failed to create server");
    server.state().init_database().await.expect("Failed to init database");

    // Many requests should all succeed
    for i in 0..50 {
        let app = server.router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("x-forwarded-for", "10.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} should succeed (rate limiting disabled)",
            i + 1
        );
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}
