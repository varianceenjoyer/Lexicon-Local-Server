//! Lexicon FFI - UniFFI bindings for Swift/Kotlin
//!
//! This crate provides cross-platform bindings for iOS (Swift) and Android (Kotlin)
//! using Mozilla's UniFFI framework.
//!
//! # Usage
//!
//! ## iOS (Swift)
//!
//! ```swift
//! import Lexicon
//!
//! // Initialize
//! try initLexicon(dataDir: documentsDir)
//!
//! // Create transcriber
//! let transcriber = try Transcriber()
//! try transcriber.loadModel(modelPath: modelPath)
//!
//! // Transcribe
//! let config = TranscriptionFfiConfig(language: "en", translate: false, ...)
//! let result = try transcriber.transcribe(audioData: data, config: config)
//! ```
//!
//! ## Android (Kotlin)
//!
//! ```kotlin
//! import uniffi.lexicon.*
//!
//! // Initialize
//! initLexicon(dataDir = context.filesDir.absolutePath)
//!
//! // Create transcriber
//! val transcriber = Transcriber()
//! transcriber.loadModel(modelPath)
//!
//! // Transcribe
//! val config = TranscriptionFfiConfig(language = "en", translate = false, ...)
//! val result = transcriber.transcribe(audioData, config)
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use tokio_util::sync::CancellationToken;

// Include the generated UniFFI scaffolding
uniffi::include_scaffolding!("lexicon");

// Re-export error type for uniffi
pub use error::LexiconFfiError;

mod error;

// ============================================================================
// Global State
// ============================================================================

/// Global initialization state.
static INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DATA_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Initialize the Lexicon library.
///
/// Must be called before using any other functions.
/// Safe to call multiple times (subsequent calls are no-ops).
pub fn init_lexicon(data_dir: String) -> Result<(), LexiconFfiError> {
    if INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }

    let path = PathBuf::from(&data_dir);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&path).map_err(|e| {
        LexiconFfiError::InitializationFailed(format!("Failed to create data directory: {e}"))
    })?;

    // Store the data directory
    {
        let mut dir = DATA_DIR.write().map_err(|_| {
            LexiconFfiError::InitializationFailed("Failed to acquire lock".to_string())
        })?;
        *dir = Some(path);
    }

    // Initialize logging (to platform defaults)
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    INITIALIZED.store(true, std::sync::atomic::Ordering::SeqCst);
    tracing::info!("Lexicon FFI initialized with data_dir: {}", data_dir);

    Ok(())
}

/// Get the library version.
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============================================================================
// Configuration Types
// ============================================================================

/// FFI configuration for Lexicon context.
#[derive(Debug, Clone)]
pub struct LexiconFfiConfig {
    pub data_dir: String,
    pub model_dir: Option<String>,
    pub log_level: String,
}

/// FFI audio configuration.
#[derive(Debug, Clone)]
pub struct AudioFfiConfig {
    pub sample_rate: u32,
    pub chunk_ms: u32,
    pub vad_enabled: bool,
    pub vad_mode: u8,
}

/// FFI transcription configuration.
#[derive(Debug, Clone)]
pub struct TranscriptionFfiConfig {
    pub language: Option<String>,
    pub translate: bool,
    pub num_threads: u32,
    pub initial_prompt: Option<String>,
    pub temperature: f32,
}

/// FFI TTS configuration.
#[derive(Debug, Clone)]
pub struct TtsFfiConfig {
    pub language: Option<String>,
    pub voice_id: Option<String>,
    pub rate: f32,
    pub pitch: f32,
    pub volume: f32,
    pub use_kokoro: bool,
}

// ============================================================================
// Result Types
// ============================================================================

/// A transcribed segment.
#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f32>,
    pub language: Option<String>,
    pub speaker_id: Option<u32>,
}

/// Transcription result.
#[derive(Debug, Clone)]
pub struct TranscriptionFfiResult {
    pub segments: Vec<TranscriptionSegment>,
    pub detected_language: Option<String>,
    pub audio_duration_ms: i64,
    pub processing_time_ms: i64,
}

/// TTS synthesis result.
#[derive(Debug, Clone)]
pub struct SynthesisFfiResult {
    pub audio_data: Vec<u8>,
    pub sample_rate: u32,
    pub duration_ms: i64,
    pub engine: String,
}

