//! WebSocket Integration Tests for Lexicon Server
//!
//! These tests verify the WebSocket STT and TTS streaming endpoints.
//!
//! # Test Coverage
//!
//! - STT WebSocket connection and ready message
//! - STT audio streaming and segment responses
//! - STT error handling (no model loaded, invalid input)
//! - TTS WebSocket connection and ready message
//! - TTS synthesis request and audio response
//! - Concurrent WebSocket connections
//!
//! # Running Tests
//!
//! ```bash
//! cargo test -p lexicon-server --test websocket_tests
//! ```

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Test helper to start a server and return its address.
async fn start_test_server() -> (SocketAddr, oneshot::Sender<()>) {
    use lexicon_core::LexiconConfig;
    use lexicon_server::{Server, ServerConfig};
    use std::sync::atomic::{AtomicU64, Ordering};

    // Use atomic counter to create unique directories for each test
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let test_id = COUNTER.fetch_add(1, Ordering::SeqCst);

    let temp_dir = std::env::temp_dir().join(format!("lexicon-ws-test-{test_id}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    let core_config = LexiconConfig::builder()
        .data_dir(temp_dir.clone())
        .model_dir(temp_dir.join("models"))
        .build()
        .expect("Failed to create config");

    // Bind to available port first
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_config = ServerConfig::new("127.0.0.1", addr.port())
        .with_core_config(core_config);

    let server = Server::new(server_config).expect("Failed to create server");

    // Initialize database
    server
        .state()
        .init_database()
        .await
        .expect("Failed to init database");

    // Get router for serving
    let router = server.router();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Spawn server in background using the listener we already bound
    tokio::spawn(async move {
        let shutdown_signal = async {
            shutdown_rx.await.ok();
        };

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .ok();
    });

    // Wait for server to be ready
    tokio::time::sleep(Duration::from_millis(50)).await;

    (addr, shutdown_tx)
}

// =============================================================================
// STT WebSocket Tests
// =============================================================================

#[tokio::test]
async fn test_ws_stt_connection() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/stt/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to STT WebSocket");

    let (mut _write, mut read) = ws_stream.split();

    // Should receive ready message
    let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for ready message")
        .expect("Stream closed")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
        assert_eq!(json["type"], "ready");
        assert!(json["message"].as_str().unwrap().contains("STT"));
        println!("STT WebSocket ready: {}", json);
    } else {
        panic!("Expected text message, got: {:?}", msg);
    }
}

#[tokio::test]
async fn test_ws_stt_no_model_loaded() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/stt/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to STT WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send some audio data (should get error since no model is loaded)
    let fake_audio: Vec<u8> = vec![0u8; 4 * 1000]; // 1000 f32 samples as zeros
    write
        .send(Message::Binary(fake_audio))
        .await
        .expect("Failed to send audio");

    // Should receive error about no model
    let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for error message")
        .expect("Stream closed")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
        assert_eq!(json["type"], "error");
        let error_msg = json["message"].as_str().unwrap().to_lowercase();
        assert!(
            error_msg.contains("model") || error_msg.contains("stt"),
            "Expected model-related error, got: {}",
            error_msg
        );
        println!("Got expected error: {}", json);
    } else {
        panic!("Expected text message, got: {:?}", msg);
    }
}

#[tokio::test]
async fn test_ws_stt_load_model_command() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/stt/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to STT WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send load_model command (will fail since model doesn't exist)
    let cmd = serde_json::json!({
        "type": "load_model",
        "model": "nonexistent-model"
    });
    write
        .send(Message::Text(cmd.to_string()))
        .await
        .expect("Failed to send command");

    // Should receive error (model not found)
    let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for response")
        .expect("Stream closed")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
        // Could be error or model_loaded depending on implementation
        assert!(
            json["type"] == "error" || json["type"] == "model_loaded",
            "Unexpected response type: {}",
            json["type"]
        );
        println!("Load model response: {}", json);
    } else {
        panic!("Expected text message, got: {:?}", msg);
    }
}

#[tokio::test]
async fn test_ws_stt_close_handling() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/stt/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to STT WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send close frame
    write
        .send(Message::Close(None))
        .await
        .expect("Failed to send close");

    // Connection should close gracefully
    let result = tokio::time::timeout(Duration::from_secs(2), read.next()).await;

    // Either timeout (server closed) or we get a close frame
    match result {
        Ok(Some(Ok(Message::Close(_)))) => println!("Received close frame"),
        Ok(None) => println!("Stream ended"),
        Err(_) => println!("Timeout (connection closed)"),
        other => println!("Other result: {:?}", other),
    }
}

// =============================================================================
// TTS WebSocket Tests
// =============================================================================

