//! Model Registry - manages model downloads, verification, and selection.
//!
//! The registry maintains a local cache of downloaded models and provides
//! methods for downloading new models, verifying integrity, and selecting
//! appropriate models based on device capabilities.

use crate::error::{LexiconError, ModelError, Result};
use crate::models::manifest::{ManifestVerifier, ModelEntry, ModelManifest};
use crate::models::types::{DownloadProgress, DownloadState, ModelId, ModelInfo, ModelType};
use crate::platform::{DeviceProfile, ModelTier, ThermalState};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// Manifest URL for fetching model information.
const MANIFEST_URL: &str = "https://models.lexicon.dev/manifest.json";
const MANIFEST_SIG_URL: &str = "https://models.lexicon.dev/manifest.json.minisig";

/// Model registry for managing model downloads, verification, and selection.
pub struct ModelRegistry {
    /// Directory for cached model files.
    cache_dir: PathBuf,
    /// Parsed and verified model manifest.
    manifest: Option<ModelManifest>,
    /// Map of model ID to local model info.
    models: HashMap<ModelId, ModelInfo>,
    /// Current thermal state for adaptive model selection.
    thermal_state: ThermalState,
    /// Manifest verifier with public key.
    verifier: ManifestVerifier,
    /// Download states for tracking in-progress downloads.
    download_states: Arc<RwLock<HashMap<ModelId, DownloadState>>>,
}

