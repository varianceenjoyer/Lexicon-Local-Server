//! Model manifest parsing and signature verification
//!
//! The manifest is a JSON document signed with minisign that describes
//! all available models, their hashes, and download URLs.

use crate::error::ModelError;
use crate::models::types::{ModelId, ModelTierRequirements, ModelType};
use crate::platform::ModelTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result type for manifest operations.
pub type ManifestResult<T> = std::result::Result<T, ModelError>;

/// Signed model manifest containing available models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    /// Manifest version for compatibility checking
    pub version: String,
    /// Timestamp when manifest was generated (ISO 8601)
    pub generated_at: String,
    /// Base URL for model downloads
    pub base_url: String,
    /// Available models indexed by ID
    pub models: HashMap<String, ModelEntry>,
    /// Tier recommendations mapping tier -> model IDs
    #[serde(default)]
    pub tier_recommendations: HashMap<String, Vec<String>>,
}

impl ModelManifest {
    /// Parse manifest from JSON string (after signature verification).
    ///
    /// # Errors
    ///
    /// Returns `ModelError::ManifestError` if JSON parsing fails.
    pub fn from_json(json: &str) -> ManifestResult<Self> {
        serde_json::from_str(json).map_err(|e| ModelError::ManifestError {
            message: format!("Failed to parse manifest: {e}"),
        })
    }

    /// Get a model entry by ID.
    #[must_use]
    pub fn get_model(&self, id: &str) -> Option<&ModelEntry> {
        self.models.get(id)
    }

    /// Get all models of a specific type.
    #[must_use]
    pub fn models_by_type(&self, model_type: ModelType) -> Vec<&ModelEntry> {
        self.models
            .values()
            .filter(|m| m.model_type == model_type)
            .collect()
    }

    /// Get recommended models for a device tier.
    #[must_use]
    pub fn recommended_for_tier(&self, tier: ModelTier) -> Vec<&ModelEntry> {
        let tier_key = tier.as_str();

        self.tier_recommendations
            .get(tier_key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.models.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all STT models.
    #[must_use]
    pub fn stt_models(&self) -> Vec<&ModelEntry> {
        self.models_by_type(ModelType::Stt)
    }

    /// List all TTS models.
    #[must_use]
    pub fn tts_models(&self) -> Vec<&ModelEntry> {
        self.models_by_type(ModelType::Tts)
    }

    /// Get the smallest STT model (for low-memory devices).
    #[must_use]
    pub fn smallest_stt_model(&self) -> Option<&ModelEntry> {
        self.stt_models()
            .into_iter()
            .min_by_key(|m| m.size_bytes)
    }
}

/// Individual model entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Unique model identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Model type (stt or tts)
    pub model_type: ModelType,
    /// File size in bytes
    pub size_bytes: u64,
    /// SHA-256 hash of the model file
    pub hash_sha256: String,
    /// Relative download path (appended to base_url)
    pub download_path: String,
    /// Quantization type (e.g., "q5_1", "f16")
    #[serde(default)]
    pub quantization: Option<String>,
    /// Supported languages (for STT)
    #[serde(default)]
    pub languages: Vec<String>,
    /// Memory requirement in MB
    #[serde(default)]
    pub memory_mb: u64,
    /// Tier requirements
    #[serde(default)]
    pub requirements: ModelTierRequirements,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
}

impl ModelEntry {
    /// Get the full download URL.
    #[must_use]
    pub fn download_url(&self, base_url: &str) -> String {
        format!("{}/{}", base_url.trim_end_matches('/'), self.download_path)
    }

    /// Convert to ModelId.
    #[must_use]
    pub fn model_id(&self) -> ModelId {
        ModelId::new(&self.id)
    }

