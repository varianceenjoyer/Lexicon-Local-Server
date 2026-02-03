//! Sherpa-ONNX Transcriber Implementation
//!
//! Provides speech-to-text using sherpa-rs bindings to sherpa-onnx.
//! Uses streaming Zipformer architecture for low-latency transcription.
//!
//! # Architecture Comparison
//!
//! | Engine | Architecture | Latency | Best For |
//! |--------|--------------|---------|----------|
//! | Whisper | Batch (30s chunks) | ~3000ms | Accuracy, translation |
//! | Sherpa | Streaming (incremental) | ~300-500ms | Real-time, low latency |
//!
//! # Model Requirements
//!
//! Sherpa uses transducer models with three components:
//! - `encoder-*.onnx` - Audio encoder (largest, ~50MB)
//! - `decoder-*.onnx` - Token decoder (~5MB)
//! - `joiner-*.onnx` - Encoder-decoder connector (~1MB)
//! - `tokens.txt` - Vocabulary file
//!
//! Recommended model: `sherpa-onnx-streaming-zipformer-en-20M-2023-06-26`
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::stt::{SherpaTranscriber, Transcriber, TranscriptionConfig};
//! use std::path::Path;
//!
//! // Create transcriber pointing to model directory
//! let mut transcriber = SherpaTranscriber::new()?;
//! transcriber.load_model(Path::new("models/sherpa"))?;
//!
//! // Transcribe audio (16kHz mono f32 samples)
//! let config = TranscriptionConfig::new().with_language("en");
//! let samples: Vec<f32> = vec![0.0; 16000]; // 1 second
//! let result = transcriber.transcribe(&samples, &config)?;
//! # Ok::<(), lexicon_core::error::LexiconError>(())
//! ```

use std::cell::RefCell;
use std::path::Path;
use std::time::Instant;

use sherpa_rs::zipformer::{ZipFormer, ZipFormerConfig};
use tracing::{debug, info};

use super::{Segment, Transcriber, TranscriptionConfig, TranscriptionResult};
use crate::error::{InferenceError, LexiconError, Result};

/// Sherpa-ONNX based speech-to-text transcriber.
///
/// Uses streaming Zipformer architecture for low-latency transcription.
/// This is recommended for real-time use cases where latency is critical.
///
/// # Latency Comparison
///
/// On ARM64 physical device (Samsung RFCR8069FBK):
/// - Whisper (tiny-q5_1): ~3250ms per 3s audio
/// - Sherpa (Zipformer-20M): ~300-500ms per 3s audio (expected)
///
/// # Note on Interior Mutability
///
/// Uses `RefCell` because `ZipFormer::decode` requires `&mut self` but the
/// `Transcriber` trait uses `&self` for transcription. This is safe for
/// single-threaded use. For multi-threaded use, consider wrapping in a Mutex.
pub struct SherpaTranscriber {
    /// The Zipformer model instance (None until loaded).
    /// Uses RefCell because ZipFormer::decode requires &mut self.
    zipformer: RefCell<Option<ZipFormer>>,

    /// Path to the model directory.
    model_dir: Option<String>,

    /// Number of threads to use (0 = auto).
    num_threads: u32,
}