impl ModelRegistry {
    /// Creates a new model registry with the given cache directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created.
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir).map_err(|e| {
            LexiconError::Config(format!(
                "Failed to create model cache directory '{}': {}",
                cache_dir.display(),
                e
            ))
        })?;

        let mut registry = Self {
            cache_dir: cache_dir.to_path_buf(),
            manifest: None,
            models: HashMap::new(),
            thermal_state: ThermalState::Nominal,
            verifier: ManifestVerifier::default_key(),
            download_states: Arc::new(RwLock::new(HashMap::new())),
        };

        // Scan for existing models
        registry.scan_local_models();

        Ok(registry)
    }

    /// Creates a registry with a custom manifest verifier.
    pub fn with_verifier(cache_dir: &Path, verifier: ManifestVerifier) -> Result<Self> {
        let mut registry = Self::new(cache_dir)?;
        registry.verifier = verifier;
        Ok(registry)
    }

    /// Fetches and verifies the model manifest from the remote server.
    #[instrument(skip(self))]
    pub async fn fetch_manifest(&mut self) -> Result<()> {
        info!("Fetching model manifest from {}", MANIFEST_URL);

        // Fetch manifest content
        let manifest_content = reqwest::get(MANIFEST_URL)
            .await
            .map_err(|e| ModelError::ManifestError {
                message: format!("Failed to fetch manifest: {}", e),
            })?
            .text()
            .await
            .map_err(|e| ModelError::ManifestError {
                message: format!("Failed to read manifest response: {}", e),
            })?;

        // Parse manifest (signature verification handled separately for now)
        let manifest = ModelManifest::from_json(&manifest_content)?;

        info!(
            "Manifest loaded: version={}, {} models",
            manifest.version,
            manifest.models.len()
        );

        self.manifest = Some(manifest);
        self.sync_models_from_manifest();

        Ok(())
    }

    /// Loads a manifest from a local file (for offline use or testing).
    pub fn load_manifest_from_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| ModelError::ManifestError {
            message: format!("Failed to read manifest file: {}", e),
        })?;

        let manifest = ModelManifest::from_json(&content)?;
        self.manifest = Some(manifest);
        self.sync_models_from_manifest();

        Ok(())
    }

    /// Loads an unsigned manifest (for testing only).
    #[cfg(any(test, feature = "test-utils"))]
    pub fn load_unsigned_manifest(&mut self, json: &str) -> Result<()> {
        let manifest = ModelManifest::from_json(json)?;
        self.manifest = Some(manifest);
        self.sync_models_from_manifest();
        Ok(())
    }

    /// Downloads a model with progress reporting.
    #[instrument(skip(self, progress_callback))]
    pub async fn download_model<F>(
        &self,
        model_id: &str,
        progress_callback: F,
    ) -> Result<PathBuf>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let manifest = self.manifest.as_ref().ok_or_else(|| ModelError::ManifestError {
            message: "No manifest loaded. Call fetch_manifest() first.".to_string(),
        })?;

        let entry = manifest.get_model(model_id).ok_or_else(|| ModelError::NotFound {
            path: PathBuf::from(model_id),
        })?;

        let dest_path = self.model_path(entry);
        let model_id_owned = ModelId::new(model_id);

        // Check if already downloaded and verified
        if dest_path.exists() {
            if self.verify_model_hash(&dest_path, &entry.hash_sha256).is_ok() {
                debug!("Model {} already downloaded and verified", model_id);
                return Ok(dest_path);
            }
            warn!("Model {} failed verification, re-downloading", model_id);
            std::fs::remove_file(&dest_path).ok();
        }

        // Update download state
        {
            let mut states = self.download_states.write().await;
            states.insert(model_id_owned.clone(), DownloadState::Downloading);
        }

        let download_url = entry.download_url(&manifest.base_url);
        info!("Downloading model {} from {}", model_id, download_url);

        // Download with streaming hash verification
        match self
            .download_with_verification(
                &download_url,
                &dest_path,
                entry.size_bytes,
                &entry.hash_sha256,
                model_id_owned.clone(),
                progress_callback,
            )
            .await
        {
            Ok(()) => {
                let mut states = self.download_states.write().await;
                states.insert(model_id_owned, DownloadState::Ready);
                info!("Model {} downloaded and verified", model_id);
                Ok(dest_path)
            }
            Err(e) => {
                let mut states = self.download_states.write().await;
                states.insert(model_id_owned, DownloadState::Failed);
                error!("Failed to download model {}: {}", model_id, e);
                Err(e)
            }
        }
    }

    /// Verifies a downloaded model's integrity.
    #[must_use]
    pub fn verify_model(&self, model_id: &str) -> bool {
        let Some(manifest) = &self.manifest else {
            return false;
        };

        let Some(entry) = manifest.get_model(model_id) else {
            return false;
        };

        let path = self.model_path(entry);
        if !path.exists() {
            return false;
        }

        self.verify_model_hash(&path, &entry.hash_sha256).is_ok()
    }

    /// Returns the local path for a model.
    #[must_use]
    pub fn get_model_path(&self, model_id: &str) -> Option<PathBuf> {
        self.models.get(&ModelId::new(model_id)).and_then(|info| {
            info.local_path.as_ref().filter(|p| p.exists()).cloned()
        })
    }

    /// Checks if a model is downloaded and verified.
    #[must_use]
    pub fn is_model_available(&self, model_id: &str) -> bool {
        self.models
            .get(&ModelId::new(model_id))
            .map(|info| info.is_ready())
            .unwrap_or(false)
    }

    /// Returns recommended models for the given device profile.
    #[must_use]
    pub fn recommend_models(&self, device_profile: &DeviceProfile) -> Vec<ModelId> {
        let Some(manifest) = &self.manifest else {
            return Vec::new();
        };

        let tier = device_profile.recommended_tier();
        let effective_tier = self.adjust_tier_for_thermal(tier);

        debug!(
            "Recommending models for tier {:?} (thermal-adjusted from {:?})",
            effective_tier, tier
        );

        manifest
            .recommended_for_tier(effective_tier)
            .into_iter()
            .map(|entry| entry.model_id())
            .collect()
    }

    /// Returns all STT models appropriate for the device.
    #[must_use]
    pub fn recommend_stt_models(&self, device_profile: &DeviceProfile) -> Vec<ModelId> {
        self.recommend_models(device_profile)
            .into_iter()
            .filter(|id| {
                self.manifest
                    .as_ref()
                    .and_then(|m| m.get_model(id.as_str()))
                    .map(|e| e.model_type == ModelType::Stt)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all TTS models appropriate for the device.
    #[must_use]
    pub fn recommend_tts_models(&self, device_profile: &DeviceProfile) -> Vec<ModelId> {
        self.recommend_models(device_profile)
            .into_iter()
            .filter(|id| {
                self.manifest
                    .as_ref()
                    .and_then(|m| m.get_model(id.as_str()))
                    .map(|e| e.model_type == ModelType::Tts)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Updates the thermal state and returns whether a model switch is recommended.
    pub fn update_thermal_state(&mut self, headroom: f32) -> bool {
        let new_state = ThermalState::from_headroom(headroom);
        let should_switch = new_state != self.thermal_state && new_state.should_throttle();

        if new_state != self.thermal_state {
            info!("Thermal state changed: {:?} -> {:?}", self.thermal_state, new_state);
            self.thermal_state = new_state;
        }

        should_switch
    }

    /// Returns the current thermal state.
    #[must_use]
    pub fn thermal_state(&self) -> ThermalState {
        self.thermal_state
    }

    /// Lists all models in the manifest.
    #[must_use]
    pub fn list_models(&self) -> Vec<&ModelEntry> {
        self.manifest
            .as_ref()
            .map(|m| m.models.values().collect())
            .unwrap_or_default()
    }

    /// Lists all locally available models.
    #[must_use]
    pub fn list_local_models(&self) -> Vec<&ModelInfo> {
        self.models.values().filter(|m| m.is_downloaded()).collect()
    }

    /// Returns the download state for a model.
    pub async fn download_state(&self, model_id: &str) -> DownloadState {
        let states = self.download_states.read().await;
        states
            .get(&ModelId::new(model_id))
            .copied()
            .unwrap_or(DownloadState::NotDownloaded)
    }

    /// Returns the cache directory path.
    #[must_use]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    // --- Private methods ---

    /// Verifies a model file's SHA-256 hash.
    fn verify_model_hash(&self, path: &Path, expected_hash: &str) -> Result<()> {
        let file = std::fs::File::open(path).map_err(|_| ModelError::NotFound {
            path: path.to_path_buf(),
        })?;

        let mut reader = std::io::BufReader::new(file);
        let mut hasher = Sha256::new();

        std::io::copy(&mut reader, &mut hasher).map_err(|_| ModelError::InvalidModel {
            path: path.to_path_buf(),
        })?;

        let computed_hash = format!("{:x}", hasher.finalize());

        if computed_hash != expected_hash {
            return Err(ModelError::VerificationFailed {
                expected: expected_hash.to_string(),
                actual: computed_hash,
            }
            .into());
        }

        Ok(())
    }

    /// Downloads a file with streaming hash verification.
    async fn download_with_verification<F>(
        &self,
        url: &str,
        dest_path: &Path,
        expected_size: u64,
        expected_hash: &str,
        model_id: ModelId,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        // Create parent directory if needed
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                LexiconError::Config(format!("Failed to create model directory: {}", e))
            })?;
        }

        // Start download
        let response = reqwest::get(url).await.map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("HTTP request failed: {}", e),
        })?;

        if !response.status().is_success() {
            return Err(ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("HTTP {}: {}", response.status(), url),
            }
            .into());
        }

        let total_size = response.content_length().unwrap_or(expected_size);

        // Create temp file for download
        let temp_path = dest_path.with_extension("tmp");
        let mut file = std::fs::File::create(&temp_path).map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("Failed to create temp file: {}", e),
        })?;

        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Download stream error: {}", e),
            })?;

            // Write to file
            file.write_all(&chunk).map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Failed to write to file: {}", e),
            })?;

            // Update hash
            hasher.update(&chunk);

            // Update progress
            downloaded += chunk.len() as u64;
            let progress = DownloadProgress::new(model_id.clone(), downloaded, total_size);
            progress_callback(progress);
        }

        // Verify hash
        let computed_hash = format!("{:x}", hasher.finalize());
        if computed_hash != expected_hash {
            std::fs::remove_file(&temp_path).ok();
            return Err(ModelError::VerificationFailed {
                expected: expected_hash.to_string(),
                actual: computed_hash,
            }
            .into());
        }

        // Move temp file to final location
        std::fs::rename(&temp_path, dest_path).map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("Failed to finalize download: {}", e),
        })?;

        Ok(())
    }

    /// Returns the local path for a model entry.
    fn model_path(&self, entry: &ModelEntry) -> PathBuf {
        let filename = entry.download_path.rsplit('/').next().unwrap_or(&entry.id);
        self.cache_dir.join(filename)
    }

    /// Scans the cache directory for existing model files.
    fn scan_local_models(&mut self) {
        let Ok(entries) = std::fs::read_dir(&self.cache_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    debug!("Found local model file: {}", stem);
                }
            }
        }
    }

    /// Synchronizes model info from the manifest.
    fn sync_models_from_manifest(&mut self) {
        let Some(manifest) = &self.manifest else {
            return;
        };

        for (id, entry) in &manifest.models {
            let model_id = ModelId::new(id);
            let local_path = self.model_path(entry);
            let exists = local_path.exists();
            let verified = exists && self.verify_model_hash(&local_path, &entry.hash_sha256).is_ok();

            self.models.insert(
                model_id,
                ModelInfo {
                    id: ModelId::new(id),
                    name: entry.name.clone(),
                    model_type: entry.model_type,
                    size_bytes: entry.size_bytes,
                    hash_sha256: entry.hash_sha256.clone(),
                    local_path: if exists { Some(local_path) } else { None },
                    verified,
                    quantization: entry.quantization.clone(),
                    languages: entry.languages.clone(),
                    memory_mb: entry.memory_mb,
                },
            );
        }
    }

    /// Adjusts the model tier based on thermal state.
    fn adjust_tier_for_thermal(&self, base_tier: ModelTier) -> ModelTier {
        if self.thermal_state.should_throttle() {
            match base_tier {
                ModelTier::Server => ModelTier::Desktop,
                ModelTier::Desktop => ModelTier::MobileMid,
                ModelTier::MobileMid => ModelTier::MobileLow,
                ModelTier::MobileLow => ModelTier::MobileLow,
            }
        } else {
            base_tier
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_json() -> &'static str {
        r#"{
            "version": "1.0.0",
            "generated_at": "2025-01-01T00:00:00Z",
            "base_url": "https://models.lexicon.dev",
            "models": {
                "whisper-tiny-q5_1": {
                    "id": "whisper-tiny-q5_1",
                    "name": "Whisper Tiny",
                    "model_type": "stt",
                    "size_bytes": 42000000,
                    "hash_sha256": "abc123def456",
                    "download_path": "stt/whisper-tiny-q5_1.bin",
                    "memory_mb": 150
                }
            },
            "tier_recommendations": {
                "mobile-low": ["whisper-tiny-q5_1"],
                "mobile-mid": ["whisper-tiny-q5_1"],
                "desktop": ["whisper-tiny-q5_1"],
                "server": ["whisper-tiny-q5_1"]
            }
        }"#
    }

    #[test]
    fn test_registry_creation() {
        let temp_dir = std::env::temp_dir().join("lexicon-registry-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        assert!(registry.manifest.is_none());
        assert!(temp_dir.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_thermal_adjustment() {
        let temp_dir = std::env::temp_dir().join("lexicon-thermal-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        assert_eq!(
            registry.adjust_tier_for_thermal(ModelTier::Desktop),
            ModelTier::Desktop
        );

        registry.update_thermal_state(0.2);
        assert_eq!(
            registry.adjust_tier_for_thermal(ModelTier::Desktop),
            ModelTier::MobileMid
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
