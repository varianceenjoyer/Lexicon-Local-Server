//! Model Hash Registry - SHA-256 hashes for fallback model verification
//!
//! This module provides known SHA-256 hashes for models downloaded from
//! Hugging Face as a fallback source. Hashes are compiled into the binary
//! and cannot be bypassed by configuration.
//!
//! # Security
//!
//! - Hashes are compile-time constants, not fetched at runtime
//! - Hash verification cannot be disabled
//! - On mismatch, partial files are deleted
//!
//! # Updating Hashes
//!
//! When model versions change upstream, update the hashes by:
//! 1. Download each model file
//! 2. Run `sha256sum <model_file>`
//! 3. Update the corresponding constant below
//! 4. Update the LAST_VERIFIED date
//!
//! See .dev/human.md for detailed instructions.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

/// Date when hashes were last verified against Hugging Face
pub const HASHES_LAST_VERIFIED: &str = "2026-01-25";

// =============================================================================
// Whisper Model Hashes (ggerganov/whisper.cpp on Hugging Face)
// =============================================================================

/// SHA-256 hash for ggml-tiny.bin (whisper-tiny multilingual)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin
pub const HASH_WHISPER_TINY: &str = "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21";

/// SHA-256 hash for ggml-tiny-q5_1.bin (whisper-tiny-q5_1 quantized)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_1.bin
pub const HASH_WHISPER_TINY_Q5_1: &str = "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7";

/// SHA-256 hash for ggml-base.bin (whisper-base multilingual)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
pub const HASH_WHISPER_BASE: &str = "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe";

/// SHA-256 hash for ggml-base-q5_1.bin (whisper-base-q5_1 quantized)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin
pub const HASH_WHISPER_BASE_Q5_1: &str = "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898";

/// SHA-256 hash for ggml-small.bin (whisper-small multilingual)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin
pub const HASH_WHISPER_SMALL: &str = "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b";

/// SHA-256 hash for ggml-small-q5_1.bin (whisper-small-q5_1 quantized)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin
pub const HASH_WHISPER_SMALL_Q5_1: &str = "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30";

/// SHA-256 hash for ggml-medium.bin (whisper-medium multilingual)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin
pub const HASH_WHISPER_MEDIUM: &str = "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208";

/// SHA-256 hash for ggml-large-v3.bin (whisper-large-v3)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin
pub const HASH_WHISPER_LARGE_V3: &str = "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2";

/// SHA-256 hash for ggml-large-v3-turbo.bin (whisper-large-v3-turbo)
/// Source: https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin
pub const HASH_WHISPER_LARGE_V3_TURBO: &str = "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";

// =============================================================================
// Kokoro TTS Model Hashes (hexgrad/Kokoro-82M on Hugging Face)
// =============================================================================

/// SHA-256 hash for kokoro-v0_19.onnx (Kokoro TTS v0.19)
/// Source: https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx
pub const HASH_KOKORO_TTS: &str = "ebef42457f7efee9b60b4f1d5aec7692f2925923948a0d7a2a49d2c9edf57e49";

// =============================================================================
// Hash Registry Implementation
// =============================================================================

/// Hash verification errors
#[derive(Debug, thiserror::Error)]
pub enum HashError {
    /// No hash is available for the requested model
    #[error("No hash available for model: {0}")]
    NoHashAvailable(String),

    /// Hash mismatch detected during verification
    #[error("Hash mismatch for {model_id}: expected {expected}, got {actual}")]
    HashMismatch {
        model_id: String,
        expected: String,
        actual: String,
    },

    /// Hash verification is pending human action
    #[error("Hash for model {0} requires human verification - see .dev/human.md")]
    PendingVerification(String),

    /// I/O error during file reading
    #[error("Failed to read file for hashing: {0}")]
    Io(#[from] std::io::Error),
}

/// Registry of known SHA-256 hashes for fallback model sources
pub struct ModelHashRegistry {
    hashes: HashMap<&'static str, &'static str>,
}

/// Global singleton instance
static REGISTRY: OnceLock<ModelHashRegistry> = OnceLock::new();

impl ModelHashRegistry {
    /// Create a new hash registry with all known model hashes
    pub fn new() -> Self {
        let mut hashes = HashMap::new();

        // Whisper models
        hashes.insert("whisper-tiny", HASH_WHISPER_TINY);
        hashes.insert("whisper-tiny-q5_1", HASH_WHISPER_TINY_Q5_1);
        hashes.insert("whisper-base", HASH_WHISPER_BASE);
        hashes.insert("whisper-base-q5_1", HASH_WHISPER_BASE_Q5_1);
        hashes.insert("whisper-small", HASH_WHISPER_SMALL);
        hashes.insert("whisper-small-q5_1", HASH_WHISPER_SMALL_Q5_1);
        hashes.insert("whisper-medium", HASH_WHISPER_MEDIUM);
        hashes.insert("whisper-large-v3", HASH_WHISPER_LARGE_V3);
        hashes.insert("whisper-large-v3-turbo", HASH_WHISPER_LARGE_V3_TURBO);

        // TTS models
        hashes.insert("kokoro-tts", HASH_KOKORO_TTS);

        Self { hashes }
    }

    /// Get the global singleton registry
    pub fn global() -> &'static Self {
        REGISTRY.get_or_init(Self::new)
    }