impl SherpaTranscriber {
    /// Creates a new Sherpa transcriber without loading a model.
    ///
    /// Call `load_model` before attempting transcription.
    ///
    /// # Errors
    ///
    /// This method doesn't fail, but transcription will fail until a model is loaded.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            zipformer: RefCell::new(None),
            model_dir: None,
            num_threads: 0, // Auto-detect
        })
    }

    /// Creates a new Sherpa transcriber with specified thread count.
    ///
    /// # Arguments
    ///
    /// * `num_threads` - Number of threads (0 = auto-detect)
    pub fn with_threads(num_threads: u32) -> Result<Self> {
        Ok(Self {
            zipformer: RefCell::new(None),
            model_dir: None,
            num_threads,
        })
    }

    /// Creates a Sherpa transcriber with a model already loaded.
    ///
    /// # Arguments
    ///
    /// * `model_dir` - Path to directory containing encoder, decoder, joiner, tokens
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    pub fn with_model(model_dir: &Path) -> Result<Self> {
        let mut transcriber = Self::new()?;
        transcriber.load_model(model_dir)?;
        Ok(transcriber)
    }

    /// Warms up the model by running a short inference on silence.
    ///
    /// This primes the model's internal caches and eliminates the "first inference"
    /// latency penalty.
    ///
    /// # Errors
    ///
    /// Returns an error if no model is loaded or inference fails.
    pub fn warm_up(&self) -> Result<()> {
        let start = Instant::now();

        // Create 100ms of silence at 16kHz (1600 samples)
        let silence = vec![0.0f32; 1600];

        // Run with minimal config to just warm caches
        let config = TranscriptionConfig::new();

        // Discard result - we just want to warm the model
        let _ = self.transcribe(&silence, &config)?;

        let elapsed_ms = start.elapsed().as_millis();
        info!(elapsed_ms = elapsed_ms, "Sherpa model warm-up complete");

        Ok(())
    }

    /// Finds the model files in a directory.
    ///
    /// Looks for encoder, decoder, joiner, and tokens files.
    fn find_model_files(model_dir: &Path) -> Result<(String, String, String, String)> {
        let dir = model_dir.to_string_lossy();

        // Look for model files with common naming patterns
        let encoder = Self::find_file(model_dir, &["encoder", "encoder-epoch"], ".onnx")?;
        let decoder = Self::find_file(model_dir, &["decoder", "decoder-epoch"], ".onnx")?;
        let joiner = Self::find_file(model_dir, &["joiner", "joiner-epoch"], ".onnx")?;
        let tokens = Self::find_file(model_dir, &["tokens"], ".txt")?;

        debug!(
            encoder = %encoder,
            decoder = %decoder,
            joiner = %joiner,
            tokens = %tokens,
            "Found Sherpa model files in {}",
            dir
        );

        Ok((encoder, decoder, joiner, tokens))
    }

    /// Finds a file matching a pattern in a directory.
    ///
    /// Prefers int8 quantized models for mobile efficiency.
    fn find_file(dir: &Path, prefixes: &[&str], suffix: &str) -> Result<String> {
        let mut candidates = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                for prefix in prefixes {
                    if name.starts_with(prefix) && name.ends_with(suffix) {
                        candidates.push(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }

        if candidates.is_empty() {
            return Err(LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!(
                    "Could not find file matching {:?}*{} in {}",
                    prefixes,
                    suffix,
                    dir.display()
                ),
            }));
        }

        // Prefer int8 quantized models for mobile efficiency
        if let Some(int8_path) = candidates.iter().find(|p| p.contains("int8")) {
            return Ok(int8_path.clone());
        }

        // Fall back to first match
        Ok(candidates.into_iter().next().unwrap())
    }
}

impl Default for SherpaTranscriber {
    fn default() -> Self {
        Self {
            zipformer: RefCell::new(None),
            model_dir: None,
            num_threads: 0,
        }
    }
}

