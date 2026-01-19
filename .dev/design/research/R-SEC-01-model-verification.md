# Research Plan: Model File Verification Patterns

**ID:** R-SEC-01
**Status:** Ongoing
**Category:** Security
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
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

### RQ1: What verification mechanisms exist for ML model files?

**Areas to investigate:**
- [ ] Checksum verification (SHA-256, SHA-512)
- [ ] Cryptographic signatures (GPG, Minisign, Signify)
- [ ] Certificate-based verification
- [ ] TOFU (Trust On First Use) patterns
- [ ] Content-addressable storage patterns

**Existing solutions to evaluate:**
- [ ] How does Hugging Face verify model downloads?
- [ ] How does GGML/whisper.cpp handle model integrity?
- [ ] How does ONNX Hub verify models?
- [ ] How do package managers (cargo, npm) verify downloads?

### RQ2: What is the threat model for local ML deployments?

**Areas to investigate:**
- [ ] Supply chain attack vectors for ML models
- [ ] Download tampering scenarios
- [ ] Cache poisoning attacks
- [ ] Offline verification requirements

### RQ3: How should model verification integrate with the download flow?

**Areas to investigate:**
- [ ] Pre-download hash verification (manifest-based)
- [ ] Post-download verification
- [ ] Incremental/resume-safe verification
- [ ] Failed verification recovery UX

### RQ4: How do we bootstrap trust for the verification system?

**Areas to investigate:**
- [ ] Hardcoded public keys/root hashes
- [ ] Certificate pinning
- [ ] First-run trust establishment
- [ ] Key rotation strategies

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

## Agent Tasks

### Task 1: Survey existing ML model verification approaches
- Research Hugging Face Hub security model
- Research GGML model distribution patterns
- Document how whisper.cpp examples handle models
- Check if model files include embedded hashes

### Task 2: Evaluate Rust cryptographic crates
- [ ] `sha2` crate for hashing
- [ ] `ed25519-dalek` for signatures
- [ ] `ring` for cryptographic primitives
- [ ] `minisign` crate compatibility

### Task 3: Design verification architecture
- Define manifest format (JSON schema)
- Define signature scheme
- Define key distribution method
- Document verification flow

### Task 4: Prototype implementation
- Implement hash verification
- Implement signature verification
- Test with actual model files
- Measure performance impact

---

## Success Criteria

- [ ] Clear recommendation for verification approach
- [ ] Documented threat model
- [ ] Rust crate selection with rationale
- [ ] Manifest schema definition
- [ ] Key management strategy
- [ ] Integration guide for download flow

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
