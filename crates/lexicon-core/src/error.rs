//! Error types for the Lexicon Core library.
//!
//! This module provides a comprehensive error hierarchy using `thiserror` for
//! structured error handling. Errors are categorized by subsystem (Audio, Model,
//! Inference, Database, Config) to enable specific recovery strategies.
//!
//! # Error Recovery
//!
//! | Error Type | Recoverable | Recovery Strategy |
//! |------------|-------------|-------------------|
//! | `AudioError::PermissionDenied` | Yes | Prompt user to grant permission |
//! | `AudioError::DeviceDisconnected` | Yes | Reconnect or prompt user |
//! | `ModelError::NotFound` | Yes | Trigger download |
//! | `ModelError::OutOfMemory` | Maybe | Suggest smaller model |
//! | `InferenceError::Timeout` | Yes | Retry with shorter audio |
//!
//! # FFI Considerations
//!
//! These error types are designed to work with UniFFI for automatic translation
//! to Swift/Kotlin exceptions. When using C ABI directly, use `LexiconErrorCode`
//! and wrap FFI calls with `catch_unwind`.
//!
//! # Examples
//!
//! ```
//! use lexicon_core::error::{LexiconError, AudioError};
//!
//! fn handle_error(err: LexiconError) {
//!     match err {
//!         LexiconError::Audio(AudioError::PermissionDenied) => {
//!             println!("Please grant microphone permission");
//!         }
//!         LexiconError::Model(model_err) => {
//!             println!("Model error: {}", model_err);
//!         }
//!         _ => println!("Error: {}", err),
//!     }
//! }
//! ```

use std::path::PathBuf;

/// Result type alias using `LexiconError`.
pub type Result<T> = std::result::Result<T, LexiconError>;

