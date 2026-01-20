//! Text-to-Speech Engine
//!
//! This module provides text-to-speech synthesis using Kokoro (ONNX-based neural TTS)
//! with automatic fallback to platform-native TTS via tts-rs.
//!
//! # Architecture
//!
//! ```text
//! Text → Engine Selection → Kokoro/Platform → AudioBuffer
//!                ↓ (fallback on failure)
//!           Platform TTS
//! ```
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::tts::{TtsManager, TtsConfig};
//! use std::path::Path;
//!
//! // Create TTS manager
//! let mut manager = TtsManager::new()?;
//!
//! // Load Kokoro model (optional - will use platform TTS if not loaded)
//! manager.load_kokoro_model(Path::new("models/kokoro-v1.onnx"))?;
//!
//! // Synthesize speech
//! let config = TtsConfig::new();
//! let result = manager.synthesize("Hello, world!", &config)?;
//!
//! println!("Generated {} samples", result.audio.len());
//! # Ok::<(), lexicon_core::error::LexiconError>(())
//! ```
//!
//! # Fallback Behavior
//!
//! Fallback to platform TTS occurs when:
//! - Kokoro model is not loaded
//! - Non-English text is detected
//! - User explicitly requests platform voice
//! - Kokoro synthesis fails

mod kokoro;
mod platform;
mod types;

pub use kokoro::KokoroSynthesizer;
pub use platform::PlatformSynthesizer;
pub use types::{
    AudioBuffer, AudioChunk, SynthesisResult, StreamingReceiver, StreamingSender,
    TtsConfig, TtsEngine, VoiceGender, VoiceInfo, streaming_channel, TTS_SAMPLE_RATE,
};

use crate::error::{InferenceError, LexiconError, Result};
use std::path::Path;
use std::thread;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Default chunk size for streaming TTS (100ms of audio at 24kHz).
pub const DEFAULT_STREAM_CHUNK_SIZE: usize = 2400;

/// Trait for text-to-speech synthesis engines.
///
/// Implementations should:
/// - Produce audio at a consistent sample rate
/// - Handle various input text lengths
/// - Support voice selection where applicable
pub trait Synthesizer: Send {
    /// Loads a model from the given path (if applicable).
    ///
    /// Not all synthesizers require model loading (e.g., platform TTS).
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    fn load_model(&mut self, path: &Path) -> Result<()>;

    /// Synthesizes text to audio.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to synthesize
    /// * `config` - Synthesis configuration
    ///
    /// # Returns
    ///
    /// An `AudioBuffer` containing the synthesized speech.
    ///
    /// # Errors
    ///
    /// Returns an error if synthesis fails.
    fn synthesize(&self, text: &str, config: &TtsConfig) -> Result<AudioBuffer>;

    /// Returns true if the synthesizer is ready to synthesize.
    fn is_ready(&self) -> bool;

    /// Returns the available voices.
    fn list_voices(&self) -> Vec<VoiceInfo>;

    /// Returns the engine type.
    fn engine_type(&self) -> TtsEngine;
}

/// TTS Manager that handles engine selection and fallback.
///
/// This is the main entry point for TTS functionality. It manages both
/// Kokoro and platform synthesizers, automatically falling back when needed.
pub struct TtsManager {
    kokoro: KokoroSynthesizer,
    platform: PlatformSynthesizer,
}

impl TtsManager {
    /// Creates a new TTS manager.
    ///
    /// # Errors
    ///
    /// Returns an error if platform TTS initialization fails.
    pub fn new() -> Result<Self> {
        let kokoro = KokoroSynthesizer::new();
        let platform = PlatformSynthesizer::new()?;

        Ok(Self { kokoro, platform })
    }

