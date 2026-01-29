//! Flutter Rust Bridge API for Lexicon.
//!
//! This module provides the public API surface for Flutter/Dart applications.
//! Functions annotated with `#[frb]` macros are exposed to Dart.

use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use flutter_rust_bridge::frb;

use crate::error::LexiconFlutterError;
use crate::types::*;

// ============================================================================
// Global State
// ============================================================================

/// Global initialization state.
static INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DATA_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the flutter_rust_bridge runtime.
/// Must be called before any other functions.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Initialize the Lexicon library with data directory.
///
/// This must be called before using transcription or synthesis features.
/// It's safe to call multiple times (subsequent calls are no-ops).
#[frb(sync)]
pub fn init_lexicon(data_dir: String) -> Result<(), LexiconFlutterError> {
    if INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }

    let path = PathBuf::from(&data_dir);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&path).map_err(|e| {
        LexiconFlutterError::InitializationFailed(format!("Failed to create data directory: {e}"))
    })?;

    // Store the data directory
    {
        let mut dir = DATA_DIR.write().map_err(|_| {
            LexiconFlutterError::InitializationFailed("Failed to acquire lock".to_string())
        })?;
        *dir = Some(path);
    }

    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    INITIALIZED.store(true, std::sync::atomic::Ordering::SeqCst);
    tracing::info!("Lexicon Flutter initialized with data_dir: {}", data_dir);

    Ok(())
}

/// Get the library version.
#[frb(sync)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============================================================================
// Platform Info
// ============================================================================

/// Get platform information including recommended model tier.
#[frb(sync)]
pub fn get_platform_info() -> PlatformInfo {
    let platform = lexicon_core::Platform::detect();
    let profile = lexicon_core::DeviceProfile::detect();

    PlatformInfo {
        platform: format!("{:?}", platform),
        model_tier: format!("{:?}", profile.recommended_tier()),
        available_memory_mb: profile.available_memory_mb,
        has_gpu: profile.has_gpu,
    }
}

// ============================================================================
// Model Management
// ============================================================================

/// List available models.
pub fn list_models(model_dir: String) -> Result<Vec<ModelInfo>, LexiconFlutterError> {
    let model_path = PathBuf::from(&model_dir);

    let models = [
        ("whisper-tiny", "Whisper Tiny", "stt", 75_000_000u64),
        ("whisper-base", "Whisper Base", "stt", 142_000_000),
        ("whisper-small", "Whisper Small", "stt", 466_000_000),
        ("whisper-medium", "Whisper Medium", "stt", 1_500_000_000),
        ("whisper-large", "Whisper Large", "stt", 2_900_000_000),
    ];

    let results: Vec<ModelInfo> = models
        .iter()
        .map(|(id, name, model_type, size)| {
            let path = model_path.join(format!("{id}.bin"));
            let downloaded = path.exists();
            ModelInfo {
                id: id.to_string(),
                name: name.to_string(),
                model_type: model_type.to_string(),
                size_bytes: *size,
                downloaded,
                local_path: if downloaded {
                    Some(path.display().to_string())
                } else {
                    None
                },
            }
        })
        .collect();

    Ok(results)
}

/// Check if a model is downloaded.
#[frb(sync)]
pub fn is_model_downloaded(model_dir: String, model_id: String) -> bool {
    let model_path = PathBuf::from(&model_dir).join(format!("{model_id}.bin"));
    model_path.exists()
}

/// Delete a downloaded model.
pub fn delete_model(model_dir: String, model_id: String) -> Result<(), LexiconFlutterError> {
    let model_path = PathBuf::from(&model_dir).join(format!("{model_id}.bin"));

    if !model_path.exists() {
        return Err(LexiconFlutterError::ModelNotFound(model_id));
    }

    std::fs::remove_file(&model_path)?;
    tracing::info!("Deleted model: {}", model_id);

    Ok(())
}

// ============================================================================
// Transcriber
// ============================================================================

/// Transcriber state held in Dart via opaque handle.
#[frb(opaque)]
pub struct Transcriber {
    #[cfg(feature = "stt")]
    inner: Mutex<Option<lexicon_core::stt::WhisperTranscriber>>,
    #[cfg(not(feature = "stt"))]
    _marker: std::marker::PhantomData<()>,
}

impl Transcriber {
    /// Create a new transcriber.
    pub fn new() -> Result<Transcriber, LexiconFlutterError> {
        Ok(Self {
            #[cfg(feature = "stt")]
            inner: Mutex::new(None),
            #[cfg(not(feature = "stt"))]
            _marker: std::marker::PhantomData,
        })
    }

    /// Load a Whisper model.
    pub fn load_model(&self, model_path: String) -> Result<(), LexiconFlutterError> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            let path = PathBuf::from(&model_path);
            if !path.exists() {
                return Err(LexiconFlutterError::ModelNotFound(model_path));
            }

            let mut transcriber = lexicon_core::stt::WhisperTranscriber::new()
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            transcriber
                .load_model(&path)
                .map_err(|e| LexiconFlutterError::ModelLoadFailed(e.to_string()))?;

            let mut inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;
            *inner = Some(transcriber);

