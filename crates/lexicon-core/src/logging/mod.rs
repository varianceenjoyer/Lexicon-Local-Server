//! Logging infrastructure for Lexicon
//!
//! This module provides tracing-based structured logging with configurable
//! output formats and levels.

use crate::config::LogLevel;
use crate::error::{LexiconError, Result};

/// Initialize the logging system with the given configuration.
///
/// This should be called once at application startup. Subsequent calls
/// will return an error (tracing subscriber can only be set once).
///
/// # Arguments
///
/// * `level` - The minimum log level to capture
/// * `json` - If true, output logs as JSON; otherwise use human-readable format
///
/// # Errors
///
/// Returns an error if the logging system has already been initialized.
///
/// # Examples
///
/// ```ignore
/// use lexicon_core::logging::init_logging;
/// use lexicon_core::LogLevel;
///
/// init_logging(LogLevel::Debug, false).unwrap();
/// ```
pub fn init_logging(level: LogLevel, json: bool) -> Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    let subscriber = tracing_subscriber::registry().with(filter);

    let result = if json {
        subscriber
            .with(fmt::layer().json().with_target(true))
            .try_init()
    } else {
        subscriber
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_file(false)
                    .with_line_number(false),
            )
            .try_init()
    };

    result.map_err(|e| LexiconError::Config(format!("Failed to initialize logging: {}", e)))
}

/// Creates a tracing span for a transcription session.
///
/// # Arguments
///
/// * `session_id` - Unique identifier for the transcription session
#[must_use]
pub fn session_span(session_id: &str) -> tracing::Span {
    tracing::info_span!("session", session_id = %session_id)
}

/// Creates a tracing span for audio processing.
///
/// # Arguments
///
/// * `chunk_id` - Identifier for the audio chunk being processed
#[must_use]
pub fn audio_span(chunk_id: u64) -> tracing::Span {
    tracing::debug_span!("audio", chunk_id = %chunk_id)
}

/// Creates a tracing span for model operations.
///
/// # Arguments
///
/// * `model_id` - Identifier for the model being used
#[must_use]
pub fn model_span(model_id: &str) -> tracing::Span {
    tracing::info_span!("model", model_id = %model_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging_human() {
        // This test mainly verifies that initialization doesn't panic
        // Note: may fail if logging already initialized by another test
        let _ = init_logging(LogLevel::Info, false);
        tracing::info!("Test log message");
    }

    #[test]
    fn test_init_logging_json() {
        // Second initialization returns an error (expected)
        let _ = init_logging(LogLevel::Debug, true);
        tracing::debug!("Test debug message");
    }

    #[test]
    fn test_session_span() {
        let span = session_span("test-session-123");
        let _guard = span.enter();
        tracing::info!("Inside session span");
    }

    #[test]
    fn test_audio_span() {
        let span = audio_span(42);
        let _guard = span.enter();
        tracing::debug!("Inside audio span");
    }

    #[test]
    fn test_model_span() {
        let span = model_span("whisper-base");
        let _guard = span.enter();
        tracing::info!("Inside model span");
    }
}
