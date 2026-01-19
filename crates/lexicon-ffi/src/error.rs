//! FFI Error types.
//!
//! These errors are exposed to Swift/Kotlin via UniFFI.

/// FFI error type exposed to foreign code.
///
/// This enum is converted to Swift/Kotlin exceptions by UniFFI.
#[derive(Debug, thiserror::Error)]
pub enum LexiconFfiError {
    /// Library initialization failed.
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Model not found.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Model loading failed.
    #[error("Model load failed: {0}")]
    ModelLoadFailed(String),

    /// Transcription failed.
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    /// TTS synthesis failed.
    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),

    /// Invalid input provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(String),

    /// Database error.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl From<std::io::Error> for LexiconFfiError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<anyhow::Error> for LexiconFfiError {
    fn from(err: anyhow::Error) -> Self {
        Self::InitializationFailed(err.to_string())
    }
}