/// Model information.
#[derive(Debug, Clone)]
pub struct ModelFfiInfo {
    pub id: String,
    pub name: String,
    pub model_type: String,
    pub size_bytes: u64,
    pub downloaded: bool,
    pub local_path: Option<String>,
}

/// Download progress.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_seconds: Option<u64>,
}

/// Platform information.
#[derive(Debug, Clone)]
pub struct PlatformFfiInfo {
    pub platform: String,
    pub model_tier: String,
    pub available_memory_mb: u64,
    pub has_gpu: bool,
}

// ============================================================================
// Callbacks
// ============================================================================

/// Callback for download progress.
pub trait DownloadCallback: Send + Sync {
    fn on_progress(&self, progress: DownloadProgress);
    fn on_complete(&self, model_id: String);
    fn on_error(&self, model_id: String, error: String);
    fn on_cancelled(&self, model_id: String);
}

/// Callback for streaming transcription.
pub trait TranscriptionCallback: Send + Sync {
    fn on_segment(&self, segment: TranscriptionSegment);
    fn on_final(&self, result: TranscriptionFfiResult);
    fn on_error(&self, error: String);
}

// ============================================================================
// LexiconContext
// ============================================================================

/// Main Lexicon context for FFI.
pub struct LexiconContext {
    config: lexicon_core::LexiconConfig,
}

impl LexiconContext {
    /// Create a new context.
    pub fn new(config: LexiconFfiConfig) -> Result<Self, LexiconFfiError> {
        let mut builder = lexicon_core::LexiconConfig::builder()
            .data_dir(PathBuf::from(&config.data_dir));

        if let Some(model_dir) = config.model_dir {
            builder = builder.model_dir(PathBuf::from(model_dir));
        }

        let core_config = builder.build().map_err(|e| {
            LexiconFfiError::ConfigError(e.to_string())
        })?;

        Ok(Self { config: core_config })
    }

    /// Get platform information.
    pub fn get_platform_info(&self) -> PlatformFfiInfo {
        let platform = lexicon_core::Platform::detect();
        let profile = lexicon_core::DeviceProfile::detect();

        PlatformFfiInfo {
            platform: format!("{:?}", platform),
            model_tier: format!("{:?}", profile.recommended_tier()),
            available_memory_mb: profile.available_memory_mb,
            has_gpu: profile.has_gpu,
        }
    }

    /// List available models.
    pub fn list_models(&self) -> Result<Vec<ModelFfiInfo>, LexiconFfiError> {
        let models = vec![
            ("whisper-tiny", "Whisper Tiny", "stt", 75_000_000u64),
            ("whisper-base", "Whisper Base", "stt", 142_000_000),
            ("whisper-small", "Whisper Small", "stt", 466_000_000),
            ("whisper-medium", "Whisper Medium", "stt", 1_500_000_000),
            ("whisper-large", "Whisper Large", "stt", 2_900_000_000),
        ];

        let model_dir = &self.config.model_dir;
        let results: Vec<ModelFfiInfo> = models
            .iter()
            .map(|(id, name, model_type, size)| {
                let model_path = model_dir.join(format!("{id}.bin"));
                let downloaded = model_path.exists();
                ModelFfiInfo {
                    id: id.to_string(),
                    name: name.to_string(),
                    model_type: model_type.to_string(),
                    size_bytes: *size,
                    downloaded,
                    local_path: if downloaded {
                        Some(model_path.display().to_string())
                    } else {
                        None
                    },
                }
            })
            .collect();

        Ok(results)
    }

    /// Check if a model is downloaded.
    pub fn is_model_downloaded(&self, model_id: String) -> Result<bool, LexiconFfiError> {
        let model_path = self.config.model_dir.join(format!("{model_id}.bin"));
        Ok(model_path.exists())
    }

    /// Delete a model.
    pub fn delete_model(&self, model_id: String) -> Result<(), LexiconFfiError> {
        let model_path = self.config.model_dir.join(format!("{model_id}.bin"));

        if !model_path.exists() {
            return Err(LexiconFfiError::ModelNotFound(model_id));
        }

        std::fs::remove_file(&model_path).map_err(|e| {
            LexiconFfiError::IoError(format!("Failed to delete model: {e}"))
        })?;

        Ok(())
    }
}

// ============================================================================
// Transcriber
// ============================================================================

