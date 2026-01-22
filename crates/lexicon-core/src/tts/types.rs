//! TTS types - Configuration, audio buffers, and engine selection.

use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::time::Instant;

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
#[derive(Default)]
pub enum TtsEngine {
    /// Kokoro neural TTS (high quality, English only).
    #[default]
    Kokoro,
    /// Platform-native TTS via tts-rs (multi-language support).
    Platform,
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

/// A chunk of audio in a streaming synthesis.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// Audio samples in f32 format, normalized to [-1.0, 1.0].
    pub samples: Vec<f32>,

    /// Sample rate in Hz.
    pub sample_rate: u32,

    /// Sequence number of this chunk (0-indexed).
    pub sequence: u32,

    /// Whether this is the final chunk.
    pub is_final: bool,

    /// Time from synthesis start to this chunk being ready (ms).
    pub latency_ms: u64,
}

impl AudioChunk {
    /// Creates a new audio chunk.
    #[must_use]
    pub fn new(samples: Vec<f32>, sample_rate: u32, sequence: u32, is_final: bool, latency_ms: u64) -> Self {
        Self {
            samples,
            sample_rate,
            sequence,
            is_final,
            latency_ms,
        }
    }

    /// Returns the duration of this chunk in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        (self.samples.len() as u64 * 1000) / self.sample_rate as u64
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

/// Receiver for streaming audio chunks.
///
/// This wraps an `mpsc::Receiver` and provides iteration over audio chunks.
pub struct StreamingReceiver {
    receiver: mpsc::Receiver<AudioChunk>,
    first_chunk_latency_ms: Option<u64>,
}

impl StreamingReceiver {
    /// Creates a new streaming receiver.
    #[must_use]
    pub fn new(receiver: mpsc::Receiver<AudioChunk>) -> Self {
        Self {
            receiver,
            first_chunk_latency_ms: None,
        }
    }

    /// Returns the latency until the first chunk was received.
    ///
    /// Returns `None` until the first chunk has been received.
    #[must_use]
    pub fn first_chunk_latency_ms(&self) -> Option<u64> {
        self.first_chunk_latency_ms
    }

    /// Receives the next audio chunk, blocking until available.
    ///
    /// Returns `None` when the stream is complete.
    pub fn recv(&mut self) -> Option<AudioChunk> {
        match self.receiver.recv() {
            Ok(chunk) => {
                if self.first_chunk_latency_ms.is_none() {
                    self.first_chunk_latency_ms = Some(chunk.latency_ms);
                }
                Some(chunk)
            }
            Err(_) => None,
        }
    }

    /// Tries to receive the next audio chunk without blocking.
    ///
    /// Returns `Ok(Some(chunk))` if a chunk is available,
    /// `Ok(None)` if no chunk is ready yet,
    /// `Err(())` if the stream is complete.
    pub fn try_recv(&mut self) -> Result<Option<AudioChunk>, ()> {
        match self.receiver.try_recv() {
            Ok(chunk) => {
                if self.first_chunk_latency_ms.is_none() {
                    self.first_chunk_latency_ms = Some(chunk.latency_ms);
                }
                Ok(Some(chunk))
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(()),
        }
    }

    /// Collects all remaining chunks into an AudioBuffer.
    ///
    /// This consumes the receiver and blocks until all chunks are received.
    #[must_use]
    pub fn collect_all(mut self) -> AudioBuffer {
        let mut all_samples = Vec::new();
        let mut sample_rate = TTS_SAMPLE_RATE;

        while let Some(chunk) = self.recv() {
            sample_rate = chunk.sample_rate;
            all_samples.extend(chunk.samples);
        }

        AudioBuffer::new(all_samples, sample_rate, 1)
    }
}

impl Iterator for StreamingReceiver {
    type Item = AudioChunk;

    fn next(&mut self) -> Option<Self::Item> {
        self.recv()
    }
}

/// Handle for sending streaming audio chunks.
pub struct StreamingSender {
    sender: mpsc::Sender<AudioChunk>,
    start_time: Instant,
    sequence: u32,
    sample_rate: u32,
}

impl StreamingSender {
    /// Creates a new streaming sender.
    #[must_use]
    pub fn new(sender: mpsc::Sender<AudioChunk>, sample_rate: u32) -> Self {
        Self {
            sender,
            start_time: Instant::now(),
            sequence: 0,
            sample_rate,
        }
    }

    /// Sends an audio chunk.
    ///
    /// Returns `Err` if the receiver has been dropped.
    pub fn send(&mut self, samples: Vec<f32>, is_final: bool) -> Result<(), mpsc::SendError<AudioChunk>> {
        let latency_ms = self.start_time.elapsed().as_millis() as u64;
        let chunk = AudioChunk::new(samples, self.sample_rate, self.sequence, is_final, latency_ms);
        self.sequence += 1;
        self.sender.send(chunk)
    }

