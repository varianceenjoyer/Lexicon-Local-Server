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
//! - Simulated audio source for Android/headless testing

pub mod app;
mod audio;
pub mod audio_sim;
mod buffer;
pub mod dialogs;
mod transcription;
pub mod views;
pub mod widgets;

pub use app::TuiApp;

// Re-export internal types for use by other modules
#[allow(unused_imports)]
pub use app::TuiMode;
#[allow(unused_imports)]
pub use audio::AudioDeviceManager;
#[allow(unused_imports)]
pub use audio_sim::{AudioSimError, SimulatedAudioConfig, SimulatedAudioSource};
#[allow(unused_imports)]
pub use buffer::TranscriptionBuffer;
#[allow(unused_imports)]
pub use transcription::{TranscriptionManager, TranscriptionManagerConfig, TranscriptionMessage};

use thiserror::Error;

/// Errors that can occur in the TUI.
#[derive(Debug, Error)]
#[allow(dead_code)] // Some variants reserved for future use
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
