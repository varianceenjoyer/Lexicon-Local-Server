# WP-HASH-VERIFY: Model Hash Verification for Fallback Downloads

> **Status**: In Progress (hashes verified, integration pending) | **Priority**: HIGH | **Phase**: Beta
>
> **Related**: WP-MODEL (existing), R-SEC-01 (security patterns)
>
> **Created**: 2026-01-23 | **Last Updated**: 2026-01-25

---

## Overview

Re-implement SHA-256 hash verification for fallback model downloads from Hugging Face. Currently, fallback downloads bypass hash verification (hash set to "skip"), which doesn't meet security requirements for Beta release.

**Background**: The primary model download path verifies SHA-256 hashes from signed manifests. When primary sources are unavailable, the system falls back to Hugging Face URLs but skips hash verification. This work package closes that security gap.

---

## Scope

### Included
- SHA-256 hash values for all Hugging Face fallback URLs
- Verification during fallback download (streaming hash)
- Rejection and cleanup on hash mismatch
- Hash storage in model manifest or embedded constants
- Documentation of hash verification process

### Excluded
- Changes to primary download verification (already working)
- Manifest signing changes (out of scope)
- New model source integration

---

## Interface Contract

### Hash Registry

```rust
// crates/lexicon-core/src/models/hashes.rs

/// Known SHA-256 hashes for fallback model sources
pub struct ModelHashRegistry {
    hashes: HashMap<String, String>,
}

impl ModelHashRegistry {
    /// Get the singleton registry (populated at compile time or first load)
    pub fn global() -> &'static Self;

    /// Get SHA-256 hash for a model ID from fallback source
    pub fn get_fallback_hash(&self, model_id: &str) -> Option<&str>;

    /// Verify a downloaded file matches expected hash
    pub fn verify_file(&self, model_id: &str, path: &Path) -> Result<bool, HashError>;
}

/// Hash verification errors
#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("No hash available for model: {0}")]
    NoHashAvailable(String),

    #[error("Hash mismatch for {model_id}: expected {expected}, got {actual}")]
    HashMismatch {
        model_id: String,
        expected: String,
        actual: String,
    },

    #[error("Failed to read file for hashing: {0}")]
    IoError(#[from] std::io::Error),
}
```

### Updated Download Function

```rust
// crates/lexicon-core/src/models/downloader.rs

impl ModelDownloader {
    /// Download model with hash verification
    ///
    /// For primary sources: uses manifest hash
    /// For fallback sources: uses embedded hash registry
    pub async fn download_model_verified(
        &self,
        model_id: &str,
        source: ModelSource,
        progress: impl Fn(u64, u64),
    ) -> Result<PathBuf, ModelError>;

    /// Internal: verify hash during streaming download
    async fn download_with_streaming_hash(
        &self,
        url: &str,
        dest: &Path,
        expected_hash: &str,
        progress: impl Fn(u64, u64),
    ) -> Result<(), ModelError>;
}

/// Model download source type
pub enum ModelSource {
    /// Primary source with manifest-provided hash
    Primary { url: String, hash: String },
    /// Fallback source using embedded hash registry
    Fallback { url: String },
}
```

---

## Acceptance Criteria

### Hash Coverage
- [ ] SHA-256 hashes documented for all fallback model URLs:
  - `whisper-tiny.en` (ggml format)
  - `whisper-base.en` (ggml format)
  - `whisper-small.en` (ggml format)
  - `whisper-medium.en` (ggml format)
  - `whisper-large-v3` (ggml format)
  - Kokoro TTS ONNX model
- [ ] Hashes verified against official sources (Hugging Face checksums)

### Download Verification
- [ ] Fallback downloads compute SHA-256 during streaming
- [ ] Downloads rejected on hash mismatch
- [ ] Partial files deleted on verification failure
- [ ] Clear error message includes expected vs actual hash
- [ ] Retry possible after failure

### Security Properties
- [ ] No "skip" hash values in production code
- [ ] Hash values compiled into binary (not fetched at runtime)
- [ ] Hash verification cannot be bypassed by configuration

