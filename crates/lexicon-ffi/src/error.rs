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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LexiconFfiError::InitializationFailed("test".to_string());
        assert_eq!(format!("{err}"), "Initialization failed: test");

        let err = LexiconFfiError::ModelNotFound("whisper".to_string());
        assert_eq!(format!("{err}"), "Model not found: whisper");

        let err = LexiconFfiError::TranscriptionFailed("timeout".to_string());
        assert_eq!(format!("{err}"), "Transcription failed: timeout");

        let err = LexiconFfiError::SynthesisFailed("model error".to_string());
        assert_eq!(format!("{err}"), "Synthesis failed: model error");

        let err = LexiconFfiError::InvalidInput("empty audio".to_string());
        assert_eq!(format!("{err}"), "Invalid input: empty audio");

        let err = LexiconFfiError::IoError("file not found".to_string());
        assert_eq!(format!("{err}"), "I/O error: file not found");

        let err = LexiconFfiError::DatabaseError("connection failed".to_string());
        assert_eq!(format!("{err}"), "Database error: connection failed");

        let err = LexiconFfiError::ConfigError("invalid".to_string());
        assert_eq!(format!("{err}"), "Configuration error: invalid");

        let err = LexiconFfiError::ModelLoadFailed("corrupt".to_string());
        assert_eq!(format!("{err}"), "Model load failed: corrupt");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let ffi_err: LexiconFfiError = io_err.into();
        assert!(matches!(ffi_err, LexiconFfiError::IoError(_)));
        assert!(format!("{ffi_err}").contains("file not found"));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let ffi_err: LexiconFfiError = anyhow_err.into();
        assert!(matches!(ffi_err, LexiconFfiError::InitializationFailed(_)));
        assert!(format!("{ffi_err}").contains("something went wrong"));
    }
}
