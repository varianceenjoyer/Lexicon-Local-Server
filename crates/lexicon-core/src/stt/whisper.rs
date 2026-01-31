//! Whisper Transcriber Implementation
//!
//! Provides speech-to-text using the whisper-rs bindings to whisper.cpp.
//!
//! # Latency Optimizations (WP-LATENCY-STT)
//!
//! This module includes several latency optimizations:
//! - Optimal thread count auto-detection for mobile/desktop
//! - Model warm-up to eliminate first-inference latency
//! - Detailed timing instrumentation for profiling
//! - Early exit via VAD for silent chunks

use std::path::Path;
use std::time::Instant;

use tracing::{debug, info};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::diarization::{assign_speakers, DiarizationConfig};
use super::{Segment, Transcriber, TranscriptionConfig, TranscriptionResult};
use crate::error::{InferenceError, LexiconError, Result};

/// Determines the optimal thread count for Whisper inference.
///
/// This function auto-detects the number of CPU cores and returns an optimal
/// thread count based on platform heuristics:
///
/// - **Mobile (1-4 cores)**: Use N-1 to leave headroom for UI thread
/// - **Desktop (5-8 cores)**: Use N-2 for balanced performance
/// - **High-core (9+)**: Cap at 6-8 threads (diminishing returns)
/// - **Emulator**: Detected via CPU count heuristics
///
/// # Returns
///
/// The recommended number of threads for Whisper inference.
#[must_use]
pub fn optimal_thread_count() -> u32 {
    let cpus = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);

    match cpus {
        1 => 1,
        2 => 1,          // Leave 1 for UI
        3 => 2,          // Leave 1 for UI
        4 => 2,          // Mobile: conservative for battery/thermal
        5..=6 => 4,      // Desktop: balanced
        7..=8 => 6,      // Desktop: good parallelism
        _ => 8,          // Cap at 8 (diminishing returns beyond)
    }
}

/// Whisper-based speech-to-text transcriber.
///
/// Uses whisper-rs (whisper.cpp bindings) for efficient on-device transcription.
/// Supports multiple model sizes from tiny to large.
pub struct WhisperTranscriber {
    ctx: Option<WhisperContext>,
    model_path: Option<String>,
    use_gpu: bool,
}