### Backward Compatibility
- [ ] Primary download path unchanged
- [ ] Existing tests continue to pass
- [ ] CLI behavior unchanged (just more secure)

---

## Dependencies

### Requires
- `sha2` crate (already in use)
- Access to official model hashes from Hugging Face

### Blocks
- Beta release exit criteria

---

## Files

### Modified Files
- `crates/lexicon-core/src/models/downloader.rs` - Add verification to fallback path
- `crates/lexicon-core/src/models/mod.rs` - Export hash types

### New Files
- `crates/lexicon-core/src/models/hashes.rs` - Hash registry

### Test Files
- `crates/lexicon-core/src/models/tests/hash_tests.rs` - Verification tests

---

## Implementation Notes

### Hash Acquisition Process

1. Download each model from Hugging Face
2. Compute SHA-256: `sha256sum <model_file>`
3. Verify against any published checksums
4. Add to hash registry in code

### Streaming Hash Computation

```rust
use sha2::{Sha256, Digest};

async fn download_with_streaming_hash(
    url: &str,
    dest: &Path,
    expected_hash: &str,
) -> Result<(), ModelError> {
    let response = reqwest::get(url).await?;
    let total_size = response.content_length().unwrap_or(0);

    let mut hasher = Sha256::new();
    let mut file = tokio::fs::File::create(dest).await?;
    let mut downloaded = 0u64;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        // progress callback
    }

    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != expected_hash {
        // Delete partial file
        tokio::fs::remove_file(dest).await.ok();
        return Err(ModelError::HashMismatch { ... });
    }

    Ok(())
}
```

### Known Model URLs

Models from Hugging Face ggerganov/whisper.cpp:
- `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin`
- `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin`
- `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin`
- `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin`
- `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin`

---

## Test Plan

### Unit Tests

| Test | Input | Expected Output |
|------|-------|-----------------|
| `hash_registry_has_all_models` | Query each model ID | Non-empty hash returned |
| `verify_correct_hash` | File with matching hash | `Ok(true)` |
| `verify_wrong_hash` | File with wrong content | `Err(HashMismatch)` |
| `verify_missing_hash` | Unknown model ID | `Err(NoHashAvailable)` |

### Integration Tests

| Test | Scenario | Pass Criteria |
|------|----------|---------------|
| `fallback_download_verified` | Download via fallback | File verified, usable |
| `fallback_reject_corrupt` | Mock corrupt response | File deleted, error returned |
| `primary_still_works` | Download via primary | Unchanged behavior |

### Security Tests

| Test | Attack Vector | Expected Behavior |
|------|---------------|-------------------|
| `reject_mitm_substitution` | Different file served | Hash mismatch, file deleted |
| `no_skip_in_production` | Grep for "skip" hash | No matches in release code |

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Hash changes upstream | HIGH | Pin to specific commits/versions |
| Hash registry out of date | MEDIUM | CI job to verify hashes monthly |
| Performance overhead | LOW | Streaming hash adds minimal latency |

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work package created from Beta exit criteria |
| 2026-01-23 | Scaffolding complete: `hashes.rs` with registry structure and tests |
| 2026-01-25 | **All hashes acquired and verified** - human intervention complete |
| 2026-01-25 | Work package updated with Phase 3 integration plan |

---

## Current Progress

### ✅ Phase 1: Hash Registry Implementation (Complete)

**Scaffolding Complete** - The `ModelHashRegistry` implementation exists in `crates/lexicon-core/src/models/hashes.rs` with:
- `ModelHashRegistry` struct with global singleton pattern
- `get()`, `get_strict()`, `verify_file()`, `compute_hash()` methods
- `HashError` enum with proper error types
- Comprehensive unit tests

### ✅ Phase 2: Hash Acquisition (Complete - 2026-01-25)

All SHA-256 hashes have been acquired and verified:

