//! STT types - Segments, configuration, and transcription results.

use serde::{Deserialize, Serialize};

/// A transcribed segment with timing and metadata.
///
/// Segments represent discrete portions of recognized speech with their
/// corresponding timestamps, confidence scores, and optional metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Start time in milliseconds from audio start.
    pub start_ms: i64,

    /// End time in milliseconds from audio start.
    pub end_ms: i64,

    /// Transcribed text content.
    pub text: String,

    /// Confidence score (0.0 to 1.0), if available.
    pub confidence: Option<f32>,

    /// Detected or forced language code (e.g., "en", "es").
    pub language: Option<String>,

    /// Speaker identifier for diarization (when enabled).
    pub speaker_id: Option<u32>,
}

impl Segment {
    /// Creates a new segment with the given timing and text.
    #[must_use]
    pub fn new(start_ms: i64, end_ms: i64, text: impl Into<String>) -> Self {
        Self {
            start_ms,
            end_ms,
            text: text.into(),
            confidence: None,
            language: None,
            speaker_id: None,
        }
    }

    /// Sets the confidence score.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Sets the language code.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Sets the speaker ID.
    #[must_use]
    pub fn with_speaker(mut self, speaker_id: u32) -> Self {
        self.speaker_id = Some(speaker_id);
        self
    }

    /// Returns the duration of this segment in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> i64 {
        self.end_ms - self.start_ms
    }
}

/// Configuration for transcription operations.
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    /// Language code for transcription (None = auto-detect).
    pub language: Option<String>,

    /// Enable translation to English (for non-English audio).
    pub translate: bool,

    /// Enable speaker diarization.
    pub enable_diarization: bool,

    /// Maximum number of speakers for diarization.
    pub max_speakers: Option<u32>,

    /// Number of threads to use for inference (0 = auto).
    pub num_threads: u32,

    /// Initial prompt to guide transcription.
    pub initial_prompt: Option<String>,

    /// Temperature for sampling (0.0 = greedy, higher = more random).
    pub temperature: f32,

    /// Beam size for beam search (1 = greedy, higher = better quality).
    pub beam_size: u32,

    /// Maximum segment length in tokens.
    pub max_segment_length: u32,

    /// Whether to suppress non-speech tokens.
    pub suppress_non_speech: bool,

    /// Force single segment output (faster for short audio, good for streaming).
    /// WP-LATENCY-STT: Reduces segmentation overhead by ~5-10%.
    pub single_segment: bool,

    /// Skip timestamp generation (faster when timestamps not needed).
    /// WP-LATENCY-STT: Reduces latency by ~2-3% when enabled.
    pub no_timestamps: bool,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            language: None,
            translate: false,
            enable_diarization: false,
            max_speakers: None,
            num_threads: 0, // Auto-detect
            initial_prompt: None,
            temperature: 0.0,
            beam_size: 5,
            max_segment_length: 0, // No limit
            suppress_non_speech: true,
            single_segment: true,  // WP-LATENCY-STT: Default on for lower latency
            no_timestamps: false,  // Keep timestamps by default for segment timing
        }
    }
}

impl TranscriptionConfig {
    /// Creates a new transcription config with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the language code.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Enables translation to English.
    #[must_use]
    pub fn with_translation(mut self) -> Self {
        self.translate = true;
        self
    }

    /// Enables speaker diarization.
    #[must_use]
    pub fn with_diarization(mut self, max_speakers: Option<u32>) -> Self {
        self.enable_diarization = true;
        self.max_speakers = max_speakers;
        self
    }

    /// Sets the number of threads.
    #[must_use]
    pub fn with_threads(mut self, num_threads: u32) -> Self {
        self.num_threads = num_threads;
        self
    }

    /// Sets the initial prompt.
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.initial_prompt = Some(prompt.into());
        self
    }

    /// Sets the temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Sets the beam size.
    #[must_use]
    pub fn with_beam_size(mut self, beam_size: u32) -> Self {
        self.beam_size = beam_size;
        self
    }

    /// Sets single segment mode (faster for short audio).
    #[must_use]
    pub fn with_single_segment(mut self, single_segment: bool) -> Self {
        self.single_segment = single_segment;
        self
    }

    /// Disables timestamp generation (faster when not needed).
    #[must_use]
    pub fn with_no_timestamps(mut self, no_timestamps: bool) -> Self {
        self.no_timestamps = no_timestamps;
        self
    }
}

/// Result of a transcription operation.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// Transcribed segments.
    pub segments: Vec<Segment>,

    /// Detected language (if auto-detection was used).
    pub detected_language: Option<String>,

    /// Total audio duration in milliseconds.
    pub audio_duration_ms: i64,

    /// Processing time in milliseconds.
    pub processing_time_ms: i64,
}

