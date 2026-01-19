//! Model Manager - Download, verification, and registry
//!
//! This module handles model lifecycle management including:
//! - Manifest fetching and signature verification (minisign)
//! - Model downloading with SHA-256 hash verification
//! - Local cache management
//! - Device profiling and tier-based model selection
//! - Thermal-aware model switching
//!
//! # Security
//!
//! Models are verified in two stages:
//! 1. Manifest signature verification using minisign public key
//! 2. Individual model hash verification (SHA-256) during download and load
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::models::{ModelRegistry, DownloadProgress};
//! use lexicon_core::platform::DeviceProfile;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = ModelRegistry::new(Path::new("/models"))?;
//!
//! // Get recommended models for this device
//! let profile = DeviceProfile::detect();
//! let recommended = registry.recommend_models(&profile);
//!
//! // Download a model with progress reporting
//! let model_path = registry.download_model("whisper-base-q5_1", |progress: DownloadProgress| {
//!     println!("Progress: {:.1}%", progress.percentage());
//! }).await?;
//! # Ok(())
//! # }
//! ```

mod manifest;
mod registry;
mod types;

pub use manifest::{ManifestVerifier, ModelEntry, ModelManifest};
pub use registry::ModelRegistry;
pub use types::{
    DownloadProgress, DownloadState, ModelId, ModelInfo, ModelReadinessStatus,
    ModelTierRequirements, ModelType,
};

#[cfg(test)]
mod tests;
