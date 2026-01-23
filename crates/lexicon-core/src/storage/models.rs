//! Data models for the storage layer.
//!
//! These models represent the core entities stored in the database:
//! sessions, segments, models, settings, and audio files.

use serde::{Deserialize, Serialize};

/// Session identifier (wraps a database row ID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub i64);

/// Segment identifier (wraps a database row ID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SegmentId(pub i64);

/// Metadata for creating or updating a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Human-readable session name.
    pub name: Option<String>,
    /// Primary language code (e.g., "en", "es", "auto").
    pub language: Option<String>,
}

impl SessionMetadata {
    /// Create new session metadata with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set session name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set session language.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

/// A transcription session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier.
    pub id: SessionId,
    /// Human-readable name.
    pub name: Option<String>,
    /// Primary language code.
    pub language: Option<String>,
    /// Creation timestamp (milliseconds since epoch).
    pub created_at: i64,
    /// Last update timestamp (milliseconds since epoch).
    pub updated_at: i64,
}

/// Data for creating a new segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSegment {
    /// Start time in milliseconds.
    pub start_ms: i64,
    /// End time in milliseconds.
    pub end_ms: i64,
    /// Transcribed text.
    pub text: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: Option<f64>,
    /// Detected language code.
    pub language: Option<String>,
    /// Speaker ID for diarization.
    pub speaker_id: Option<i32>,
}

impl NewSegment {
    /// Create a new segment with required fields.
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

    /// Set confidence score.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Set language.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Set speaker ID.
    #[must_use]
    pub fn with_speaker(mut self, speaker_id: i32) -> Self {
        self.speaker_id = Some(speaker_id);
        self
    }
}

/// A stored transcription segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Unique identifier.
    pub id: SegmentId,
    /// Parent session ID.
    pub session_id: SessionId,
    /// Start time in milliseconds.
    pub start_ms: i64,
    /// End time in milliseconds.
    pub end_ms: i64,
    /// Transcribed text.
    pub text: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: Option<f64>,
    /// Detected language code.
    pub language: Option<String>,
    /// Speaker ID for diarization.
    pub speaker_id: Option<i32>,
}

/// Search result with ranking information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matching segment.
    pub segment: Segment,
    /// Name of the session containing this segment.
    pub session_name: Option<String>,
    /// BM25 relevance rank (lower is better).
    pub rank: f64,
}

/// Model information for the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique model identifier (e.g., "whisper-tiny-q5_1").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// File size in bytes.
    pub size_bytes: i64,
    /// SHA-256 hash for verification.
    pub hash_sha256: String,
    /// Download timestamp (milliseconds since epoch).
    pub downloaded_at: Option<i64>,
    /// Whether the model has been verified.
    pub verified: bool,
}

/// Audio file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFileInfo {
    /// Database ID (None if not yet stored).
    pub id: Option<i64>,
    /// Associated session ID (optional).
    pub session_id: Option<SessionId>,
    /// Original filename.
    pub filename: String,
    /// File size in bytes.
    pub size_bytes: i64,
    /// MIME type (e.g., "audio/wav").
    pub mime_type: String,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Sample rate in Hz.
    pub sample_rate: Option<i32>,
    /// Number of channels.
    pub channels: Option<i32>,
    /// External file path (for files > 100KB).
    pub external_path: Option<String>,
}

impl AudioFileInfo {
    /// Blob storage threshold (100KB).
    pub const BLOB_THRESHOLD: i64 = 100 * 1024;

    /// Check if this file should be stored externally.
    #[must_use]
    pub fn should_store_externally(&self) -> bool {
        self.size_bytes > Self::BLOB_THRESHOLD
    }
}

/// Identifier for a saved transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SavedTranscriptId(pub i64);

/// A saved transcript from the TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTranscript {
    /// Unique identifier.
    pub id: SavedTranscriptId,
    /// User-provided name.
    pub name: String,
    /// Full transcript content.
    pub content: String,
    /// Character count.
    pub char_count: i64,
    /// Associated session ID (optional).
    pub session_id: Option<SessionId>,
    /// Creation timestamp (ISO 8601 string).
    pub created_at: String,
    /// Last update timestamp (ISO 8601 string).
    pub updated_at: String,
}

impl SavedTranscript {
    /// Returns a preview of the transcript (first ~50 characters).
    #[must_use]
    pub fn preview(&self) -> String {
        if self.content.len() > 50 {
            format!("{}...", &self.content[..47])
        } else {
            self.content.clone()
        }
    }
}

/// Data for creating a new saved transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSavedTranscript {
    /// User-provided name.
    pub name: String,
    /// Full transcript content.
    pub content: String,
    /// Associated session ID (optional).
    pub session_id: Option<SessionId>,
}

impl NewSavedTranscript {
    /// Create a new saved transcript with required fields.
    #[must_use]
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            name: name.into(),
            content,
            session_id: None,
        }
    }

    /// Associate with a session.
    #[must_use]
    pub fn with_session(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Returns the character count.
    #[must_use]
    pub fn char_count(&self) -> i64 {
        self.content.chars().count() as i64
    }
}
