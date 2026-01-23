//! Model Registry - manages model downloads, verification, and selection.
//!
//! The registry maintains a local cache of downloaded models and provides
//! methods for downloading new models, verifying integrity, and selecting
//! appropriate models based on device capabilities.

use crate::error::{LexiconError, ModelError, Result};
use crate::models::manifest::{ManifestVerifier, ModelEntry, ModelManifest};
use crate::models::types::{
    DownloadProgress, DownloadState, ModelId, ModelInfo, ModelReadinessStatus, ModelType,
};
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
#[allow(dead_code)]
const MANIFEST_SIG_URL: &str = "https://models.lexicon.dev/manifest.json.minisig";

/// Embedded fallback manifest JSON with Hugging Face URLs.
/// Used when the primary manifest server is unavailable.
/// These models are from the official ggerganov/whisper.cpp repository on Hugging Face.
///
/// Note: Hash values are set to "skip" to bypass verification for fallback downloads.
/// This is acceptable because:
/// 1. Hugging Face uses HTTPS with certificate validation
/// 2. The models are from the official ggerganov/whisper.cpp repository
/// 3. This is only used as a fallback when the primary signed manifest is unavailable
///
/// The download_path uses the model ID with .bin extension to match CLI expectations,
/// while the actual Hugging Face URL is constructed using the ggml-*.bin naming convention.
const FALLBACK_MANIFEST_JSON: &str = r#"{
    "version": "1.0.0-fallback",
    "generated_at": "2025-01-01T00:00:00Z",
    "base_url": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main",
    "models": {
        "whisper-tiny": {
            "id": "whisper-tiny",
            "name": "Whisper Tiny",
            "model_type": "stt",
            "size_bytes": 77704715,
            "hash_sha256": "skip",
            "download_path": "ggml-tiny.bin",
            "quantization": null,
            "languages": ["multilingual"],
            "memory_mb": 150,
            "description": "Whisper Tiny - fastest, lowest accuracy"
        },
        "whisper-tiny-q5_1": {
            "id": "whisper-tiny-q5_1",
            "name": "Whisper Tiny Q5_1",
            "model_type": "stt",
            "size_bytes": 33800000,
            "hash_sha256": "skip",
            "download_path": "ggml-tiny-q5_1.bin",
            "quantization": "q5_1",
            "languages": ["multilingual"],
            "memory_mb": 100,
            "description": "Whisper Tiny Q5_1 quantized - faster, lower memory"
        },
        "whisper-base": {
            "id": "whisper-base",
            "name": "Whisper Base",
            "model_type": "stt",
            "size_bytes": 148000000,
            "hash_sha256": "skip",
            "download_path": "ggml-base.bin",
            "quantization": null,
            "languages": ["multilingual"],
            "memory_mb": 300,
            "description": "Whisper Base - balanced speed/accuracy"
        },
        "whisper-base-q5_1": {
            "id": "whisper-base-q5_1",
            "name": "Whisper Base Q5_1",
            "model_type": "stt",
            "size_bytes": 62600000,
            "hash_sha256": "skip",
            "download_path": "ggml-base-q5_1.bin",
            "quantization": "q5_1",
            "languages": ["multilingual"],
            "memory_mb": 200,
            "description": "Whisper Base Q5_1 quantized - balanced, lower memory"
        },
        "whisper-small": {
            "id": "whisper-small",
            "name": "Whisper Small",
            "model_type": "stt",
            "size_bytes": 488000000,
            "hash_sha256": "skip",
            "download_path": "ggml-small.bin",
            "quantization": null,
            "languages": ["multilingual"],
            "memory_mb": 600,
            "description": "Whisper Small - higher accuracy, slower"
        },
        "whisper-small-q5_1": {
            "id": "whisper-small-q5_1",
            "name": "Whisper Small Q5_1",
            "model_type": "stt",
            "size_bytes": 190000000,
            "hash_sha256": "skip",
            "download_path": "ggml-small-q5_1.bin",
            "quantization": "q5_1",
            "languages": ["multilingual"],
            "memory_mb": 400,
            "description": "Whisper Small Q5_1 quantized - good accuracy, moderate memory"
        },
        "whisper-medium": {
            "id": "whisper-medium",
            "name": "Whisper Medium",
            "model_type": "stt",
            "size_bytes": 1530000000,
            "hash_sha256": "skip",
            "download_path": "ggml-medium.bin",
            "quantization": null,
            "languages": ["multilingual"],
            "memory_mb": 1500,
            "description": "Whisper Medium - high accuracy"
        },
        "whisper-large-v3": {
            "id": "whisper-large-v3",
            "name": "Whisper Large v3",
            "model_type": "stt",
            "size_bytes": 3100000000,
            "hash_sha256": "skip",
            "download_path": "ggml-large-v3.bin",
            "quantization": null,
            "languages": ["multilingual"],
            "memory_mb": 3000,
            "description": "Whisper Large v3 - highest accuracy"
        },
        "whisper-large-v3-turbo": {
            "id": "whisper-large-v3-turbo",
            "name": "Whisper Large v3 Turbo",
            "model_type": "stt",
            "size_bytes": 1620000000,
            "hash_sha256": "skip",
            "download_path": "ggml-large-v3-turbo.bin",
            "quantization": null,
            "languages": ["multilingual"],
            "memory_mb": 1800,
            "description": "Whisper Large v3 Turbo - fast high accuracy"
        }
    },
    "tier_recommendations": {
        "mobile-low": ["whisper-tiny-q5_1"],
        "mobile-mid": ["whisper-tiny-q5_1", "whisper-base-q5_1"],
        "desktop": ["whisper-base", "whisper-small"],
        "server": ["whisper-medium", "whisper-large-v3"]
    }
}"#;

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
    /// HTTP client with browser-like headers for model downloads.
    /// Some CDNs/firewalls block requests without proper User-Agent headers.
    http_client: reqwest::Client,
}

