//! FFI types for Flutter bindings.
//!
//! These types mirror `lexicon-ffi` for API consistency.

/// FFI configuration for Lexicon context.
#[derive(Debug, Clone)]
pub struct LexiconConfig {
    pub data_dir: String,
    pub model_dir: Option<String>,
    pub log_level: String,
}

impl Default for LexiconConfig {
    fn default() -> Self {
        Self {
            data_dir: String::new(),
            model_dir: None,
            log_level: "info".to_string(),
        }
    }
}

/// FFI audio configuration.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub chunk_ms: u32,
    pub vad_enabled: bool,
    pub vad_mode: u8,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            chunk_ms: 100,
            vad_enabled: true,
            vad_mode: 2,
        }
    }
}

/// FFI transcription configuration.
#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    pub language: Option<String>,
    pub translate: bool,
    pub num_threads: u32,
    pub initial_prompt: Option<String>,
    pub temperature: f32,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            language: None,
            translate: false,
            num_threads: 4,
            initial_prompt: None,
            temperature: 0.0,
        }
    }
}

/// FFI TTS configuration.
#[derive(Debug, Clone)]
pub struct TtsConfig {
    pub language: Option<String>,
    pub voice_id: Option<String>,
    pub rate: f32,
    pub pitch: f32,
    pub volume: f32,
    pub use_kokoro: bool,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            language: None,
            voice_id: None,
            rate: 1.0,
            pitch: 0.0,
            volume: 1.0,
            use_kokoro: true,
        }
    }
}

/// A transcribed segment.
#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f32>,
    pub language: Option<String>,
    pub speaker_id: Option<u32>,
}

/// Transcription result.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub segments: Vec<TranscriptionSegment>,
    pub detected_language: Option<String>,
    pub audio_duration_ms: i64,
    pub processing_time_ms: i64,
}

/// TTS synthesis result.
#[derive(Debug, Clone)]
pub struct SynthesisResult {
    pub audio_data: Vec<u8>,
    pub sample_rate: u32,
    pub duration_ms: i64,
    pub engine: String,
}

/// Model information.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub model_type: String,
    pub size_bytes: u64,
    pub downloaded: bool,
    pub local_path: Option<String>,
}

/// Download progress.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_seconds: Option<u64>,
}

/// Platform information.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub platform: String,
    pub model_tier: String,
    pub available_memory_mb: u64,
    pub has_gpu: bool,
}

// ============================================================================
// Streaming Types (re-exported from api.rs for convenience)
// ============================================================================

// Note: The following streaming types are defined in api.rs:
// - AudioChunk: Input for streaming transcription
// - TranscriptionEvent: Output events from streaming transcription
// - TtsAudioChunk: Output chunks from streaming TTS
// - DownloadEvent: Progress events from model download
