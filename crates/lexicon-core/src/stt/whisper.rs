//! Whisper Transcriber Implementation
//!
//! Provides speech-to-text using the whisper-rs bindings to whisper.cpp.

use std::path::Path;
use std::time::Instant;

use tracing::{debug, info};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::diarization::{assign_speakers, DiarizationConfig};
use super::{Segment, Transcriber, TranscriptionConfig, TranscriptionResult};
use crate::error::{InferenceError, LexiconError, Result};

/// Whisper-based speech-to-text transcriber.
///
/// Uses whisper-rs (whisper.cpp bindings) for efficient on-device transcription.
/// Supports multiple model sizes from tiny to large.
pub struct WhisperTranscriber {
    ctx: Option<WhisperContext>,
    model_path: Option<String>,
}

impl WhisperTranscriber {
    /// Creates a new Whisper transcriber without loading a model.
    ///
    /// Call `load_model` before attempting transcription.
    ///
    /// # Errors
    ///
    /// This method doesn't fail, but transcription will fail until a model is loaded.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            ctx: None,
            model_path: None,
        })
    }

    /// Creates a Whisper transcriber with a model already loaded.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the Whisper model file
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    pub fn with_model(model_path: &Path) -> Result<Self> {
        let mut transcriber = Self::new()?;
        transcriber.load_model(model_path)?;
        Ok(transcriber)
    }

    /// Converts whisper-rs segments to our Segment type.
    fn convert_segments(
        &self,
        state: &whisper_rs::WhisperState,
        num_segments: i32,
    ) -> Vec<Segment> {
        let mut segments = Vec::with_capacity(num_segments as usize);

        for i in 0..num_segments {
            let start_ms = state.full_get_segment_t0(i).unwrap_or(0) * 10; // Convert centiseconds to ms
            let end_ms = state.full_get_segment_t1(i).unwrap_or(0) * 10;
            let text = state.full_get_segment_text(i).unwrap_or_default();

            // Skip empty segments
            if text.trim().is_empty() {
                continue;
            }

            segments.push(Segment::new(start_ms, end_ms, text.trim()));
        }

        segments
    }
}

impl Default for WhisperTranscriber {
    fn default() -> Self {
        Self {
            ctx: None,
            model_path: None,
        }
    }
}

impl Transcriber for WhisperTranscriber {
    fn load_model(&mut self, path: &Path) -> Result<()> {
        info!("Loading Whisper model from {:?}", path);

        if !path.exists() {
            return Err(LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Model file not found: {}", path.display()),
            }));
        }

        let path_str = path.to_string_lossy();
        let params = WhisperContextParameters::default();

        let ctx = WhisperContext::new_with_params(&path_str, params).map_err(|e| {
            LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Failed to load Whisper model: {e}"),
            })
        })?;

        self.ctx = Some(ctx);
        self.model_path = Some(path_str.to_string());

        info!("Whisper model loaded successfully");
        Ok(())
    }

    fn transcribe(&self, samples: &[f32], config: &TranscriptionConfig) -> Result<TranscriptionResult> {
        let ctx = self.ctx.as_ref().ok_or_else(|| {
            LexiconError::Inference(InferenceError::ModelNotLoaded)
        })?;

        // Validate input
        if samples.is_empty() {
            return Err(LexiconError::Inference(InferenceError::AudioTooShort {
                min_seconds: 0.1,
                actual_seconds: 0.0,
            }));
        }

        let audio_duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;

        // Minimum audio length check (Whisper needs at least ~100ms)
        if audio_duration_ms < 100 {
            return Err(LexiconError::Inference(InferenceError::AudioTooShort {
                min_seconds: 0.1,
                actual_seconds: audio_duration_ms as f32 / 1000.0,
            }));
        }

        debug!(
            audio_duration_ms = audio_duration_ms,
            samples = samples.len(),
            "Starting transcription"
        );

        let start_time = Instant::now();

        // Create state for this transcription
        let mut state = ctx.create_state().map_err(|e| {
            LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Failed to create Whisper state: {e}"),
            })
        })?;

        // Configure parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Set language (None = auto-detect)
        if let Some(ref lang) = config.language {
            params.set_language(Some(lang));
        } else {
            params.set_language(None); // Auto-detect
        }

        // Translation mode
        params.set_translate(config.translate);

        // Threading
        if config.num_threads > 0 {
            params.set_n_threads(config.num_threads as i32);
        }

        // Initial prompt for context
        if let Some(ref prompt) = config.initial_prompt {
            params.set_initial_prompt(prompt);
        }

        // Suppress non-speech tokens
        params.set_suppress_nst(config.suppress_non_speech);

        // Don't print progress to stdout
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);

        // Run inference
        state.full(params, samples).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Whisper inference failed: {e}"),
            })
        })?;

        let processing_time_ms = start_time.elapsed().as_millis() as i64;

        // Extract segments
        let num_segments = state.full_n_segments().map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Failed to get segment count: {e}"),
            })
        })?;

        let mut segments = self.convert_segments(&state, num_segments);

        // Apply speaker diarization if enabled
        if config.enable_diarization && !segments.is_empty() {
            debug!("Applying speaker diarization to {} segments", segments.len());
            let diarization_config = DiarizationConfig::new()
                .with_max_speakers(config.max_speakers.unwrap_or(4));
            segments = assign_speakers(segments, samples, &diarization_config);
        }

        // Try to get detected language
        let detected_language = state.full_lang_id_from_state().ok().and_then(|id| {
            whisper_rs::get_lang_str(id).map(String::from)
        });

        let mut result = TranscriptionResult::new(segments, audio_duration_ms, processing_time_ms);

        if let Some(lang) = detected_language {
            result = result.with_detected_language(lang);
        }

        debug!(
            segments = result.segments.len(),
            processing_time_ms = processing_time_ms,
            rtf = result.real_time_factor(),
            "Transcription complete"
        );

        Ok(result)
    }

    fn is_model_loaded(&self) -> bool {
        self.ctx.is_some()
    }

    fn model_name(&self) -> Option<&str> {
        self.model_path.as_deref()
    }

    fn unload_model(&mut self) {
        if self.ctx.is_some() {
            info!("Unloading Whisper model");
        }
        self.ctx = None;
        self.model_path = None;
    }
}

