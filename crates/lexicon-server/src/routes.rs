//! HTTP route definitions for the Lexicon server.
//!
//! This module contains route handlers for:
//! - WebSocket STT streaming
//! - WebSocket TTS streaming
//! - REST endpoints for models, sessions, and export

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::AppState;

// ============================================================================
// API Router
// ============================================================================

/// Creates the API router with all v1 routes.
pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Models
        .route("/models", get(list_models))
        .route("/models/download", post(download_model))
        .route("/models/:model_id", get(get_model_info))
        .route("/models/:model_id", delete(delete_model))
        // Transcription
        .route("/transcribe", post(transcribe_batch))
        // Sessions
        .route("/sessions", get(list_sessions))
        .route("/sessions/:session_id", get(get_session))
        .route("/sessions/:session_id", delete(delete_session))
        .route("/sessions/:session_id/export", get(export_session))
        // WebSocket streaming
        .route("/stt/stream", get(ws_stt_stream))
        .route("/tts/stream", get(ws_tts_stream))
}

// ============================================================================
// Health Check
// ============================================================================

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stt_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_ready: Option<bool>,
}

/// Health check endpoint.
pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    #[cfg(feature = "stt")]
    let stt_ready = Some(state.transcriber.read().await.is_some());
    #[cfg(not(feature = "stt"))]
    let stt_ready = None;

    #[cfg(feature = "tts")]
    let tts_ready = Some(state.tts.read().await.is_some());
    #[cfg(not(feature = "tts"))]
    let tts_ready = None;

    #[cfg(feature = "storage")]
    let db_ready = Some(state.database.lock().await.is_some());
    #[cfg(not(feature = "storage"))]
    let db_ready = None;

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        stt_ready,
        tts_ready,
        db_ready,
    })
}

// ============================================================================
// Error Response
// ============================================================================

/// API error response.
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ErrorResponse {
    fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            details: None,
        }
    }

    #[allow(dead_code)]
    fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// API error type.
pub struct ApiError {
    status: StatusCode,
    response: ErrorResponse,
}

impl ApiError {
    fn bad_request(error: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            response: ErrorResponse::new(error, "BAD_REQUEST"),
        }
    }

    fn not_found(error: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            response: ErrorResponse::new(error, "NOT_FOUND"),
        }
    }

    fn internal(error: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            response: ErrorResponse::new(error, "INTERNAL_ERROR"),
        }
    }

    fn service_unavailable(error: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            response: ErrorResponse::new(error, "SERVICE_UNAVAILABLE"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.response)).into_response()
    }
}

// ============================================================================
// Models Endpoints
// ============================================================================

/// Model information response.
#[derive(Serialize)]
pub struct ModelResponse {
    pub id: String,
    pub name: String,
    pub size_bytes: Option<u64>,
    pub downloaded: bool,
    pub path: Option<String>,
}

/// List available models.
pub async fn list_models(State(state): State<Arc<AppState>>) -> Json<Vec<ModelResponse>> {
    let models = vec![
        ("whisper-tiny", "Whisper Tiny", 75_000_000u64),
        ("whisper-base", "Whisper Base", 142_000_000),
        ("whisper-small", "Whisper Small", 466_000_000),
        ("whisper-medium", "Whisper Medium", 1_500_000_000),
        ("whisper-large", "Whisper Large", 2_900_000_000),
    ];

    let model_dir = &state.config.model_dir;
    let responses: Vec<ModelResponse> = models
        .iter()
        .map(|(id, name, size)| {
            let model_path = model_dir.join(format!("{id}.bin"));
            let downloaded = model_path.exists();
            ModelResponse {
                id: id.to_string(),
                name: name.to_string(),
                size_bytes: Some(*size),
                downloaded,
                path: if downloaded {
                    Some(model_path.display().to_string())
                } else {
                    None
                },
            }
        })
        .collect();

    Json(responses)
}