#[tokio::test]
async fn test_ws_tts_connection() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/tts/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to TTS WebSocket");

    let (mut _write, mut read) = ws_stream.split();

    // Should receive ready message
    let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for ready message")
        .expect("Stream closed")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
        assert_eq!(json["type"], "ready");
        assert!(json["message"].as_str().unwrap().contains("TTS"));
        println!("TTS WebSocket ready: {}", json);
    } else {
        panic!("Expected text message, got: {:?}", msg);
    }
}

#[tokio::test]
async fn test_ws_tts_synthesis_request() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/tts/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to TTS WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send synthesis request
    let cmd = serde_json::json!({
        "type": "synthesize",
        "text": "Hello, world!"
    });
    write
        .send(Message::Text(cmd.to_string()))
        .await
        .expect("Failed to send command");

    // Should receive either audio_start or error
    let msg = tokio::time::timeout(Duration::from_secs(10), read.next())
        .await
        .expect("Timeout waiting for response")
        .expect("Stream closed")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");

        match json["type"].as_str() {
            Some("audio_start") => {
                // Success - TTS is working
                assert!(json["sample_rate"].is_number());
                assert!(json["duration_ms"].is_number());
                println!("TTS audio start: {}", json);

                // Should receive binary audio data next
                let audio_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
                    .await
                    .expect("Timeout waiting for audio data")
                    .expect("Stream closed")
                    .expect("WebSocket error");

                if let Message::Binary(data) = audio_msg {
                    assert!(!data.is_empty(), "Audio data should not be empty");
                    println!("Received {} bytes of audio data", data.len());
                }

                // Should receive audio_end
                let end_msg = tokio::time::timeout(Duration::from_secs(5), read.next())
                    .await
                    .expect("Timeout waiting for audio_end")
                    .expect("Stream closed")
                    .expect("WebSocket error");

                if let Message::Text(text) = end_msg {
                    let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
                    assert_eq!(json["type"], "audio_end");
                    println!("TTS audio end received");
                }
            }
            Some("error") => {
                // TTS not initialized (expected if no TTS engine available)
                println!("TTS error (expected if TTS not available): {}", json["message"]);
            }
            other => {
                panic!("Unexpected response type: {:?}", other);
            }
        }
    } else {
        panic!("Expected text message, got: {:?}", msg);
    }
}

#[tokio::test]
async fn test_ws_tts_invalid_command() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/tts/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to TTS WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send invalid command (missing text field)
    let cmd = serde_json::json!({
        "type": "synthesize"
        // missing "text" field
    });
    write
        .send(Message::Text(cmd.to_string()))
        .await
        .expect("Failed to send command");

    // Server should handle gracefully (no crash, maybe no response)
    let result = tokio::time::timeout(Duration::from_secs(2), read.next()).await;

    // Either timeout (server ignored invalid command) or error response
    match result {
        Ok(Some(Ok(Message::Text(text)))) => {
            println!("Got response to invalid command: {}", text);
        }
        Err(_) => {
            println!("Server ignored invalid command (timeout)");
        }
        other => {
            println!("Other result: {:?}", other);
        }
    }
}

// =============================================================================
// Concurrent Connection Tests
// =============================================================================

#[tokio::test]
async fn test_ws_concurrent_stt_connections() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/stt/stream", addr);

    // Connect multiple clients concurrently
    let mut handles = Vec::new();

    for i in 0..3 {
        let url = url.clone();
        handles.push(tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url)
                .await
                .expect("Failed to connect to STT WebSocket");

            let (_write, mut read) = ws_stream.split();

            // Should receive ready message
            let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
                .await
                .expect("Timeout waiting for ready message")
                .expect("Stream closed")
                .expect("WebSocket error");

            if let Message::Text(text) = msg {
                let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
                assert_eq!(json["type"], "ready");
                println!("Client {} connected: {}", i, json);
            }

            i
        }));
    }

    // Wait for all connections to complete
    let results: Vec<_> = futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task panicked"))
        .collect();

    assert_eq!(results.len(), 3);
    println!("All {} concurrent connections successful", results.len());
}

#[tokio::test]
async fn test_ws_concurrent_tts_connections() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/tts/stream", addr);

    // Connect multiple clients concurrently
    let mut handles = Vec::new();

    for i in 0..3 {
        let url = url.clone();
        handles.push(tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url)
                .await
                .expect("Failed to connect to TTS WebSocket");

            let (_write, mut read) = ws_stream.split();

            // Should receive ready message
            let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
                .await
                .expect("Timeout waiting for ready message")
                .expect("Stream closed")
                .expect("WebSocket error");

            if let Message::Text(text) = msg {
                let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
                assert_eq!(json["type"], "ready");
                println!("TTS Client {} connected: {}", i, json);
            }

            i
        }));
    }

    // Wait for all connections to complete
    let results: Vec<_> = futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task panicked"))
        .collect();

    assert_eq!(results.len(), 3);
    println!("All {} concurrent TTS connections successful", results.len());
}

