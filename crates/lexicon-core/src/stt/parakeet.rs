//! Parakeet ONNX Transcriber Implementation
//!
//! Provides speech-to-text using parakeet-rs bindings to NVIDIA Parakeet models.
//! Supports CTC (English, built-in punctuation) and TDT (25 languages) variants.
//!
//! # Architecture Comparison
//!
//! | Engine | Architecture | Punctuation | Languages | Runtime |
//! |--------|--------------|-------------|-----------|---------|
//! | Whisper | Batch (30s) | Post-process | 99 | whisper.cpp |
//! | Sherpa | Transducer | Post-process | English | sherpa-onnx |
//! | Parakeet CTC | CTC encoder | Post-process | English | ort (shared) |
//! | Parakeet TDT | Transducer | Post-process | 25 | ort (shared) |
//!
//! # Model Requirements
//!
//! Parakeet models are directories containing:
//! - `model.onnx` or `encoder.onnx` - The ONNX model file
//! - `tokenizer.json` - HuggingFace tokenizer
//! - `config.json` / `preprocessor_config.json` - Model configuration
//!
//! # Shared ONNX Runtime
//!
//! Unlike Whisper (bundled whisper.cpp) and Sherpa (bundled sherpa-onnx),
//! Parakeet shares the `ort` crate with TTS. This means one ONNX Runtime
//! instance for both STT and TTS, reducing memory usage.
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::stt::{ParakeetTranscriber, Transcriber, TranscriptionConfig};
//! use std::path::Path;
//!
//! let mut transcriber = ParakeetTranscriber::new()?;
//! transcriber.load_model(Path::new("models/parakeet-ctc-0.6b"))?;
//!
//! let config = TranscriptionConfig::new().with_language("en");
//! let samples: Vec<f32> = vec![0.0; 16000]; // 1 second
//! let result = transcriber.transcribe(&samples, &config)?;
//! # Ok::<(), lexicon_core::error::LexiconError>(())
//! ```

use std::sync::Mutex;
use std::path::Path;
use std::time::Instant;

use parakeet_rs::{self, TimestampMode, Transcriber as ParakeetTranscriberTrait};
use tracing::{debug, info, warn};

use super::{Segment, Transcriber, TranscriptionConfig, TranscriptionResult};
use crate::error::{InferenceError, LexiconError, Result};

/// Parakeet model variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParakeetVariant {
    /// CTC: English-only, batch mode. Outputs lowercase without punctuation.
    /// ~4-5 minute audio limit per call.
    Ctc,
    /// TDT: 25 languages, auto language detection, batch mode.
    /// ~4-5 minute audio limit per call.
    Tdt,
}

/// The loaded parakeet model instance.
enum ParakeetModel {
    Ctc(parakeet_rs::Parakeet),
    Tdt(parakeet_rs::ParakeetTDT),
}

/// Parakeet ONNX speech-to-text transcriber.
///
/// Uses NVIDIA Parakeet models via ONNX Runtime for transcription.
/// Supports CTC (English with built-in punctuation) and TDT (multilingual).
///
/// # ONNX Runtime Sharing
///
/// Shares the `ort` ONNX Runtime with the TTS subsystem. Both use
/// `ort = "2.0.0-rc.11"`, so a single runtime instance serves both.
///
/// # Interior Mutability
///
/// Uses `Mutex` because parakeet-rs `transcribe_samples` requires `&mut self`
/// but the `Transcriber` trait uses `&self`. The Mutex makes this type `Sync`,
/// allowing it to be used in async/multi-threaded server contexts.
pub struct ParakeetTranscriber {
    model: Mutex<Option<ParakeetModel>>,
    model_dir: Option<String>,
    variant: Option<ParakeetVariant>,
}