    /// Loads a Kokoro model from the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    pub fn load_kokoro_model(&mut self, path: &Path) -> Result<()> {
        self.kokoro.load_model(path)
    }

    /// Unloads the Kokoro model to free memory.
    pub fn unload_kokoro_model(&mut self) {
        self.kokoro.unload_model();
    }

    /// Returns true if Kokoro is ready.
    #[must_use]
    pub fn is_kokoro_ready(&self) -> bool {
        self.kokoro.is_ready()
    }

    /// Returns true if platform TTS is ready.
    #[must_use]
    pub fn is_platform_ready(&self) -> bool {
        self.platform.is_ready()
    }

    /// Synthesizes text to audio.
    ///
    /// Uses the preferred engine from config, falling back if needed.
    ///
    /// # Errors
    ///
    /// Returns an error if all synthesis attempts fail.
    pub fn synthesize(&self, text: &str, config: &TtsConfig) -> Result<SynthesisResult> {
        if text.is_empty() {
            return Err(LexiconError::Inference(InferenceError::InvalidInput {
                message: "Text cannot be empty".to_string(),
            }));
        }

        let start_time = Instant::now();

        // Determine which engine to try first
        let should_use_platform = match config.preferred_engine {
            TtsEngine::Platform => true,
            TtsEngine::Kokoro => {
                // Use platform if Kokoro isn't ready or text is non-English
                !self.kokoro.is_ready() || self.is_non_english(text, config)
            }
        };

        if should_use_platform {
            debug!("Using platform TTS");
            return self.synthesize_with_platform(text, config, start_time, false);
        }

        // Try Kokoro first
        debug!("Attempting Kokoro TTS");
        match self.kokoro.synthesize(text, config) {
            Ok(audio) => {
                let processing_time_ms = start_time.elapsed().as_millis() as u64;
                info!(
                    "Kokoro synthesis complete: {}ms audio in {}ms",
                    audio.duration_ms(),
                    processing_time_ms
                );
                Ok(SynthesisResult::new(audio, TtsEngine::Kokoro, processing_time_ms))
            }
            Err(e) => {
                if config.fallback_enabled {
                    warn!("Kokoro synthesis failed, falling back to platform: {}", e);
                    self.synthesize_with_platform(text, config, start_time, true)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Synthesizes with platform TTS.
    fn synthesize_with_platform(
        &self,
        text: &str,
        config: &TtsConfig,
        start_time: Instant,
        is_fallback: bool,
    ) -> Result<SynthesisResult> {
        let audio = self.platform.synthesize(text, config)?;
        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "Platform synthesis complete: {}ms audio in {}ms",
            audio.duration_ms(),
            processing_time_ms
        );

        let mut result = SynthesisResult::new(audio, TtsEngine::Platform, processing_time_ms);
        if is_fallback {
            result = result.with_fallback();
        }
        Ok(result)
    }

    /// Checks if text appears to be non-English.
    ///
    /// This is a simple heuristic based on character analysis.
    fn is_non_english(&self, text: &str, config: &TtsConfig) -> bool {
        // If language is explicitly set, use that
        if let Some(ref lang) = config.language {
            return !lang.starts_with("en");
        }

        // Simple heuristic: check for non-ASCII characters that suggest other languages
        let non_ascii_ratio = text.chars().filter(|c| !c.is_ascii()).count() as f32
            / text.chars().count().max(1) as f32;

        // If more than 20% non-ASCII, assume non-English
        non_ascii_ratio > 0.2
    }

    /// Returns available voices from all engines.
    #[must_use]
    pub fn list_voices(&self) -> Vec<VoiceInfo> {
        let mut voices = Vec::new();

        if self.kokoro.is_ready() {
            voices.extend(self.kokoro.list_voices());
        }

        voices.extend(self.platform.list_voices());
        voices
    }

    /// Forces synthesis with a specific engine (no fallback).
    ///
    /// # Errors
    ///
    /// Returns an error if the specified engine fails.
    pub fn synthesize_with_engine(
        &self,
        text: &str,
        config: &TtsConfig,
        engine: TtsEngine,
    ) -> Result<SynthesisResult> {
        let start_time = Instant::now();

        let audio = match engine {
            TtsEngine::Kokoro => self.kokoro.synthesize(text, config)?,
            TtsEngine::Platform => self.platform.synthesize(text, config)?,
        };

        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        Ok(SynthesisResult::new(audio, engine, processing_time_ms))
    }

    /// Synthesizes text to streaming audio.
    ///
    /// This method starts synthesis in a background thread and returns immediately
    /// with a receiver for audio chunks. The first chunk should arrive within 200ms
    /// for short text inputs.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to synthesize
    /// * `config` - Synthesis configuration
    /// * `chunk_size` - Optional chunk size in samples (default: 2400 = 100ms at 24kHz)
    ///
    /// # Returns
    ///
    /// A `StreamingReceiver` that yields `AudioChunk`s as they become available.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lexicon_core::tts::{TtsManager, TtsConfig};
    ///
    /// let manager = TtsManager::new().unwrap();
    /// let config = TtsConfig::new();
    /// let mut receiver = manager.synthesize_streaming("Hello, world!", &config, None);
    ///
    /// // Process chunks as they arrive
    /// for chunk in receiver {
    ///     println!("Received chunk {} ({} samples)", chunk.sequence, chunk.samples.len());
    ///     if chunk.is_final {
    ///         break;
    ///     }
    /// }
    /// ```
    pub fn synthesize_streaming(
        &self,
        text: &str,
        config: &TtsConfig,
        chunk_size: Option<usize>,
    ) -> StreamingReceiver {
        let chunk_size = chunk_size.unwrap_or(DEFAULT_STREAM_CHUNK_SIZE);
        let (mut sender, receiver) = streaming_channel(TTS_SAMPLE_RATE);

        // Clone what we need for the synthesis thread
        let text = text.to_string();
        let config = config.clone();
        let use_platform = match config.preferred_engine {
            TtsEngine::Platform => true,
            TtsEngine::Kokoro => !self.kokoro.is_ready() || self.is_non_english(&text, &config),
        };

        // Determine which engine to use
        let kokoro_ready = self.kokoro.is_ready();
        let kokoro_path = self.kokoro.model_path.clone();

        thread::spawn(move || {
            // Perform synthesis (this is the slow part)
            let samples = if use_platform || !kokoro_ready {
                debug!("Streaming with platform TTS");
                // For platform TTS, we can't easily stream, so we synthesize all at once
                // and chunk the result
                match PlatformSynthesizer::new() {
                    Ok(platform) => match platform.synthesize(&text, &config) {
                        Ok(buffer) => buffer.samples,
                        Err(e) => {
                            warn!("Platform TTS streaming failed: {}", e);
                            return;
                        }
                    },
                    Err(e) => {
                        warn!("Platform TTS init failed: {}", e);
                        return;
                    }
                }
            } else {
                debug!("Streaming with Kokoro TTS");
                // For Kokoro, we also synthesize all at once and chunk the result
                // A future improvement could implement true incremental synthesis
                let mut kokoro = KokoroSynthesizer::new();
                if let Some(ref path) = kokoro_path {
                    if let Err(e) = kokoro.load_model(Path::new(path)) {
                        warn!("Kokoro model load failed in streaming: {}", e);
                        return;
                    }
                }
                match kokoro.synthesize(&text, &config) {
                    Ok(buffer) => buffer.samples,
                    Err(e) => {
                        warn!("Kokoro TTS streaming failed: {}", e);
                        return;
                    }
                }
            };

            // Stream chunks
            let total_samples = samples.len();
            let mut offset = 0;

            while offset < total_samples {
                let end = (offset + chunk_size).min(total_samples);
                let is_final = end >= total_samples;
                let chunk_samples = samples[offset..end].to_vec();

                if sender.send(chunk_samples, is_final).is_err() {
                    // Receiver dropped, stop streaming
                    debug!("Streaming receiver dropped, stopping");
                    return;
                }

                offset = end;
            }

            debug!("Streaming synthesis complete: {} samples in {} chunks",
                total_samples, (total_samples + chunk_size - 1) / chunk_size);
        });

        receiver
    }
}

impl std::fmt::Debug for TtsManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TtsManager")
            .field("kokoro_ready", &self.is_kokoro_ready())
            .field("platform_ready", &self.is_platform_ready())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_english_detection() {
        // This test may fail in CI if platform TTS isn't available
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => {
                // Skip test if TTS initialization fails (expected in CI/headless environments)
                return;
            }
        };
        let config = TtsConfig::new();

        // English text
        assert!(!manager.is_non_english("Hello, world!", &config));

        // Japanese text
        assert!(manager.is_non_english("こんにちは世界", &config));

        // Mixed text with explicit language
        let config_es = TtsConfig::new().with_language("es-ES");
        assert!(manager.is_non_english("Hello", &config_es));

        let config_en = TtsConfig::new().with_language("en-US");
        assert!(!manager.is_non_english("Hola", &config_en));
    }

    #[test]
    fn test_tts_config_presets() {
        let kokoro_config = TtsConfig::kokoro();
        assert_eq!(kokoro_config.preferred_engine, TtsEngine::Kokoro);

        let platform_config = TtsConfig::platform();
        assert_eq!(platform_config.preferred_engine, TtsEngine::Platform);
    }
}
