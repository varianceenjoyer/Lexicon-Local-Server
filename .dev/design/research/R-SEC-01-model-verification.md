# Research Plan: Model File Verification Patterns

**ID:** R-SEC-01
**Status:** Complete
**Category:** Security
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Researched By:** BronzeCliff (Researcher Agent)
**Output:** Best practices documentation

---

## Problem Statement

The project downloads ML models (Whisper GGML, Piper ONNX) from external sources at runtime. Without verification, users are vulnerable to:
- Tampered model files (supply chain attacks)
- Model substitution attacks
- Corrupted downloads
- Man-in-the-middle attacks

**Engineering Challenge:** Design a model verification system that balances security, usability, and offline-first requirements.

---

## Research Questions

### RQ1: What verification mechanisms exist for ML model files? ✅ COMPLETE

**Findings:**

| Mechanism | Description | Verified |
|-----------|-------------|----------|
| SHA-256 checksums | Hash comparison for integrity | ✅ |
| Minisign (Ed25519) | Simple, modern file signatures | ✅ |
| Sigstore/cosign | Industry-standard supply chain signing | ✅ |
| TOFU | Trust on first use | ✅ |
| Manifest files | Signed list of hashes | ✅ |

**Existing solutions evaluated:**

| Platform | Verification Method | Finding |
|----------|---------------------|---------|
| **Hugging Face** | SHA-256 via ETag/git-lfs | Files identified by git-sha1 (git) or SHA-256 (git-lfs). Checksum visible in file details. Verification available but disabled by default. |
| **whisper.cpp** | **NONE** | `download-ggml-model.sh` has NO hash verification. Only checks download success, not integrity. **Security gap.** |
| **ONNX** | `onnx.checker.check_model()` | Structural validation only. Recommends sigstore/cosign for integrity. Known path traversal CVE in v1.4-1.15. |
| **Cargo** | SHA-256 in registry index | Verifies checksum on download. Fails if mismatch. Proposal to add checksums to Cargo.lock. |

### RQ2: What is the threat model for local ML deployments? ✅ COMPLETE

**OWASP ML Security Top 10 - ML06: Supply Chain Attacks**

| Threat | Vector | Severity | Mitigation |
|--------|--------|----------|------------|
| **Data poisoning** | Malicious training data | HIGH | Use verified model sources |
| **Model substitution** | Replace model file | HIGH | Hash verification on load |
| **Backdoor injection** | Triggers in pre-trained models | HIGH | Verify provenance + signatures |
| **MITM download** | Intercept model download | MEDIUM | HTTPS + hash verification |
| **Cache poisoning** | Corrupt local cache | MEDIUM | Verify on every load |
| **Namespace reuse** | Reclaim abandoned model names | MEDIUM | Pin to specific versions + hashes |
| **Pickle deserialization** | RCE via pickle files | CRITICAL | Use GGML/ONNX (safer than pickle) |

**Key insight:** "ML models are not pure functions. They are full-blown malware vectors." Models can execute arbitrary code on load (especially pickle format). GGML and ONNX are safer but still require integrity verification.

### RQ3: How should model verification integrate with the download flow? ✅ COMPLETE

**Recommended flow:**

```
1. PRE-DOWNLOAD
   └─ Fetch manifest (signed JSON with model hashes)
   └─ Verify manifest signature
   └─ Check local cache for matching hash

2. DOWNLOAD
   └─ Stream download with progress
   └─ Compute hash during download (streaming)
   └─ Reject early if size mismatch

3. POST-DOWNLOAD
   └─ Compare computed hash to manifest
   └─ If mismatch: delete file, report error, offer retry
   └─ If match: move to verified cache

4. ON LOAD
   └─ Re-verify hash before loading model
   └─ Prevents cache tampering between download and use
```

### RQ4: How do we bootstrap trust for the verification system? ✅ COMPLETE

**Recommended: Hardcoded public key + signed manifest**

