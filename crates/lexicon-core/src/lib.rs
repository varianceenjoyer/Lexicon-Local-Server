//! Lexicon Core Library
//!
//! Core library for Lexicon Local Server providing STT, translation, and TTS capabilities.
//! This crate exposes a C ABI-compatible interface for cross-platform integration.
//!
//! # Quick Start
//!
//! ```no_run
//! use lexicon_core::{LexiconConfig, LexiconContext, LogLevel};
//! use lexicon_core::logging::init_logging;
//!
//! // Initialize logging
//! init_logging(LogLevel::Info, false).unwrap();
//!
//! // Create configuration
//! let config = LexiconConfig::builder().build().unwrap();
//!
//! // Create context
//! let ctx = LexiconContext::new(config).unwrap();
//! ```

pub mod config;
pub mod error;
pub mod logging;
pub mod platform;

#[cfg(feature = "storage")]
pub mod storage;

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "stt")]
pub mod stt;

#[cfg(feature = "tts")]
pub mod tts;

pub mod models;

pub use config::{LexiconConfig, LogLevel};
pub use error::{LexiconError, Result};
pub use platform::{DeviceProfile, ModelTier, Platform, ThermalState};

#[cfg(feature = "storage")]
pub use storage::Database;

#[cfg(feature = "audio")]
pub use audio::{AudioChunk, AudioConfig, AudioSource, CpalAudioSource, Resampler, VoiceActivityDetector};

#[cfg(feature = "stt")]
pub use stt::{Segment, Transcriber, TranscriptionConfig, TranscriptionResult, WhisperTranscriber};

#[cfg(feature = "tts")]
pub use tts::{AudioBuffer, KokoroSynthesizer, PlatformSynthesizer, Synthesizer, TtsConfig, TtsEngine, TtsManager};

pub use models::{ModelRegistry, ModelManifest, ModelId, ModelInfo};

/// Main application context holding configuration and component references.
///
/// This is the primary entry point for interacting with Lexicon functionality.
/// Components (STT, TTS, storage) are added as they become available through
/// feature flags.
pub struct LexiconContext {
    config: LexiconConfig,
    // Future: audio_source, transcriber, synthesizer, database, model_registry
}

impl LexiconContext {
    /// Creates a new Lexicon context with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data directory cannot be created or accessed
    /// - The model directory cannot be created or accessed
    /// - Platform-specific initialization fails
    ///
    /// # Examples
    ///
    /// ```
    /// use lexicon_core::{LexiconConfig, LexiconContext};
    /// use std::path::PathBuf;
    ///
    /// let config = LexiconConfig::builder()
    ///     .data_dir(PathBuf::from("/tmp/lexicon-test"))
    ///     .build()
    ///     .unwrap();
    ///
    /// let ctx = LexiconContext::new(config).unwrap();
    /// ```
    pub fn new(config: LexiconConfig) -> Result<Self> {
        // Ensure directories exist
        std::fs::create_dir_all(&config.data_dir).map_err(|e| {
            LexiconError::Config(format!(
                "Failed to create data directory '{}': {}",
                config.data_dir.display(),
                e
            ))
        })?;

        std::fs::create_dir_all(&config.model_dir).map_err(|e| {
            LexiconError::Config(format!(
                "Failed to create model directory '{}': {}",
                config.model_dir.display(),
                e
            ))
        })?;

        Ok(Self { config })
    }

    /// Returns a reference to the current configuration.
    #[must_use]
    pub fn config(&self) -> &LexiconConfig {
        &self.config
    }

    /// Returns the detected platform.
    #[must_use]
    pub fn platform(&self) -> Platform {
        Platform::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_context_creation() {
        let temp_dir = std::env::temp_dir().join("lexicon-test-context");
        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .unwrap();

        let ctx = LexiconContext::new(config).unwrap();
        assert!(ctx.config().data_dir.exists());
        assert!(ctx.config().model_dir.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_context_invalid_directory() {
        // Test with a path that cannot be created (on Unix, /proc is read-only)
        #[cfg(unix)]
        {
            let config = LexiconConfig::builder()
                .data_dir(PathBuf::from("/proc/lexicon-cannot-create"))
                .build()
                .unwrap();

            let result = LexiconContext::new(config);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_context_creates_nested_directories() {
        let temp_dir = std::env::temp_dir().join("lexicon-nested-test");
        let nested_data = temp_dir.join("a").join("b").join("c").join("data");
        let nested_models = temp_dir.join("x").join("y").join("z").join("models");

        let config = LexiconConfig::builder()
            .data_dir(nested_data.clone())
            .model_dir(nested_models.clone())
            .build()
            .unwrap();

        let ctx = LexiconContext::new(config).unwrap();
        assert!(nested_data.exists());
        assert!(nested_models.exists());
        assert_eq!(ctx.config().data_dir, nested_data);
        assert_eq!(ctx.config().model_dir, nested_models);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_context_platform_detection() {
        let temp_dir = std::env::temp_dir().join("lexicon-platform-test");
        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .unwrap();

        let ctx = LexiconContext::new(config).unwrap();
        let platform = ctx.platform();

        // Should detect one of the supported platforms
        assert!(matches!(
            platform,
            Platform::MacOS | Platform::Windows | Platform::Linux | Platform::IOS | Platform::Android
        ));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_context_reuses_existing_directories() {
        let temp_dir = std::env::temp_dir().join("lexicon-existing-test");
        let data_dir = temp_dir.join("data");
        let model_dir = temp_dir.join("models");

        // Pre-create directories
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&model_dir).unwrap();

        // Create a marker file to verify directory wasn't recreated
        let marker_file = data_dir.join("marker.txt");
        std::fs::write(&marker_file, "test").unwrap();

        let config = LexiconConfig::builder()
            .data_dir(data_dir.clone())
            .model_dir(model_dir.clone())
            .build()
            .unwrap();

        let ctx = LexiconContext::new(config).unwrap();

        // Marker file should still exist
        assert!(marker_file.exists());
        assert!(ctx.config().data_dir.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_context_config_accessor() {
        let temp_dir = std::env::temp_dir().join("lexicon-config-access-test");
        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .log_level(LogLevel::Debug)
            .build()
            .unwrap();

        let ctx = LexiconContext::new(config).unwrap();

        // Verify config accessors work correctly
        assert_eq!(ctx.config().log_level, LogLevel::Debug);
        assert_eq!(ctx.config().data_dir, temp_dir);
        assert_eq!(ctx.config().model_dir, temp_dir.join("models"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_context_invalid_model_directory() {
        // Test with valid data dir but invalid model dir
        #[cfg(unix)]
        {
            let temp_dir = std::env::temp_dir().join("lexicon-invalid-model-test");
            let config = LexiconConfig::builder()
                .data_dir(temp_dir.clone())
                .model_dir(PathBuf::from("/proc/lexicon-model-cannot-create"))
                .build()
                .unwrap();

            let result = LexiconContext::new(config);
            assert!(result.is_err());

            // Data dir might have been created before model dir failed
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
    }
}