impl std::fmt::Debug for WhisperTranscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhisperTranscriber")
            .field("model_loaded", &self.is_model_loaded())
            .field("model_path", &self.model_path)
            .finish()
    }
}

/// Mock transcriber for testing without a real Whisper model.
#[cfg(test)]
pub struct MockTranscriber {
    loaded: bool,
    response_segments: Vec<Segment>,
}

#[cfg(test)]
impl MockTranscriber {
    /// Creates a mock transcriber with predefined response segments.
    pub fn new(segments: Vec<Segment>) -> Self {
        Self {
            loaded: false,
            response_segments: segments,
        }
    }

    /// Creates a mock that returns a simple "Hello world" transcription.
    pub fn hello_world() -> Self {
        Self::new(vec![Segment::new(0, 1000, "Hello world")])
    }
}

#[cfg(test)]
impl Transcriber for MockTranscriber {
    fn load_model(&mut self, _path: &Path) -> Result<()> {
        self.loaded = true;
        Ok(())
    }

    fn transcribe(&self, samples: &[f32], _config: &TranscriptionConfig) -> Result<TranscriptionResult> {
        if !self.loaded {
            return Err(LexiconError::Inference(InferenceError::ModelNotLoaded));
        }

        let audio_duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;
        Ok(TranscriptionResult::new(
            self.response_segments.clone(),
            audio_duration_ms,
            50, // Mock 50ms processing time
        ))
    }

    fn is_model_loaded(&self) -> bool {
        self.loaded
    }

    fn model_name(&self) -> Option<&str> {
        if self.loaded {
            Some("mock-model")
        } else {
            None
        }
    }

    fn unload_model(&mut self) {
        self.loaded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_transcriber() {
        let mut transcriber = MockTranscriber::hello_world();

        // Should fail without model
        let config = TranscriptionConfig::new();
        let samples = vec![0.0f32; 16000];
        assert!(transcriber.transcribe(&samples, &config).is_err());

        // Load model
        transcriber.load_model(Path::new("fake.bin")).unwrap();
        assert!(transcriber.is_model_loaded());
        assert_eq!(transcriber.model_name(), Some("mock-model"));

        // Should succeed after loading
        let result = transcriber.transcribe(&samples, &config).unwrap();
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].text, "Hello world");
        assert_eq!(result.audio_duration_ms, 1000);

        // Unload
        transcriber.unload_model();
        assert!(!transcriber.is_model_loaded());
    }

    #[test]
    fn test_whisper_transcriber_no_model() {
        let transcriber = WhisperTranscriber::new().unwrap();
        assert!(!transcriber.is_model_loaded());
        assert!(transcriber.model_name().is_none());

        let config = TranscriptionConfig::new();
        let samples = vec![0.0f32; 16000];

        // Should fail without model
        let result = transcriber.transcribe(&samples, &config);
        assert!(matches!(
            result,
            Err(LexiconError::Inference(InferenceError::ModelNotLoaded))
        ));
    }

    #[test]
    fn test_whisper_transcriber_missing_model_file() {
        let mut transcriber = WhisperTranscriber::new().unwrap();

        let result = transcriber.load_model(Path::new("/nonexistent/model.bin"));
        assert!(result.is_err());
    }
}
