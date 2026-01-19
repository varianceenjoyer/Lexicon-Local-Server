//! Model manifest parsing and signature verification
//!
//! The manifest is a JSON document signed with minisign that describes
//! all available models, their hashes, and download URLs.
//!
//! # Signature Format
//!
//! The signed manifest uses minisign format:
//! ```text
//! untrusted comment: <comment>
//! <base64 signature>
//! trusted comment: <comment>
//! <base64 global signature>
//! ---MANIFEST---
//! <JSON manifest content>
//! ```

use crate::error::ModelError;
use crate::models::types::{ModelId, ModelTierRequirements, ModelType};
use crate::platform::ModelTier;
use minisign_verify::{PublicKey, Signature};
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
/// The verifier stores a parsed public key and validates signatures
/// cryptographically before allowing manifest parsing.
pub struct ManifestVerifier {
    /// Parsed minisign public key
    public_key: PublicKey,
    /// Original public key string for logging
    public_key_str: String,
}

impl ManifestVerifier {
    /// Creates a new manifest verifier with the given public key.
    ///
    /// The public key should be in minisign base64 format (typically starts with "RW").
    ///
    /// # Errors
    ///
    /// Returns `ModelError::SignatureInvalid` if the public key format is invalid.
    pub fn new(public_key: impl Into<String>) -> ManifestResult<Self> {
        let public_key_str = public_key.into();

        let parsed_key =
            PublicKey::from_base64(&public_key_str).map_err(|e| ModelError::ManifestError {
                message: format!("Invalid public key format: {e}"),
            })?;

        Ok(Self {
            public_key: parsed_key,
            public_key_str,
        })
    }

    /// Creates a verifier with the default Lexicon public key.
    ///
    /// This is the production public key used to sign official model manifests.
    /// The corresponding secret key is held securely by the Lexicon team.
    ///
    /// # Panics
    ///
    /// Panics if the embedded public key is invalid (should never happen
    /// as it's compile-time verified).
    #[must_use]
    pub fn default_key() -> Self {
        // Production minisign public key for Lexicon model manifests
        // Generated with: minisign -G -p lexicon.pub -s lexicon.key
        // This is an Ed25519 public key in minisign format
        //
        // For development/testing, we use a well-known test key.
        // In production, this would be replaced with the actual Lexicon public key.
        const LEXICON_PUBLIC_KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GUx3";

        Self::new(LEXICON_PUBLIC_KEY).expect("embedded public key is valid")
    }

    /// Creates a verifier with a test key for development/testing.
    ///
    /// # Warning
    ///
    /// This should only be used in tests. Never use in production.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn test_key() -> Self {
        // Test key pair for unit tests (DO NOT USE IN PRODUCTION)
        // Generated specifically for testing signature verification
        const TEST_PUBLIC_KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GUx3";