    /// Returns the elapsed time since streaming started.
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

/// Creates a streaming channel for audio synthesis.
///
/// Returns a sender/receiver pair for streaming audio chunks.
#[must_use]
pub fn streaming_channel(sample_rate: u32) -> (StreamingSender, StreamingReceiver) {
    let (tx, rx) = mpsc::channel();
    (StreamingSender::new(tx, sample_rate), StreamingReceiver::new(rx))
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

    #[test]
    fn test_audio_chunk_creation() {
        let chunk = AudioChunk::new(vec![0.0; 2400], 24000, 0, false, 50);

        assert_eq!(chunk.samples.len(), 2400);
        assert_eq!(chunk.sample_rate, 24000);
        assert_eq!(chunk.sequence, 0);
        assert!(!chunk.is_final);
        assert_eq!(chunk.latency_ms, 50);
        assert_eq!(chunk.duration_ms(), 100); // 2400 samples at 24kHz = 100ms
    }

    #[test]
    fn test_audio_chunk_to_i16() {
        let chunk = AudioChunk::new(vec![0.0, 0.5, -0.5, 1.0, -1.0], 24000, 0, false, 0);
        let i16_samples = chunk.to_i16();

        assert_eq!(i16_samples[0], 0);
        assert_eq!(i16_samples[1], 16383);
        assert_eq!(i16_samples[2], -16383);
        assert_eq!(i16_samples[3], 32767);
        assert_eq!(i16_samples[4], -32767);
    }

    #[test]
    fn test_audio_chunk_zero_sample_rate() {
        let chunk = AudioChunk::new(vec![0.0; 1000], 0, 0, true, 0);
        assert_eq!(chunk.duration_ms(), 0);
    }

    #[test]
    fn test_streaming_channel() {
        let (mut sender, mut receiver) = streaming_channel(24000);

        // Send some chunks
        sender.send(vec![0.0; 2400], false).unwrap();
        sender.send(vec![0.5; 2400], false).unwrap();
        sender.send(vec![1.0; 2400], true).unwrap();

        // Receive and verify
        let chunk1 = receiver.recv().unwrap();
        assert_eq!(chunk1.sequence, 0);
        assert!(!chunk1.is_final);
        assert!(receiver.first_chunk_latency_ms().is_some());

        let chunk2 = receiver.recv().unwrap();
        assert_eq!(chunk2.sequence, 1);
        assert!(!chunk2.is_final);

        let chunk3 = receiver.recv().unwrap();
        assert_eq!(chunk3.sequence, 2);
        assert!(chunk3.is_final);

        // Drop sender
        drop(sender);

        // No more chunks
        assert!(receiver.recv().is_none());
    }

    #[test]
    fn test_streaming_receiver_collect() {
        let (mut sender, receiver) = streaming_channel(24000);

        sender.send(vec![0.0; 2400], false).unwrap();
        sender.send(vec![0.5; 2400], false).unwrap();
        sender.send(vec![1.0; 2400], true).unwrap();
        drop(sender);

        let buffer = receiver.collect_all();
        assert_eq!(buffer.samples.len(), 7200); // 3 chunks * 2400 samples
        assert_eq!(buffer.sample_rate, 24000);
    }

    #[test]
    fn test_streaming_receiver_iterator() {
        let (mut sender, receiver) = streaming_channel(24000);

        sender.send(vec![0.0; 100], false).unwrap();
        sender.send(vec![0.0; 100], true).unwrap();
        drop(sender);

        let chunks: Vec<_> = receiver.collect();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].sequence, 0);
        assert_eq!(chunks[1].sequence, 1);
    }

    #[test]
    fn test_streaming_sender_elapsed() {
        let (sender, _receiver) = streaming_channel(24000);

        // Elapsed time should be very small immediately after creation
        assert!(sender.elapsed_ms() < 100);
    }

    #[test]
    fn test_streaming_try_recv() {
        let (mut sender, mut receiver) = streaming_channel(24000);

        // Nothing available yet, but channel still open
        assert!(matches!(receiver.try_recv(), Ok(None)));

        sender.send(vec![0.0; 100], false).unwrap();

        // Now something is available
        let chunk = receiver.try_recv().unwrap().unwrap();
        assert_eq!(chunk.sequence, 0);

        // Channel still open but empty
        assert!(matches!(receiver.try_recv(), Ok(None)));

        // Close the channel
        drop(sender);

        // Channel disconnected
        assert!(matches!(receiver.try_recv(), Err(())));
    }
}