    /// Get the expected SHA-256 hash for a model ID
    ///
    /// Returns `None` if the model ID is not in the registry.
    pub fn get(&self, model_id: &str) -> Option<&'static str> {
        self.hashes.get(model_id).copied()
    }

    /// Check if a hash value is pending human verification
    pub fn is_pending(hash: &str) -> bool {
        hash == "PENDING_HUMAN_VERIFICATION" || hash == "skip"
    }

    /// Get the expected hash for a model, returning an error if pending
    ///
    /// Use this for strict verification that fails if hashes haven't been set.
    pub fn get_strict(&self, model_id: &str) -> Result<&'static str, HashError> {
        let hash = self
            .get(model_id)
            .ok_or_else(|| HashError::NoHashAvailable(model_id.to_string()))?;

        if Self::is_pending(hash) {
            return Err(HashError::PendingVerification(model_id.to_string()));
        }

        Ok(hash)
    }

    /// Verify a downloaded file matches the expected hash
    ///
    /// # Arguments
    ///
    /// * `model_id` - The model identifier to look up in the registry
    /// * `path` - Path to the downloaded file
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - File hash matches expected hash
    /// * `Err(HashMismatch)` - File hash doesn't match
    /// * `Err(NoHashAvailable)` - Model ID not in registry
    /// * `Err(PendingVerification)` - Hash hasn't been set yet
    /// * `Err(Io)` - Failed to read the file
    pub fn verify_file(&self, model_id: &str, path: &Path) -> Result<bool, HashError> {
        let expected = self.get_strict(model_id)?;
        let actual = Self::compute_hash(path)?;

        if actual == expected {
            Ok(true)
        } else {
            Err(HashError::HashMismatch {
                model_id: model_id.to_string(),
                expected: expected.to_string(),
                actual,
            })
        }
    }

    /// Compute SHA-256 hash of a file
    ///
    /// Returns the hash as a lowercase hexadecimal string.
    pub fn compute_hash(path: &Path) -> Result<String, HashError> {
        use std::io::Read;

        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Check if all required hashes have been verified
    ///
    /// Returns a list of model IDs that still need hash verification.
    pub fn pending_models(&self) -> Vec<&'static str> {
        self.hashes
            .iter()
            .filter(|(_, hash)| Self::is_pending(hash))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Check if the registry is ready for production use
    ///
    /// Returns `true` if all hashes have been verified.
    pub fn is_production_ready(&self) -> bool {
        self.pending_models().is_empty()
    }
}

impl Default for ModelHashRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_registry_creation() {
        let registry = ModelHashRegistry::new();
        assert!(registry.hashes.len() > 0);
    }

    #[test]
    fn test_registry_global_singleton() {
        let r1 = ModelHashRegistry::global();
        let r2 = ModelHashRegistry::global();
        assert!(std::ptr::eq(r1, r2));
    }

    #[test]
    fn test_get_known_model() {
        let registry = ModelHashRegistry::new();
        assert!(registry.get("whisper-tiny").is_some());
        assert!(registry.get("whisper-base").is_some());
        assert!(registry.get("kokoro-tts").is_some());
    }

    #[test]
    fn test_get_unknown_model() {
        let registry = ModelHashRegistry::new();
        assert!(registry.get("unknown-model").is_none());
    }

    #[test]
    fn test_is_pending() {
        assert!(ModelHashRegistry::is_pending("PENDING_HUMAN_VERIFICATION"));
        assert!(ModelHashRegistry::is_pending("skip"));
        assert!(!ModelHashRegistry::is_pending(
            "abc123def456789012345678901234567890123456789012345678901234"
        ));
    }

    #[test]
    fn test_get_strict_pending() {
        let registry = ModelHashRegistry::new();
        let result = registry.get_strict("whisper-tiny");

        // Should return error because hash is pending
        assert!(matches!(result, Err(HashError::PendingVerification(_))));
    }

    #[test]
    fn test_get_strict_unknown() {
        let registry = ModelHashRegistry::new();
        let result = registry.get_strict("unknown-model");

        assert!(matches!(result, Err(HashError::NoHashAvailable(_))));
    }

    #[test]
    fn test_compute_hash() {
        // Create a temp file with known content
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello world").unwrap();
        temp.flush().unwrap();

        let hash = ModelHashRegistry::compute_hash(temp.path()).unwrap();

        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_hash_nonexistent_file() {
        let result = ModelHashRegistry::compute_hash(Path::new("/nonexistent/file"));
        assert!(matches!(result, Err(HashError::Io(_))));
    }

    #[test]
    fn test_pending_models() {
        let registry = ModelHashRegistry::new();
        let pending = registry.pending_models();

        // All models should be pending since we used placeholder hashes
        assert!(!pending.is_empty());
        assert!(pending.contains(&"whisper-tiny"));
    }

    #[test]
    fn test_is_production_ready() {
        let registry = ModelHashRegistry::new();

        // Not production ready with placeholder hashes
        assert!(!registry.is_production_ready());
    }

    #[test]
    fn test_verify_file_pending_hash() {
        let registry = ModelHashRegistry::new();
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"test content").unwrap();
        temp.flush().unwrap();

        let result = registry.verify_file("whisper-tiny", temp.path());

        // Should fail because hash is pending verification
        assert!(matches!(result, Err(HashError::PendingVerification(_))));
    }

    #[test]
    fn test_verify_file_unknown_model() {
        let registry = ModelHashRegistry::new();
        let temp = NamedTempFile::new().unwrap();

        let result = registry.verify_file("unknown-model", temp.path());
        assert!(matches!(result, Err(HashError::NoHashAvailable(_))));
    }

    #[test]
    fn test_hash_error_display() {
        let err = HashError::NoHashAvailable("test-model".to_string());
        assert!(err.to_string().contains("test-model"));

        let err = HashError::HashMismatch {
            model_id: "test".to_string(),
            expected: "abc".to_string(),
            actual: "def".to_string(),
        };
        assert!(err.to_string().contains("abc"));
        assert!(err.to_string().contains("def"));

        let err = HashError::PendingVerification("test".to_string());
        assert!(err.to_string().contains("human verification"));
    }
}