/// Get model information.
pub async fn get_model_info(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Result<Json<ModelResponse>, ApiError> {
    let model_path = state.config.model_dir.join(format!("{model_id}.bin"));
    let downloaded = model_path.exists();

    let size = if downloaded {
        std::fs::metadata(&model_path).ok().map(|m| m.len())
    } else {
        None
    };

    Ok(Json(ModelResponse {
        id: model_id.clone(),
        name: model_id,
        size_bytes: size,
        downloaded,
        path: if downloaded {
            Some(model_path.display().to_string())
        } else {
            None
        },
    }))
}

/// Download model request.
#[derive(Deserialize)]
pub struct DownloadModelRequest {
    pub model_id: String,
}

/// Download model response.
#[derive(Serialize)]
pub struct DownloadModelResponse {
    pub status: String,
    pub model_id: String,
    pub message: String,
}

/// Download a model.
pub async fn download_model(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DownloadModelRequest>,
) -> Result<Json<DownloadModelResponse>, ApiError> {
    let model_id = request.model_id;

    // Check if already downloaded
    let model_path = state.config.model_dir.join(format!("{model_id}.bin"));
    if model_path.exists() {
        return Ok(Json(DownloadModelResponse {
            status: "exists".to_string(),
            model_id,
            message: "Model already downloaded".to_string(),
        }));
    }

    // Initiate download
    let mut registry = state.registry.write().await;

    // Fetch manifest first
    registry.fetch_manifest().await.map_err(|e| {
        error!("Failed to fetch manifest: {}", e);
        ApiError::internal(format!("Failed to fetch model manifest: {e}"))
    })?;

    // Download model
    registry
        .download_model(&model_id, |progress| {
            debug!(
                "Download progress: {}/{} bytes",
                progress.downloaded_bytes, progress.total_bytes
            );
        })
        .await
        .map_err(|e| {
            error!("Failed to download model: {}", e);
            ApiError::internal(format!("Failed to download model: {e}"))
        })?;

    info!("Model downloaded: {}", model_id);

    Ok(Json(DownloadModelResponse {
        status: "complete".to_string(),
        model_id,
        message: "Model downloaded successfully".to_string(),
    }))
}

/// Delete a model.
pub async fn delete_model(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let model_path = state.config.model_dir.join(format!("{model_id}.bin"));

    if !model_path.exists() {
        return Err(ApiError::not_found(format!("Model not found: {model_id}")));
    }

    std::fs::remove_file(&model_path).map_err(|e| {
        error!("Failed to delete model: {}", e);
        ApiError::internal(format!("Failed to delete model: {e}"))
    })?;

    info!("Model deleted: {}", model_id);

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "model_id": model_id
    })))
}

// ============================================================================
// Transcription Endpoints
// ============================================================================

/// Batch transcription request.
#[derive(Deserialize)]
pub struct TranscribeRequest {
    /// Base64-encoded audio data (16kHz mono f32 PCM).
    pub audio: String,
    /// Language code (optional, auto-detect if not specified).
    pub language: Option<String>,
    /// Translate to English.
    #[serde(default)]
    pub translate: bool,
    /// Model to use (default: base).
    pub model: Option<String>,
}

/// Transcription segment response.
#[derive(Serialize)]
pub struct SegmentResponse {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f32>,
}

/// Batch transcription response.
#[derive(Serialize)]
pub struct TranscribeResponse {
    pub segments: Vec<SegmentResponse>,
    pub detected_language: Option<String>,
    pub duration_ms: i64,
    pub processing_time_ms: i64,
}

/// Batch transcription endpoint.
#[cfg(feature = "stt")]
pub async fn transcribe_batch(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TranscribeRequest>,
) -> Result<Json<TranscribeResponse>, ApiError> {
    use base64::Engine;
    use lexicon_core::stt::{TranscriptionConfig, Transcriber};

    // Decode audio
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.audio)
        .map_err(|e| ApiError::bad_request(format!("Invalid base64 audio: {e}")))?;

    // Convert to f32 samples
    let samples: Vec<f32> = audio_bytes
        .chunks(4)
        .filter_map(|chunk| {
            if chunk.len() == 4 {
                Some(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            } else {
                None
            }
        })
        .collect();

    if samples.is_empty() {
        return Err(ApiError::bad_request("Empty audio data"));
    }

    // Get or load transcriber
    let model_name = request.model.as_deref().unwrap_or("base");

    // Check if model needs to be loaded
    {
        let transcriber = state.transcriber.read().await;
        if transcriber.is_none() {
            drop(transcriber);
            state.load_stt_model(model_name).await.map_err(|e| {
                error!("Failed to load model: {}", e);
                ApiError::service_unavailable(format!("Model not available: {e}"))
            })?;
        }
    }

    // Configure transcription
    let mut config = TranscriptionConfig::new();
    if let Some(lang) = request.language {
        config = config.with_language(lang);
    }
    if request.translate {
        config = config.with_translation();
    }

    // Transcribe
    let transcriber = state.transcriber.read().await;
    let transcriber = transcriber
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Transcriber not loaded"))?;

    let result = transcriber.transcribe(&samples, &config).map_err(|e| {
        error!("Transcription failed: {}", e);
        ApiError::internal(format!("Transcription failed: {e}"))
    })?;

    // Convert to response
    let segments: Vec<SegmentResponse> = result
        .segments
        .iter()
        .map(|s| SegmentResponse {
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            text: s.text.clone(),
            confidence: s.confidence,
        })
        .collect();

    Ok(Json(TranscribeResponse {
        segments,
        detected_language: result.detected_language,
        duration_ms: result.audio_duration_ms,
        processing_time_ms: result.processing_time_ms,
    }))
}

