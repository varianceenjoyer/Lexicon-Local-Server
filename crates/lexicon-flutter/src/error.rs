//! Error types for Lexicon Flutter bindings.

use thiserror::Error;

/// FFI errors for Flutter bindings.
#[derive(Debug, Error)]
pub enum LexiconFlutterError {
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Model load failed: {0}")]
    ModelLoadFailed(String),

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Operation cancelled")]
    Cancelled,
}

impl From<std::io::Error> for LexiconFlutterError {
    fn from(e: std::io::Error) -> Self {
        LexiconFlutterError::IoError(e.to_string())
    }
}