impl ModelRegistry {
    /// Creates a new model registry with the given cache directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or if
    /// the default manifest verifier key is invalid.
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir).map_err(|e| {
            LexiconError::Config(format!(
                "Failed to create model cache directory '{}': {}",
                cache_dir.display(),
                e
            ))
        })?;

        let http_client = Self::build_http_client()?;

        let mut registry = Self {
            cache_dir: cache_dir.to_path_buf(),
            manifest: None,
            models: HashMap::new(),
            thermal_state: ThermalState::Nominal,
            verifier: ManifestVerifier::default_key(),
            download_states: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        };

        // Scan for existing models
        registry.scan_local_models();

        Ok(registry)
    }

    /// Creates a registry with a custom manifest verifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created.
    pub fn with_verifier(cache_dir: &Path, verifier: ManifestVerifier) -> Result<Self> {
        std::fs::create_dir_all(cache_dir).map_err(|e| {
            LexiconError::Config(format!(
                "Failed to create model cache directory '{}': {}",
                cache_dir.display(),
                e
            ))
        })?;

        let http_client = Self::build_http_client()?;

        let mut registry = Self {
            cache_dir: cache_dir.to_path_buf(),
            manifest: None,
            models: HashMap::new(),
            thermal_state: ThermalState::Nominal,
            verifier,
            download_states: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        };

        // Scan for existing models
        registry.scan_local_models();

        Ok(registry)
    }

    /// Builds an HTTP client with browser-like headers.
    ///
    /// Some CDNs and firewalls (particularly in corporate/WSL environments)
    /// block requests that don't have proper User-Agent headers.
    /// Headers match the working curl command for Hugging Face downloads.
    fn build_http_client() -> Result<reqwest::Client> {
        use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));

        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .default_headers(headers)
            .build()
            .map_err(|e| LexiconError::Config(format!("Failed to create HTTP client: {e}")))
    }

    /// Fetches and verifies the model manifest from the remote server.
    ///
    /// This method fetches the signed manifest and verifies its signature
    /// before parsing. If signature verification fails, the manifest is rejected.
    ///
    /// If the primary manifest server is unavailable, falls back to an embedded
    /// manifest with Hugging Face URLs for trusted model downloads.
    ///
    /// For development without a signed manifest server, use `fetch_manifest_unsigned`.
    #[instrument(skip(self))]
    pub async fn fetch_manifest(&mut self) -> Result<()> {
        info!("Fetching signed model manifest from {}", MANIFEST_URL);

        // Try to fetch signed manifest content (includes signature + JSON)
        let fetch_result = self.http_client.get(MANIFEST_URL).send().await;

        match fetch_result {
            Ok(response) => {
                match response.text().await {
                    Ok(signed_content) => {
                        // Try to verify signature and parse manifest
                        match self.verifier.verify_and_parse(&signed_content) {
                            Ok(manifest) => {
                                info!(
                                    "Manifest loaded and verified: version={}, {} models",
                                    manifest.version,
                                    manifest.models.len()
                                );
                                self.manifest = Some(manifest);
                                self.sync_models_from_manifest();
                                return Ok(());
                            }
                            Err(e) => {
                                warn!("Manifest signature verification failed: {}, using fallback", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read manifest response: {}, using fallback", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to fetch manifest from {}: {}, using fallback", MANIFEST_URL, e);
            }
        }

        // Fall back to embedded manifest with Hugging Face URLs
        self.load_fallback_manifest()
    }

    /// Loads the embedded fallback manifest with Hugging Face URLs.
    ///
    /// This manifest contains trusted model URLs from the official
    /// ggerganov/whisper.cpp repository on Hugging Face.
    fn load_fallback_manifest(&mut self) -> Result<()> {
        info!("Loading fallback manifest with Hugging Face model URLs");

        let manifest = ModelManifest::from_json(FALLBACK_MANIFEST_JSON).map_err(|e| {
            error!("Failed to parse fallback manifest: {}", e);
            e
        })?;

        info!(
            "Fallback manifest loaded: version={}, {} models (source: Hugging Face)",
            manifest.version,
            manifest.models.len()
        );

        self.manifest = Some(manifest);
        self.sync_models_from_manifest();

        Ok(())
    }

    /// Fetches an unsigned manifest (for development/testing only).
    ///
    /// # Security Warning
    ///
    /// This bypasses signature verification and should only be used in
    /// development environments where the manifest source is trusted.
    #[instrument(skip(self))]
    pub async fn fetch_manifest_unsigned(&mut self) -> Result<()> {
        warn!("Fetching UNSIGNED manifest - do not use in production!");
        info!("Fetching model manifest from {}", MANIFEST_URL);

        // Fetch manifest content using client with browser-like headers
        let manifest_content = self
            .http_client
            .get(MANIFEST_URL)
            .send()
            .await
            .map_err(|e| ModelError::ManifestError {
                message: format!("Failed to fetch manifest: {e}"),
            })?
            .text()
            .await
            .map_err(|e| ModelError::ManifestError {
                message: format!("Failed to read manifest response: {e}"),
            })?;

        // Parse manifest without signature verification
        let manifest = ModelManifest::from_json(&manifest_content)?;

        info!(
            "Manifest loaded (UNSIGNED): version={}, {} models",
            manifest.version,
            manifest.models.len()
        );

        self.manifest = Some(manifest);
        self.sync_models_from_manifest();

        Ok(())
    }

    /// Loads a signed manifest from a local file.
    ///
    /// The file should contain a minisign signature followed by the manifest JSON,
    /// separated by `---MANIFEST---`.
    pub fn load_manifest_from_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| ModelError::ManifestError {
            message: format!("Failed to read manifest file: {e}"),
        })?;

        let manifest = self.verifier.verify_and_parse(&content)?;
        self.manifest = Some(manifest);
        self.sync_models_from_manifest();

        Ok(())
    }

    /// Loads an unsigned manifest from a local file (for testing only).
    ///
    /// # Security Warning
    ///
    /// This bypasses signature verification and should only be used in
    /// development/testing scenarios.
    pub fn load_manifest_from_file_unsigned(&mut self, path: &Path) -> Result<()> {
        warn!("Loading UNSIGNED manifest from file - do not use in production!");

        let content = std::fs::read_to_string(path).map_err(|e| ModelError::ManifestError {
            message: format!("Failed to read manifest file: {e}"),
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

    /// Downloads a model with progress reporting and cancellation support.
    ///
    /// The progress callback returns a boolean: `true` to continue, `false` to cancel.
    /// On cancellation, the download state is set to `Cancelled` and partial files are cleaned up.
    #[instrument(skip(self, progress_callback))]
    pub async fn download_model_cancellable<F>(
        &self,
        model_id: &str,
        progress_callback: F,
    ) -> Result<PathBuf>
    where
        F: Fn(DownloadProgress) -> bool + Send + Sync + 'static,
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

        // Download with streaming hash verification and cancellation
        match self
            .download_with_verification_cancellable(
                &download_url,
                &dest_path,
                entry.size_bytes,
                &entry.hash_sha256,
                model_id_owned.clone(),
                progress_callback,
            )
            .await
        {
            Ok(completed) => {
                if completed {
                    let mut states = self.download_states.write().await;
                    states.insert(model_id_owned, DownloadState::Ready);
                    info!("Model {} downloaded and verified", model_id);
                    Ok(dest_path)
                } else {
                    // Download was cancelled
                    let mut states = self.download_states.write().await;
                    states.insert(model_id_owned, DownloadState::Cancelled);
                    info!("Model {} download cancelled", model_id);
                    Err(ModelError::DownloadFailed {
                        model_id: model_id.to_string(),
                        message: "Download cancelled by user".to_string(),
                    }
                    .into())
                }
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

    // --- D1: Immediate Download Flow (First Launch) ---

    /// Checks if at least one STT model is downloaded and ready.
    ///
    /// Use this to determine if transcription features can be enabled.
    #[must_use]
    pub fn is_stt_ready(&self) -> bool {
        self.models.values().any(|m| m.model_type == ModelType::Stt && m.is_ready())
    }

    /// Checks if at least one TTS model is downloaded and ready.
    ///
    /// Use this to determine if synthesis features can be enabled.
    #[must_use]
    pub fn is_tts_ready(&self) -> bool {
        self.models.values().any(|m| m.model_type == ModelType::Tts && m.is_ready())
    }

    /// Returns the best available STT model for the device.
    ///
    /// Returns the highest-tier downloaded STT model that fits the device profile.
    #[must_use]
    pub fn best_stt_model(&self, device_profile: &DeviceProfile) -> Option<&ModelInfo> {
        let recommended = self.recommend_stt_models(device_profile);

        // Try to find the first recommended model that's ready
        for model_id in &recommended {
            if let Some(info) = self.models.get(model_id) {
                if info.is_ready() {
                    return Some(info);
                }
            }
        }

        // Fall back to any ready STT model
        self.models
            .values()
            .filter(|m| m.model_type == ModelType::Stt && m.is_ready())
            .min_by_key(|m| m.memory_mb) // Prefer smaller model as fallback
    }

    /// Returns the best available TTS model for the device.
    ///
    /// Returns the highest-tier downloaded TTS model that fits the device profile.
    #[must_use]
    pub fn best_tts_model(&self, device_profile: &DeviceProfile) -> Option<&ModelInfo> {
        let recommended = self.recommend_tts_models(device_profile);

        for model_id in &recommended {
            if let Some(info) = self.models.get(model_id) {
                if info.is_ready() {
                    return Some(info);
                }
            }
        }

        self.models
            .values()
            .find(|m| m.model_type == ModelType::Tts && m.is_ready())
    }

    /// Ensures at least one STT model is ready, blocking until download completes.
    ///
    /// This is the key D1 (Decision 1) method: blocks STT features until a model
    /// is available. Should be called during application startup.
    ///
    /// # Arguments
    ///
    /// * `device_profile` - Device capabilities for model selection
    /// * `progress_callback` - Called with download progress updates
    ///
    /// # Errors
    ///
    /// Returns an error if no manifest is loaded or download fails.
    #[instrument(skip(self, progress_callback))]
    pub async fn ensure_stt_ready<F>(
        &self,
        device_profile: &DeviceProfile,
        progress_callback: F,
    ) -> Result<PathBuf>
    where
        F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
    {
        // Check if already ready
        if let Some(model) = self.best_stt_model(device_profile) {
            info!("STT model already available: {}", model.name);
            return model.local_path.clone().ok_or_else(|| {
                ModelError::NotFound {
                    path: self.cache_dir.clone(),
                }
                .into()
            });
        }

        // Get recommended STT models for this device
        let recommended = self.recommend_stt_models(device_profile);
        if recommended.is_empty() {
            return Err(ModelError::ManifestError {
                message: "No STT models recommended for this device tier".to_string(),
            }
            .into());
        }

        // Download the first (highest priority) recommended model
        let model_id = recommended[0].as_str();
        info!(
            "Downloading STT model for first launch: {} (tier: {:?})",
            model_id,
            device_profile.recommended_tier()
        );

        self.download_model(model_id, progress_callback).await
    }

    /// Starts background download of TTS model (non-blocking per D1).
    ///
    /// Unlike STT, TTS download does not block application startup.
    /// Returns immediately after spawning the download task.
    ///
    /// # Arguments
    ///
    /// * `device_profile` - Device capabilities for model selection
    /// * `progress_callback` - Called with download progress updates
    /// * `completion_callback` - Called when download completes (success or failure)
    #[instrument(skip(self, progress_callback, completion_callback))]
    pub fn start_tts_download<F, C>(
        &self,
        device_profile: &DeviceProfile,
        progress_callback: F,
        completion_callback: C,
    ) where
        F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
        C: FnOnce(Result<PathBuf>) + Send + 'static,
    {
        // Check if already ready
        if self.is_tts_ready() {
            if let Some(model) = self.best_tts_model(device_profile) {
                info!("TTS model already available: {}", model.name);
                if let Some(path) = model.local_path.clone() {
                    completion_callback(Ok(path));
                    return;
                }
            }
        }

        // Get recommended TTS models
        let recommended = self.recommend_tts_models(device_profile);
        if recommended.is_empty() {
            info!("No TTS models recommended for device - TTS will use platform fallback");
            completion_callback(Err(
                ModelError::ManifestError {
                    message: "No TTS models recommended for this device tier".to_string(),
                }
                .into(),
            ));
            return;
        }

        let model_id = recommended[0].to_string();
        let manifest = self.manifest.clone();
        let cache_dir = self.cache_dir.clone();
        let download_states = self.download_states.clone();
        let http_client = self.http_client.clone();

        info!(
            "Starting background TTS download: {} (tier: {:?})",
            model_id,
            device_profile.recommended_tier()
        );

        // Spawn background download task
        tokio::spawn(async move {
            let result = Self::download_model_static(
                &http_client,
                &manifest,
                &cache_dir,
                &download_states,
                &model_id,
                progress_callback,
            )
            .await;

            match &result {
                Ok(path) => info!("TTS model download complete: {}", path.display()),
                Err(e) => warn!("TTS model download failed: {} - will use platform TTS", e),
            }

            completion_callback(result);
        });
    }

    /// First launch initialization flow (D1 implementation).
    ///
    /// This method implements the complete first-launch flow:
    /// 1. Detects device capabilities
    /// 2. Fetches and verifies the model manifest
    /// 3. Downloads STT model (blocking - required for transcription)
    /// 4. Starts TTS model download (non-blocking - uses platform fallback until ready)
    ///
    /// # Arguments
    ///
    /// * `progress_callback` - Called with download progress updates
    /// * `tts_completion` - Called when TTS download completes
    ///
    /// # Returns
    ///
    /// The path to the downloaded STT model.
    ///
    /// # Errors
    ///
    /// Returns an error if manifest fetch fails or STT download fails.
    #[instrument(skip(self, progress_callback, tts_completion))]
    pub async fn first_launch_init<F, C>(
        &mut self,
        progress_callback: F,
        tts_completion: C,
    ) -> Result<PathBuf>
    where
        F: Fn(DownloadProgress) + Send + Sync + Clone + 'static,
        C: FnOnce(Result<PathBuf>) + Send + 'static,
    {
        let device_profile = DeviceProfile::detect();
        info!(
            "First launch init: platform={}, tier={:?}, memory={}MB",
            device_profile.platform,
            device_profile.recommended_tier(),
            device_profile.available_memory_mb
        );

        // Fetch manifest if not already loaded
        if self.manifest.is_none() {
            info!("Fetching model manifest...");
            self.fetch_manifest().await?;
        }

        // Download STT model (blocking - required before transcription is available)
        let stt_path = self
            .ensure_stt_ready(&device_profile, progress_callback.clone())
            .await?;

        // Start TTS download in background (non-blocking)
        self.start_tts_download(&device_profile, progress_callback, tts_completion);

        Ok(stt_path)
    }

    /// Checks if first launch setup is complete.
    ///
    /// Returns true if at least one STT model is ready for use.
    #[must_use]
    pub fn is_first_launch_complete(&self) -> bool {
        self.is_stt_ready()
    }

    /// Returns a summary of model readiness status.
    #[must_use]
    pub fn readiness_status(&self) -> ModelReadinessStatus {
        ModelReadinessStatus {
            stt_ready: self.is_stt_ready(),
            tts_ready: self.is_tts_ready(),
            stt_models_available: self
                .models
                .values()
                .filter(|m| m.model_type == ModelType::Stt && m.is_ready())
                .count(),
            tts_models_available: self
                .models
                .values()
                .filter(|m| m.model_type == ModelType::Tts && m.is_ready())
                .count(),
            total_models_in_manifest: self
                .manifest
                .as_ref()
                .map(|m| m.models.len())
                .unwrap_or(0),
        }
    }

    /// Static download method for use in spawned tasks.
    async fn download_model_static<F>(
        http_client: &reqwest::Client,
        manifest: &Option<ModelManifest>,
        cache_dir: &Path,
        download_states: &Arc<RwLock<HashMap<ModelId, DownloadState>>>,
        model_id: &str,
        progress_callback: F,
    ) -> Result<PathBuf>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let manifest = manifest.as_ref().ok_or_else(|| ModelError::ManifestError {
            message: "No manifest loaded".to_string(),
        })?;

        let entry = manifest.get_model(model_id).ok_or_else(|| ModelError::NotFound {
            path: PathBuf::from(model_id),
        })?;

        let filename = entry.download_path.rsplit('/').next().unwrap_or(&entry.id);
        let dest_path = cache_dir.join(filename);
        let model_id_owned = ModelId::new(model_id);

        // Check if already downloaded
        if dest_path.exists() {
            // Verify hash
            let file = std::fs::File::open(&dest_path).map_err(|_| ModelError::NotFound {
                path: dest_path.clone(),
            })?;
            let mut reader = std::io::BufReader::new(file);
            let mut hasher = Sha256::new();
            std::io::copy(&mut reader, &mut hasher).map_err(|_| ModelError::InvalidModel {
                path: dest_path.clone(),
            })?;
            let computed = format!("{:x}", hasher.finalize());

            if computed == entry.hash_sha256 {
                return Ok(dest_path);
            }
            std::fs::remove_file(&dest_path).ok();
        }

        // Update state
        {
            let mut states = download_states.write().await;
            states.insert(model_id_owned.clone(), DownloadState::Downloading);
        }

        // Download using client with browser-like headers
        let download_url = entry.download_url(&manifest.base_url);
        let response = http_client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("HTTP request failed: {e}"),
            })?;

        if !response.status().is_success() {
            let mut states = download_states.write().await;
            states.insert(model_id_owned, DownloadState::Failed);
            let status = response.status();
            return Err(ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("HTTP {status}"),
            }
            .into());
        }

        let total_size = response.content_length().unwrap_or(entry.size_bytes);
        let temp_path = dest_path.with_extension("tmp");

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let mut file =
            std::fs::File::create(&temp_path).map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Failed to create file: {e}"),
            })?;

        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Stream error: {e}"),
            })?;

            file.write_all(&chunk).map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Write error: {e}"),
            })?;

            hasher.update(&chunk);
            downloaded += chunk.len() as u64;

            progress_callback(DownloadProgress::new(
                model_id_owned.clone(),
                downloaded,
                total_size,
            ));
        }

        // Verify (skip if hash is "skip" for fallback manifest)
        if entry.hash_sha256 != "skip" {
            let computed_hash = format!("{:x}", hasher.finalize());
            if computed_hash != entry.hash_sha256 {
                std::fs::remove_file(&temp_path).ok();
                let mut states = download_states.write().await;
                states.insert(model_id_owned, DownloadState::Failed);
                return Err(ModelError::VerificationFailed {
                    expected: entry.hash_sha256.clone(),
                    actual: computed_hash,
                }
                .into());
            }
        } else {
            debug!("Skipping hash verification (fallback manifest)");
        }

        // Finalize
        std::fs::rename(&temp_path, &dest_path).map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("Failed to finalize: {e}"),
        })?;

        {
            let mut states = download_states.write().await;
            states.insert(model_id_owned, DownloadState::Ready);
        }

        Ok(dest_path)
    }

    // --- Private methods ---

    /// Verifies a model file's SHA-256 hash.
    ///
    /// If `expected_hash` is "skip", verification is bypassed (used for fallback downloads).
    fn verify_model_hash(&self, path: &Path, expected_hash: &str) -> Result<()> {
        // Skip verification if hash is "skip" (fallback manifest)
        if expected_hash == "skip" {
            debug!("Skipping hash verification for {} (fallback manifest)", path.display());
            return Ok(());
        }

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
                LexiconError::Config(format!("Failed to create model directory: {e}"))
            })?;
        }

        // Start download using client with browser-like headers
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("HTTP request failed: {e}"),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("HTTP {status}: {url}"),
            }
            .into());
        }

        let total_size = response.content_length().unwrap_or(expected_size);

        // Create temp file for download
        let temp_path = dest_path.with_extension("tmp");
        let mut file = std::fs::File::create(&temp_path).map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("Failed to create temp file: {e}"),
        })?;

        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Download stream error: {e}"),
            })?;

            // Write to file
            file.write_all(&chunk).map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Failed to write to file: {e}"),
            })?;

            // Update hash
            hasher.update(&chunk);

            // Update progress
            downloaded += chunk.len() as u64;
            let progress = DownloadProgress::new(model_id.clone(), downloaded, total_size);
            progress_callback(progress);
        }

        // Verify hash (skip if hash is "skip" for fallback manifest)
        if expected_hash != "skip" {
            let computed_hash = format!("{:x}", hasher.finalize());
            if computed_hash != expected_hash {
                std::fs::remove_file(&temp_path).ok();
                return Err(ModelError::VerificationFailed {
                    expected: expected_hash.to_string(),
                    actual: computed_hash,
                }
                .into());
            }
        } else {
            debug!("Skipping hash verification (fallback manifest)");
        }

        // Move temp file to final location
        std::fs::rename(&temp_path, dest_path).map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("Failed to finalize download: {e}"),
        })?;

        Ok(())
    }

    /// Downloads a file with streaming hash verification and cancellation support.
    ///
    /// Returns `Ok(true)` if download completed, `Ok(false)` if cancelled.
    async fn download_with_verification_cancellable<F>(
        &self,
        url: &str,
        dest_path: &Path,
        expected_size: u64,
        expected_hash: &str,
        model_id: ModelId,
        progress_callback: F,
    ) -> Result<bool>
    where
        F: Fn(DownloadProgress) -> bool + Send + Sync + 'static,
    {
        // Create parent directory if needed
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                LexiconError::Config(format!("Failed to create model directory: {e}"))
            })?;
        }

        // Start download using client with browser-like headers
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("HTTP request failed: {e}"),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("HTTP {status}: {url}"),
            }
            .into());
        }

        let total_size = response.content_length().unwrap_or(expected_size);

        // Create temp file for download
        let temp_path = dest_path.with_extension("tmp");
        let mut file = std::fs::File::create(&temp_path).map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("Failed to create temp file: {e}"),
        })?;

        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Download stream error: {e}"),
            })?;

            // Write to file
            file.write_all(&chunk).map_err(|e| ModelError::DownloadFailed {
                model_id: model_id.to_string(),
                message: format!("Failed to write to file: {e}"),
            })?;

            // Update hash
            hasher.update(&chunk);

            // Update progress and check for cancellation
            downloaded += chunk.len() as u64;
            let progress = DownloadProgress::new(model_id.clone(), downloaded, total_size);
            let should_continue = progress_callback(progress);

            if !should_continue {
                // Cancellation requested - clean up temp file
                drop(file); // Close file handle before removing
                std::fs::remove_file(&temp_path).ok();
                info!("Download cancelled for model {}", model_id);
                return Ok(false);
            }
        }

        // Verify hash (skip if hash is "skip" for fallback manifest)
        if expected_hash != "skip" {
            let computed_hash = format!("{:x}", hasher.finalize());
            if computed_hash != expected_hash {
                std::fs::remove_file(&temp_path).ok();
                return Err(ModelError::VerificationFailed {
                    expected: expected_hash.to_string(),
                    actual: computed_hash,
                }
                .into());
            }
        } else {
            debug!("Skipping hash verification (fallback manifest)");
        }

        // Move temp file to final location
        std::fs::rename(&temp_path, dest_path).map_err(|e| ModelError::DownloadFailed {
            model_id: model_id.to_string(),
            message: format!("Failed to finalize download: {e}"),
        })?;

        Ok(true)
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

    fn sample_manifest_with_tts() -> &'static str {
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
                },
                "kokoro-v1": {
                    "id": "kokoro-v1",
                    "name": "Kokoro TTS",
                    "model_type": "tts",
                    "size_bytes": 100000000,
                    "hash_sha256": "def789ghi012",
                    "download_path": "tts/kokoro-v1.bin",
                    "memory_mb": 200
                }
            },
            "tier_recommendations": {
                "mobile-low": ["whisper-tiny-q5_1"],
                "mobile-mid": ["whisper-tiny-q5_1", "kokoro-v1"],
                "desktop": ["whisper-tiny-q5_1", "kokoro-v1"],
                "server": ["whisper-tiny-q5_1", "kokoro-v1"]
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

    #[test]
    fn test_load_unsigned_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-manifest-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        // Load manifest
        registry.load_unsigned_manifest(sample_manifest_json()).unwrap();

        assert!(registry.manifest.is_some());
        assert_eq!(registry.list_models().len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_readiness_status_no_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-readiness-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        let status = registry.readiness_status();
        assert!(!status.stt_ready);
        assert!(!status.tts_ready);
        assert_eq!(status.stt_models_available, 0);
        assert_eq!(status.tts_models_available, 0);
        assert_eq!(status.total_models_in_manifest, 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_readiness_status_with_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-readiness-manifest-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        registry.load_unsigned_manifest(sample_manifest_with_tts()).unwrap();

        let status = registry.readiness_status();
        // Models are in manifest but not downloaded
        assert!(!status.stt_ready);
        assert!(!status.tts_ready);
        assert_eq!(status.total_models_in_manifest, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_model_available_no_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-avail-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        assert!(!registry.is_model_available("whisper-tiny-q5_1"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_model_available_not_downloaded() {
        let temp_dir = std::env::temp_dir().join("lexicon-avail-manifest-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        registry.load_unsigned_manifest(sample_manifest_json()).unwrap();

        // Model is in manifest but not downloaded
        assert!(!registry.is_model_available("whisper-tiny-q5_1"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_model_path_no_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-path-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        assert!(registry.get_model_path("whisper-tiny-q5_1").is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cache_dir_accessor() {
        let temp_dir = std::env::temp_dir().join("lexicon-cache-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        assert_eq!(registry.cache_dir(), &temp_dir);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_thermal_state_accessor() {
        let temp_dir = std::env::temp_dir().join("lexicon-thermal-state-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        assert_eq!(registry.thermal_state(), ThermalState::Nominal);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_thermal_state_transitions() {
        let temp_dir = std::env::temp_dir().join("lexicon-thermal-trans-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        // Nominal (headroom >= 0.7)
        let should_switch = registry.update_thermal_state(0.8);
        assert!(!should_switch);
        assert_eq!(registry.thermal_state(), ThermalState::Nominal);

        // Fair (0.5 <= headroom < 0.7)
        let should_switch = registry.update_thermal_state(0.6);
        assert!(!should_switch);
        assert_eq!(registry.thermal_state(), ThermalState::Fair);

        // Serious (0.3 <= headroom < 0.5) - should trigger throttle
        let should_switch = registry.update_thermal_state(0.35);
        assert!(should_switch);
        assert_eq!(registry.thermal_state(), ThermalState::Serious);

        // Critical (headroom < 0.3)
        let should_switch = registry.update_thermal_state(0.1);
        assert!(should_switch);
        assert_eq!(registry.thermal_state(), ThermalState::Critical);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_local_models_empty() {
        let temp_dir = std::env::temp_dir().join("lexicon-local-list-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        let local_models = registry.list_local_models();
        assert!(local_models.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_verify_model_without_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-verify-no-manifest-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        // Should return false without manifest
        assert!(!registry.verify_model("whisper-tiny-q5_1"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_verify_model_not_downloaded() {
        let temp_dir = std::env::temp_dir().join("lexicon-verify-no-download-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        registry.load_unsigned_manifest(sample_manifest_json()).unwrap();

        // Should return false when file doesn't exist
        assert!(!registry.verify_model("whisper-tiny-q5_1"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_models_no_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-recommend-no-manifest-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        let device_profile = DeviceProfile::detect();
        let recommendations = registry.recommend_models(&device_profile);

        assert!(recommendations.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_models_with_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-recommend-manifest-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        registry.load_unsigned_manifest(sample_manifest_json()).unwrap();

        let device_profile = DeviceProfile::detect();
        let recommendations = registry.recommend_models(&device_profile);

        // Should recommend the model for any tier
        assert!(!recommendations.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_first_launch_complete_no_models() {
        let temp_dir = std::env::temp_dir().join("lexicon-first-launch-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        assert!(!registry.is_first_launch_complete());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_registry_invalid_directory() {
        // Test with a path that cannot be created (on Unix, /proc is read-only)
        #[cfg(unix)]
        {
            let result = ModelRegistry::new(std::path::Path::new("/proc/lexicon-cannot-create"));
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_list_models_with_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-list-models-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        registry.load_unsigned_manifest(sample_manifest_with_tts()).unwrap();

        let models = registry.list_models();
        assert_eq!(models.len(), 2);

        // Verify model types
        let has_stt = models.iter().any(|m| m.model_type == ModelType::Stt);
        let has_tts = models.iter().any(|m| m.model_type == ModelType::Tts);
        assert!(has_stt);
        assert!(has_tts);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_stt_models() {
        let temp_dir = std::env::temp_dir().join("lexicon-recommend-stt-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        registry.load_unsigned_manifest(sample_manifest_with_tts()).unwrap();

        let device_profile = DeviceProfile::detect();
        let stt_models = registry.recommend_stt_models(&device_profile);

        // Should only include STT models
        for model_id in &stt_models {
            let manifest = registry.manifest.as_ref().unwrap();
            let entry = manifest.get_model(model_id.as_str()).unwrap();
            assert_eq!(entry.model_type, ModelType::Stt);
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_tts_models() {
        let temp_dir = std::env::temp_dir().join("lexicon-recommend-tts-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        registry.load_unsigned_manifest(sample_manifest_with_tts()).unwrap();

        let device_profile = DeviceProfile::detect();
        let tts_models = registry.recommend_tts_models(&device_profile);

        // Should only include TTS models
        for model_id in &tts_models {
            let manifest = registry.manifest.as_ref().unwrap();
            let entry = manifest.get_model(model_id.as_str()).unwrap();
            assert_eq!(entry.model_type, ModelType::Tts);
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_download_state_not_downloaded() {
        let temp_dir = std::env::temp_dir().join("lexicon-download-state-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        let state = registry.download_state("whisper-tiny-q5_1").await;
        assert_eq!(state, DownloadState::NotDownloaded);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_download_state_cancelled_display() {
        assert_eq!(format!("{}", DownloadState::Cancelled), "cancelled");
        assert_eq!(format!("{}", DownloadState::NotDownloaded), "not_downloaded");
        assert_eq!(format!("{}", DownloadState::Downloading), "downloading");
        assert_eq!(format!("{}", DownloadState::Ready), "ready");
        assert_eq!(format!("{}", DownloadState::Failed), "failed");
    }

    #[tokio::test]
    async fn test_download_model_cancellable_no_manifest() {
        let temp_dir = std::env::temp_dir().join("lexicon-cancel-no-manifest-test");
        let registry = ModelRegistry::new(&temp_dir).unwrap();

        // Should fail without manifest
        let result = registry
            .download_model_cancellable("whisper-tiny", |_| true)
            .await;
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_download_model_cancellable_model_not_found() {
        let temp_dir = std::env::temp_dir().join("lexicon-cancel-not-found-test");
        let mut registry = ModelRegistry::new(&temp_dir).unwrap();

        // Load manifest
        registry.load_unsigned_manifest(sample_manifest_json()).unwrap();

        // Should fail with non-existent model
        let result = registry
            .download_model_cancellable("non-existent-model", |_| true)
            .await;
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