#[cfg(not(feature = "stt"))]
pub async fn transcribe_batch() -> ApiError {
    ApiError::service_unavailable("STT feature not enabled")
}

// ============================================================================
// Sessions Endpoints
// ============================================================================

/// Session response.
#[derive(Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub name: Option<String>,
    pub language: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Convert milliseconds since epoch to RFC3339 string.
fn ms_to_rfc3339(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string())
}

/// List sessions.
#[cfg(feature = "storage")]
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SessionResponse>>, ApiError> {
    #[allow(unused_imports)]
    use lexicon_core::storage::SessionId;

    let db = state.database.lock().await;
    let db = db
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Database not initialized"))?;

    let sessions = db.list_sessions().map_err(|e| {
        error!("Failed to list sessions: {}", e);
        ApiError::internal(format!("Database error: {e}"))
    })?;

    let responses: Vec<SessionResponse> = sessions
        .iter()
        .map(|s| SessionResponse {
            id: s.id.0.to_string(),
            name: s.name.clone(),
            language: s.language.clone(),
            created_at: ms_to_rfc3339(s.created_at),
            updated_at: ms_to_rfc3339(s.updated_at),
        })
        .collect();

    Ok(Json(responses))
}

#[cfg(not(feature = "storage"))]
pub async fn list_sessions() -> ApiError {
    ApiError::service_unavailable("Storage feature not enabled")
}

/// Get session details.
#[cfg(feature = "storage")]
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use lexicon_core::storage::SessionId;

    let db = state.database.lock().await;
    let db = db
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Database not initialized"))?;

    let session_id_num: i64 = session_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid session ID"))?;
    let session_id = SessionId(session_id_num);

    let session = db.get_session(session_id).map_err(|e| {
        error!("Failed to get session: {}", e);
        ApiError::not_found(format!("Session not found: {e}"))
    })?;

    let segments = db.get_session_segments(session_id).map_err(|e| {
        error!("Failed to get segments: {}", e);
        ApiError::internal(format!("Database error: {e}"))
    })?;

    // Calculate duration from segments
    let duration_ms = segments.iter().map(|s| s.end_ms).max();

    Ok(Json(serde_json::json!({
        "id": session.id.0,
        "name": session.name,
        "language": session.language,
        "created_at": ms_to_rfc3339(session.created_at),
        "updated_at": ms_to_rfc3339(session.updated_at),
        "duration_ms": duration_ms,
        "segment_count": segments.len(),
        "segments": segments.iter().map(|s| serde_json::json!({
            "id": s.id.0,
            "start_ms": s.start_ms,
            "end_ms": s.end_ms,
            "text": s.text,
            "confidence": s.confidence,
            "language": s.language,
            "speaker_id": s.speaker_id
        })).collect::<Vec<_>>()
    })))
}

#[cfg(not(feature = "storage"))]
pub async fn get_session(Path(_): Path<String>) -> ApiError {
    ApiError::service_unavailable("Storage feature not enabled")
}

/// Delete session.
#[cfg(feature = "storage")]
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use lexicon_core::storage::SessionId;

    let db = state.database.lock().await;
    let db = db
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Database not initialized"))?;

    let session_id_num: i64 = session_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid session ID"))?;

    db.delete_session(SessionId(session_id_num)).map_err(|e| {
        error!("Failed to delete session: {}", e);
        ApiError::internal(format!("Failed to delete session: {e}"))
    })?;

    info!("Session deleted: {}", session_id_num);

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "session_id": session_id_num
    })))
}

