//! TTS types - Configuration, audio buffers, and engine selection.

use serde::{Deserialize, Serialize};

/// Audio sample rate for TTS output (standard CD quality).
pub const TTS_SAMPLE_RATE: u32 = 24_000;

/// Audio buffer containing synthesized speech.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Audio samples in f32 format, normalized to [-1.0, 1.0].
    pub samples: Vec<f32>,

    /// Sample rate in Hz (typically 24kHz for Kokoro).
    pub sample_rate: u32,

    /// Number of channels (1 = mono, 2 = stereo).
    pub channels: u16,
}

impl AudioBuffer {
    /// Creates a new audio buffer.
    #[must_use]
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
        }
    }

    /// Creates an empty audio buffer.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            samples: Vec::new(),
            sample_rate: TTS_SAMPLE_RATE,
            channels: 1,
        }
    }

    /// Returns the duration in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        let samples_per_channel = self.samples.len() / self.channels as usize;
        (samples_per_channel as u64 * 1000) / self.sample_rate as u64
    }

    /// Returns true if the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns the number of samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Converts samples to i16 format.
    #[must_use]
    pub fn to_i16(&self) -> Vec<i16> {
        self.samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect()
    }
}

/// TTS engine type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TtsEngine {
    /// Kokoro neural TTS (high quality, English only).
    Kokoro,
    /// Platform-native TTS via tts-rs (multi-language support).
    Platform,
}

impl Default for TtsEngine {
    fn default() -> Self {
        Self::Kokoro
    }
}

impl std::fmt::Display for TtsEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kokoro => write!(f, "kokoro"),
            Self::Platform => write!(f, "platform"),
        }
    }
}

/// Configuration for text-to-speech synthesis.
#[derive(Debug, Clone)]
pub struct TtsConfig {
    /// Preferred TTS engine.
    pub preferred_engine: TtsEngine,

    /// Enable fallback to platform TTS on Kokoro failure.
    pub fallback_enabled: bool,

    /// Voice identifier (engine-specific).
    pub voice: Option<String>,

    /// Speech rate multiplier (0.5 = half speed, 2.0 = double speed).
    pub rate: f32,

    /// Pitch adjustment in semitones (-12 to +12).
    pub pitch: f32,

    /// Volume level (0.0 to 1.0).
    pub volume: f32,

    /// Language code for platform TTS (e.g., "en-US", "es-ES").
    pub language: Option<String>,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            preferred_engine: TtsEngine::Kokoro,
            fallback_enabled: true,
            voice: None,
            rate: 1.0,
            pitch: 0.0,
            volume: 1.0,
            language: None,
        }
    }
}

impl TtsConfig {
    /// Creates a new TTS configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the preferred engine.
    #[must_use]
    pub fn with_engine(mut self, engine: TtsEngine) -> Self {
        self.preferred_engine = engine;
        self
    }

    /// Enables or disables fallback.
    #[must_use]
    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_enabled = enabled;
        self
    }

    /// Sets the voice identifier.
    #[must_use]
    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.voice = Some(voice.into());
        self
    }

    /// Sets the speech rate.
    #[must_use]
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.rate = rate.clamp(0.1, 10.0);
        self
    }

    /// Sets the pitch adjustment.
    #[must_use]
    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch.clamp(-12.0, 12.0);
        self
    }

    /// Sets the volume.
    #[must_use]
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Sets the language for platform TTS.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Creates a config for Kokoro engine.
    #[must_use]
    pub fn kokoro() -> Self {
        Self::new().with_engine(TtsEngine::Kokoro)
    }

    /// Creates a config for platform engine.
    #[must_use]
    pub fn platform() -> Self {
        Self::new().with_engine(TtsEngine::Platform)
    }
}

/// Result of a synthesis operation.
#[derive(Debug, Clone)]
pub struct SynthesisResult {
    /// The synthesized audio.
    pub audio: AudioBuffer,

    /// The engine that was used.
    pub engine_used: TtsEngine,

    /// Whether fallback was triggered.
    pub used_fallback: bool,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
}

impl SynthesisResult {
    /// Creates a new synthesis result.
    #[must_use]
    pub fn new(audio: AudioBuffer, engine_used: TtsEngine, processing_time_ms: u64) -> Self {
        Self {
            audio,
            engine_used,
            used_fallback: false,
            processing_time_ms,
        }
    }

    /// Marks this result as having used fallback.
    #[must_use]
    pub fn with_fallback(mut self) -> Self {
        self.used_fallback = true;
        self
    }

    /// Returns the real-time factor (processing time / audio duration).
    #[must_use]
    pub fn real_time_factor(&self) -> f64 {
        let audio_duration = self.audio.duration_ms();
        if audio_duration > 0 {
            self.processing_time_ms as f64 / audio_duration as f64
        } else {
            0.0
        }
    }
}

/// Available voice information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    /// Voice identifier.
    pub id: String,

    /// Display name.
    pub name: String,

    /// Language code (e.g., "en-US").
    pub language: String,

    /// Gender (if known).
    pub gender: Option<VoiceGender>,

    /// Which engine provides this voice.
    pub engine: TtsEngine,
}