/// Top-level error type for the Lexicon library.
///
/// This enum wraps all subsystem-specific errors and provides a unified
/// error interface for library consumers.
#[derive(Debug, thiserror::Error)]
pub enum LexiconError {
    /// Audio capture or playback error.
    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    /// Model loading, verification, or management error.
    #[error("Model error: {0}")]
    Model(#[from] ModelError),

    /// Speech recognition or synthesis error.
    #[error("Inference error: {0}")]
    Inference(#[from] InferenceError),

    /// Database storage error.
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    /// I/O error (file operations, network, etc.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Audio subsystem errors.
///
/// These errors relate to audio capture, playback, and device management.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// No audio input device is available.
    #[error("No audio input device available")]
    NoInputDevice,

    /// No audio output device is available.
    #[error("No audio output device available")]
    NoOutputDevice,

    /// Microphone permission was denied by the user or system.
    #[error("Microphone permission denied")]
    PermissionDenied,

    /// Audio device was disconnected during operation.
    #[error("Audio device disconnected")]
    DeviceDisconnected,

    /// Audio format is not supported by the device.
    #[error("Unsupported audio format: {format}")]
    UnsupportedFormat {
        /// Description of the unsupported format.
        format: String,
    },

    /// Audio stream encountered an error.
    #[error("Audio stream error: {message}")]
    StreamError {
        /// Error message from the audio backend.
        message: String,
    },

    /// Audio session was interrupted (e.g., phone call on iOS).
    #[error("Audio session interrupted: {reason}")]
    SessionInterrupted {
        /// Reason for the interruption.
        reason: String,
    },

    /// Bluetooth audio negotiation failed.
    #[error("Bluetooth audio error: {message}")]
    BluetoothError {
        /// Error message describing the Bluetooth issue.
        message: String,
    },

    /// Audio buffer underrun or overrun.
    #[error("Audio buffer {kind}")]
    BufferError {
        /// Either "underrun" or "overrun".
        kind: String,
    },

    /// Device enumeration failed.
    #[error("Failed to enumerate audio devices: {message}")]
    DeviceEnumerationFailed {
        /// Error message from the audio backend.
        message: String,
    },

    /// Invalid audio frame size for VAD.
    #[error("Invalid frame size: expected {expected}, got {actual}")]
    InvalidFrameSize {
        /// Expected frame sizes.
        expected: String,
        /// Actual frame size provided.
        actual: usize,
    },

    /// Voice activity detection error.
    #[error("VAD error: {message}")]
    VadError {
        /// Error message from the VAD engine.
        message: String,
    },

    /// Resampler initialization failed.
    #[error("Resampler initialization failed: {message}")]
    ResamplerInitFailed {
        /// Error message from the resampler.
        message: String,
    },

    /// Resampling operation failed.
    #[error("Resampling failed: {message}")]
    ResamplerError {
        /// Error message from the resampler.
        message: String,
    },

    /// Audio stream not started.
    #[error("Audio stream not started")]
    StreamNotStarted,

    /// Audio stream not running (stopped or never started).
    #[error("Audio stream not running")]
    StreamNotRunning,

    /// Requested audio device was not found.
    #[error("Audio device not found: {name}")]
    DeviceNotFound {
        /// Name of the device that was not found.
        name: String,
    },

    /// Failed to get device configuration.
    #[error("Failed to get device configuration: {message}")]
    DeviceConfigFailed {
        /// Error message from the audio backend.
        message: String,
    },

    /// Failed to build the audio stream.
    #[error("Failed to build audio stream: {message}")]
    StreamBuildFailed {
        /// Error message from the audio backend.
        message: String,
    },

    /// Failed to start the audio stream.
    #[error("Failed to start audio stream: {message}")]
    StreamStartFailed {
        /// Error message from the audio backend.
        message: String,
    },

    /// Audio operation timed out.
    #[error("Audio operation timed out: {operation}")]
    Timeout {
        /// Operation that timed out.
        operation: String,
    },

    /// Audio format conversion error.
    #[error("Audio conversion error: {message}")]
    ConversionError {
        /// Error message describing the conversion failure.
        message: String,
    },
}

/// Model management errors.
///
/// These errors relate to model downloading, verification, and loading.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// Model file was not found at the expected path.
    #[error("Model not found: {path}")]
    NotFound {
        /// Path where the model was expected.
        path: PathBuf,
    },

    /// Model file is corrupted or invalid.
    #[error("Model file corrupted or invalid: {path}")]
    InvalidModel {
        /// Path to the invalid model.
        path: PathBuf,
    },

    /// Model hash verification failed.
    #[error("Model verification failed: expected {expected}, got {actual}")]
    VerificationFailed {
        /// Expected SHA-256 hash.
        expected: String,
        /// Actual SHA-256 hash.
        actual: String,
    },

    /// Model manifest signature verification failed.
    #[error("Model manifest signature invalid")]
    SignatureInvalid,

    /// Insufficient memory to load the model.
    #[error("Insufficient memory to load model (need {required_mb}MB, have {available_mb}MB)")]
    OutOfMemory {
        /// Memory required by the model in megabytes.
        required_mb: u64,
        /// Memory available on the system in megabytes.
        available_mb: u64,
    },

    /// Model version is incompatible with the current library.
    #[error("Model version incompatible: expected {expected}, got {actual}")]
    VersionMismatch {
        /// Expected version string.
        expected: String,
        /// Actual version string.
        actual: String,
    },

    /// Model download failed.
    #[error("Model download failed for '{model_id}': {message}")]
    DownloadFailed {
        /// Model ID that failed to download.
        model_id: String,
        /// Error message describing the download failure.
        message: String,
    },

    /// Model manifest could not be fetched or parsed.
    #[error("Model manifest error: {message}")]
    ManifestError {
        /// Error message describing the manifest issue.
        message: String,
    },

    /// Model not found in manifest.
    #[error("Model not found in manifest: {model_id}")]
    NotInManifest {
        /// Model ID that was not found.
        model_id: String,
    },
}

/// Inference (STT/TTS) errors.
///
/// These errors occur during speech recognition or synthesis operations.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    /// Audio input is too short for processing.
    #[error("Audio too short for processing (minimum {min_seconds}s, got {actual_seconds}s)")]
    AudioTooShort {
        /// Minimum required duration in seconds.
        min_seconds: f32,
        /// Actual duration provided in seconds.
        actual_seconds: f32,
    },

    /// Inference operation timed out.
    #[error("Inference timeout after {timeout_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Model is not loaded.
    #[error("Model not loaded: load a model before inference")]
    ModelNotLoaded,

    /// Whisper context creation failed.
    #[error("Failed to create inference context: {message}")]
    ContextCreationFailed {
        /// Error message from the inference engine.
        message: String,
    },

    /// Inference engine returned an error.
    #[error("Inference failed: {message}")]
    InferenceFailed {
        /// Error message from the inference engine.
        message: String,
    },