/// Speech-to-text transcriber.
pub struct Transcriber {
    #[cfg(feature = "stt")]
    inner: Mutex<Option<lexicon_core::stt::WhisperTranscriber>>,
    #[cfg(not(feature = "stt"))]
    _marker: std::marker::PhantomData<()>,
}

impl Transcriber {
    /// Create a new transcriber.
    pub fn new() -> Result<Self, LexiconFfiError> {
        Ok(Self {
            #[cfg(feature = "stt")]
            inner: Mutex::new(None),
            #[cfg(not(feature = "stt"))]
            _marker: std::marker::PhantomData,
        })
    }

    /// Load a model.
    pub fn load_model(&self, model_path: String) -> Result<(), LexiconFfiError> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            let path = PathBuf::from(&model_path);
            if !path.exists() {
                return Err(LexiconFfiError::ModelNotFound(model_path));
            }

            let mut transcriber = lexicon_core::stt::WhisperTranscriber::new()
                .map_err(|e| LexiconFfiError::ModelLoadFailed(e.to_string()))?;

            transcriber.load_model(&path)
                .map_err(|e| LexiconFfiError::ModelLoadFailed(e.to_string()))?;

            let mut inner = self.inner.lock().map_err(|_| {
                LexiconFfiError::TranscriptionFailed("Lock poisoned".to_string())
            })?;
            *inner = Some(transcriber);

            tracing::info!("Model loaded: {}", model_path);
            Ok(())
        }

        #[cfg(not(feature = "stt"))]
        {
            Err(LexiconFfiError::TranscriptionFailed("STT feature not enabled".to_string()))
        }
    }

    /// Unload the current model.
    pub fn unload_model(&self) {
        #[cfg(feature = "stt")]
        {
            if let Ok(mut inner) = self.inner.lock() {
                *inner = None;
                tracing::info!("Model unloaded");
            }
        }
    }

    /// Check if a model is loaded.
    pub fn is_model_loaded(&self) -> bool {
        #[cfg(feature = "stt")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "stt"))]
        false
    }

    /// Get the loaded model name.
    pub fn get_model_name(&self) -> Option<String> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            self.inner.lock().ok().and_then(|inner| {
                inner.as_ref().and_then(|t| t.model_name().map(String::from))
            })
        }

        #[cfg(not(feature = "stt"))]
        None
    }

    /// Transcribe audio data.
    ///
    /// Audio should be raw PCM f32 samples at 16kHz mono.
    pub fn transcribe(
        &self,
        audio_data: Vec<u8>,
        config: TranscriptionFfiConfig,
    ) -> Result<TranscriptionFfiResult, LexiconFfiError> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            // Convert bytes to f32 samples
            let samples: Vec<f32> = audio_data
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
                return Err(LexiconFfiError::InvalidInput("Empty audio data".to_string()));
            }

            // Build config
            let mut tx_config = lexicon_core::stt::TranscriptionConfig::new();
            if let Some(lang) = config.language {
                tx_config = tx_config.with_language(lang);
            }
            if config.translate {
                tx_config = tx_config.with_translation();
            }
            if config.num_threads > 0 {
                tx_config = tx_config.with_threads(config.num_threads);
            }
            if let Some(prompt) = config.initial_prompt {
                tx_config = tx_config.with_prompt(prompt);
            }
            tx_config = tx_config.with_temperature(config.temperature);

            // Transcribe
            let inner = self.inner.lock().map_err(|_| {
                LexiconFfiError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let transcriber = inner.as_ref().ok_or_else(|| {
                LexiconFfiError::TranscriptionFailed("No model loaded".to_string())
            })?;

            let result = transcriber.transcribe(&samples, &tx_config)
                .map_err(|e| LexiconFfiError::TranscriptionFailed(e.to_string()))?;

            // Convert to FFI result
            let segments: Vec<TranscriptionSegment> = result.segments
                .iter()
                .map(|s| TranscriptionSegment {
                    start_ms: s.start_ms,
                    end_ms: s.end_ms,
                    text: s.text.clone(),
                    confidence: s.confidence,
                    language: s.language.clone(),
                    speaker_id: s.speaker_id,
                })
                .collect();

            Ok(TranscriptionFfiResult {
                segments,
                detected_language: result.detected_language,
                audio_duration_ms: result.audio_duration_ms,
                processing_time_ms: result.processing_time_ms,
            })
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFfiError::TranscriptionFailed("STT feature not enabled".to_string()))
    }

    /// Translate audio to English.
    pub fn translate(
        &self,
        audio_data: Vec<u8>,
        config: TranscriptionFfiConfig,
    ) -> Result<TranscriptionFfiResult, LexiconFfiError> {
        let mut translate_config = config;
        translate_config.translate = true;
        self.transcribe(audio_data, translate_config)
    }
}

