# Research Plan: Translation & TTS Engine Selection

**ID:** 003
**Status:** Complete
**Impact:** MEDIUM-HIGH
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Adapted for:** Rust implementation

---

## Overview

The project requires translation and text-to-speech capabilities. This research evaluates options across offline capability, quality, size, platform support, and Rust integration feasibility.

---

## Key Findings

### Translation Engine Analysis

**Research Date:** 2026-01-19

| Engine | Rust Integration | Platform Support | Offline | Notes |
|--------|------------------|------------------|---------|-------|
| **Whisper (translation)** | ✅ Native (whisper-rs) | All | Yes | X→English only, no separate engine needed |
| **LibreTranslate** | ✅ HTTP client (reqwest) | Server-only | Yes | Requires self-hosted Python server |
| **Argos Translate** | ⚠️ Subprocess only | Python required | Yes | Powers LibreTranslate, no direct Rust bindings |
| **NLLB (Meta)** | ⚠️ Via ONNX Runtime | Python/C++ | Yes | Would require ort crate integration |
| **Cloud APIs** | ✅ HTTP client (reqwest) | N/A | No | Google, DeepL, Azure - excellent quality |

**Key Finding:** No pure Rust offline translation library exists with broad language support. Options:
1. Use Whisper's built-in translation (limited to X→English)
2. Run a Python-based server (LibreTranslate/Argos) with Rust HTTP client
3. Integrate ONNX models via the `ort` crate
4. Cloud API fallback

### TTS Engine Analysis

**Research Date:** 2026-01-19
**Updated:** 2026-01-19 - **Kokoro TTS selected** to avoid Piper's espeak-ng GPL dependency. See [R-TTS-01](R-TTS-01-alternative-tts-engines.md).

| Engine | Rust Integration | Platform Support | Quality | Size | Notes |
|--------|------------------|------------------|---------|------|-------|
| **Kokoro TTS (via ONNX)** | ✅ ort crate, kokorox/kokoroxide | All platforms | Excellent | ~350MB | **SELECTED** - Apache 2.0, Misaki G2P avoids GPL |
| **Piper (via ONNX)** | ✅ ort crate | All platforms | Good-Excellent | 20-100MB/voice | ⚠️ espeak-ng GPL dependency - **Not selected** |
| **espeak-ng** | ✅ espeak-ng-sys crate | All platforms | Robotic | <10MB | ⚠️ **GPLv3** - Not compatible with commercial use |
| **Coqui TTS** | ❌ HTTP only | Server | Excellent | 100MB-1GB | Python server mode |
| **System TTS** | ⚠️ Platform-specific | Per-platform | Varies | N/A | macOS: objc bindings, Windows: windows crate |

### Rust Crates for Audio/ML

| Crate | Purpose | Maturity | Notes |
|-------|---------|----------|-------|
| `whisper-rs` | Whisper bindings | ✅ Stable | Wraps whisper.cpp |
| `ort` | ONNX Runtime | ✅ Stable | For Piper TTS, NLLB models |
| `cpal` | Cross-platform audio | ✅ Stable | Audio I/O |
| `hound` | WAV file I/O | ✅ Stable | Audio file handling |
| `reqwest` | HTTP client | ✅ Stable | For cloud APIs, LibreTranslate |
| `espeak-ng-sys` | eSpeak bindings | ⚠️ Basic | Low-quality fallback |

### Whisper Translation Mode

**Status:** ✅ Viable for X→English translation

- **Source languages:** All 99 languages Whisper supports
- **Target language:** English only (Whisper limitation)
- **Rust bindings:** whisper-rs exposes translation mode
- **Latency:** Same as transcription (no additional overhead)
- **Quality:** Good - competitive with dedicated translation for many language pairs

**Limitation:** Cannot translate TO non-English languages. For multilingual output, a separate translation engine is required.

---

## Recommendations

### R1: Use Kokoro TTS via ONNX Runtime ⭐ (Updated)

> **Decision:** Kokoro TTS was selected over Piper to avoid GPL licensing issues with espeak-ng. See [R-TTS-01](R-TTS-01-alternative-tts-engines.md) for full analysis.

The `ort` crate provides Rust bindings to ONNX Runtime:
- Load Kokoro ONNX models via kokorox or kokoroxide crates
- Excellent quality (top-ranked on HuggingFace TTS Arena)
- Cross-platform (including mobile via static linking)
- ~350MB model size
- **Apache 2.0 license** - fully permissive for commercial use
- Misaki G2P avoids espeak-ng GPL contamination

**Implementation approach:**
```rust
// Using kokorox or kokoroxide crate
// Load Kokoro ONNX model and run inference via ort
```

### R2: Use Whisper Translation for X→English

Already available through whisper-rs:
- No additional dependencies
- Good quality for most language pairs
- Set `translate: true` in transcription params

### R3: For Multilingual Translation Output

If translation to non-English languages is required:
- **Option A:** Self-hosted LibreTranslate server + reqwest HTTP client (offline capable)
- **Option B:** Cloud API fallback (Google Translate, DeepL) when online
- **Option C:** NLLB model via ort crate (requires model download, ~600MB-3GB)
- **Option D:** Defer multilingual output to future version

### R4: Architecture Recommendation

**Tiered approach:**
1. **Core (offline):** whisper-rs for STT + translation (X→English), Kokoro TTS via ort (Apache 2.0, no GPL)
2. **Extended (server):** LibreTranslate for multilingual translation
3. **Fallback (online):** Cloud APIs for highest quality when available

---

## References

- whisper-rs: https://github.com/tazz4843/whisper-rs
- ort (ONNX Runtime): https://crates.io/crates/ort
- Piper TTS: https://github.com/rhasspy/piper
- Piper ONNX models: https://github.com/rhasspy/piper/releases
- LibreTranslate: https://libretranslate.com
- reqwest: https://crates.io/crates/reqwest
- cpal: https://crates.io/crates/cpal
- espeak-ng-sys: https://crates.io/crates/espeak-ng-sys

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial Go-based research |
| 2026-01-19 | Adapted for Rust implementation; replaced Go-specific content with Rust crates and approaches |