impl ParakeetTranscriber {
    /// Creates a new Parakeet transcriber without loading a model.
    ///
    /// The model variant (CTC or TDT) is auto-detected when `load_model` is called.
    /// Call `load_model` before attempting transcription.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Result<Self> {
        Ok(Self {
            model: Mutex::new(None),
            model_dir: None,
            variant: None,
        })
    }

    /// Creates a Parakeet transcriber with a model already loaded.
    ///
    /// # Arguments
    ///
    /// * `model_dir` - Path to directory containing the ONNX model files
    pub fn with_model(model_dir: &Path) -> Result<Self> {
        let mut transcriber = Self::new()?;
        transcriber.load_model(model_dir)?;
        Ok(transcriber)
    }

    /// Returns the model variant (CTC or TDT), if a model is loaded.
    #[must_use]
    pub fn variant(&self) -> Option<ParakeetVariant> {
        self.variant
    }

    /// Returns true if this model variant produces punctuated/capitalized output.
    ///
    /// Currently returns false for all variants:
    /// - CTC models output lowercase text without punctuation (despite HuggingFace
    ///   description claiming "built-in punctuation"). Post-processing is needed.
    /// - TDT models also need post-processing.
    #[must_use]
    pub fn has_builtin_punctuation(&self) -> bool {
        false
    }

    /// Warms up the model by running a short inference on silence.
    ///
    /// Eliminates the "first inference" latency penalty by priming caches.
    pub fn warm_up(&self) -> Result<()> {
        let start = Instant::now();
        let silence = vec![0.0f32; 1600]; // 100ms at 16kHz
        let config = TranscriptionConfig::new();
        let _ = self.transcribe(&silence, &config)?;
        let elapsed_ms = start.elapsed().as_millis();
        info!(elapsed_ms = elapsed_ms, "Parakeet model warm-up complete");
        Ok(())
    }

    /// Detects the model variant from the directory contents.
    ///
    /// Heuristic: if the directory contains `decoder*.onnx` or `joiner*.onnx`,
    /// it's a TDT (transducer) model. Otherwise it's CTC (encoder-only).
    fn detect_variant(model_dir: &Path) -> ParakeetVariant {
        if let Ok(entries) = std::fs::read_dir(model_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if (name.starts_with("decoder") || name.starts_with("joiner"))
                    && (name.ends_with(".onnx") || name.ends_with(".ort"))
                {
                    return ParakeetVariant::Tdt;
                }
            }
        }
        ParakeetVariant::Ctc
    }

    /// Finds the best model file in a directory.
    ///
    /// Prefers .ort (optimized ORT format) over .onnx for faster loading.
    /// Priority: int8*.ort > *.ort > int8*.onnx > *.onnx
    fn find_model_file(dir: &Path) -> Option<PathBuf> {
        let mut ort_files = Vec::new();
        let mut onnx_files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                let path = entry.path();
                if name.ends_with(".ort") && !name.contains("runtime_opt") {
                    ort_files.push(path);
                } else if name.ends_with(".onnx") && !name.ends_with(".onnx_data") {
                    onnx_files.push(path);
                }
            }
        }

        // Prefer .ort files, then .onnx
        if !ort_files.is_empty() {
            // Prefer int8 variant
            if let Some(p) = ort_files.iter().find(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().contains("int8"))
                    .unwrap_or(false)
            }) {
                return Some(p.clone());
            }
            return Some(ort_files.remove(0));
        }

        // Fall back to .onnx (parakeet-rs handles these natively via directory)
        None
    }
}

impl Default for ParakeetTranscriber {
    fn default() -> Self {
        Self {
            model: Mutex::new(None),
            model_dir: None,
            variant: None,
        }
    }
}