impl TranscriptionResult {
    /// Creates a new transcription result.
    #[must_use]
    pub fn new(segments: Vec<Segment>, audio_duration_ms: i64, processing_time_ms: i64) -> Self {
        Self {
            segments,
            detected_language: None,
            audio_duration_ms,
            processing_time_ms,
        }
    }

    /// Sets the detected language.
    #[must_use]
    pub fn with_detected_language(mut self, language: impl Into<String>) -> Self {
        self.detected_language = Some(language.into());
        self
    }

    /// Returns the full transcription text by joining all segments.
    #[must_use]
    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Returns the real-time factor (processing time / audio duration).
    ///
    /// A value < 1.0 means faster than real-time.
    #[must_use]
    pub fn real_time_factor(&self) -> f64 {
        if self.audio_duration_ms > 0 {
            self.processing_time_ms as f64 / self.audio_duration_ms as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_creation() {
        let segment = Segment::new(0, 1000, "Hello world")
            .with_confidence(0.95)
            .with_language("en")
            .with_speaker(1);

        assert_eq!(segment.start_ms, 0);
        assert_eq!(segment.end_ms, 1000);
        assert_eq!(segment.text, "Hello world");
        assert_eq!(segment.confidence, Some(0.95));
        assert_eq!(segment.language, Some("en".to_string()));
        assert_eq!(segment.speaker_id, Some(1));
        assert_eq!(segment.duration_ms(), 1000);
    }

    #[test]
    fn test_config_builder() {
        let config = TranscriptionConfig::new()
            .with_language("en")
            .with_translation()
            .with_diarization(Some(4))
            .with_threads(4)
            .with_temperature(0.2);

        assert_eq!(config.language, Some("en".to_string()));
        assert!(config.translate);
        assert!(config.enable_diarization);
        assert_eq!(config.max_speakers, Some(4));
        assert_eq!(config.num_threads, 4);
        assert!((config.temperature - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_transcription_result() {
        let segments = vec![
            Segment::new(0, 1000, "Hello"),
            Segment::new(1000, 2000, "world"),
        ];

        let result = TranscriptionResult::new(segments, 2000, 500)
            .with_detected_language("en");

        assert_eq!(result.full_text(), "Hello world");
        assert_eq!(result.real_time_factor(), 0.25); // 500ms / 2000ms
        assert_eq!(result.detected_language, Some("en".to_string()));
    }

    #[test]
    fn test_segment_empty_text() {
        let segment = Segment::new(0, 100, "");
        assert!(segment.text.is_empty());
        assert_eq!(segment.duration_ms(), 100);
    }

    #[test]
    fn test_segment_negative_duration() {
        // Edge case: end before start
        let segment = Segment::new(1000, 500, "Inverted");
        assert_eq!(segment.duration_ms(), -500);
    }

    #[test]
    fn test_transcription_result_empty_segments() {
        let result = TranscriptionResult::new(vec![], 1000, 100);
        assert!(result.full_text().is_empty());
    }

    #[test]
    fn test_transcription_result_zero_duration() {
        let result = TranscriptionResult::new(vec![], 0, 100);
        assert_eq!(result.real_time_factor(), 0.0);
    }

    #[test]
    fn test_config_defaults() {
        let config = TranscriptionConfig::default();

        assert!(config.language.is_none());
        assert!(!config.translate);
        assert!(!config.enable_diarization);
        assert!(config.max_speakers.is_none());
        assert_eq!(config.num_threads, 0);
        assert!(config.initial_prompt.is_none());
        assert_eq!(config.temperature, 0.0);
        assert_eq!(config.beam_size, 5);
        assert_eq!(config.max_segment_length, 0);
        assert!(config.suppress_non_speech);
        // WP-LATENCY-STT optimizations
        assert!(config.single_segment); // Default on for lower latency
        assert!(!config.no_timestamps);  // Keep timestamps by default
    }

    #[test]
    fn test_config_with_prompt() {
        let config = TranscriptionConfig::new()
            .with_prompt("Medical terminology: patient, diagnosis");

        assert_eq!(
            config.initial_prompt,
            Some("Medical terminology: patient, diagnosis".to_string())
        );
    }

    #[test]
    fn test_config_with_beam_size() {
        let config = TranscriptionConfig::new().with_beam_size(10);
        assert_eq!(config.beam_size, 10);
    }

    #[test]
    fn test_segment_serialization() {
        let segment = Segment::new(0, 1000, "Test")
            .with_confidence(0.95)
            .with_language("en");

        let json = serde_json::to_string(&segment).unwrap();
        let deserialized: Segment = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.start_ms, 0);
        assert_eq!(deserialized.end_ms, 1000);
        assert_eq!(deserialized.text, "Test");
        assert_eq!(deserialized.confidence, Some(0.95));
        assert_eq!(deserialized.language, Some("en".to_string()));
    }
}