/// Voice gender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_buffer() {
        let samples = vec![0.0f32; 24000]; // 1 second at 24kHz
        let buffer = AudioBuffer::new(samples, 24000, 1);

        assert_eq!(buffer.duration_ms(), 1000);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 24000);
    }

    #[test]
    fn test_audio_buffer_to_i16() {
        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let buffer = AudioBuffer::new(samples, 24000, 1);
        let i16_samples = buffer.to_i16();

        assert_eq!(i16_samples[0], 0);
        assert_eq!(i16_samples[1], 16383); // ~0.5 * 32767
        assert_eq!(i16_samples[2], -16383);
        assert_eq!(i16_samples[3], 32767);
        assert_eq!(i16_samples[4], -32767);
    }

    #[test]
    fn test_tts_config_builder() {
        let config = TtsConfig::new()
            .with_engine(TtsEngine::Platform)
            .with_rate(1.5)
            .with_pitch(2.0)
            .with_volume(0.8)
            .with_language("es-ES");

        assert_eq!(config.preferred_engine, TtsEngine::Platform);
        assert!((config.rate - 1.5).abs() < f32::EPSILON);
        assert!((config.pitch - 2.0).abs() < f32::EPSILON);
        assert!((config.volume - 0.8).abs() < f32::EPSILON);
        assert_eq!(config.language, Some("es-ES".to_string()));
    }

    #[test]
    fn test_tts_config_clamping() {
        let config = TtsConfig::new()
            .with_rate(100.0)  // Should clamp to 10.0
            .with_pitch(50.0)  // Should clamp to 12.0
            .with_volume(5.0); // Should clamp to 1.0

        assert!((config.rate - 10.0).abs() < f32::EPSILON);
        assert!((config.pitch - 12.0).abs() < f32::EPSILON);
        assert!((config.volume - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_synthesis_result() {
        let audio = AudioBuffer::new(vec![0.0; 24000], 24000, 1);
        let result = SynthesisResult::new(audio, TtsEngine::Kokoro, 200);

        assert_eq!(result.engine_used, TtsEngine::Kokoro);
        assert!(!result.used_fallback);
        assert!((result.real_time_factor() - 0.2).abs() < 0.01); // 200ms / 1000ms
    }

    #[test]
    fn test_audio_buffer_empty() {
        let buffer = AudioBuffer::empty();

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.duration_ms(), 0);
        assert_eq!(buffer.sample_rate, TTS_SAMPLE_RATE);
        assert_eq!(buffer.channels, 1);
    }

    #[test]
    fn test_audio_buffer_zero_sample_rate() {
        let buffer = AudioBuffer::new(vec![0.0; 1000], 0, 1);
        assert_eq!(buffer.duration_ms(), 0);
    }

    #[test]
    fn test_audio_buffer_stereo() {
        let samples = vec![0.0f32; 48000]; // 2 channels * 24000 samples
        let buffer = AudioBuffer::new(samples, 24000, 2);

        // Duration should be 1 second (24000 samples per channel at 24kHz)
        assert_eq!(buffer.duration_ms(), 1000);
    }

    #[test]
    fn test_tts_engine_display() {
        assert_eq!(TtsEngine::Kokoro.to_string(), "kokoro");
        assert_eq!(TtsEngine::Platform.to_string(), "platform");
    }

    #[test]
    fn test_tts_engine_default() {
        assert_eq!(TtsEngine::default(), TtsEngine::Kokoro);
    }

    #[test]
    fn test_tts_config_defaults() {
        let config = TtsConfig::default();

        assert_eq!(config.preferred_engine, TtsEngine::Kokoro);
        assert!(config.fallback_enabled);
        assert!(config.voice.is_none());
        assert_eq!(config.rate, 1.0);
        assert_eq!(config.pitch, 0.0);
        assert_eq!(config.volume, 1.0);
        assert!(config.language.is_none());
    }

    #[test]
    fn test_tts_config_convenience_constructors() {
        let kokoro = TtsConfig::kokoro();
        assert_eq!(kokoro.preferred_engine, TtsEngine::Kokoro);

        let platform = TtsConfig::platform();
        assert_eq!(platform.preferred_engine, TtsEngine::Platform);
    }

    #[test]
    fn test_tts_config_with_voice() {
        let config = TtsConfig::new().with_voice("af_sarah");
        assert_eq!(config.voice, Some("af_sarah".to_string()));
    }

    #[test]
    fn test_tts_config_with_fallback() {
        let config = TtsConfig::new().with_fallback(false);
        assert!(!config.fallback_enabled);
    }

    #[test]
    fn test_synthesis_result_with_fallback() {
        let audio = AudioBuffer::empty();
        let result = SynthesisResult::new(audio, TtsEngine::Platform, 100)
            .with_fallback();

        assert!(result.used_fallback);
        assert_eq!(result.engine_used, TtsEngine::Platform);
    }

    #[test]
    fn test_synthesis_result_zero_duration() {
        let audio = AudioBuffer::empty();
        let result = SynthesisResult::new(audio, TtsEngine::Kokoro, 100);

        assert_eq!(result.real_time_factor(), 0.0);
    }

    #[test]
    fn test_voice_info() {
        let voice = VoiceInfo {
            id: "en-us-1".to_string(),
            name: "Sarah".to_string(),
            language: "en-US".to_string(),
            gender: Some(VoiceGender::Female),
            engine: TtsEngine::Platform,
        };

        assert_eq!(voice.id, "en-us-1");
        assert_eq!(voice.gender, Some(VoiceGender::Female));
    }

    #[test]
    fn test_voice_info_serialization() {
        let voice = VoiceInfo {
            id: "test".to_string(),
            name: "Test Voice".to_string(),
            language: "en-US".to_string(),
            gender: Some(VoiceGender::Male),
            engine: TtsEngine::Kokoro,
        };

        let json = serde_json::to_string(&voice).unwrap();
        let deserialized: VoiceInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test");
        assert_eq!(deserialized.gender, Some(VoiceGender::Male));
    }

    #[test]
    fn test_tts_sample_rate_constant() {
        assert_eq!(TTS_SAMPLE_RATE, 24_000);
    }
}