| Model | Hash Status |
|-------|-------------|
| whisper-tiny | ✅ Verified |
| whisper-tiny-q5_1 | ✅ Verified |
| whisper-base | ✅ Verified |
| whisper-base-q5_1 | ✅ Verified |
| whisper-small | ✅ Verified |
| whisper-small-q5_1 | ✅ Verified |
| whisper-medium | ✅ Verified |
| whisper-large-v3 | ✅ Verified |
| whisper-large-v3-turbo | ✅ Verified |
| kokoro-tts | ✅ Verified |

`HASHES_LAST_VERIFIED` updated to `2026-01-25`.

### 🔄 Phase 3: Download Integration (Pending)

**Remaining Work**:

1. **Integrate `ModelHashRegistry` into `registry.rs`**
   - Replace `"skip"` hash logic in fallback manifest with registry lookup
   - Modify `verify_model_hash()` to use `ModelHashRegistry::global().verify_file()`
   - Update `download_with_verification()` to consult hash registry for fallback downloads

2. **Update fallback manifest handling**
   - Remove `"skip"` hashes from `FALLBACK_MANIFEST_JSON`
   - Use `ModelHashRegistry::get()` to resolve hashes at runtime
   - Ensure hash verification cannot be bypassed

3. **Add integration tests**
   - Test fallback download with hash verification
   - Test rejection of corrupted/tampered files
   - Test error messages on hash mismatch

4. **CLI enhancements** (optional)
   - `lexicon models hash <model>` - compute hash of local model
   - `lexicon models hash-status` - show verification status for all models

---

## Implementation Plan

### Step 1: Refactor `verify_model_hash()` in `registry.rs`

```rust
fn verify_model_hash(&self, path: &Path, expected_hash: &str) -> Result<()> {
    // If hash is "skip", try to resolve from compiled hash registry
    let hash_to_verify = if expected_hash == "skip" {
        // Extract model_id from path or entry
        let model_id = self.model_id_from_path(path)?;
        ModelHashRegistry::global()
            .get(&model_id)
            .ok_or_else(|| HashError::NoHashAvailable(model_id.clone()))?
    } else {
        expected_hash
    };

    // Use registry's verify_file for consistent hashing
    ModelHashRegistry::global()
        .verify_file(&model_id, path)
        .map(|_| ())
}
```

### Step 2: Update fallback manifest to reference registry

Instead of embedding `"skip"` in the JSON, modify `load_fallback_manifest()` to inject real hashes:

```rust
fn load_fallback_manifest(&mut self) -> Result<()> {
    let mut manifest = ModelManifest::from_json(FALLBACK_MANIFEST_JSON)?;

    // Replace "skip" hashes with actual hashes from registry
    for (id, entry) in manifest.models.iter_mut() {
        if entry.hash_sha256 == "skip" {
            if let Some(hash) = ModelHashRegistry::global().get(id) {
                entry.hash_sha256 = hash.to_string();
            }
        }
    }

    self.manifest = Some(manifest);
    Ok(())
}
```

### Step 3: Remove "skip" bypass paths

Search for and eliminate all code paths that skip verification:
- `if expected_hash == "skip"` branches
- `if entry.hash_sha256 != "skip"` conditionals

### Step 4: Add security assertion

In release builds, assert no "skip" values exist:

```rust
#[cfg(not(debug_assertions))]
fn validate_no_skip_hashes(&self) {
    assert!(
        ModelHashRegistry::global().pending_models().is_empty(),
        "Production build contains unverified model hashes"
    );
}
```

---

## Acceptance Criteria Update

### Hash Coverage ✅
- [x] SHA-256 hashes documented for all fallback model URLs
- [x] Hashes verified against official sources

### Download Verification (Phase 3)
- [ ] Fallback downloads compute SHA-256 during streaming
- [ ] Downloads rejected on hash mismatch
- [ ] Partial files deleted on verification failure
- [ ] Clear error message includes expected vs actual hash
- [ ] Retry possible after failure

### Security Properties (Phase 3)
- [ ] No "skip" hash values in production code paths
- [ ] Hash values compiled into binary (already done)
- [ ] Hash verification cannot be bypassed by configuration