impl Transcriber for ParakeetTranscriber {
    fn load_model(&mut self, path: &Path) -> Result<()> {
        info!("Loading Parakeet model from {:?}", path);

        if !path.exists() {
            return Err(LexiconError::Inference(
                InferenceError::ContextCreationFailed {
                    message: format!("Model directory not found: {}", path.display()),
                },
            ));
        }

        // Auto-detect variant from directory contents
        let variant = Self::detect_variant(path);
        info!(variant = ?variant, "Detected Parakeet model variant");

        // Resolve model file: prefer .ort (faster loading) over .onnx
        // parakeet-rs only searches for *.onnx in directories, so we resolve
        // .ort files explicitly and pass the file path directly.
        let path_str = if path.is_dir() {
            if let Some(ort_file) = Self::find_model_file(path) {
                info!("Using optimized model: {}", ort_file.display());
                ort_file.to_string_lossy().to_string()
            } else {
                path.to_string_lossy().to_string()
            }
        } else {
            path.to_string_lossy().to_string()
        };
        let model = match variant {
            ParakeetVariant::Ctc => {
                let m = parakeet_rs::Parakeet::from_pretrained(&path_str, None).map_err(|e| {
                    LexiconError::Inference(InferenceError::ContextCreationFailed {
                        message: format!("Failed to load Parakeet CTC model: {e}"),
                    })
                })?;
                ParakeetModel::Ctc(m)
            }
            ParakeetVariant::Tdt => {
                let m =
                    parakeet_rs::ParakeetTDT::from_pretrained(&path_str, None).map_err(|e| {
                        LexiconError::Inference(InferenceError::ContextCreationFailed {
                            message: format!("Failed to load Parakeet TDT model: {e}"),
                        })
                    })?;
                ParakeetModel::Tdt(m)
            }
        };

        *self.model.lock().expect("model lock poisoned") = Some(model);
        self.model_dir = Some(path_str);
        self.variant = Some(variant);

        info!(variant = ?variant, "Parakeet model loaded successfully");
        Ok(())
    }