impl Transcriber for SherpaTranscriber {
    fn load_model(&mut self, path: &Path) -> Result<()> {
        info!("Loading Sherpa model from {:?}", path);

        if !path.exists() {
            return Err(LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Model directory not found: {}", path.display()),
            }));
        }

        // Find model files in directory
        let (encoder, decoder, joiner, tokens) = Self::find_model_files(path)?;

        // Determine thread count
        let threads = if self.num_threads > 0 {
            self.num_threads as i32
        } else {
            // Auto-detect: use N-1 to leave headroom for UI
            let cpus = std::thread::available_parallelism()
                .map(|p| p.get() as i32)
                .unwrap_or(4);
            (cpus - 1).max(1)
        };

        // Use CPU provider on all platforms
        // Note: NNAPI was tested but showed 70x slower inference than CPU on Snapdragon 888
        // The DSP/NPU compilation overhead outweighs any potential acceleration benefits
        let config = ZipFormerConfig {
            encoder: encoder.into(),
            decoder: decoder.into(),
            joiner: joiner.into(),
            tokens: tokens.into(),
            num_threads: Some(threads),
            ..Default::default()
        };

        let zipformer = ZipFormer::new(config).map_err(|e| {
            LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Failed to create Sherpa ZipFormer: {e}"),
            })
        })?;

        *self.zipformer.borrow_mut() = Some(zipformer);
        self.model_dir = Some(path.to_string_lossy().to_string());

        info!("Sherpa model loaded successfully (threads={})", threads);
        Ok(())
    }

    fn transcribe(&self, samples: &[f32], _config: &TranscriptionConfig) -> Result<TranscriptionResult> {
        let total_start = Instant::now();

        let mut zipformer_ref = self.zipformer.borrow_mut();
        let zipformer = zipformer_ref.as_mut().ok_or_else(|| {
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

        // Minimum audio length check
        if audio_duration_ms < 100 {
            return Err(LexiconError::Inference(InferenceError::AudioTooShort {
                min_seconds: 0.1,
                actual_seconds: audio_duration_ms as f32 / 1000.0,
            }));
        }

        debug!(
            audio_duration_ms = audio_duration_ms,
            samples = samples.len(),
            "Starting Sherpa transcription"
        );

        // --- Run inference ---
        let inference_start = Instant::now();

        // ZipFormer expects owned Vec<f32> for decode
        let text = zipformer.decode(16000, samples.to_vec());
        let inference_time_ms = inference_start.elapsed().as_millis();

        let total_time_ms = total_start.elapsed().as_millis() as i64;

        // Create segments from result
        // Note: Sherpa returns full text without segment timing in basic mode
        // For now, treat entire transcription as one segment
        let segments = if text.trim().is_empty() {
            vec![]
        } else {
            vec![Segment::new(0, audio_duration_ms, text.trim())]
        };

        let result = TranscriptionResult::new(segments, audio_duration_ms, total_time_ms);

        debug!(
            audio_duration_ms = audio_duration_ms,
            total_time_ms = total_time_ms,
            inference_time_ms = inference_time_ms,
            text_len = text.len(),
            rtf = result.real_time_factor(),
            "Sherpa transcription complete"
        );

        Ok(result)
    }

    fn is_model_loaded(&self) -> bool {
        self.zipformer.borrow().is_some()
    }

    fn model_name(&self) -> Option<&str> {
        self.model_dir.as_deref()
    }

    fn unload_model(&mut self) {
        if self.zipformer.borrow().is_some() {
            info!("Unloading Sherpa model");
        }
        *self.zipformer.borrow_mut() = None;
        self.model_dir = None;
    }
}

impl std::fmt::Debug for SherpaTranscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SherpaTranscriber")
            .field("model_loaded", &self.is_model_loaded())
            .field("model_dir", &self.model_dir)
            .field("num_threads", &self.num_threads)
            .finish()
    }
}

/// Configuration for Sherpa-ONNX streaming transcriber.
///
/// This extends the base `TranscriptionConfig` with Sherpa-specific options.
#[derive(Debug, Clone)]
pub struct SherpaConfig {
    /// Path to model directory containing encoder, decoder, joiner, tokens.
    pub model_dir: std::path::PathBuf,

    /// Number of threads (0 = auto-detect).
    pub num_threads: u32,

    /// Enable NNAPI acceleration on Android.
    pub enable_nnapi: bool,

    /// Decoding method: "greedy" or "modified_beam_search".
    pub decoding_method: String,

    /// Maximum active paths for beam search.
    pub max_active_paths: u32,
}

