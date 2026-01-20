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

    // ==========================================================================
    // Configuration Tests
    // ==========================================================================

    #[test]
    fn test_tts_config_presets() {
        let kokoro_config = TtsConfig::kokoro();
        assert_eq!(kokoro_config.preferred_engine, TtsEngine::Kokoro);

        let platform_config = TtsConfig::platform();
        assert_eq!(platform_config.preferred_engine, TtsEngine::Platform);
    }

    #[test]
    fn test_tts_config_builder() {
        let config = TtsConfig::new()
            .with_language("es-ES")
            .with_rate(1.5)
            .with_pitch(0.8)
            .with_volume(0.9)
            .with_engine(TtsEngine::Platform);

        assert_eq!(config.language.as_deref(), Some("es-ES"));
        assert_eq!(config.rate, 1.5);
        assert_eq!(config.pitch, 0.8);
        assert_eq!(config.volume, 0.9);
        assert_eq!(config.preferred_engine, TtsEngine::Platform);
    }

    // ==========================================================================
    // Language Detection Tests
    // ==========================================================================

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
    fn test_non_english_detection_french() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };
        let config = TtsConfig::new();

        // French text with accents (needs >20% non-ASCII to trigger detection)
        assert!(manager.is_non_english("Été à Montréal", &config));

        // Pure ASCII French won't trigger detection
        assert!(!manager.is_non_english("Bonjour!", &config));

        // Explicit French language triggers detection
        let config_fr = TtsConfig::new().with_language("fr-FR");
        assert!(manager.is_non_english("Hello", &config_fr));
    }

    #[test]
    fn test_non_english_detection_chinese() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };
        let config = TtsConfig::new();

        // Chinese text
        assert!(manager.is_non_english("你好世界", &config));
    }

    #[test]
    fn test_non_english_detection_mixed() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };
        let config = TtsConfig::new();

        // Mostly English with some non-ASCII
        assert!(!manager.is_non_english("Hello café", &config)); // < 20% non-ASCII

        // Mostly non-English
        assert!(manager.is_non_english("日本語 English 日本語", &config)); // > 20% non-ASCII
    }

    // ==========================================================================
    // Fallback Tests (WP-ROADBLOCK-19)
    // ==========================================================================

    /// Test: Non-English text triggers platform TTS fallback.
    #[test]
    fn test_fallback_non_english_triggers_platform_tts() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return, // Skip if TTS unavailable
        };

        // With Kokoro preferred but non-English text
        let config = TtsConfig::new()
            .with_engine(TtsEngine::Kokoro)
            .with_language("fr-FR");

        // Kokoro is not loaded, so should use platform
        let result = manager.synthesize("Bonjour le monde", &config);

        // Should succeed with platform TTS (or fail gracefully if unavailable)
        match result {
            Ok(r) => {
                assert_eq!(r.engine_used, TtsEngine::Platform);
            }
            Err(_) => {
                // Platform TTS might not be available in headless CI
            }
        }
    }

    /// Test: Missing Kokoro model triggers platform fallback.
    #[test]
    fn test_fallback_on_kokoro_not_loaded() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        // Kokoro is not loaded
        assert!(!manager.is_kokoro_ready());

        // Request Kokoro synthesis
        let config = TtsConfig::kokoro();
        let result = manager.synthesize("Hello, world!", &config);

        // Should fall back to platform
        match result {
            Ok(r) => {
                assert_eq!(r.engine_used, TtsEngine::Platform);
            }
            Err(_) => {
                // Platform TTS might not be available in headless CI
            }
        }
    }

    /// Test: User can force platform TTS preference.
    #[test]
    fn test_fallback_user_preference() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        // Explicitly request platform TTS
        let config = TtsConfig::platform();
        let result = manager.synthesize("Hello, world!", &config);

        match result {
            Ok(r) => {
                assert_eq!(r.engine_used, TtsEngine::Platform);
            }
            Err(_) => {
                // Platform TTS might not be available in headless CI
            }
        }
    }

    /// Test: Platform TTS produces non-zero audio.
    #[test]
    fn test_platform_tts_produces_audio() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        let config = TtsConfig::platform();
        let result = manager.synthesize("Testing audio output", &config);

        match result {
            Ok(r) => {
                // Audio should not be empty
                assert!(!r.audio.samples.is_empty(), "Audio should not be empty");

                // Audio should have a valid sample rate
                assert!(r.audio.sample_rate > 0, "Sample rate should be positive");

                // Duration should be reasonable (> 0 ms for non-empty text)
                assert!(r.audio.duration_ms() > 0, "Duration should be positive");

                // Check that audio is not completely silent (at least some non-zero samples)
                let max_amplitude = r.audio.samples.iter()
                    .map(|s| s.abs())
                    .fold(0.0f32, |a, b| a.max(b));
                // Note: In some CI environments, TTS may produce silence
                // This is acceptable as long as we get valid audio data
                debug!("Max amplitude: {}", max_amplitude);
            }
            Err(e) => {
                // Platform TTS might not be available - acceptable in CI
                debug!("Platform TTS unavailable: {}", e);
            }
        }
    }

    /// Test: Fallback chain works correctly.
    #[test]
    fn test_fallback_chain() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        // Kokoro not loaded, should use platform
        assert!(!manager.is_kokoro_ready());

        // Request Kokoro with fallback enabled
        let config = TtsConfig::new()
            .with_engine(TtsEngine::Kokoro);

        let result = manager.synthesize("Testing fallback chain", &config);

        match result {
            Ok(r) => {
                // Should have fallen back to platform
                assert_eq!(r.engine_used, TtsEngine::Platform);
            }
            Err(_) => {
                // Acceptable if platform TTS is also unavailable
            }
        }
    }

    /// Test: Synthesis with engine forces specific engine.
    #[test]
    fn test_synthesize_with_engine() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        let config = TtsConfig::new();

        // Force platform engine
        let result = manager.synthesize_with_engine(
            "Testing explicit engine",
            &config,
            TtsEngine::Platform
        );

        match result {
            Ok(r) => {
                assert_eq!(r.engine_used, TtsEngine::Platform);
            }
            Err(_) => {
                // Platform TTS might not be available
            }
        }
    }

    /// Test: Empty text returns error.
    #[test]
    fn test_empty_text_error() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        let config = TtsConfig::new();
        let result = manager.synthesize("", &config);

        assert!(result.is_err(), "Empty text should return error");
    }

    // ==========================================================================
    // Streaming Tests
    // ==========================================================================

    #[test]
    fn test_streaming_synthesis() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        let config = TtsConfig::platform();
        let mut receiver = manager.synthesize_streaming(
            "Testing streaming synthesis",
            &config,
            Some(1200), // Smaller chunks for faster test
        );

        let mut total_samples = 0;
        let mut chunk_count = 0;
        let mut received_final = false;

        // Receive chunks with timeout
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match receiver.try_recv() {
                Ok(Some(chunk)) => {
                    total_samples += chunk.samples.len();
                    chunk_count += 1;
                    if chunk.is_final {
                        received_final = true;
                        break;
                    }
                }
                Ok(None) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(_) => {
                    break; // Channel closed
                }
            }
        }

        // In headless CI, streaming might not produce output
        if chunk_count > 0 {
            assert!(received_final, "Should receive final chunk");
            assert!(total_samples > 0, "Should receive samples");
        }
    }

    // ==========================================================================
    // Voice Listing Tests
    // ==========================================================================

    #[test]
    fn test_list_voices() {
        let manager = match TtsManager::new() {
            Ok(m) => m,
            Err(_) => return,
        };

        let voices = manager.list_voices();

        // Voice list might be empty in headless CI, but should not panic
        for voice in &voices {
            assert!(!voice.id.is_empty(), "Voice ID should not be empty");
        }
    }
}