#[cfg(not(feature = "storage"))]
pub async fn delete_session(Path(_): Path<String>) -> ApiError {
    ApiError::service_unavailable("Storage feature not enabled")
}

/// Export query parameters.
#[derive(Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

/// Export session.
#[cfg(feature = "storage")]
pub async fn export_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    use lexicon_core::storage::SessionId;

    let db = state.database.lock().await;
    let db = db
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Database not initialized"))?;

    let session_id_num: i64 = session_id
        .parse()
        .map_err(|_| ApiError::bad_request("Invalid session ID"))?;
    let session_id = SessionId(session_id_num);

    let format = query.format.as_deref().unwrap_or("srt");

    match format {
        "srt" => {
            let srt = db.export_srt(session_id).map_err(|e| {
                error!("Failed to export SRT: {}", e);
                // Check if it's a NotFound error
                if matches!(e, lexicon_core::error::DatabaseError::NotFound { .. }) {
                    ApiError::not_found(format!("Session not found: {e}"))
                } else {
                    ApiError::internal(format!("Export failed: {e}"))
                }
            })?;

            Ok((
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; charset=utf-8",
                )],
                srt,
            )
                .into_response())
        }
        "json" => {
            let session = db.get_session(session_id).map_err(|e| {
                error!("Failed to get session: {}", e);
                ApiError::not_found(format!("Session not found: {e}"))
            })?;

            let segments = db.get_session_segments(session_id).map_err(|e| {
                error!("Failed to get segments: {}", e);
                ApiError::internal(format!("Database error: {e}"))
            })?;

            let json = serde_json::json!({
                "session_id": session.id.0,
                "name": session.name,
                "language": session.language,
                "created_at": ms_to_rfc3339(session.created_at),
                "updated_at": ms_to_rfc3339(session.updated_at),
                "segments": segments.iter().map(|s| serde_json::json!({
                    "id": s.id.0,
                    "start_ms": s.start_ms,
                    "end_ms": s.end_ms,
                    "text": s.text,
                    "confidence": s.confidence,
                    "language": s.language,
                    "speaker_id": s.speaker_id
                })).collect::<Vec<_>>()
            });

            Ok(Json(json).into_response())
        }
        _ => Err(ApiError::bad_request(format!(
            "Unsupported format: {format}. Use 'srt' or 'json'."
        ))),
    }
}

#[cfg(not(feature = "storage"))]
pub async fn export_session(Path(_): Path<String>, Query(_): Query<ExportQuery>) -> ApiError {
    ApiError::service_unavailable("Storage feature not enabled")
}

// ============================================================================
// WebSocket Endpoints
// ============================================================================

/// WebSocket STT streaming.
pub async fn ws_stt_stream(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_stt_websocket(socket, state))
}