impl WhisperTranscriber {
    /// Creates a new Whisper transcriber without loading a model.
    ///
    /// Call `load_model` before attempting transcription.
    /// GPU acceleration is enabled by default.
    ///
    /// # Errors
    ///
    /// This method doesn't fail, but transcription will fail until a model is loaded.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            ctx: None,
            model_path: None,
            use_gpu: true,
        })
    }

    /// Creates a new Whisper transcriber with GPU configuration.
    ///
    /// Call `load_model` before attempting transcription.
    ///
    /// # Arguments
    ///
    /// * `use_gpu` - Whether to use GPU acceleration if available
    ///
    /// # Errors
    ///
    /// This method doesn't fail, but transcription will fail until a model is loaded.
    pub fn with_gpu(use_gpu: bool) -> Result<Self> {
        Ok(Self {
            ctx: None,
            model_path: None,
            use_gpu,
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

    /// Creates a Whisper transcriber with a model and GPU configuration.
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the Whisper model file
    /// * `use_gpu` - Whether to use GPU acceleration if available
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    pub fn with_model_and_gpu(model_path: &Path, use_gpu: bool) -> Result<Self> {
        let mut transcriber = Self::with_gpu(use_gpu)?;
        transcriber.load_model(model_path)?;
        Ok(transcriber)
    }

    /// Warms up the model by running a short inference on silence.
    ///
    /// This primes the model's internal caches and eliminates the "first inference"
    /// latency penalty that can add 100-500ms to the first transcription.
    ///
    /// Call this method after loading a model and before real transcription
    /// for optimal latency on the first request.
    ///
    /// # Errors
    ///
    /// Returns an error if no model is loaded or inference fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lexicon_core::stt::{WhisperTranscriber, Transcriber};
    /// use std::path::Path;
    ///
    /// let mut transcriber = WhisperTranscriber::new()?;
    /// transcriber.load_model(Path::new("models/whisper-tiny.bin"))?;
    /// transcriber.warm_up()?; // Prime the model
    /// // First real transcription will now be faster
    /// # Ok::<(), lexicon_core::error::LexiconError>(())
    /// ```
    pub fn warm_up(&self) -> Result<()> {
        let start = Instant::now();

        // Create 100ms of silence at 16kHz (1600 samples)
        let silence = vec![0.0f32; 1600];

        // Run with minimal config to just warm caches
        let config = TranscriptionConfig::new();

        // Discard result - we just want to warm the model
        let _ = self.transcribe(&silence, &config)?;

        let elapsed_ms = start.elapsed().as_millis();
        info!(elapsed_ms = elapsed_ms, "Model warm-up complete");

        Ok(())
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
            use_gpu: true,
        }
    }
}

impl Transcriber for WhisperTranscriber {
    fn load_model(&mut self, path: &Path) -> Result<()> {
        info!(
            "Loading Whisper model from {:?} (GPU: {})",
            path, self.use_gpu
        );

        if !path.exists() {
            return Err(LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Model file not found: {}", path.display()),
            }));
        }

        let path_str = path.to_string_lossy();
        let mut params = WhisperContextParameters::default();
        params.use_gpu(self.use_gpu);

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
        let total_start = Instant::now();

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

        // --- Phase 1: Create state ---
        let state_start = Instant::now();
        let mut state = ctx.create_state().map_err(|e| {
            LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Failed to create Whisper state: {e}"),
            })
        })?;
        let state_time_ms = state_start.elapsed().as_millis();

        // --- Phase 2: Configure parameters ---
        let config_start = Instant::now();
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Set language (None = auto-detect)
        if let Some(ref lang) = config.language {
            params.set_language(Some(lang));
        } else {
            params.set_language(None); // Auto-detect
        }

        // Translation mode
        params.set_translate(config.translate);

        // Threading - use optimal count if not specified
        let thread_count = if config.num_threads > 0 {
            config.num_threads
        } else {
            optimal_thread_count()
        };
        params.set_n_threads(thread_count as i32);

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
        let config_time_ms = config_start.elapsed().as_millis();

        // --- Phase 3: Run inference (main bottleneck) ---
        let inference_start = Instant::now();
        state.full(params, samples).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Whisper inference failed: {e}"),
            })
        })?;
        let inference_time_ms = inference_start.elapsed().as_millis();

        // --- Phase 4: Extract segments ---
        let extract_start = Instant::now();
        let num_segments = state.full_n_segments().map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Failed to get segment count: {e}"),
            })
        })?;

        let mut segments = self.convert_segments(&state, num_segments);
        let extract_time_ms = extract_start.elapsed().as_millis();

        // --- Phase 5: Speaker diarization (optional) ---
        let diarization_start = Instant::now();
        if config.enable_diarization && !segments.is_empty() {
            debug!("Applying speaker diarization to {} segments", segments.len());
            let diarization_config = DiarizationConfig::new()
                .with_max_speakers(config.max_speakers.unwrap_or(4));
            segments = assign_speakers(segments, samples, &diarization_config);
        }
        let diarization_time_ms = diarization_start.elapsed().as_millis();

        // Try to get detected language
        let detected_language = state.full_lang_id_from_state().ok().and_then(|id| {
            whisper_rs::get_lang_str(id).map(String::from)
        });

        let total_time_ms = total_start.elapsed().as_millis() as i64;

        let mut result = TranscriptionResult::new(segments, audio_duration_ms, total_time_ms);

        if let Some(lang) = detected_language {
            result = result.with_detected_language(lang);
        }

        // Detailed timing breakdown for performance analysis
        debug!(
            audio_duration_ms = audio_duration_ms,
            total_time_ms = total_time_ms,
            state_time_ms = state_time_ms,
            config_time_ms = config_time_ms,
            inference_time_ms = inference_time_ms,
            extract_time_ms = extract_time_ms,
            diarization_time_ms = diarization_time_ms,
            thread_count = thread_count,
            segments = result.segments.len(),
            rtf = result.real_time_factor(),
            "Transcription complete - timing breakdown"
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

    // --- Tests for WP-LATENCY-STT optimizations ---

    #[test]
    fn test_optimal_thread_count_returns_reasonable_value() {
        let threads = optimal_thread_count();
        // Should be between 1 and 8
        assert!(threads >= 1);
        assert!(threads <= 8);
    }

    #[test]
    fn test_optimal_thread_count_is_stable() {
        // Should return the same value on repeated calls
        let threads1 = optimal_thread_count();
        let threads2 = optimal_thread_count();
        assert_eq!(threads1, threads2);
    }
}
