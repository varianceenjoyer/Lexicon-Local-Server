//! Speech-to-Text Engine
//!
//! This module provides speech recognition using the Whisper model via whisper-rs.
//! It supports transcription, translation (X→English), and optional speaker diarization.
//!
//! # Architecture
//!
//! ```text
//! AudioChunk (16kHz mono) → Transcriber → Vec<Segment>
//! ```
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::stt::{WhisperTranscriber, Transcriber, TranscriptionConfig};
//! use std::path::Path;
//!
//! // Create transcriber
//! let mut transcriber = WhisperTranscriber::new()?;
//!
//! // Load model
//! transcriber.load_model(Path::new("models/whisper-base.bin"))?;
//!
//! // Configure transcription
//! let config = TranscriptionConfig::new()
//!     .with_language("en");
//!
//! // Transcribe audio (16kHz mono f32 samples)
//! let samples: Vec<f32> = vec![0.0; 16000]; // 1 second of audio
//! let result = transcriber.transcribe(&samples, &config)?;
//!
//! for segment in result.segments {
//!     println!("[{}-{}] {}", segment.start_ms, segment.end_ms, segment.text);
//! }
//! # Ok::<(), lexicon_core::error::LexiconError>(())
//! ```
//!
//! # GPU Acceleration
//!
//! The Whisper model supports GPU acceleration on:
//! - macOS: Metal (automatic)
//! - Linux: CUDA (requires CUDA toolkit)
//! - Windows: CUDA or Vulkan
//!
//! GPU support is enabled at compile time via whisper-rs features.

mod types;
mod whisper;

pub use types::{Segment, TranscriptionConfig, TranscriptionResult};
pub use whisper::WhisperTranscriber;

use crate::error::Result;
use std::path::Path;

/// Trait for speech-to-text transcription engines.
///
/// Implementations should:
/// - Accept 16kHz mono f32 audio samples
/// - Return timestamped segments with confidence scores
/// - Support translation mode (non-English → English)
/// - Handle GPU acceleration when available
pub trait Transcriber: Send {
    /// Loads a model from the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the model file (e.g., whisper-base.bin)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The model file doesn't exist
    /// - The model is invalid or corrupted
    /// - There's insufficient memory to load the model
    fn load_model(&mut self, path: &Path) -> Result<()>;

    /// Transcribes audio samples to text segments.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples at 16kHz mono f32 format
    /// * `config` - Transcription configuration
    ///
    /// # Returns
    ///
    /// A `TranscriptionResult` containing:
    /// - Timestamped segments with text
    /// - Detected language (if auto-detection was used)
    /// - Processing metrics
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No model is loaded
    /// - Audio is too short for processing
    /// - Inference fails
    fn transcribe(&self, samples: &[f32], config: &TranscriptionConfig) -> Result<TranscriptionResult>;

    /// Translates non-English audio to English text.
    ///
    /// This is a convenience method that sets `config.translate = true`.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples at 16kHz mono f32 format
    /// * `config` - Transcription configuration (translate flag will be set)
    ///
    /// # Errors
    ///
    /// Same as `transcribe`.
    fn translate(&self, samples: &[f32], config: &TranscriptionConfig) -> Result<TranscriptionResult> {
        let mut translate_config = config.clone();
        translate_config.translate = true;
        self.transcribe(samples, &translate_config)
    }

    /// Returns true if a model is currently loaded.
    fn is_model_loaded(&self) -> bool;

    /// Returns the name of the currently loaded model, if any.
    fn model_name(&self) -> Option<&str>;

    /// Unloads the current model to free memory.
    fn unload_model(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_exports() {
        let segment = Segment::new(0, 1000, "test");
        assert_eq!(segment.duration_ms(), 1000);
    }

    #[test]
    fn test_config_exports() {
        let config = TranscriptionConfig::new()
            .with_language("en");
        assert_eq!(config.language, Some("en".to_string()));
    }
}