    fn transcribe(
        &self,
        samples: &[f32],
        _config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        let total_start = Instant::now();

        let mut model_ref = self.model.lock().expect("model lock poisoned");
        let model = model_ref.as_mut().ok_or_else(|| {
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

        if audio_duration_ms < 100 {
            return Err(LexiconError::Inference(InferenceError::AudioTooShort {
                min_seconds: 0.1,
                actual_seconds: audio_duration_ms as f32 / 1000.0,
            }));
        }

        // Warn about audio length limit
        if audio_duration_ms > 300_000 {
            warn!(
                audio_duration_ms = audio_duration_ms,
                "Audio exceeds ~5 minute recommended limit for Parakeet batch models"
            );
        }

        debug!(
            audio_duration_ms = audio_duration_ms,
            samples = samples.len(),
            "Starting Parakeet transcription"
        );

        // Run inference with word-level timestamps
        let inference_start = Instant::now();
        let pk_result = match model {
            ParakeetModel::Ctc(m) => m
                .transcribe_samples(samples.to_vec(), 16000, 1, Some(TimestampMode::Words))
                .map_err(|e| {
                    LexiconError::Inference(InferenceError::InferenceFailed {
                        message: format!("Parakeet CTC inference failed: {e}"),
                    })
                })?,
            ParakeetModel::Tdt(m) => m
                .transcribe_samples(samples.to_vec(), 16000, 1, Some(TimestampMode::Words))
                .map_err(|e| {
                    LexiconError::Inference(InferenceError::InferenceFailed {
                        message: format!("Parakeet TDT inference failed: {e}"),
                    })
                })?,
        };
        let inference_time_ms = inference_start.elapsed().as_millis();
        let total_time_ms = total_start.elapsed().as_millis() as i64;

        // Convert parakeet tokens to segments
        let segments = if pk_result.text.trim().is_empty() {
            vec![]
        } else if pk_result.tokens.is_empty() {
            // No token-level timing — return single segment
            vec![Segment::new(0, audio_duration_ms, pk_result.text.trim())]
        } else {
            // Group tokens into sentence-like segments by punctuation boundaries
            group_tokens_into_segments(&pk_result.tokens, audio_duration_ms)
        };

        let result = TranscriptionResult::new(segments, audio_duration_ms, total_time_ms);

        debug!(
            audio_duration_ms = audio_duration_ms,
            total_time_ms = total_time_ms,
            inference_time_ms = inference_time_ms,
            text_len = pk_result.text.len(),
            rtf = result.real_time_factor(),
            "Parakeet transcription complete"
        );

        Ok(result)
    }

    fn translate(
        &self,
        _samples: &[f32],
        _config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        Err(LexiconError::Inference(
            InferenceError::InferenceFailed {
                message: "Parakeet models do not support translation. Use Whisper for X→English translation.".to_string(),
            },
        ))
    }

    fn is_model_loaded(&self) -> bool {
        self.model.lock().expect("model lock poisoned").is_some()
    }

    fn model_name(&self) -> Option<&str> {
        self.model_dir.as_deref()
    }

    fn unload_model(&mut self) {
        if self.model.lock().expect("model lock poisoned").is_some() {
            info!("Unloading Parakeet model");
        }
        *self.model.lock().expect("model lock poisoned") = None;
        self.model_dir = None;
        self.variant = None;
    }
}

/// Groups parakeet `TimedToken`s into sentence-like segments.
///
/// Splits on sentence-ending punctuation (`.`, `?`, `!`) when present,
/// otherwise returns a single segment spanning all tokens.
fn group_tokens_into_segments(
    tokens: &[parakeet_rs::TimedToken],
    audio_duration_ms: i64,
) -> Vec<Segment> {
    if tokens.is_empty() {
        return vec![];
    }

    let mut segments = Vec::new();
    let mut current_words: Vec<&str> = Vec::new();
    let mut segment_start_ms: Option<i64> = None;
    let mut segment_end_ms: i64 = 0;

    for token in tokens {
        let text = token.text.trim();
        if text.is_empty() {
            continue;
        }

        let start_ms = (token.start * 1000.0) as i64;
        let end_ms = (token.end * 1000.0) as i64;

        if segment_start_ms.is_none() {
            segment_start_ms = Some(start_ms);
        }
        segment_end_ms = end_ms;
        current_words.push(text);

        // Split on sentence-ending punctuation
        if text.ends_with('.') || text.ends_with('?') || text.ends_with('!') {
            let joined = current_words.join(" ");
            segments.push(Segment::new(
                segment_start_ms.unwrap_or(0),
                segment_end_ms,
                joined,
            ));
            current_words.clear();
            segment_start_ms = None;
        }
    }

    // Remaining tokens become the final segment
    if !current_words.is_empty() {
        let joined = current_words.join(" ");
        segments.push(Segment::new(
            segment_start_ms.unwrap_or(0),
            segment_end_ms.min(audio_duration_ms),
            joined,
        ));
    }

    segments
}

impl std::fmt::Debug for ParakeetTranscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParakeetTranscriber")
            .field("model_loaded", &self.is_model_loaded())
            .field("model_dir", &self.model_dir)
            .field("variant", &self.variant)
            .finish()
    }
}

/// Configuration for Parakeet transcriber.
#[derive(Debug, Clone)]
pub struct ParakeetConfig {
    /// Path to model directory.
    pub model_dir: std::path::PathBuf,

    /// Model variant override (None = auto-detect from directory).
    pub variant: Option<ParakeetVariant>,
}

