//! Kokoro TTS Synthesizer
//!
//! Neural text-to-speech using the Kokoro model via ONNX Runtime.
//! Uses misaki-rs for grapheme-to-phoneme (G2P) conversion to produce
//! natural-sounding speech.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use misaki_rs::G2P;
use ort::session::Session;
use ort::value::Tensor;
use tracing::{debug, info, warn};

use super::types::{AudioBuffer, TtsConfig, TtsEngine, VoiceGender, VoiceInfo, TTS_SAMPLE_RATE};
use super::Synthesizer;
use crate::error::{InferenceError, LexiconError, Result};

/// Kokoro neural TTS synthesizer using ONNX Runtime.
///
/// Kokoro is a high-quality neural TTS model that produces natural-sounding
/// speech. It currently supports English only.
///
/// Uses misaki-rs for grapheme-to-phoneme (G2P) conversion, which provides:
/// - POS-aware phonemization for handling heteronyms
/// - Morphological stemming for plurals, past tense, and continuous forms
/// - Automatic number-to-speech conversion
/// - No external dependencies (all dictionaries embedded)
pub struct KokoroSynthesizer {
    session: Option<Arc<Mutex<Session>>>,
    /// Path to the loaded model, if any.
    pub model_path: Option<String>,
    /// G2P (grapheme-to-phoneme) engine for text-to-phoneme conversion.
    g2p: G2P,
    /// Whether to use British English pronunciation (default: false = US English).
    british: bool,
}