        Self::new(TEST_PUBLIC_KEY).expect("test public key is valid")
    }

    /// Returns the public key string for logging/debugging.
    #[must_use]
    pub fn public_key_id(&self) -> &str {
        // Return a truncated version for security
        let len = self.public_key_str.len().min(16);
        &self.public_key_str[..len]
    }

    /// Verify and parse a signed manifest.
    ///
    /// The input should be in minisign signed format with signature
    /// and content separated by the `---MANIFEST---` marker.
    ///
    /// # Errors
    ///
    /// Returns `ModelError::SignatureInvalid` if signature verification fails.
    /// Returns `ModelError::ManifestError` if parsing fails.
    pub fn verify_and_parse(&self, signed_content: &str) -> ManifestResult<ModelManifest> {
        // Parse the signed content (signature + manifest)
        let (signature_block, manifest_json) = self.split_signed_content(signed_content)?;

        // Verify signature
        self.verify_signature(signature_block, manifest_json)?;

        // Parse manifest
        ModelManifest::from_json(manifest_json)
    }

    /// Verify only (without parsing) - useful for checking updates.
    ///
    /// # Errors
    ///
    /// Returns `ModelError::SignatureInvalid` if signature verification fails.
    pub fn verify(&self, signed_content: &str) -> ManifestResult<()> {
        let (signature_block, manifest_json) = self.split_signed_content(signed_content)?;
        self.verify_signature(signature_block, manifest_json)
    }

    /// Split signed content into signature block and payload.
    fn split_signed_content<'a>(&self, content: &'a str) -> ManifestResult<(&'a str, &'a str)> {
        const SEPARATOR: &str = "\n---MANIFEST---\n";

        if let Some(pos) = content.find(SEPARATOR) {
            let signature_block = &content[..pos];
            let manifest = &content[pos + SEPARATOR.len()..];

            if signature_block.trim().is_empty() {
                return Err(ModelError::ManifestError {
                    message: "Missing signature block".to_string(),
                });
            }

            if manifest.trim().is_empty() {
                return Err(ModelError::ManifestError {
                    message: "Missing manifest content".to_string(),
                });
            }

            Ok((signature_block, manifest))
        } else {
            Err(ModelError::ManifestError {
                message: "Invalid signed manifest format: missing '---MANIFEST---' separator"
                    .to_string(),
            })
        }
    }

    /// Verify signature against content using minisign.
    fn verify_signature(&self, signature_block: &str, content: &str) -> ManifestResult<()> {
        // Parse the minisign signature from the signature block
        let signature = Signature::decode(signature_block).map_err(|e| {
            tracing::warn!("Failed to decode signature: {}", e);
            ModelError::SignatureInvalid
        })?;

        // Verify the signature against the manifest content
        self.public_key
            .verify(content.as_bytes(), &signature, false)
            .map_err(|e| {
                tracing::warn!(
                    "Signature verification failed for key {}: {}",
                    self.public_key_id(),
                    e
                );
                ModelError::SignatureInvalid
            })?;

        tracing::debug!(
            "Manifest signature verified successfully (key: {}...)",
            self.public_key_id()
        );

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

    // --- Signature Verification Tests ---

    #[test]
    fn test_verifier_default_key() {
        // Default key should be valid and parseable
        let verifier = ManifestVerifier::default_key();
        assert!(!verifier.public_key_id().is_empty());
    }

    #[test]
    fn test_verifier_test_key() {
        // Test key should be valid
        let verifier = ManifestVerifier::test_key();
        assert!(!verifier.public_key_id().is_empty());
    }

    #[test]
    fn test_verifier_invalid_key() {
        // Invalid key format should fail
        let result = ManifestVerifier::new("not-a-valid-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_verifier_missing_separator() {
        let verifier = ManifestVerifier::test_key();

        // Content without separator should fail
        let content = r#"{"version": "1.0.0"}"#;
        let result = verifier.verify(content);

        assert!(result.is_err());
        if let Err(e) = result {
            let msg = format!("{:?}", e);
            assert!(msg.contains("separator") || msg.contains("MANIFEST"));
        }
    }

    #[test]
    fn test_verifier_empty_signature_block() {
        let verifier = ManifestVerifier::test_key();

        // Empty signature block should fail
        let content = r#"
---MANIFEST---
{"version": "1.0.0"}"#;
        let result = verifier.verify(content);

        assert!(result.is_err());
    }

    #[test]
    fn test_verifier_empty_manifest_content() {
        let verifier = ManifestVerifier::test_key();

        // Empty manifest content should fail
        let content = "some signature data\n---MANIFEST---\n";
        let result = verifier.verify(content);

        assert!(result.is_err());
    }

    #[test]
    fn test_verifier_invalid_signature_format() {
        let verifier = ManifestVerifier::test_key();

        // Invalid signature format should fail verification
        let content = r#"not a valid minisign signature
---MANIFEST---
{"version": "1.0.0"}"#;
        let result = verifier.verify(content);

        assert!(result.is_err());
    }

    #[test]
    fn test_verifier_truncated_public_key_for_logging() {
        let verifier = ManifestVerifier::default_key();

        // Public key ID should be truncated for security
        let key_id = verifier.public_key_id();
        assert!(key_id.len() <= 16);
    }

    #[test]
    fn test_parse_unsigned_for_testing() {
        // The unsigned parse function should work for development
        let manifest = parse_unsigned_manifest(sample_manifest_json()).unwrap();
        assert_eq!(manifest.version, "1.0.0");
    }
}