    /// Invalid input provided to the inference engine.
    #[error("Invalid input: {message}")]
    InvalidInput {
        /// Description of why the input is invalid.
        message: String,
    },

    /// GPU/accelerator is not available.
    #[error("Hardware accelerator not available: {accelerator}")]
    AcceleratorUnavailable {
        /// Name of the unavailable accelerator (e.g., "Metal", "CUDA").
        accelerator: String,
    },
}

/// Database storage errors.
///
/// These errors relate to SQLite operations and data persistence.
#[derive(Debug, thiserror::Error)]
#[cfg(feature = "storage")]
pub enum DatabaseError {
    /// Database file could not be opened or created.
    #[error("Failed to open database '{path}': {source}")]
    OpenFailed {
        /// Path to the database file.
        path: PathBuf,
        /// Underlying error from rusqlite.
        #[source]
        source: rusqlite::Error,
    },

    /// Database schema migration failed.
    #[error("Database migration failed: {message}")]
    MigrationFailed {
        /// Error message describing the migration failure.
        message: String,
    },

    /// Database query failed.
    #[error("Database query failed: {message}")]
    QueryFailed {
        /// Error message from SQLite.
        message: String,
    },

    /// Requested record was not found.
    #[error("Record not found: {entity} with id {id}")]
    NotFound {
        /// Type of entity (e.g., "session", "segment").
        entity: String,
        /// ID of the missing record.
        id: String,
    },

    /// Database constraint was violated.
    #[error("Constraint violation: {message}")]
    ConstraintViolation {
        /// Description of the constraint that was violated.
        message: String,
    },

    /// Database is locked by another process.
    #[error("Database is locked")]
    Locked,

    /// Disk is full or write failed.
    #[error("Database write failed (disk full or I/O error): {message}")]
    WriteFailed {
        /// Error message describing the write failure.
        message: String,
    },
}

/// Database storage errors (stub when storage feature is disabled).
#[derive(Debug, thiserror::Error)]
#[cfg(not(feature = "storage"))]
pub enum DatabaseError {
    /// Database file could not be opened or created.
    #[error("Failed to open database: {path}")]
    OpenFailed {
        /// Path to the database file.
        path: PathBuf,
    },

    /// Database schema migration failed.
    #[error("Database migration failed: {message}")]
    MigrationFailed {
        /// Error message describing the migration failure.
        message: String,
    },

    /// Database query failed.
    #[error("Database query failed: {message}")]
    QueryFailed {
        /// Error message from SQLite.
        message: String,
    },

    /// Requested record was not found.
    #[error("Record not found: {entity} with id {id}")]
    NotFound {
        /// Type of entity (e.g., "session", "segment").
        entity: String,
        /// ID of the missing record.
        id: String,
    },

    /// Database constraint was violated.
    #[error("Constraint violation: {message}")]
    ConstraintViolation {
        /// Description of the constraint that was violated.
        message: String,
    },

    /// Database is locked by another process.
    #[error("Database is locked")]
    Locked,

    /// Disk is full or write failed.
    #[error("Database write failed (disk full or I/O error): {message}")]
    WriteFailed {
        /// Error message describing the write failure.
        message: String,
    },
}

/// C-compatible error codes for FFI boundaries.
///
/// When using the C ABI directly (not UniFFI), these error codes provide
/// a stable numeric interface for error handling across FFI boundaries.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexiconErrorCode {
    /// Operation completed successfully.
    Ok = 0,

    // Audio errors (100-199)
    /// No audio input device available.
    AudioNoDevice = 100,
    /// Microphone permission denied.
    AudioPermissionDenied = 101,
    /// Audio device disconnected.
    AudioDisconnected = 102,
    /// Audio format not supported.
    AudioUnsupportedFormat = 103,
    /// Audio stream error.
    AudioStreamError = 104,
    /// Audio session interrupted.
    AudioInterrupted = 105,
    /// Bluetooth audio error.
    AudioBluetoothError = 106,

    // Model errors (200-299)
    /// Model file not found.
    ModelNotFound = 200,
    /// Model file invalid or corrupted.
    ModelInvalid = 201,
    /// Out of memory for model.
    ModelOom = 202,
    /// Model verification failed.
    ModelVerificationFailed = 203,
    /// Model manifest signature invalid.
    ModelSignatureInvalid = 204,
    /// Model download failed.
    ModelDownloadFailed = 205,

    // Inference errors (300-399)
    /// Audio input too short.
    InferenceTooShort = 300,
    /// Inference timeout.
    InferenceTimeout = 301,
    /// Inference internal error.
    InferenceInternal = 302,
    /// Model not loaded.
    InferenceModelNotLoaded = 303,

    // Database errors (400-499)
    /// Database open failed.
    DatabaseOpenFailed = 400,
    /// Database query failed.
    DatabaseQueryFailed = 401,
    /// Database record not found.
    DatabaseNotFound = 402,
    /// Database locked.
    DatabaseLocked = 403,

    // I/O and config errors (500-599)
    /// I/O error.
    IoError = 500,
    /// Configuration error.
    ConfigError = 501,

    // General errors (900-999)
    /// Unknown or unclassified error.
    Unknown = 999,
}