| Approach | Pros | Cons | Recommendation |
|----------|------|------|----------------|
| **Hardcoded public key** | Simple, works offline | Key rotation requires app update | ✅ PRIMARY |
| **Certificate pinning** | Standard TLS approach | Complex, platform-specific | ❌ Overkill |
| **TOFU** | No initial trust needed | First download vulnerable | ❌ Insufficient |
| **Key in DNS/HTTPS** | Updatable without app release | Requires network | ⚠️ FALLBACK |

**Key rotation strategy:**
1. Embed current public key + next public key in app
2. Manifest includes key validity period
3. Warn users to update app before key expiry
4. Support multi-signature during transition

---

## Known Solutions & Prior Art

| Solution | Approach | Pros | Cons |
|----------|----------|------|------|
| SHA-256 checksums | Hash comparison | Simple, fast | No authenticity proof |
| GPG signatures | Public key cryptography | Strong authenticity | Complex key management |
| Minisign | Ed25519 signatures | Simple, modern | Less ecosystem support |
| TOFU | Trust on first use | No initial trust needed | Vulnerable to first download |
| Manifest files | Signed list of hashes | Single signature covers all | Manifest distribution challenge |

---

## Technical Constraints

1. **Offline-first:** Verification must work without network after initial download
2. **Cross-platform:** Must work on iOS, Android, Windows, macOS, Linux
3. **No runtime dependencies:** Rust-native solution preferred
4. **Update-friendly:** Must support model updates/new versions
5. **Graceful degradation:** Clear failure modes when verification fails

---

## Rust Crate Evaluation ✅ COMPLETE

| Crate | Purpose | Recommendation | Notes |
|-------|---------|----------------|-------|
| **sha2** | SHA-256 hashing | ✅ USE | Pure Rust, no dependencies, streaming support |
| **minisign-verify** | Ed25519 signature verification | ✅ USE | Zero dependencies, simple API, verification-only |
| **minisign** | Full Ed25519 signing + verification | ⚠️ Optional | Use if project needs to sign models |
| **sigstore** | Industry-standard signing | ⚠️ Complex | Good for CI/CD, experimental Rust crate |
| **ring** | Crypto primitives | ❌ Skip | Overkill for this use case |
| **ed25519-dalek** | Ed25519 signatures | ❌ Skip | Use minisign-verify instead (simpler) |

**Recommended stack:**
```toml
[dependencies]
sha2 = "0.10"           # SHA-256 hashing
minisign-verify = "0.2" # Ed25519 signature verification
serde = "1.0"           # Manifest JSON parsing
serde_json = "1.0"      # Manifest JSON parsing
```

---

## Agent Tasks ✅ COMPLETE

### Task 1: Survey existing ML model verification approaches ✅
- [x] Research Hugging Face Hub security model — SHA-256 via ETag/git-lfs
- [x] Research GGML model distribution patterns — **No verification in whisper.cpp**
- [x] Document how whisper.cpp examples handle models — Downloads without integrity check
- [x] Check if model files include embedded hashes — No, hashes must be external

### Task 2: Evaluate Rust cryptographic crates ✅
- [x] `sha2` crate for hashing — Recommended
- [x] `ed25519-dalek` for signatures — Skip (use minisign-verify)
- [x] `ring` for cryptographic primitives — Skip (overkill)
- [x] `minisign` crate compatibility — minisign-verify recommended

### Task 3: Design verification architecture ✅
- [x] Define manifest format (JSON schema) — See below
- [x] Define signature scheme — Minisign (Ed25519)
- [x] Define key distribution method — Hardcoded public key
- [x] Document verification flow — See RQ3

### Task 4: Prototype implementation 🔄 (Deferred to implementation phase)
- [ ] Implement hash verification
- [ ] Implement signature verification
- [ ] Test with actual model files
- [ ] Measure performance impact

---

## Recommended Manifest Schema