            tracing::info!("Model loaded: {}", model_path);
            Ok(())
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }

    /// Unload the current model.
    #[frb(sync)]
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
    #[frb(sync)]
    pub fn is_model_loaded(&self) -> bool {
        #[cfg(feature = "stt")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "stt"))]
        false
    }

    /// Get the loaded model name.
    #[frb(sync)]
    pub fn get_model_name(&self) -> Option<String> {
        #[cfg(feature = "stt")]
        {
            use lexicon_core::stt::Transcriber as _;

            self.inner
                .lock()
                .ok()
                .and_then(|inner| inner.as_ref().and_then(|t| t.model_name().map(String::from)))
        }

        #[cfg(not(feature = "stt"))]
        None
    }

    /// Transcribe audio data (batch mode).
    ///
    /// Audio should be raw PCM f32 samples at 16kHz mono, packed as bytes.
    pub fn transcribe(
        &self,
        audio_data: Vec<u8>,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult, LexiconFlutterError> {
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
                return Err(LexiconFlutterError::InvalidInput(
                    "Empty audio data".to_string(),
                ));
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
                LexiconFlutterError::TranscriptionFailed("Lock poisoned".to_string())
            })?;

            let transcriber = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::TranscriptionFailed("No model loaded".to_string())
            })?;

            let result = transcriber
                .transcribe(&samples, &tx_config)
                .map_err(|e| LexiconFlutterError::TranscriptionFailed(e.to_string()))?;

            // Convert to FFI result
            let segments: Vec<TranscriptionSegment> = result
                .segments
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

            Ok(TranscriptionResult {
                segments,
                detected_language: result.detected_language,
                audio_duration_ms: result.audio_duration_ms,
                processing_time_ms: result.processing_time_ms,
            })
        }

        #[cfg(not(feature = "stt"))]
        Err(LexiconFlutterError::TranscriptionFailed(
            "STT feature not enabled".to_string(),
        ))
    }

    /// Translate audio to English (batch mode).
    pub fn translate(
        &self,
        audio_data: Vec<u8>,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult, LexiconFlutterError> {
        let mut translate_config = config;
        translate_config.translate = true;
        self.transcribe(audio_data, translate_config)
    }
}

// ============================================================================
// Synthesizer
// ============================================================================

/// Synthesizer state held in Dart via opaque handle.
#[frb(opaque)]
pub struct Synthesizer {
    #[cfg(feature = "tts")]
    inner: Mutex<Option<lexicon_core::tts::TtsManager>>,
    #[cfg(not(feature = "tts"))]
    _marker: std::marker::PhantomData<()>,
}

impl Synthesizer {
    /// Create a new synthesizer.
    pub fn new() -> Result<Synthesizer, LexiconFlutterError> {
        #[cfg(feature = "tts")]
        {
            let manager = lexicon_core::tts::TtsManager::new()
                .map_err(|e| LexiconFlutterError::SynthesisFailed(e.to_string()))?;

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

    /// Check if the synthesizer is ready.
    #[frb(sync)]
    pub fn is_ready(&self) -> bool {
        #[cfg(feature = "tts")]
        {
            self.inner.lock().map(|i| i.is_some()).unwrap_or(false)
        }

        #[cfg(not(feature = "tts"))]
        false
    }

    /// List available voices.
    #[frb(sync)]
    pub fn list_voices(&self) -> Vec<String> {
        #[cfg(feature = "tts")]
        {
            self.inner
                .lock()
                .ok()
                .and_then(|inner| {
                    inner
                        .as_ref()
                        .map(|manager| manager.list_voices().iter().map(|v| v.id.clone()).collect())
                })
                .unwrap_or_default()
        }

        #[cfg(not(feature = "tts"))]
        Vec::new()
    }

    /// Synthesize text to audio (batch mode).
    pub fn synthesize(
        &self,
        text: String,
        config: TtsConfig,
    ) -> Result<SynthesisResult, LexiconFlutterError> {
        #[cfg(feature = "tts")]
        {
            let inner = self.inner.lock().map_err(|_| {
                LexiconFlutterError::SynthesisFailed("Lock poisoned".to_string())
            })?;

            let manager = inner.as_ref().ok_or_else(|| {
                LexiconFlutterError::SynthesisFailed("TTS not initialized".to_string())
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
            let result = manager
                .synthesize(&text, &tts_config)
                .map_err(|e| LexiconFlutterError::SynthesisFailed(e.to_string()))?;

            // Convert audio to bytes
            let audio_bytes: Vec<u8> = result
                .audio
                .samples
                .iter()
                .flat_map(|&s| s.to_le_bytes())
                .collect();

            Ok(SynthesisResult {
                audio_data: audio_bytes,
                sample_rate: result.audio.sample_rate,
                duration_ms: result.audio.duration_ms() as i64,
                engine: format!("{:?}", result.engine_used),
            })
        }

        #[cfg(not(feature = "tts"))]
        Err(LexiconFlutterError::SynthesisFailed(
            "TTS feature not enabled".to_string(),
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_platform_info() {
        let info = get_platform_info();
        assert!(!info.platform.is_empty());
        assert!(info.available_memory_mb > 0);
    }

    #[test]
    fn test_transcriber_creation() {
        let transcriber = Transcriber::new();
        assert!(transcriber.is_ok());
        let t = transcriber.unwrap();
        assert!(!t.is_model_loaded());
    }

    #[test]
    fn test_config_defaults() {
        let config = TranscriptionConfig::default();
        assert!(!config.translate);
        assert_eq!(config.num_threads, 4);

        let tts = TtsConfig::default();
        assert!(tts.use_kokoro);
        assert!((tts.rate - 1.0).abs() < f32::EPSILON);
    }
}