impl Default for SherpaConfig {
    fn default() -> Self {
        Self {
            model_dir: std::path::PathBuf::new(),
            num_threads: 0, // auto
            enable_nnapi: cfg!(target_os = "android"),
            decoding_method: "greedy".to_string(),
            max_active_paths: 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sherpa_transcriber_creation() {
        let transcriber = SherpaTranscriber::new().unwrap();
        assert!(!transcriber.is_model_loaded());
        assert!(transcriber.model_name().is_none());
    }

    #[test]
    fn test_sherpa_transcriber_no_model() {
        let transcriber = SherpaTranscriber::new().unwrap();

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
    fn test_sherpa_transcriber_missing_model_dir() {
        let mut transcriber = SherpaTranscriber::new().unwrap();

        let result = transcriber.load_model(Path::new("/nonexistent/model/dir"));
        assert!(result.is_err());
    }

    #[test]
    fn test_sherpa_transcriber_empty_audio() {
        let transcriber = SherpaTranscriber::default();
        // Even without model loaded, empty audio should be rejected
        let config = TranscriptionConfig::new();
        let samples: Vec<f32> = vec![];

        let result = transcriber.transcribe(&samples, &config);
        // Will fail either because no model or empty audio
        assert!(result.is_err());
    }

    #[test]
    fn test_sherpa_config_defaults() {
        let config = SherpaConfig::default();

        assert!(config.model_dir.as_os_str().is_empty());
        assert_eq!(config.num_threads, 0);
        assert_eq!(config.decoding_method, "greedy");
        assert_eq!(config.max_active_paths, 4);
    }

    #[test]
    fn test_sherpa_transcriber_with_threads() {
        let transcriber = SherpaTranscriber::with_threads(4).unwrap();
        assert_eq!(transcriber.num_threads, 4);
    }

    #[test]
    fn test_sherpa_transcriber_debug() {
        let transcriber = SherpaTranscriber::new().unwrap();
        let debug_str = format!("{:?}", transcriber);
        assert!(debug_str.contains("SherpaTranscriber"));
        assert!(debug_str.contains("model_loaded: false"));
    }

    /// Integration test that loads a real model if available.
    /// Run with: cargo test --features stt-sherpa test_sherpa_with_real_model -- --ignored
    ///
    /// Note: sherpa-rs ZipFormer only supports OFFLINE models, not streaming.
    /// Use the offline Gigaspeech model for testing:
    /// ```bash
    /// mkdir -p models/sherpa-offline && cd models/sherpa-offline
    /// wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-zipformer-gigaspeech-2023-12-12.tar.bz2
    /// tar xjf sherpa-onnx-zipformer-gigaspeech-2023-12-12.tar.bz2
    /// ```
    #[test]
    #[ignore = "requires offline model download (see doc comment)"]
    fn test_sherpa_with_real_model() {
        use super::Transcriber as _;

        // Model path is relative to workspace root, not crate directory
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent() // crates/
            .and_then(|p| p.parent()) // workspace root
            .expect("Could not find workspace root");

        // Use offline model (sherpa-rs ZipFormer only supports offline models)
        let model_dir = workspace_root.join("models/sherpa-offline/sherpa-onnx-zipformer-gigaspeech-2023-12-12");

        if !model_dir.exists() {
            eprintln!("Model not found at {:?}.", model_dir);
            eprintln!("Download with:");
            eprintln!("  wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-zipformer-gigaspeech-2023-12-12.tar.bz2");
            return;
        }

        // Load model
        let mut transcriber = SherpaTranscriber::new().unwrap();
        transcriber.load_model(&model_dir).expect("Failed to load model");
        assert!(transcriber.is_model_loaded());

        // Generate 1 second of silence at 16kHz
        let samples = vec![0.0f32; 16000];
        let config = TranscriptionConfig::new();

        // Transcribe
        let result = transcriber.transcribe(&samples, &config)
            .expect("Transcription failed");

        eprintln!("Transcription result: {:?}", result);
        eprintln!("Processing time: {}ms", result.processing_time_ms);
        eprintln!("RTF: {:.3}", result.real_time_factor());

        // Silent audio should produce empty or minimal transcription
        assert!(result.audio_duration_ms == 1000);
    }
}
