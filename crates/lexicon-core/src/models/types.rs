//! Core types for model management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl ModelId {
    /// Creates a new model ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ModelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Type of model (STT or TTS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    /// Speech-to-text (Whisper) model
    Stt,
    /// Text-to-speech (Kokoro) model
    Tts,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Stt => write!(f, "stt"),
            ModelType::Tts => write!(f, "tts"),
        }
    }
}

/// Information about a downloaded/cached model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Unique model identifier
    pub id: ModelId,
    /// Human-readable name
    pub name: String,
    /// Model type (STT or TTS)
    pub model_type: ModelType,
    /// Size in bytes
    pub size_bytes: u64,
    /// Expected SHA-256 hash
    pub hash_sha256: String,
    /// Local file path (if downloaded)
    pub local_path: Option<PathBuf>,
    /// Whether the model has been verified
    pub verified: bool,
    /// Quantization type (e.g., "q5_1", "f16")
    pub quantization: Option<String>,
    /// Supported languages (for STT models)
    pub languages: Vec<String>,
    /// Memory requirement in MB
    pub memory_mb: u64,
}

impl ModelInfo {
    /// Check if the model is downloaded and available locally.
    #[must_use]
    pub fn is_downloaded(&self) -> bool {
        self.local_path
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Check if the model is ready for use (downloaded and verified).
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.is_downloaded() && self.verified
    }
}

/// Download progress information.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Model being downloaded
    pub model_id: ModelId,
    /// Bytes downloaded so far
    pub downloaded_bytes: u64,
    /// Total bytes to download
    pub total_bytes: u64,
    /// Download speed in bytes per second
    pub speed_bps: Option<u64>,
    /// Estimated time remaining in seconds
    pub eta_seconds: Option<u64>,
}

impl DownloadProgress {
    /// Creates a new download progress instance.
    #[must_use]
    pub fn new(model_id: ModelId, downloaded: u64, total: u64) -> Self {
        Self {
            model_id,
            downloaded_bytes: downloaded,
            total_bytes: total,
            speed_bps: None,
            eta_seconds: None,
        }
    }

    /// Returns the progress as a percentage (0.0 to 100.0).
    #[must_use]
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }

    /// Updates speed and ETA calculations.
    pub fn update_speed(&mut self, elapsed_seconds: f64) {
        if elapsed_seconds > 0.0 {
            self.speed_bps = Some((self.downloaded_bytes as f64 / elapsed_seconds) as u64);

            if let Some(speed) = self.speed_bps {
                if speed > 0 {
                    let remaining = self.total_bytes.saturating_sub(self.downloaded_bytes);
                    self.eta_seconds = Some(remaining / speed);
                }
            }
        }
    }
}

/// Model tier requirements for device capability matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTierRequirements {
    /// Minimum available memory in MB
    pub min_memory_mb: u64,
    /// Whether GPU is required
    pub requires_gpu: bool,
    /// Supported platforms (empty = all platforms)
    pub platforms: Vec<String>,
}

impl Default for ModelTierRequirements {
    fn default() -> Self {
        Self {
            min_memory_mb: 512,
            requires_gpu: false,
            platforms: Vec::new(),
        }
    }
}

/// Download state for tracking model download status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadState {
    /// Model is not downloaded
    NotDownloaded,
    /// Download is in progress
    Downloading,
    /// Download completed, verification pending
    PendingVerification,
    /// Model is downloaded and verified
    Ready,
    /// Download or verification failed
    Failed,
    /// Download was cancelled by user
    Cancelled,
}

impl std::fmt::Display for DownloadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadState::NotDownloaded => write!(f, "not_downloaded"),
            DownloadState::Downloading => write!(f, "downloading"),
            DownloadState::PendingVerification => write!(f, "pending_verification"),
            DownloadState::Ready => write!(f, "ready"),
            DownloadState::Failed => write!(f, "failed"),
            DownloadState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Summary of model readiness status.
///
/// Used to report the current state of model availability
/// for application status displays and feature enablement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReadinessStatus {
    /// Whether at least one STT model is ready
    pub stt_ready: bool,
    /// Whether at least one TTS model is ready
    pub tts_ready: bool,
    /// Number of STT models available locally
    pub stt_models_available: usize,
    /// Number of TTS models available locally
    pub tts_models_available: usize,
    /// Total number of models in the manifest
    pub total_models_in_manifest: usize,
}

impl ModelReadinessStatus {
    /// Returns true if the application is ready for basic functionality.
    ///
    /// Basic functionality requires at least one STT model.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.stt_ready
    }

    /// Returns true if all features are ready.
    ///
    /// Full functionality requires both STT and TTS models.
    #[must_use]
    pub fn is_fully_ready(&self) -> bool {
        self.stt_ready && self.tts_ready
    }
}