// ============================================================================
// Synthesizer
// ============================================================================

/// Text-to-speech synthesizer.
pub struct Synthesizer {
    #[cfg(feature = "tts")]
    inner: Mutex<Option<lexicon_core::tts::TtsManager>>,
    #[cfg(not(feature = "tts"))]
    _marker: std::marker::PhantomData<()>,
}

impl Synthesizer {
    /// Create a new synthesizer.
    pub fn new() -> Result<Self, LexiconFfiError> {
        #[cfg(feature = "tts")]
        {
            let manager = lexicon_core::tts::TtsManager::new()
                .map_err(|e| LexiconFfiError::SynthesisFailed(e.to_string()))?;

            Ok(Self {
                inner: Mutex::new(Some(manager)),
            })
        }

        #[cfg(not(feature = "tts"))]
        {
            Ok(Self {
                _marker: std::marker::PhantomData,
            })
        }
    }

    /// Check if ready.
    pub fn is_ready(&self) -> bool {
        #[cfg(feature = "tts")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "tts"))]
        false
    }

    /// Synthesize text to audio.
    pub fn synthesize(
        &self,
        text: String,
        config: TtsFfiConfig,
    ) -> Result<SynthesisFfiResult, LexiconFfiError> {
        #[cfg(feature = "tts")]
        {
            let inner = self.inner.lock().map_err(|_| {
                LexiconFfiError::SynthesisFailed("Lock poisoned".to_string())
            })?;

            let manager = inner.as_ref().ok_or_else(|| {
                LexiconFfiError::SynthesisFailed("TTS not initialized".to_string())
            })?;

            // Build config
            let mut tts_config = lexicon_core::tts::TtsConfig::new();
            if let Some(lang) = config.language {
                tts_config = tts_config.with_language(lang);
            }
            tts_config = tts_config
                .with_rate(config.rate)
                .with_pitch(config.pitch)
                .with_volume(config.volume);

            if config.use_kokoro {
                tts_config = tts_config.with_engine(lexicon_core::tts::TtsEngine::Kokoro);
            }

            // Synthesize
            let result = manager.synthesize(&text, &tts_config)
                .map_err(|e| LexiconFfiError::SynthesisFailed(e.to_string()))?;

            // Convert audio to bytes
            let audio_bytes: Vec<u8> = result.audio.samples
                .iter()
                .flat_map(|&s| s.to_le_bytes())
                .collect();

            Ok(SynthesisFfiResult {
                audio_data: audio_bytes,
                sample_rate: result.audio.sample_rate,
                duration_ms: result.audio.duration_ms() as i64,
                engine: format!("{:?}", result.engine_used),
            })
        }

        #[cfg(not(feature = "tts"))]
        Err(LexiconFfiError::SynthesisFailed("TTS feature not enabled".to_string()))
    }

    /// List available voices.
    pub fn list_voices(&self) -> Vec<String> {
        #[cfg(feature = "tts")]
        {
            self.inner
                .lock()
                .ok()
                .and_then(|inner| {
                    inner.as_ref().map(|manager| {
                        manager.list_voices()
                            .iter()
                            .map(|v| v.id.clone())
                            .collect()
                    })
                })
                .unwrap_or_default()
        }

        #[cfg(not(feature = "tts"))]
        Vec::new()
    }
}

// ============================================================================
// Model Downloader
// ============================================================================

/// Model downloader for async operations.
pub struct ModelDownloader {
    model_dir: PathBuf,
    runtime: tokio::runtime::Runtime,
    /// Cancellation tokens for active downloads, keyed by model_id.
    cancel_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl ModelDownloader {
    /// Create a new downloader.
    pub fn new(model_dir: String) -> Result<Self, LexiconFfiError> {
        let path = PathBuf::from(&model_dir);

        std::fs::create_dir_all(&path).map_err(|e| {
            LexiconFfiError::IoError(format!("Failed to create model directory: {e}"))
        })?;

        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            LexiconFfiError::InitializationFailed(format!("Failed to create runtime: {e}"))
        })?;