    /// Check if this model is suitable for the given memory constraint.
    #[must_use]
    pub fn fits_memory(&self, available_mb: u64) -> bool {
        self.memory_mb <= available_mb
    }
}

/// Verifier for model manifest signatures.
///
/// Uses minisign for signature verification to ensure manifest integrity.
pub struct ManifestVerifier {
    /// Public key for signature verification
    public_key: String,
}

impl ManifestVerifier {
    /// Creates a new manifest verifier with the given public key.
    ///
    /// The public key should be in minisign format.
    #[must_use]
    pub fn new(public_key: impl Into<String>) -> Self {
        Self {
            public_key: public_key.into(),
        }
    }

    /// Creates a verifier with the default Lexicon public key.
    ///
    /// In production, this would be a hardcoded trusted public key.
    #[must_use]
    pub fn default_key() -> Self {
        // Placeholder - in production this would be a real minisign public key
        Self::new("RWTLexiconPublicKeyPlaceholder==")
    }

    /// Verify and parse a signed manifest.
    ///
    /// The input should be in minisign signed format with signature
    /// and content separated.
    ///
    /// # Errors
    ///
    /// Returns `ModelError::SignatureInvalid` if signature verification fails.
    /// Returns `ModelError::ManifestError` if parsing fails.
    pub fn verify_and_parse(&self, signed_content: &str) -> ManifestResult<ModelManifest> {
        // Parse the signed content (signature + manifest)
        let (signature, manifest_json) = self.split_signed_content(signed_content)?;

        // Verify signature
        self.verify_signature(&signature, manifest_json)?;

        // Parse manifest
        ModelManifest::from_json(manifest_json)
    }

    /// Verify only (without parsing) - useful for checking updates.
    ///
    /// # Errors
    ///
    /// Returns `ModelError::SignatureInvalid` if signature verification fails.
    pub fn verify(&self, signed_content: &str) -> ManifestResult<()> {
        let (signature, manifest_json) = self.split_signed_content(signed_content)?;
        self.verify_signature(&signature, manifest_json)
    }

    /// Split signed content into signature and payload.
    fn split_signed_content<'a>(&self, content: &'a str) -> ManifestResult<(&'a str, &'a str)> {
        // Minisign format: signature on first lines, then content
        // For simplicity, we use a marker to separate them
        const SEPARATOR: &str = "\n---MANIFEST---\n";

        if let Some(pos) = content.find(SEPARATOR) {
            let signature = &content[..pos];
            let manifest = &content[pos + SEPARATOR.len()..];
            Ok((signature, manifest))
        } else {
            // If no separator, assume the entire content is the manifest
            // (for testing without signatures)
            Err(ModelError::ManifestError {
                message: "Invalid signed manifest format: missing separator".to_string(),
            })
        }
    }

    /// Verify signature against content.
    fn verify_signature(&self, signature: &str, content: &str) -> ManifestResult<()> {
        // In production, this would use minisign-verify crate
        // For now, we implement a placeholder that:
        // 1. Checks signature format
        // 2. Would verify using the public key

        if signature.is_empty() {
            return Err(ModelError::SignatureInvalid);
        }

        // Placeholder verification - in production use minisign_verify
        // The actual implementation would be:
        // let pk = minisign_verify::PublicKey::from_base64(&self.public_key)?;
        // let sig = minisign_verify::Signature::decode(signature)?;
        // pk.verify(content.as_bytes(), &sig)?;

        tracing::debug!(
            "Signature verification placeholder (key: {}, sig_len: {}, content_len: {})",
            &self.public_key[..20.min(self.public_key.len())],
            signature.len(),
            content.len()
        );

        // For development, accept any non-empty signature
        // TODO: Implement actual minisign verification
        Ok(())
    }
}