```json
{
  "$schema": "https://lexicon.app/schemas/models/v1.json",
  "version": "1.0.0",
  "generated": "2026-01-19T10:00:00Z",
  "models": {
    "whisper-base-q5_1": {
      "filename": "ggml-base-q5_1.bin",
      "size_bytes": 27328512,
      "sha256": "abc123...",
      "url": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
      "mirrors": [
        "https://mirror1.example.com/ggml-base-q5_1.bin"
      ],
      "type": "stt",
      "languages": ["multilingual"],
      "quality_tier": "standard"
    },
    "piper-amy-medium": {
      "filename": "en_US-amy-medium.onnx",
      "size_bytes": 45678901,
      "sha256": "def456...",
      "url": "https://huggingface.co/rhasspy/piper-voices/resolve/main/en_US-amy-medium.onnx",
      "type": "tts",
      "languages": ["en-US"],
      "license": "CC-BY-4.0"
    }
  }
}
```

**Manifest distribution:**
- Hosted at `https://lexicon.app/models/manifest.json`
- Signed with Minisign: `manifest.json.minisig`
- Public key embedded in app binary

---

## Verification Implementation Guide

### Rust Code Pattern

```rust
use sha2::{Sha256, Digest};
use minisign_verify::{PublicKey, Signature};
use std::io::Read;

// Embedded public key (compile-time)
const MODEL_SIGNING_KEY: &str = "RWQf6LRCGA9i53v1QWeSLW...";

pub fn verify_model(path: &Path, expected_hash: &str) -> Result<(), VerifyError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();

    // Stream hash (memory efficient for large models)
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 { break; }
        hasher.update(&buffer[..bytes_read]);
    }

    let computed = format!("{:x}", hasher.finalize());
    if computed != expected_hash {
        return Err(VerifyError::HashMismatch { expected: expected_hash.into(), actual: computed });
    }

    Ok(())
}

pub fn verify_manifest(manifest: &[u8], signature: &[u8]) -> Result<(), VerifyError> {
    let pk = PublicKey::from_base64(MODEL_SIGNING_KEY)?;
    let sig = Signature::decode(signature)?;
    pk.verify(manifest, &sig, false)?;
    Ok(())
}
```

---

## Summary & Recommendations

### Key Findings

1. **whisper.cpp has no verification** — This is a security gap we must address
2. **Hugging Face uses SHA-256** — Good pattern to follow, but no signatures
3. **Sigstore is industry standard** — Complex but comprehensive; use for CI/CD
4. **Minisign is ideal for embedded** — Simple, zero-dependency Rust crate

### Recommended Approach

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Hash algorithm** | SHA-256 | Industry standard, fast, secure |
| **Signature scheme** | Minisign (Ed25519) | Simple, modern, zero-dependency Rust crate |
| **Manifest format** | Signed JSON | Human-readable, easy to update |
| **Key distribution** | Hardcoded in binary | Works offline, simple |
| **Verification timing** | On download + on load | Defense in depth |

### Action Items for Implementation

1. **Create model manifest** with SHA-256 hashes for all supported models
2. **Generate Minisign keypair** for signing manifests
3. **Implement verification module** using sha2 + minisign-verify
4. **Add to download flow** — verify before caching
5. **Add to load flow** — re-verify before model instantiation

### Risk Assessment

| Risk | Severity | Status |
|------|----------|--------|
| No verification (current state) | HIGH | ⚠️ OPEN - Must implement |
| Hash-only verification | MEDIUM | Integrity but not authenticity |
| Hash + signature verification | LOW | Recommended approach |

---

## Success Criteria

- [x] Clear recommendation for verification approach — Minisign + SHA-256
- [x] Documented threat model — OWASP ML06 supply chain attacks
- [x] Rust crate selection with rationale — sha2, minisign-verify
- [x] Manifest schema definition — See JSON schema above
- [x] Key management strategy — Hardcoded public key with rotation plan
- [x] Integration guide for download flow — See implementation guide

---

## Dependencies

- R-UX-01 (model download UX patterns may inform verification UX)

---

## References

- Hugging Face Hub Security: https://huggingface.co/docs/hub/security
- whisper.cpp models: https://github.com/ggerganov/whisper.cpp/tree/master/models
- Minisign: https://jedisct1.github.io/minisign/
- SLSA Supply Chain Security: https://slsa.dev/
- sigstore: https://www.sigstore.dev/

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Research complete: Surveyed HF/GGML/ONNX verification, evaluated Rust crates, documented threat model, designed verification architecture |