impl From<&LexiconError> for LexiconErrorCode {
    fn from(err: &LexiconError) -> Self {
        match err {
            LexiconError::Audio(e) => match e {
                AudioError::NoInputDevice | AudioError::NoOutputDevice => Self::AudioNoDevice,
                AudioError::PermissionDenied => Self::AudioPermissionDenied,
                AudioError::DeviceDisconnected => Self::AudioDisconnected,
                AudioError::UnsupportedFormat { .. } => Self::AudioUnsupportedFormat,
                AudioError::StreamError { .. } => Self::AudioStreamError,
                AudioError::SessionInterrupted { .. } => Self::AudioInterrupted,
                AudioError::BluetoothError { .. } => Self::AudioBluetoothError,
                AudioError::BufferError { .. } => Self::AudioStreamError,
                AudioError::DeviceEnumerationFailed { .. } => Self::AudioNoDevice,
                AudioError::InvalidFrameSize { .. } => Self::AudioStreamError,
                AudioError::VadError { .. } => Self::AudioStreamError,
                AudioError::ResamplerInitFailed { .. } => Self::AudioStreamError,
                AudioError::ResamplerError { .. } => Self::AudioStreamError,
                AudioError::StreamNotStarted => Self::AudioStreamError,
                AudioError::StreamNotRunning => Self::AudioStreamError,
                AudioError::DeviceNotFound { .. } => Self::AudioNoDevice,
                AudioError::DeviceConfigFailed { .. } => Self::AudioStreamError,
                AudioError::StreamBuildFailed { .. } => Self::AudioStreamError,
                AudioError::StreamStartFailed { .. } => Self::AudioStreamError,
                AudioError::Timeout { .. } => Self::AudioStreamError,
                AudioError::ConversionError { .. } => Self::AudioUnsupportedFormat,
            },
            LexiconError::Model(e) => match e {
                ModelError::NotFound { .. } => Self::ModelNotFound,
                ModelError::InvalidModel { .. } => Self::ModelInvalid,
                ModelError::VerificationFailed { .. } => Self::ModelVerificationFailed,
                ModelError::SignatureInvalid => Self::ModelSignatureInvalid,
                ModelError::OutOfMemory { .. } => Self::ModelOom,
                ModelError::VersionMismatch { .. } => Self::ModelInvalid,
                ModelError::DownloadFailed { .. } => Self::ModelDownloadFailed,
                ModelError::ManifestError { .. } => Self::ModelSignatureInvalid,
                ModelError::NotInManifest { .. } => Self::ModelNotFound,
            },
            LexiconError::Inference(e) => match e {
                InferenceError::AudioTooShort { .. } => Self::InferenceTooShort,
                InferenceError::Timeout { .. } => Self::InferenceTimeout,
                InferenceError::ModelNotLoaded => Self::InferenceModelNotLoaded,
                InferenceError::ContextCreationFailed { .. }
                | InferenceError::InferenceFailed { .. }
                | InferenceError::InvalidInput { .. }
                | InferenceError::AcceleratorUnavailable { .. } => Self::InferenceInternal,
            },
            LexiconError::Database(e) => match e {
                DatabaseError::OpenFailed { .. } => Self::DatabaseOpenFailed,
                DatabaseError::MigrationFailed { .. } | DatabaseError::QueryFailed { .. } => {
                    Self::DatabaseQueryFailed
                }
                DatabaseError::NotFound { .. } => Self::DatabaseNotFound,
                DatabaseError::ConstraintViolation { .. } | DatabaseError::WriteFailed { .. } => {
                    Self::DatabaseQueryFailed
                }
                DatabaseError::Locked => Self::DatabaseLocked,
            },
            LexiconError::Io(_) => Self::IoError,
            LexiconError::Config(_) => Self::ConfigError,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LexiconError::Audio(AudioError::PermissionDenied);
        assert_eq!(err.to_string(), "Audio error: Microphone permission denied");

        let err = LexiconError::Model(ModelError::NotFound {
            path: PathBuf::from("/models/tiny.bin"),
        });
        assert_eq!(
            err.to_string(),
            "Model error: Model not found: /models/tiny.bin"
        );

        let err = LexiconError::Inference(InferenceError::AudioTooShort {
            min_seconds: 0.5,
            actual_seconds: 0.1,
        });
        assert_eq!(
            err.to_string(),
            "Inference error: Audio too short for processing (minimum 0.5s, got 0.1s)"
        );
    }

    #[test]
    fn test_error_conversion() {
        // Test From<AudioError>
        let audio_err = AudioError::PermissionDenied;
        let lexicon_err: LexiconError = audio_err.into();
        assert!(matches!(lexicon_err, LexiconError::Audio(_)));

        // Test From<std::io::Error>
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let lexicon_err: LexiconError = io_err.into();
        assert!(matches!(lexicon_err, LexiconError::Io(_)));
    }

    #[test]
    fn test_error_code_conversion() {
        let err = LexiconError::Audio(AudioError::PermissionDenied);
        let code = LexiconErrorCode::from(&err);
        assert_eq!(code, LexiconErrorCode::AudioPermissionDenied);

        let err = LexiconError::Model(ModelError::OutOfMemory {
            required_mb: 1024,
            available_mb: 512,
        });
        let code = LexiconErrorCode::from(&err);
        assert_eq!(code, LexiconErrorCode::ModelOom);
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(LexiconError::Config("test error".to_string()))
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_all_audio_error_codes() {
        // Test all audio error variants map to correct codes
        let test_cases = vec![
            (AudioError::NoInputDevice, LexiconErrorCode::AudioNoDevice),
            (AudioError::NoOutputDevice, LexiconErrorCode::AudioNoDevice),
            (AudioError::PermissionDenied, LexiconErrorCode::AudioPermissionDenied),
            (AudioError::DeviceDisconnected, LexiconErrorCode::AudioDisconnected),
            (AudioError::UnsupportedFormat { format: "test".into() }, LexiconErrorCode::AudioUnsupportedFormat),
            (AudioError::StreamError { message: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::SessionInterrupted { reason: "test".into() }, LexiconErrorCode::AudioInterrupted),
            (AudioError::BluetoothError { message: "test".into() }, LexiconErrorCode::AudioBluetoothError),
            (AudioError::BufferError { kind: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::DeviceEnumerationFailed { message: "test".into() }, LexiconErrorCode::AudioNoDevice),
            (AudioError::InvalidFrameSize { expected: "160".into(), actual: 100 }, LexiconErrorCode::AudioStreamError),
            (AudioError::VadError { message: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::ResamplerInitFailed { message: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::ResamplerError { message: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::StreamNotStarted, LexiconErrorCode::AudioStreamError),
            (AudioError::StreamNotRunning, LexiconErrorCode::AudioStreamError),
            (AudioError::DeviceNotFound { name: "test".into() }, LexiconErrorCode::AudioNoDevice),
            (AudioError::DeviceConfigFailed { message: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::StreamBuildFailed { message: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::StreamStartFailed { message: "test".into() }, LexiconErrorCode::AudioStreamError),
            (AudioError::Timeout { operation: "read".into() }, LexiconErrorCode::AudioStreamError),
        ];

        for (audio_err, expected_code) in test_cases {
            let err = LexiconError::Audio(audio_err);
            let code = LexiconErrorCode::from(&err);
            assert_eq!(code, expected_code, "Failed for error: {err}");
        }
    }

    #[test]
    fn test_all_model_error_codes() {
        let test_cases = vec![
            (ModelError::NotFound { path: PathBuf::from("/test") }, LexiconErrorCode::ModelNotFound),
            (ModelError::InvalidModel { path: PathBuf::from("/test") }, LexiconErrorCode::ModelInvalid),
            (ModelError::VerificationFailed { expected: "a".into(), actual: "b".into() }, LexiconErrorCode::ModelVerificationFailed),
            (ModelError::SignatureInvalid, LexiconErrorCode::ModelSignatureInvalid),
            (ModelError::OutOfMemory { required_mb: 1024, available_mb: 512 }, LexiconErrorCode::ModelOom),
            (ModelError::VersionMismatch { expected: "1.0".into(), actual: "2.0".into() }, LexiconErrorCode::ModelInvalid),
            (ModelError::DownloadFailed { model_id: "test".into(), message: "test".into() }, LexiconErrorCode::ModelDownloadFailed),
            (ModelError::ManifestError { message: "test".into() }, LexiconErrorCode::ModelSignatureInvalid),
            (ModelError::NotInManifest { model_id: "test".into() }, LexiconErrorCode::ModelNotFound),
        ];

        for (model_err, expected_code) in test_cases {
            let err = LexiconError::Model(model_err);
            let code = LexiconErrorCode::from(&err);
            assert_eq!(code, expected_code, "Failed for error: {err}");
        }
    }

    #[test]
    fn test_all_inference_error_codes() {
        let test_cases = vec![
            (InferenceError::AudioTooShort { min_seconds: 0.5, actual_seconds: 0.1 }, LexiconErrorCode::InferenceTooShort),
            (InferenceError::Timeout { timeout_ms: 30000 }, LexiconErrorCode::InferenceTimeout),
            (InferenceError::ModelNotLoaded, LexiconErrorCode::InferenceModelNotLoaded),
            (InferenceError::ContextCreationFailed { message: "test".into() }, LexiconErrorCode::InferenceInternal),
            (InferenceError::InferenceFailed { message: "test".into() }, LexiconErrorCode::InferenceInternal),
            (InferenceError::InvalidInput { message: "test".into() }, LexiconErrorCode::InferenceInternal),
            (InferenceError::AcceleratorUnavailable { accelerator: "cuda".into() }, LexiconErrorCode::InferenceInternal),
        ];

        for (inference_err, expected_code) in test_cases {
            let err = LexiconError::Inference(inference_err);
            let code = LexiconErrorCode::from(&err);
            assert_eq!(code, expected_code, "Failed for error: {err}");
        }
    }

    #[test]
    fn test_database_error_codes_stub() {
        // Test database error codes using the non-storage stub
        #[cfg(not(feature = "storage"))]
        {
            let test_cases = vec![
                (DatabaseError::OpenFailed { path: PathBuf::from("/test") }, LexiconErrorCode::DatabaseOpenFailed),
                (DatabaseError::MigrationFailed { message: "test".into() }, LexiconErrorCode::DatabaseQueryFailed),
                (DatabaseError::QueryFailed { message: "test".into() }, LexiconErrorCode::DatabaseQueryFailed),
                (DatabaseError::NotFound { entity: "sessions".into(), id: "123".into() }, LexiconErrorCode::DatabaseNotFound),
                (DatabaseError::ConstraintViolation { message: "test".into() }, LexiconErrorCode::DatabaseQueryFailed),
                (DatabaseError::WriteFailed { message: "test".into() }, LexiconErrorCode::DatabaseQueryFailed),
                (DatabaseError::Locked, LexiconErrorCode::DatabaseLocked),
            ];

            for (db_err, expected_code) in test_cases {
                let err = LexiconError::Database(db_err);
                let code = LexiconErrorCode::from(&err);
                assert_eq!(code, expected_code, "Failed for error: {err}");
            }
        }
    }

    #[test]
    fn test_io_and_config_error_codes() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let err = LexiconError::Io(io_err);
        assert_eq!(LexiconErrorCode::from(&err), LexiconErrorCode::IoError);

        let err = LexiconError::Config("bad config".into());
        assert_eq!(LexiconErrorCode::from(&err), LexiconErrorCode::ConfigError);
    }

    #[test]
    fn test_error_code_values() {
        // Test that error codes have the expected numeric values
        assert_eq!(LexiconErrorCode::Ok as i32, 0);
        assert_eq!(LexiconErrorCode::AudioNoDevice as i32, 100);
        assert_eq!(LexiconErrorCode::ModelNotFound as i32, 200);
        assert_eq!(LexiconErrorCode::InferenceTooShort as i32, 300);
        assert_eq!(LexiconErrorCode::InferenceInternal as i32, 302);
        assert_eq!(LexiconErrorCode::DatabaseOpenFailed as i32, 400);
        assert_eq!(LexiconErrorCode::IoError as i32, 500);
        assert_eq!(LexiconErrorCode::Unknown as i32, 999);
    }
}