/// Handle STT WebSocket connection.
async fn handle_stt_websocket(
    mut socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
) {
    use axum::extract::ws::Message;

    info!("STT WebSocket connection established");

    // Send ready message
    if socket
        .send(Message::Text(
            serde_json::json!({"type": "ready", "message": "STT stream ready"}).to_string(),
        ))
        .await
        .is_err()
    {
        return;
    }

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Binary(data)) => {
                // Process audio data
                #[cfg(feature = "stt")]
                {
                    // Convert binary to f32 samples
                    let samples: Vec<f32> = data
                        .chunks(4)
                        .filter_map(|chunk| {
                            if chunk.len() == 4 {
                                Some(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !samples.is_empty() {
                        // Try to transcribe
                        let transcriber = state.transcriber.read().await;
                        if let Some(ref t) = *transcriber {
                            use lexicon_core::stt::{TranscriptionConfig, Transcriber};
                            let config = TranscriptionConfig::new();

                            match t.transcribe(&samples, &config) {
                                Ok(result) => {
                                    for segment in result.segments {
                                        let msg = serde_json::json!({
                                            "type": "segment",
                                            "start_ms": segment.start_ms,
                                            "end_ms": segment.end_ms,
                                            "text": segment.text,
                                            "confidence": segment.confidence
                                        });

                                        if socket.send(Message::Text(msg.to_string())).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Transcription error: {}", e);
                                    let msg = serde_json::json!({
                                        "type": "error",
                                        "message": e.to_string()
                                    });
                                    let _ = socket.send(Message::Text(msg.to_string())).await;
                                }
                            }
                        } else {
                            let msg = serde_json::json!({
                                "type": "error",
                                "message": "No model loaded"
                            });
                            let _ = socket.send(Message::Text(msg.to_string())).await;
                        }
                    }
                }

                #[cfg(not(feature = "stt"))]
                {
                    let msg = serde_json::json!({
                        "type": "error",
                        "message": "STT not enabled"
                    });
                    let _ = socket.send(Message::Text(msg.to_string())).await;
                }
            }
            Ok(Message::Text(text)) => {
                // Handle control messages
                if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                    if cmd.get("type").and_then(|t| t.as_str()) == Some("load_model") {
                        if let Some(model) = cmd.get("model").and_then(|m| m.as_str()) {
                            #[cfg(feature = "stt")]
                            match state.load_stt_model(model).await {
                                Ok(()) => {
                                    let msg = serde_json::json!({
                                        "type": "model_loaded",
                                        "model": model
                                    });
                                    let _ = socket.send(Message::Text(msg.to_string())).await;
                                }
                                Err(e) => {
                                    let msg = serde_json::json!({
                                        "type": "error",
                                        "message": e.to_string()
                                    });
                                    let _ = socket.send(Message::Text(msg.to_string())).await;
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("STT WebSocket closed");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    info!("STT WebSocket connection closed");
}

/// WebSocket TTS streaming.
pub async fn ws_tts_stream(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_tts_websocket(socket, state))
}

/// Handle TTS WebSocket connection.
async fn handle_tts_websocket(
    mut socket: axum::extract::ws::WebSocket,
    state: Arc<AppState>,
) {
    use axum::extract::ws::Message;

    info!("TTS WebSocket connection established");

    // Send ready message
    if socket
        .send(Message::Text(
            serde_json::json!({"type": "ready", "message": "TTS stream ready"}).to_string(),
        ))
        .await
        .is_err()
    {
        return;
    }

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Parse text message
                if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                    if cmd.get("type").and_then(|t| t.as_str()) == Some("synthesize") {
                        if let Some(text_to_speak) = cmd.get("text").and_then(|t| t.as_str()) {
                            #[cfg(feature = "tts")]
                            {
                                use lexicon_core::tts::TtsConfig;

                                let tts = state.tts.read().await;
                                if let Some(ref manager) = *tts {
                                    let config = TtsConfig::new();

                                    match manager.synthesize(text_to_speak, &config) {
                                        Ok(result) => {
                                            // Send audio data as binary
                                            let audio_bytes: Vec<u8> = result
                                                .audio
                                                .samples
                                                .iter()
                                                .flat_map(|&s| s.to_le_bytes())
                                                .collect();

                                            // Send metadata first
                                            let meta = serde_json::json!({
                                                "type": "audio_start",
                                                "sample_rate": result.audio.sample_rate,
                                                "duration_ms": result.audio.duration_ms(),
                                                "engine": format!("{:?}", result.engine_used)
                                            });
                                            let _ = socket.send(Message::Text(meta.to_string())).await;

                                            // Send audio data
                                            let _ = socket.send(Message::Binary(audio_bytes)).await;

                                            // Send completion
                                            let done = serde_json::json!({
                                                "type": "audio_end"
                                            });
                                            let _ = socket.send(Message::Text(done.to_string())).await;
                                        }
                                        Err(e) => {
                                            let msg = serde_json::json!({
                                                "type": "error",
                                                "message": e.to_string()
                                            });
                                            let _ = socket.send(Message::Text(msg.to_string())).await;
                                        }
                                    }
                                } else {
                                    let msg = serde_json::json!({
                                        "type": "error",
                                        "message": "TTS not initialized"
                                    });
                                    let _ = socket.send(Message::Text(msg.to_string())).await;
                                }
                            }

                            #[cfg(not(feature = "tts"))]
                            {
                                let msg = serde_json::json!({
                                    "type": "error",
                                    "message": "TTS not enabled"
                                });
                                let _ = socket.send(Message::Text(msg.to_string())).await;
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("TTS WebSocket closed");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    info!("TTS WebSocket connection closed");
}
