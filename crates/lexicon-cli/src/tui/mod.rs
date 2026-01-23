//! Diagnostic TUI for Real-Time Transcription
//!
//! This module provides a terminal user interface for real-time audio diagnostics
//! and live transcription. Key features:
//!
//! - Audio device selection dropdown
//! - Real-time signal level meter
//! - Ctrl+T toggle to start/stop transcription
//! - Scrollable text area for transcription output
//! - Save/rename dialog with keyboard shortcuts
//! - Saved transcriptions browser

pub mod app;
mod audio;
mod buffer;
pub mod dialogs;
mod transcription;
pub mod views;
pub mod widgets;

pub use app::{TuiApp, TuiMode};
pub use audio::AudioDeviceManager;
pub use buffer::TranscriptionBuffer;
pub use transcription::{TranscriptionManager, TranscriptionManagerConfig, TranscriptionMessage};

use thiserror::Error;

/// Errors that can occur in the TUI.
#[derive(Debug, Error)]
pub enum TuiError {
    #[error("Terminal I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),
}

/// Result type for TUI operations.
pub type Result<T> = std::result::Result<T, TuiError>;