        Ok(Self {
            model_dir: path,
            runtime,
            cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Download a model asynchronously.
    pub fn download_model(
        &self,
        model_id: String,
        callback: Box<dyn DownloadCallback>,
    ) -> Result<(), LexiconFfiError> {
        let model_dir = self.model_dir.clone();
        let callback: Arc<dyn DownloadCallback> = Arc::from(callback);
        let cancel_tokens = Arc::clone(&self.cancel_tokens);

        // Create a cancellation token for this download
        let cancel_token = CancellationToken::new();
        {
            let mut tokens = cancel_tokens.lock().map_err(|_| {
                LexiconFfiError::InitializationFailed("Failed to acquire lock".to_string())
            })?;
            tokens.insert(model_id.clone(), cancel_token.clone());
        }

        let model_id_for_cleanup = model_id.clone();
        let cancel_tokens_for_cleanup = Arc::clone(&cancel_tokens);

        self.runtime.spawn(async move {
            let mut registry = match lexicon_core::ModelRegistry::new(&model_dir) {
                Ok(r) => r,
                Err(e) => {
                    callback.on_error(model_id.clone(), e.to_string());
                    // Clean up token
                    if let Ok(mut tokens) = cancel_tokens_for_cleanup.lock() {
                        tokens.remove(&model_id_for_cleanup);
                    }
                    return;
                }
            };

            // Fetch manifest
            if let Err(e) = registry.fetch_manifest().await {
                callback.on_error(model_id.clone(), format!("Failed to fetch manifest: {e}"));
                // Clean up token
                if let Ok(mut tokens) = cancel_tokens_for_cleanup.lock() {
                    tokens.remove(&model_id_for_cleanup);
                }
                return;
            }

            // Check if cancelled before starting download
            if cancel_token.is_cancelled() {
                callback.on_cancelled(model_id.clone());
                if let Ok(mut tokens) = cancel_tokens_for_cleanup.lock() {
                    tokens.remove(&model_id_for_cleanup);
                }
                return;
            }

            // Download with progress and cancellation support
            let cb = Arc::clone(&callback);
            let mid = model_id.clone();
            let cancel_token_for_progress = cancel_token.clone();

            let result = registry.download_model_cancellable(
                &model_id,
                move |progress| {
                    cb.on_progress(DownloadProgress {
                        model_id: mid.clone(),
                        downloaded_bytes: progress.downloaded_bytes,
                        total_bytes: progress.total_bytes,
                        speed_bps: progress.speed_bps.unwrap_or(0),
                        eta_seconds: progress.eta_seconds,
                    });
                    // Return false to cancel, true to continue
                    !cancel_token_for_progress.is_cancelled()
                },
            ).await;

            // Clean up token
            if let Ok(mut tokens) = cancel_tokens_for_cleanup.lock() {
                tokens.remove(&model_id_for_cleanup);
            }

            match result {
                Ok(_path) => callback.on_complete(model_id),
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("cancelled") || cancel_token.is_cancelled() {
                        callback.on_cancelled(model_id);
                    } else {
                        callback.on_error(model_id, error_str);
                    }
                }
            }
        });

        Ok(())
    }

    /// Cancel an active download.
    ///
    /// If a download with the given model_id is in progress, it will be cancelled
    /// and the callback's `on_cancelled` method will be called.
    ///
    /// Returns `true` if a download was cancelled, `false` if no active download was found.
    pub fn cancel_download(&self, model_id: String) -> bool {
        if let Ok(tokens) = self.cancel_tokens.lock() {
            if let Some(token) = tokens.get(&model_id) {
                tracing::info!("Cancelling download for model: {}", model_id);
                token.cancel();
                true
            } else {
                tracing::warn!("No active download found for model: {}", model_id);
                false
            }
        } else {
            tracing::error!("Failed to acquire lock for cancel_download");
            false
        }
    }

    /// Check if a download is active.
    pub fn is_downloading(&self, model_id: &str) -> bool {
        self.cancel_tokens
            .lock()
            .map(|tokens| tokens.contains_key(model_id))
            .unwrap_or(false)
    }
}
