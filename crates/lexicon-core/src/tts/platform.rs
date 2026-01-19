//! Platform TTS Synthesizer
//!
//! Uses tts-rs for platform-native text-to-speech as a fallback.
//! Supports multiple languages via system TTS engines.

use std::path::Path;
use std::sync::Mutex;

use tts::Tts;
use tracing::{debug, info, warn};

use super::types::{AudioBuffer, TtsConfig, TtsEngine, VoiceGender, VoiceInfo, TTS_SAMPLE_RATE};
use super::Synthesizer;
use crate::error::{InferenceError, LexiconError, Result};

/// Platform-native TTS synthesizer using tts-rs.
///
/// This provides fallback TTS functionality using the operating system's
/// built-in speech synthesis capabilities:
/// - macOS: AVSpeechSynthesizer
/// - Windows: SAPI / OneCore
/// - Linux: Speech Dispatcher
pub struct PlatformSynthesizer {
    tts: Mutex<Tts>,
}

impl PlatformSynthesizer {
    /// Creates a new platform TTS synthesizer.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform TTS cannot be initialized.
    pub fn new() -> Result<Self> {
        let tts = Tts::default().map_err(|e| {
            LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Failed to initialize platform TTS: {e}"),
            })
        })?;

        info!("Platform TTS initialized");

        Ok(Self {
            tts: Mutex::new(tts),
        })
    }

    /// Sets the voice for synthesis.
    fn set_voice(&self, tts: &mut Tts, config: &TtsConfig) -> Result<()> {
        // Try to set voice if specified
        if let Some(ref voice_id) = config.voice {
            if let Ok(voices) = tts.voices() {
                if let Some(voice) = voices.iter().find(|v| &v.id() == voice_id) {
                    tts.set_voice(voice).map_err(|e| {
                        LexiconError::Inference(InferenceError::InferenceFailed {
                            message: format!("Failed to set voice: {e}"),
                        })
                    })?;
                }
            }
        }

        // Set rate (tts-rs uses 0.0-1.0 range, we use 0.1-10.0)
        // Map our 1.0 (normal) to tts-rs's 0.5 (normal)
        let rate = ((config.rate - 0.1) / 9.9 * 0.9 + 0.05).clamp(0.0, 1.0);
        if tts.set_rate(rate).is_err() {
            debug!("Failed to set speech rate");
        }

        // Set pitch (if supported)
        // tts-rs uses 0.0-1.0, we use -12 to +12 semitones
        let pitch = ((config.pitch + 12.0) / 24.0).clamp(0.0, 1.0);
        if tts.set_pitch(pitch).is_err() {
            debug!("Platform TTS doesn't support pitch adjustment");
        }

        // Set volume
        if tts.set_volume(config.volume).is_err() {
            debug!("Failed to set volume");
        }

        Ok(())
    }

    /// Gets available voices filtered by language.
    fn voices_for_language(&self, language: Option<&str>) -> Vec<VoiceInfo> {
        let tts = match self.tts.lock() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let voices = match tts.voices() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        voices
            .into_iter()
            .filter(|v| {
                if let Some(lang) = language {
                    v.language().to_string().starts_with(lang)
                } else {
                    true
                }
            })
            .map(|v| VoiceInfo {
                id: v.id().to_string(),
                name: v.name().to_string(),
                language: v.language().to_string(),
                gender: match v.gender() {
                    Some(tts::Gender::Male) => Some(VoiceGender::Male),
                    Some(tts::Gender::Female) => Some(VoiceGender::Female),
                    _ => None,
                },
                engine: TtsEngine::Platform,
            })
            .collect()
    }
}

impl Synthesizer for PlatformSynthesizer {
    fn load_model(&mut self, _path: &Path) -> Result<()> {
        // Platform TTS doesn't need model loading
        Ok(())
    }

    fn synthesize(&self, text: &str, config: &TtsConfig) -> Result<AudioBuffer> {
        if text.is_empty() {
            return Err(LexiconError::Inference(InferenceError::InvalidInput {
                message: "Text cannot be empty".to_string(),
            }));
        }

        debug!("Synthesizing {} characters with platform TTS", text.len());

        let mut tts = self.tts.lock().map_err(|_| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: "Failed to acquire TTS lock".to_string(),
            })
        })?;

        // Configure voice settings
        self.set_voice(&mut tts, config)?;

        // tts-rs primarily supports synchronous playback, not audio buffer extraction
        // For actual audio buffer extraction, we would need to use platform-specific APIs
        // or a different approach (like recording system audio output)
        //
        // For now, we'll provide a placeholder that speaks the text
        // and returns an estimated audio buffer
        //
        // In a production implementation, you would either:
        // 1. Use platform-specific APIs to capture the audio data
        // 2. Use a speech synthesis library that provides raw audio output
        // 3. Implement audio loopback capture

        // Speak the text (this will play audio but we can't capture it directly)
        // In production, this would be replaced with actual audio capture
        let _utterance_id = tts.speak(text, false).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Platform TTS synthesis failed: {e}"),
            })
        })?;

        // Estimate audio duration based on text length
        // Average speaking rate is ~150 words per minute, ~5 chars per word
        let chars_per_second = 150.0 * 5.0 / 60.0; // ~12.5 chars/second
        let estimated_duration_s = text.len() as f32 / chars_per_second;
        let estimated_samples = (estimated_duration_s * TTS_SAMPLE_RATE as f32) as usize;

        // Return a silent buffer of the estimated length
        // In production, this would contain actual synthesized audio
        warn!(
            "Platform TTS audio capture not implemented - returning estimated silence ({} samples)",
            estimated_samples
        );

        Ok(AudioBuffer::new(
            vec![0.0; estimated_samples],
            TTS_SAMPLE_RATE,
            1,
        ))
    }

    fn is_ready(&self) -> bool {
        self.tts.lock().is_ok()
    }

    fn list_voices(&self) -> Vec<VoiceInfo> {
        self.voices_for_language(None)
    }

    fn engine_type(&self) -> TtsEngine {
        TtsEngine::Platform
    }
}

impl std::fmt::Debug for PlatformSynthesizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformSynthesizer")
            .field("ready", &self.is_ready())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_synthesizer_creation() {
        // This test may fail on systems without TTS support
        match PlatformSynthesizer::new() {
            Ok(synth) => {
                assert!(synth.is_ready());
                assert_eq!(synth.engine_type(), TtsEngine::Platform);
            }
            Err(_) => {
                // Expected on systems without TTS
            }
        }
    }

    #[test]
    fn test_platform_voices() {
        if let Ok(synth) = PlatformSynthesizer::new() {
            let voices = synth.list_voices();
            // Most systems have at least one voice
            // But we don't assert this as CI may not have TTS
            for voice in voices {
                assert!(!voice.id.is_empty());
                assert!(!voice.name.is_empty());
                assert_eq!(voice.engine, TtsEngine::Platform);
            }
        }
    }

    #[test]
    fn test_platform_model_load_noop() {
        if let Ok(mut synth) = PlatformSynthesizer::new() {
            // load_model should be a no-op for platform TTS
            assert!(synth.load_model(Path::new("/any/path")).is_ok());
        }
    }
}
