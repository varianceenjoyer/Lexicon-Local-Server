//! Kokoro TTS Synthesizer
//!
//! Neural text-to-speech using the Kokoro model via ONNX Runtime.

use std::path::Path;
use std::sync::{Arc, Mutex};

use ort::session::Session;
use ort::value::Tensor;
use tracing::{debug, info};

use super::types::{AudioBuffer, TtsConfig, TtsEngine, VoiceGender, VoiceInfo, TTS_SAMPLE_RATE};
use super::Synthesizer;
use crate::error::{InferenceError, LexiconError, Result};

/// Kokoro neural TTS synthesizer using ONNX Runtime.
///
/// Kokoro is a high-quality neural TTS model that produces natural-sounding
/// speech. It currently supports English only.
pub struct KokoroSynthesizer {
    session: Option<Arc<Mutex<Session>>>,
    model_path: Option<String>,
}

impl KokoroSynthesizer {
    /// Creates a new Kokoro synthesizer without loading a model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: None,
            model_path: None,
        }
    }

    /// Creates a Kokoro synthesizer with a model already loaded.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    pub fn with_model(model_path: &Path) -> Result<Self> {
        let mut synth = Self::new();
        synth.load_model(model_path)?;
        Ok(synth)
    }

    /// Unloads the current model to free memory.
    pub fn unload_model(&mut self) {
        if self.session.is_some() {
            info!("Unloading Kokoro model");
        }
        self.session = None;
        self.model_path = None;
    }

    /// Converts text to phoneme tokens for the model.
    ///
    /// This is a simplified tokenization - a full implementation would use
    /// a proper phoneme converter like espeak-ng.
    fn tokenize_text(&self, text: &str) -> Vec<i64> {
        // Simplified character-based tokenization
        // In production, this would use a proper phoneme converter
        let mut tokens: Vec<i64> = Vec::new();

        // Start token
        tokens.push(0);

        for ch in text.chars() {
            // Simple ASCII mapping - real implementation would use phonemes
            let token = match ch {
                'a'..='z' => (ch as i64) - ('a' as i64) + 1,
                'A'..='Z' => (ch.to_ascii_lowercase() as i64) - ('a' as i64) + 1,
                ' ' => 27,
                '.' | '!' | '?' => 28,
                ',' => 29,
                '\'' => 30,
                '-' => 31,
                _ => 0, // Unknown characters map to padding
            };
            tokens.push(token);
        }

        // End token
        tokens.push(0);

        tokens
    }
}