impl Default for ParakeetConfig {
    fn default() -> Self {
        Self {
            model_dir: std::path::PathBuf::new(),
            variant: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parakeet_transcriber_creation() {
        let transcriber = ParakeetTranscriber::new().unwrap();
        assert!(!transcriber.is_model_loaded());
        assert!(transcriber.model_name().is_none());
        assert!(transcriber.variant().is_none());
        assert!(!transcriber.has_builtin_punctuation());
    }

    #[test]
    fn test_parakeet_transcriber_no_model() {
        let transcriber = ParakeetTranscriber::new().unwrap();
        let config = TranscriptionConfig::new();
        let samples = vec![0.0f32; 16000];

        let result = transcriber.transcribe(&samples, &config);
        assert!(matches!(
            result,
            Err(LexiconError::Inference(InferenceError::ModelNotLoaded))
        ));
    }

    #[test]
    fn test_parakeet_transcriber_missing_model_dir() {
        let mut transcriber = ParakeetTranscriber::new().unwrap();
        let result = transcriber.load_model(Path::new("/nonexistent/model/dir"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parakeet_transcriber_empty_audio() {
        let transcriber = ParakeetTranscriber::default();
        let config = TranscriptionConfig::new();
        let samples: Vec<f32> = vec![];

        let result = transcriber.transcribe(&samples, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parakeet_transcriber_short_audio() {
        let transcriber = ParakeetTranscriber::default();
        let config = TranscriptionConfig::new();
        // 50ms of audio — below 100ms minimum
        let samples = vec![0.0f32; 800];

        let result = transcriber.transcribe(&samples, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parakeet_translate_unsupported() {
        let transcriber = ParakeetTranscriber::new().unwrap();
        let config = TranscriptionConfig::new();
        let samples = vec![0.0f32; 16000];

        let result = transcriber.translate(&samples, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parakeet_config_defaults() {
        let config = ParakeetConfig::default();
        assert!(config.model_dir.as_os_str().is_empty());
        assert!(config.variant.is_none());
    }

    #[test]
    fn test_parakeet_transcriber_debug() {
        let transcriber = ParakeetTranscriber::new().unwrap();
        let debug_str = format!("{transcriber:?}");
        assert!(debug_str.contains("ParakeetTranscriber"));
        assert!(debug_str.contains("model_loaded: false"));
    }

    #[test]
    fn test_parakeet_variant_equality() {
        assert_eq!(ParakeetVariant::Ctc, ParakeetVariant::Ctc);
        assert_ne!(ParakeetVariant::Ctc, ParakeetVariant::Tdt);
    }

    #[test]
    fn test_group_tokens_empty() {
        let segments = group_tokens_into_segments(&[], 1000);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_group_tokens_single_sentence() {
        let tokens = vec![
            parakeet_rs::TimedToken {
                text: "Hello".to_string(),
                start: 0.0,
                end: 0.5,
            },
            parakeet_rs::TimedToken {
                text: "world.".to_string(),
                start: 0.5,
                end: 1.0,
            },
        ];

        let segments = group_tokens_into_segments(&tokens, 1000);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Hello world.");
        assert_eq!(segments[0].start_ms, 0);
        assert_eq!(segments[0].end_ms, 1000);
    }

    #[test]
    fn test_group_tokens_multiple_sentences() {
        let tokens = vec![
            parakeet_rs::TimedToken {
                text: "Hello.".to_string(),
                start: 0.0,
                end: 0.5,
            },
            parakeet_rs::TimedToken {
                text: "World!".to_string(),
                start: 0.5,
                end: 1.0,
            },
        ];

        let segments = group_tokens_into_segments(&tokens, 1000);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Hello.");
        assert_eq!(segments[1].text, "World!");
    }

    #[test]
    fn test_group_tokens_no_punctuation() {
        let tokens = vec![
            parakeet_rs::TimedToken {
                text: "hello".to_string(),
                start: 0.0,
                end: 0.3,
            },
            parakeet_rs::TimedToken {
                text: "world".to_string(),
                start: 0.3,
                end: 0.6,
            },
        ];

        let segments = group_tokens_into_segments(&tokens, 1000);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "hello world");
    }

    /// Integration test that loads a real model if available.
    /// Run with: cargo test --features stt-parakeet test_parakeet_with_real_model -- --ignored
    ///
    /// Download a model first:
    /// ```bash
    /// # CTC model (English, ~600MB):
    /// # Download from HuggingFace: istupakov/parakeet-ctc-0.6b-onnx
    /// # Place in models/parakeet-ctc-0.6b/
    /// ```
    #[test]
    #[ignore = "requires parakeet model download (see doc comment)"]
    fn test_parakeet_with_real_model() {
        use super::Transcriber as _;

        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("Could not find workspace root");

        let model_dir = workspace_root.join("models/parakeet-ctc-0.6b");

        if !model_dir.exists() {
            eprintln!("Model not found at {model_dir:?}.");
            eprintln!("Download from HuggingFace: istupakov/parakeet-ctc-0.6b-onnx");
            return;
        }

        let mut transcriber = ParakeetTranscriber::new().unwrap();
        transcriber
            .load_model(&model_dir)
            .expect("Failed to load model");
        assert!(transcriber.is_model_loaded());
        assert_eq!(transcriber.variant(), Some(ParakeetVariant::Ctc));
        assert!(!transcriber.has_builtin_punctuation());

        // Transcribe silence
        let samples = vec![0.0f32; 16000];
        let config = TranscriptionConfig::new();
        let result = transcriber
            .transcribe(&samples, &config)
            .expect("Transcription failed");

        eprintln!("Transcription result: {result:?}");
        eprintln!("Processing time: {}ms", result.processing_time_ms);
        eprintln!("RTF: {:.3}", result.real_time_factor());
        assert_eq!(result.audio_duration_ms, 1000);
    }

    /// Integration test that transcribes real speech audio.
    /// Run with: cargo test --features stt-parakeet test_parakeet_real_speech -- --ignored --nocapture
    #[test]
    #[ignore = "requires parakeet model and WAV fixtures"]
    fn test_parakeet_real_speech() {
        use super::Transcriber as _;

        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("Could not find workspace root");

        let model_dir = workspace_root.join("models/parakeet-ctc-0.6b");
        if !model_dir.exists() {
            eprintln!("Model not found at {model_dir:?}");
            return;
        }

        let mut transcriber = ParakeetTranscriber::new().unwrap();
        let load_start = std::time::Instant::now();
        transcriber.load_model(&model_dir).expect("Failed to load model");
        eprintln!("Model loaded in {}ms", load_start.elapsed().as_millis());

        // Warm up
        transcriber.warm_up().expect("Warm-up failed");

        let fixtures = [
            (
                "tests/fixtures/audio/sherpa/1089-134686-0001.wav",
                "after early nightfall the yellow lamps would light up",
            ),
            (
                "tests/fixtures/audio/sherpa/1221-135766-0002.wav",
                "yet these thoughts affected hester prynne",
            ),
        ];

        let config = TranscriptionConfig::new();

        for (wav_rel, expected_substr) in &fixtures {
            let wav_path = workspace_root.join(wav_rel);
            if !wav_path.exists() {
                eprintln!("SKIP: {wav_rel} not found");
                continue;
            }

            // Read WAV to f32 samples
            let mut reader = hound::WavReader::open(&wav_path).unwrap();
            let spec = reader.spec();
            let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Int {
                reader
                    .samples::<i16>()
                    .map(|s| s.unwrap() as f32 / 32768.0)
                    .collect()
            } else {
                reader.samples::<f32>().map(|s| s.unwrap()).collect()
            };

            let duration_s = samples.len() as f64 / spec.sample_rate as f64;

            let result = transcriber.transcribe(&samples, &config).expect("Transcription failed");

            eprintln!("--- {wav_rel} ---");
            eprintln!("  Audio: {duration_s:.1}s @ {}Hz", spec.sample_rate);
            eprintln!("  Time: {}ms (RTF: {:.3})", result.processing_time_ms, result.real_time_factor());
            eprintln!("  Text: {}", result.full_text());

            // CTC outputs capitalized/punctuated text — compare case-insensitively
            let text_lower = result.full_text().to_lowercase();
            assert!(
                text_lower.contains(expected_substr),
                "Expected text to contain '{expected_substr}', got: {}",
                result.full_text()
            );
        }
    }
}