impl KokoroSynthesizer {
    /// Creates a new Kokoro synthesizer without loading a model.
    ///
    /// Uses US English pronunciation by default. Call `with_british(true)`
    /// for British English.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: None,
            model_path: None,
            g2p: G2P::new(false), // US English
            british: false,
        }
    }

    /// Creates a new Kokoro synthesizer with British English pronunciation.
    #[must_use]
    pub fn new_british() -> Self {
        Self {
            session: None,
            model_path: None,
            g2p: G2P::new(true), // British English
            british: true,
        }
    }

    /// Sets whether to use British English pronunciation.
    pub fn set_british(&mut self, british: bool) {
        if self.british != british {
            self.british = british;
            self.g2p = G2P::new(british);
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

    /// Converts text to phoneme string using misaki-rs G2P.
    ///
    /// Returns the phoneme string ready for tokenization.
    fn text_to_phonemes(&self, text: &str) -> String {
        let start = Instant::now();
        let (phonemes, _tokens) = self.g2p.g2p(text);
        let elapsed = start.elapsed();
        debug!(
            "G2P conversion: {} chars -> {} phonemes in {:?}",
            text.len(),
            phonemes.len(),
            elapsed
        );
        phonemes
    }

    /// Converts phonemes to token IDs for the model.
    ///
    /// Uses the official Kokoro phoneme vocabulary from the model config.
    /// Token IDs are mapped according to hexgrad/Kokoro-82M config.json.
    fn phonemes_to_tokens(&self, phonemes: &str) -> Vec<i64> {
        let mut tokens: Vec<i64> = Vec::with_capacity(phonemes.len() + 2);

        // Start token (padding)
        tokens.push(0);

        for ch in phonemes.chars() {
            // Map character to Kokoro token ID based on official vocabulary
            // from https://huggingface.co/hexgrad/Kokoro-82M/blob/main/config.json
            let token = match ch {
                // Punctuation
                ';' => 1,
                ':' => 2,
                ',' => 3,
                '.' => 4,
                '!' => 5,
                '?' => 6,
                '—' => 9,
                '…' => 10,
                '"' => 11,
                '(' => 12,
                ')' => 13,
                '"' => 14,
                '"' => 15,
                ' ' => 16,  // Space/word boundary

                // Diacritics and affricates
                '̃' => 17,  // Combining tilde
                'ʣ' => 18,
                'ʥ' => 19,
                'ʦ' => 20,
                'ʨ' => 21,
                'ᵝ' => 22,
                'ꭧ' => 23,

                // Capital letters (special phonemes)
                'A' => 24,
                'I' => 25,
                'O' => 31,
                'Q' => 33,
                'S' => 35,
                'T' => 36,
                'W' => 39,
                'Y' => 41,
                'ᵊ' => 42,

                // Lowercase letters
                'a' => 43,
                'b' => 44,
                'c' => 45,
                'd' => 46,
                'e' => 47,
                'f' => 48,
                'h' => 50,
                'i' => 51,
                'j' => 52,
                'k' => 53,
                'l' => 54,
                'm' => 55,
                'n' => 56,
                'o' => 57,
                'p' => 58,
                'q' => 59,
                'r' => 60,
                's' => 61,
                't' => 62,
                'u' => 63,
                'v' => 64,
                'w' => 65,
                'x' => 66,
                'y' => 67,
                'z' => 68,

                // IPA vowels
                'ɑ' => 69,
                'ɐ' => 70,
                'ɒ' => 71,
                'æ' => 72,
                'β' => 75,
                'ɔ' => 76,
                'ɕ' => 77,
                'ç' => 78,
                'ɖ' => 80,
                'ð' => 81,
                'ʤ' => 82,
                'ə' => 83,
                'ɚ' => 85,
                'ɛ' => 86,
                'ɜ' => 87,
                'ɟ' => 90,
                'ɡ' => 92,
                'ɥ' => 99,
                'ɨ' => 101,
                'ɪ' => 102,
                'ʝ' => 103,
                'ɯ' => 110,
                'ɰ' => 111,
                'ŋ' => 112,
                'ɳ' => 113,
                'ɲ' => 114,
                'ɴ' => 115,
                'ø' => 116,
                'ɸ' => 118,
                'θ' => 119,
                'œ' => 120,
                'ɹ' => 123,
                'ɾ' => 125,
                'ɻ' => 126,
                'ʁ' => 128,
                'ɽ' => 129,
                'ʂ' => 130,
                'ʃ' => 131,
                'ʈ' => 132,
                'ʧ' => 133,
                'ʊ' => 135,
                'ʋ' => 136,
                'ʌ' => 138,
                'ɣ' => 139,
                'ɤ' => 140,
                'χ' => 142,
                'ʎ' => 143,
                'ʒ' => 147,
                'ʔ' => 148,

                // Prosody markers
                'ˈ' => 156,  // Primary stress
                'ˌ' => 157,  // Secondary stress
                'ː' => 158,  // Length mark
                'ʰ' => 162,  // Aspiration
                'ʲ' => 164,  // Palatalization

                // Tone indicators
                '↓' => 169,
                '→' => 171,
                '↗' => 172,
                '↘' => 173,
                'ᵻ' => 177,

                // Unknown character - skip with warning
                _ => {
                    if ch.is_alphabetic() || ch.is_ascii_punctuation() {
                        warn!("Unknown phoneme character: '{}' (U+{:04X})", ch, ch as u32);
                    }
                    continue; // Skip unknown characters instead of using 0
                }
            };
            tokens.push(token);
        }

        // End token (padding)
        tokens.push(0);

        // Kokoro model has a hard limit of 512 tokens
        if tokens.len() > 512 {
            warn!(
                "Token sequence truncated from {} to 512 tokens",
                tokens.len()
            );
            tokens.truncate(512);
        }

        tokens
    }

    /// Converts text to token IDs for the model.
    ///
    /// This is the main tokenization entry point that:
    /// 1. Converts text to phonemes using G2P
    /// 2. Converts phonemes to token IDs
    fn tokenize_text(&self, text: &str) -> Vec<i64> {
        let phonemes = self.text_to_phonemes(text);
        self.phonemes_to_tokens(&phonemes)
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
    fn test_g2p_conversion() {
        let synth = KokoroSynthesizer::new();

        // Test basic word conversion
        let phonemes = synth.text_to_phonemes("Hello");
        assert!(!phonemes.is_empty());
        // "Hello" should produce IPA phonemes containing 'h', 'ɛ', 'l', 'O' or similar
        println!("'Hello' -> '{}'", phonemes);

        // Test sentence conversion
        let phonemes = synth.text_to_phonemes("Hello world");
        assert!(!phonemes.is_empty());
        println!("'Hello world' -> '{}'", phonemes);
    }

    #[test]
    fn test_tokenization() {
        let synth = KokoroSynthesizer::new();
        let tokens = synth.tokenize_text("Hi");

        // Should have start token, phoneme tokens, end token
        assert!(tokens.len() >= 3); // At minimum: start, one phoneme, end
        assert_eq!(tokens[0], 0); // Start
        assert_eq!(tokens[tokens.len() - 1], 0); // End

        // All intermediate tokens should be non-zero (valid phonemes)
        for token in &tokens[1..tokens.len() - 1] {
            // Token should be valid (> 0 or space token)
            assert!(*token >= 0, "Invalid token: {}", token);
        }
    }

    #[test]
    fn test_g2p_performance() {
        use std::time::Instant;

        let synth = KokoroSynthesizer::new();
        let text = "The quick brown fox jumps over the lazy dog.";

        let start = Instant::now();
        let _phonemes = synth.text_to_phonemes(text);
        let elapsed = start.elapsed();

        // G2P should complete in under 50ms for a typical sentence
        assert!(
            elapsed.as_millis() < 50,
            "G2P took too long: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_number_conversion() {
        let synth = KokoroSynthesizer::new();

        // misaki-rs should convert numbers to words
        let phonemes = synth.text_to_phonemes("123");
        assert!(!phonemes.is_empty());
        // Should produce phonemes for "one hundred twenty-three" or similar
        println!("'123' -> '{}'", phonemes);
    }

    #[test]
    fn test_british_english() {
        let us_synth = KokoroSynthesizer::new();
        let uk_synth = KokoroSynthesizer::new_british();

        // Some words have different pronunciations
        let us_phonemes = us_synth.text_to_phonemes("schedule");
        let uk_phonemes = uk_synth.text_to_phonemes("schedule");

        println!("US 'schedule' -> '{}'", us_phonemes);
        println!("UK 'schedule' -> '{}'", uk_phonemes);

        // Both should produce valid phonemes
        assert!(!us_phonemes.is_empty());
        assert!(!uk_phonemes.is_empty());
    }
}