impl Default for KokoroSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Synthesizer for KokoroSynthesizer {
    fn load_model(&mut self, path: &Path) -> Result<()> {
        info!("Loading Kokoro model from {:?}", path);

        if !path.exists() {
            return Err(LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Model file not found: {}", path.display()),
            }));
        }

        // Create ONNX Runtime session
        let session = Session::builder()
            .map_err(|e| {
                LexiconError::Inference(InferenceError::ContextCreationFailed {
                    message: format!("Failed to create ONNX session builder: {e}"),
                })
            })?
            .with_intra_threads(4)
            .map_err(|e| {
                LexiconError::Inference(InferenceError::ContextCreationFailed {
                    message: format!("Failed to set thread count: {e}"),
                })
            })?
            .commit_from_file(path)
            .map_err(|e| {
                LexiconError::Inference(InferenceError::ContextCreationFailed {
                    message: format!("Failed to load ONNX model: {e}"),
                })
            })?;

        self.session = Some(Arc::new(Mutex::new(session)));
        self.model_path = Some(path.to_string_lossy().to_string());

        info!("Kokoro model loaded successfully");
        Ok(())
    }

    fn synthesize(&self, text: &str, config: &TtsConfig) -> Result<AudioBuffer> {
        let session_arc = self.session.as_ref().ok_or_else(|| {
            LexiconError::Inference(InferenceError::ModelNotLoaded)
        })?;

        let mut session = session_arc.lock().map_err(|_| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: "Failed to acquire session lock".to_string(),
            })
        })?;

        if text.is_empty() {
            return Err(LexiconError::Inference(InferenceError::InvalidInput {
                message: "Text cannot be empty".to_string(),
            }));
        }

        debug!("Synthesizing {} characters with Kokoro", text.len());

        // Tokenize the input text
        let tokens = self.tokenize_text(text);
        let token_len = tokens.len();

        // Create input tensors
        // Note: Actual tensor creation depends on the specific Kokoro model's input format
        // This is a simplified example - real implementation would match model's expected inputs
        let input_array = ndarray::Array2::from_shape_vec(
            (1, token_len),
            tokens,
        ).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Failed to create input array: {e}"),
            })
        })?;

        let input_tensor = Tensor::from_array(input_array).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Failed to create input tensor: {e}"),
            })
        })?;

        // Run inference
        // Note: Input/output names depend on the specific model
        let outputs = session.run(
            ort::inputs![
                "input_ids" => input_tensor,
            ]
        ).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Kokoro inference failed: {e}"),
            })
        })?;

        // Extract audio output
        // Note: Output tensor name and shape depend on the specific model
        let audio_tensor = outputs.get("audio")
            .ok_or_else(|| {
                LexiconError::Inference(InferenceError::InferenceFailed {
                    message: "Model output 'audio' not found".to_string(),
                })
            })?;

        let audio_data = audio_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| {
                LexiconError::Inference(InferenceError::InferenceFailed {
                    message: format!("Failed to extract audio tensor: {e}"),
                })
            })?;

        // Convert to Vec<f32>
        let (_, data) = audio_data;
        let mut samples: Vec<f32> = data.iter().copied().collect();

        // Apply volume scaling
        if (config.volume - 1.0).abs() > f32::EPSILON {
            for sample in &mut samples {
                *sample *= config.volume;
            }
        }

        debug!("Kokoro synthesis produced {} samples", samples.len());

        Ok(AudioBuffer::new(samples, TTS_SAMPLE_RATE, 1))
    }

    fn is_ready(&self) -> bool {
        self.session.is_some()
    }

    fn list_voices(&self) -> Vec<VoiceInfo> {
        if !self.is_ready() {
            return Vec::new();
        }

        // Kokoro has a single default voice
        vec![VoiceInfo {
            id: "kokoro-default".to_string(),
            name: "Kokoro Default".to_string(),
            language: "en-US".to_string(),
            gender: Some(VoiceGender::Female),
            engine: TtsEngine::Kokoro,
        }]
    }

    fn engine_type(&self) -> TtsEngine {
        TtsEngine::Kokoro
    }
}

impl std::fmt::Debug for KokoroSynthesizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KokoroSynthesizer")
            .field("model_loaded", &self.is_ready())
            .field("model_path", &self.model_path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kokoro_creation() {
        let synth = KokoroSynthesizer::new();
        assert!(!synth.is_ready());
        assert!(synth.model_path.is_none());
        assert!(synth.list_voices().is_empty());
    }

    #[test]
    fn test_kokoro_missing_model() {
        let mut synth = KokoroSynthesizer::new();
        let result = synth.load_model(Path::new("/nonexistent/model.onnx"));
        assert!(result.is_err());
    }

    #[test]
    fn test_kokoro_synthesize_without_model() {
        let synth = KokoroSynthesizer::new();
        let config = TtsConfig::new();
        let result = synth.synthesize("Hello", &config);

        assert!(matches!(
            result,
            Err(LexiconError::Inference(InferenceError::ModelNotLoaded))
        ));
    }

    #[test]
    fn test_tokenization() {
        let synth = KokoroSynthesizer::new();
        let tokens = synth.tokenize_text("Hi");

        // Should have start token, 'h', 'i', end token
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], 0); // Start
        assert_eq!(tokens[3], 0); // End
    }
}