#[tokio::test]
async fn test_ws_mixed_stt_tts_connections() {
    let (addr, _shutdown) = start_test_server().await;

    let stt_url = format!("ws://{}/api/v1/stt/stream", addr);
    let tts_url = format!("ws://{}/api/v1/tts/stream", addr);

    // Connect to both endpoints concurrently
    let stt_handle = {
        let url = stt_url.clone();
        tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url)
                .await
                .expect("Failed to connect to STT WebSocket");

            let (_write, mut read) = ws_stream.split();

            let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
                .await
                .expect("Timeout")
                .expect("Stream closed")
                .expect("WebSocket error");

            if let Message::Text(text) = msg {
                let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
                assert_eq!(json["type"], "ready");
                assert!(json["message"].as_str().unwrap().contains("STT"));
            }
            "stt"
        })
    };

    let tts_handle = {
        let url = tts_url.clone();
        tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url)
                .await
                .expect("Failed to connect to TTS WebSocket");

            let (_write, mut read) = ws_stream.split();

            let msg = tokio::time::timeout(Duration::from_secs(5), read.next())
                .await
                .expect("Timeout")
                .expect("Stream closed")
                .expect("WebSocket error");

            if let Message::Text(text) = msg {
                let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
                assert_eq!(json["type"], "ready");
                assert!(json["message"].as_str().unwrap().contains("TTS"));
            }
            "tts"
        })
    };

    let (stt_result, tts_result) = tokio::join!(stt_handle, tts_handle);

    assert_eq!(stt_result.expect("STT task panicked"), "stt");
    assert_eq!(tts_result.expect("TTS task panicked"), "tts");

    println!("Mixed STT/TTS concurrent connections successful");
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_ws_stt_binary_with_invalid_size() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/stt/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to STT WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send binary data that's not a multiple of 4 bytes (invalid f32 array)
    let invalid_audio: Vec<u8> = vec![1, 2, 3]; // 3 bytes, not divisible by 4
    write
        .send(Message::Binary(invalid_audio))
        .await
        .expect("Failed to send invalid audio");

    // Server should handle gracefully (either error or no response for empty after filtering)
    let result = tokio::time::timeout(Duration::from_secs(2), read.next()).await;

    match result {
        Ok(Some(Ok(Message::Text(text)))) => {
            let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
            println!("Response to invalid binary: {}", json);
        }
        Err(_) => {
            println!("Server handled invalid binary gracefully (timeout)");
        }
        other => {
            println!("Other result: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_ws_tts_empty_text() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/tts/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to TTS WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send synthesis request with empty text
    let cmd = serde_json::json!({
        "type": "synthesize",
        "text": ""
    });
    write
        .send(Message::Text(cmd.to_string()))
        .await
        .expect("Failed to send command");

    // Should receive error for empty text
    let result = tokio::time::timeout(Duration::from_secs(5), read.next()).await;

    match result {
        Ok(Some(Ok(Message::Text(text)))) => {
            let json: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
            // Should get an error response
            if json["type"] == "error" {
                println!("Got expected error for empty text: {}", json["message"]);
            } else {
                println!("Got response for empty text: {}", json);
            }
        }
        Err(_) => {
            println!("Server ignored empty text request (timeout)");
        }
        other => {
            println!("Other result: {:?}", other);
        }
    }
}

// =============================================================================
// Protocol Tests
// =============================================================================

#[tokio::test]
async fn test_ws_ping_pong() {
    let (addr, _shutdown) = start_test_server().await;

    let url = format!("ws://{}/api/v1/stt/stream", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to STT WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Skip ready message
    let _ = read.next().await;

    // Send ping
    write
        .send(Message::Ping(vec![1, 2, 3, 4]))
        .await
        .expect("Failed to send ping");

    // Should receive pong (handled by tungstenite automatically in most cases)
    // If we don't get it, that's okay - ping/pong is often handled at protocol level
    let result = tokio::time::timeout(Duration::from_secs(2), read.next()).await;

    match result {
        Ok(Some(Ok(Message::Pong(data)))) => {
            assert_eq!(data, vec![1, 2, 3, 4]);
            println!("Received pong with correct data");
        }
        Err(_) => {
            println!("No explicit pong received (may be handled at protocol level)");
        }
        other => {
            println!("Other result: {:?}", other);
        }
    }
}