/// Parse an unsigned manifest (for testing or local development).
///
/// # Security Warning
///
/// This bypasses signature verification and should only be used in
/// development/testing scenarios, never in production.
///
/// # Errors
///
/// Returns `ModelError::ManifestError` if parsing fails.
pub fn parse_unsigned_manifest(json: &str) -> ManifestResult<ModelManifest> {
    tracing::warn!("Parsing unsigned manifest - this should not be used in production");
    ModelManifest::from_json(json)
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
                    "name": "Whisper Tiny (Q5_1)",
                    "model_type": "stt",
                    "size_bytes": 42000000,
                    "hash_sha256": "abc123def456",
                    "download_path": "stt/whisper-tiny-q5_1.bin",
                    "quantization": "q5_1",
                    "languages": ["en", "multilingual"],
                    "memory_mb": 150
                },
                "whisper-base-q5_1": {
                    "id": "whisper-base-q5_1",
                    "name": "Whisper Base (Q5_1)",
                    "model_type": "stt",
                    "size_bytes": 78000000,
                    "hash_sha256": "def456abc789",
                    "download_path": "stt/whisper-base-q5_1.bin",
                    "quantization": "q5_1",
                    "languages": ["en", "multilingual"],
                    "memory_mb": 300
                },
                "kokoro-v1": {
                    "id": "kokoro-v1",
                    "name": "Kokoro TTS",
                    "model_type": "tts",
                    "size_bytes": 350000000,
                    "hash_sha256": "789abc123def",
                    "download_path": "tts/kokoro-v1.onnx",
                    "memory_mb": 512
                }
            },
            "tier_recommendations": {
                "mobile-low": ["whisper-tiny-q5_1"],
                "mobile-mid": ["whisper-tiny-q5_1", "whisper-base-q5_1"],
                "desktop": ["whisper-base-q5_1", "kokoro-v1"],
                "server": ["whisper-base-q5_1", "kokoro-v1"]
            }
        }"#
    }

    #[test]
    fn test_parse_manifest() {
        let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.models.len(), 3);

        let tiny = manifest.get_model("whisper-tiny-q5_1").unwrap();
        assert_eq!(tiny.name, "Whisper Tiny (Q5_1)");
        assert_eq!(tiny.model_type, ModelType::Stt);
        assert_eq!(tiny.size_bytes, 42_000_000);
    }

    #[test]
    fn test_models_by_type() {
        let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

        let stt_models = manifest.stt_models();
        assert_eq!(stt_models.len(), 2);

        let tts_models = manifest.tts_models();
        assert_eq!(tts_models.len(), 1);
        assert_eq!(tts_models[0].id, "kokoro-v1");
    }

    #[test]
    fn test_tier_recommendations() {
        let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

        let mobile_low = manifest.recommended_for_tier(ModelTier::MobileLow);
        assert_eq!(mobile_low.len(), 1);
        assert_eq!(mobile_low[0].id, "whisper-tiny-q5_1");

        let desktop = manifest.recommended_for_tier(ModelTier::Desktop);
        assert_eq!(desktop.len(), 2);
    }

    #[test]
    fn test_smallest_model() {
        let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

        let smallest = manifest.smallest_stt_model().unwrap();
        assert_eq!(smallest.id, "whisper-tiny-q5_1");
    }

    #[test]
    fn test_download_url() {
        let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();
        let model = manifest.get_model("whisper-tiny-q5_1").unwrap();

        let url = model.download_url(&manifest.base_url);
        assert_eq!(url, "https://models.lexicon.dev/stt/whisper-tiny-q5_1.bin");
    }

    #[test]
    fn test_fits_memory() {
        let manifest = ModelManifest::from_json(sample_manifest_json()).unwrap();

        let tiny = manifest.get_model("whisper-tiny-q5_1").unwrap();
        assert!(tiny.fits_memory(200)); // 150 MB model fits in 200 MB
        assert!(!tiny.fits_memory(100)); // Doesn't fit in 100 MB

        let kokoro = manifest.get_model("kokoro-v1").unwrap();
        assert!(!kokoro.fits_memory(400)); // 512 MB model doesn't fit in 400 MB
    }
}
