# Lexicon Local Server — System Architecture

> **Status**: Draft v1.0 | **Last Updated**: 2026-01-19
>
> This document is the authoritative architectural blueprint for Lexicon Local Server. All implementation decisions should reference this document.

---

## Executive Summary

Lexicon Local Server is a Rust-based speech processing library providing real-time STT (speech-to-text), translation, and TTS (text-to-speech) capabilities. The architecture follows a **hybrid tiered model**: an embedded Rust core library for mobile/desktop platforms with an optional HTTP server for web clients.

**Core Principles:**
- **Offline-first**: All core functionality works without network connectivity
- **Privacy-preserving**: All processing runs locally on-device
- **Cross-platform**: Single codebase targets iOS, Android, Windows, macOS, Linux
- **Adaptive**: Automatically adjusts to device capabilities and thermal state

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          LEXICON CORE LIBRARY                               │
│                              (Rust Crate)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Audio I/O  │  │     STT      │  │  Translation │  │     TTS      │    │
│  │    (cpal)    │─▶│ (whisper-rs) │─▶│ (whisper-rs) │─▶│ (Kokoro/ort) │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
│         ▲                                                      │            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────┴───────┐    │
│  │     VAD      │  │    Models    │  │   Storage    │  │    Error     │    │
│  │ (webrtc-vad) │  │  (verified)  │  │  (rusqlite)  │  │  (thiserror) │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
 ┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
 │   Mobile Apps   │      │  Desktop/Server │      │   Web Clients   │
 │ (Embedded Lib)  │      │  (Full Binary)  │      │  (via Server)   │
 └─────────────────┘      └─────────────────┘      └─────────────────┘
   • Static lib (iOS)       • Rust binary            • WebSocket/REST
   • Shared lib (Android)   • Axum HTTP server       • No local ML
   • UniFFI bindings        • Local web UI           • Requires server
   • tiny/base models       • base/medium models
```

### Deployment Tiers

| Tier | Platform | Integration | Model Sizes | Offline | Use Case |
|------|----------|-------------|-------------|---------|----------|
| **Tier 1: Mobile** | iOS, Android | Embedded library | tiny, base (q5_1) | Full | Personal transcription |
| **Tier 2: Desktop** | Windows, macOS, Linux | Binary + web UI | base, small | Full | Professional transcription |
| **Tier 3: Server** | Linux (primary) | HTTP/WebSocket server | medium, large-v3 | N/A | Multi-user, high quality |

### Component Boundaries

| Component | Responsibility | Interface |
|-----------|----------------|-----------|
| **Audio Pipeline** | Capture, resampling, VAD | `AudioSource` trait |
| **STT Engine** | Speech recognition, translation | `Transcriber` trait |
| **TTS Engine** | Text-to-speech synthesis | `Synthesizer` trait |
| **Storage Layer** | Sessions, segments, search | `Database` struct |
| **Model Manager** | Download, verify, load | `ModelRegistry` struct |
| **Server (optional)** | HTTP/WebSocket API | Axum routes |

---

## Latency Budget

**Target: <800ms end-to-end for conversational use**

| Stage | Budget | Implementation | Notes |
|-------|--------|----------------|-------|
| VAD | 50-100ms | webrtc-vad | Filter silence |
| STT (streaming) | 500-1500ms | whisper-rs (5s chunks) | Model-dependent |
| Translation | ~0ms | whisper-rs translate mode | Built into STT |
| TTS | 200-500ms | Kokoro ONNX via ort | Stream output |
| Audio buffering | 100-200ms | cpal + ringbuffer | Platform-dependent |
| **Total** | **850-2300ms** | | Varies by model |

### Latency Optimization Strategies

1. **VAD-driven chunking**: Process speech segments, not fixed intervals
2. **Streaming TTS**: Output audio chunks as generated (don't wait for complete synthesis)
3. **Speculative execution**: Begin processing before speech ends
4. **Model selection**: Use tiny model for <500ms; base for <800ms

### Latency by Model

| Model | Expected Latency | RTF (M1) | Use Case |
|-------|------------------|----------|----------|
| tiny.en (q5_1) | <500ms | 0.3x | Live conversation |
| base (q5_1) | <800ms | 0.4x | Quality transcription |
| small | 1-2s | 0.6x | Desktop batch |
| medium | 2-4s | 0.8x | Server/high quality |

---

## Technology Stack

### Core Dependencies

| Component | Crate | Version | License | Research Ref |
|-----------|-------|---------|---------|--------------|
| **STT Engine** | `whisper-rs` | 0.15.x | Unlicense | R-WRS-01 |
| **TTS Engine** | `ort` + Kokoro ONNX | 2.x | MIT/Apache-2.0 | R-TTS-01 |
| **Audio I/O** | `cpal` | 0.17.x | Apache-2.0 | R-AUD-01 |
| **ONNX Runtime** | `ort` | 2.x | MIT | R-ORT-01 |
| **Database** | `rusqlite` | 0.31.x | MIT | R-DB-01 |
| **Async Runtime** | `tokio` | 1.x | MIT | 004 |
| **Web Framework** | `axum` | 0.7.x | MIT | 006 |
| **Error Handling** | `thiserror` | 2.x | MIT | R-ERR-01 |
| **Logging** | `tracing` | 0.1.x | MIT | R-ERR-01 |
| **Hashing** | `sha2` | 0.10.x | MIT/Apache-2.0 | R-SEC-01 |
| **Signatures** | `minisign-verify` | 0.2.x | MIT | R-SEC-01 |
| **Resampling** | `rubato` | 0.16.x | MIT | R-AUD-01 |

### Platform-Specific Dependencies

| Platform | Audio Backend | ML Accelerator | FFI Bindings | Min Version |
|----------|---------------|----------------|--------------|-------------|
| **iOS** | CoreAudio | CoreML (ANE) | UniFFI → Swift | iOS 13+ |
| **Android** | AAudio | XNNPACK | UniFFI → Kotlin | API 26+ |
| **macOS** | CoreAudio | Metal | Native binary | 10.15+ |
| **Windows** | WASAPI | DirectML/CUDA | Native binary | Win 10+ |
| **Linux** | ALSA | CUDA/Vulkan | Native binary | Any |

### Technology Selection Rationale

**Why Rust?** (Research: 006)
- C ABI compatibility for seamless iOS/Android FFI
- No garbage collector — deterministic memory for real-time audio
- Smaller binary size vs Go (no runtime overhead)
- Memory safety without sacrificing performance

**Why whisper-rs?** (Research: R-WRS-01)
- RTF <0.3x achievable on Apple Silicon
- Quantization (q5_1) provides 45% size reduction, 19% latency improvement
- Native GPU acceleration (Metal, CUDA, Vulkan)
- Active maintenance (Codeberg)

**Why Kokoro TTS?** (Research: R-TTS-01, R-LIC-01)
- Apache 2.0 license — fully permissive for commercial use
- No GPL dependencies (Misaki G2P avoids espeak-ng)
- Top-ranked quality on HuggingFace TTS Arena
- ~350MB model suitable for mobile
- ONNX format integrates with existing ort infrastructure

**Why SQLite?** (Research: R-DB-01)
- FTS5 for full-text search across transcriptions
- Single-file database, zero configuration
- WAL mode for concurrent reads
- 35% faster than filesystem for small blobs (<100KB)

---

## Component Details

### Audio Pipeline

```
Microphone → [cpal] → [VAD] → [Resampler] → [Buffer] → [whisper-rs]
                │                               │
         (44.1/48kHz)                      (16kHz mono)
```

| Aspect | Specification |
|--------|---------------|
| Input rate | 44.1kHz or 48kHz (platform native) |
| Output rate | 16kHz mono (Whisper requirement) |
| Resampler | rubato crate (high-quality async) |
| VAD | webrtc-vad (speech/silence detection) |
| Buffer | Ring buffer for chunk accumulation |

**Platform Requirements:**

| Platform | Permission | Background Audio |
|----------|------------|------------------|
| iOS | `NSMicrophoneUsageDescription` | `UIBackgroundModes: audio` |
| Android | `RECORD_AUDIO`, `FOREGROUND_SERVICE_MICROPHONE` | Foreground service |

### STT Engine

| Aspect | Specification |
|--------|---------------|
| Engine | whisper-rs (wraps whisper.cpp) |
| Models | GGML format, quantized (q5_1) |
| Chunk size | 5-10 seconds (VAD-driven) |
| Translation | X→English via Whisper translate mode |

**Model Selection by Platform:**

| Platform | Recommended Model | Memory Budget |
|----------|-------------------|---------------|
| iOS | tiny.en q5_1 or base q5_1 | <1GB |
| Android (low-end) | tiny q5_1 | <500MB |
| Android (mid-range) | base q5_1 | <1GB |
| Desktop | base or small | <2GB |
| Server | medium or large-v3 | <10GB |

### TTS Engine

| Aspect | Specification |
|--------|---------------|
| Engine | Kokoro TTS (via ort crate) |
| Model | Kokoro-82M ONNX (~350MB) |
| G2P | Misaki (MIT, no espeak fallback) |
| Languages | English primary |
| Fallback | tts-rs (platform native) |

### Storage Layer

**Schema Overview:**

```
sessions
  └── segments (1:N, CASCADE delete)
        └── segments_fts (FTS5 virtual table)

models (downloaded model metadata)
settings (key-value store)
audio_files (optional blob/path storage)
```

**Key Design Decisions:**
- FTS5 with `unicode61` tokenizer for multilingual search
- Millisecond timestamp precision for subtitle export
- 100KB threshold for blob vs external file storage
- WAL mode for concurrent read performance

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum LexiconError {
    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    #[error("Model error: {0}")]
    Model(#[from] ModelError),

    #[error("Inference error: {0}")]
    Inference(#[from] InferenceError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

**FFI Error Handling:**
- UniFFI: `Result<T, E>` → Swift/Kotlin exceptions
- C ABI: `catch_unwind` + error codes
- Never panic across FFI boundaries

---

## Performance Constraints

### Memory Limits

| Platform | Hard Limit | Safe Working Set | Model Limit |
|----------|------------|------------------|-------------|
| iOS | ~2GB | <1GB | tiny/base q5_1 |
| Android (low) | ~2GB | <500MB | tiny q5_1 |
| Android (mid) | ~4GB | <1.5GB | base q5_1 |
| Desktop | 8GB+ | <4GB | small/medium |

### Thermal Management

| Device Tier | Throttling Onset | Mitigation |
|-------------|------------------|------------|
| Low-end | 10-15 min | Switch to tiny model |
| Mid-range | 15-25 min | Monitor thermal headroom |
| High-end | 25-40 min | Minimal intervention |

**Adaptive Strategy:**
1. Monitor platform thermal APIs
2. Switch base → tiny when thermal headroom <0.7
3. VAD provides natural cooling during silence
4. Warn users at "serious" thermal state

**Hardware Acceleration Benefits:**
- Apple ANE: 85% power reduction, 10x faster
- Android NPU: 35-70% power reduction vs GPU

---

## Security Architecture

### Model Verification

```
PRE-DOWNLOAD
  └─ Fetch signed manifest
  └─ Verify minisign signature
  └─ Check local cache

DOWNLOAD
  └─ Stream with SHA-256 hashing
  └─ Reject on size mismatch

POST-DOWNLOAD
  └─ Compare hash to manifest
  └─ Move to verified cache

ON LOAD
  └─ Re-verify hash
```

| Aspect | Choice |
|--------|--------|
| Hash algorithm | SHA-256 |
| Signature scheme | Minisign (Ed25519) |
| Key distribution | Hardcoded public key |
| Manifest format | Signed JSON |

### Threat Mitigations

| Threat | Mitigation |
|--------|------------|
| Model substitution | SHA-256 verification on download + load |
| MITM attacks | HTTPS + signature verification |
| Cache poisoning | Re-verify on every model load |
| Supply chain attacks | Signed manifests from trusted source |

---

## Cross-Platform Build

### Build Targets

| Platform | Target Triple | Binary Type |
|----------|---------------|-------------|
| iOS (device) | `aarch64-apple-ios` | Static lib |
| iOS (simulator) | `aarch64-apple-ios-sim` | Static lib |
| Android (arm64) | `aarch64-linux-android` | Shared lib |
| Android (arm32) | `armv7-linux-androideabi` | Shared lib |
| macOS (arm64) | `aarch64-apple-darwin` | Binary |
| Windows | `x86_64-pc-windows-msvc` | Binary |
| Linux | `x86_64-unknown-linux-gnu` | Binary |

### Tooling

| Tool | Purpose |
|------|---------|
| `cargo-mobile2` | iOS/Android project scaffolding |
| `cargo-ndk` | Android NDK cross-compilation |
| `cbindgen` | C header generation |
| `uniffi-rs` | Swift/Kotlin binding generation |

### Library Sizes

| Component | Size |
|-----------|------|
| Rust core | ~5-10MB |
| whisper.cpp | ~5-15MB |
| ONNX Runtime | ~15-30MB |
| **Total runtime** | **~25-55MB** |

Models (additional):
- tiny.en q5_1: ~15MB
- base q5_1: ~26MB
- Kokoro TTS: ~350MB

---

## API Surface

### Library API (Tier 1 - Mobile)

```rust
// Exposed via UniFFI
pub struct LexiconContext { ... }

impl LexiconContext {
    pub fn new(config: LexiconConfig) -> Result<Self, LexiconError>;
    pub fn load_model(&self, model_id: &str) -> Result<(), LexiconError>;
    pub fn start_session(&self) -> Result<TranscriptionSession, LexiconError>;
}

pub struct TranscriptionSession { ... }

impl TranscriptionSession {
    pub fn process_audio(&self, samples: &[f32]) -> Result<Vec<Segment>, LexiconError>;
    pub fn finish(&self) -> Result<(), LexiconError>;
}

pub struct Segment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f32>,
}
```

### Server API (Tier 2/3)

**WebSocket:**
- `ws://host/api/v1/stt/stream` — Real-time STT
- `ws://host/api/v1/tts/stream` — Streaming TTS

**REST:**
- `GET /api/v1/models` — List models
- `POST /api/v1/models/download` — Download model
- `POST /api/v1/transcribe` — Batch transcription
- `GET /api/v1/sessions/{id}/export?format=srt` — Export

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation | Research Ref |
|------|--------|------------|------------|--------------|
| ort mobile build complexity | MEDIUM | MEDIUM | tract pure-Rust fallback | R-ORT-01 |
| whisper-rs long-file stability | MEDIUM | LOW | Chunk files >30 min | R-WRS-01 |
| Android NNAPI deprecation | LOW | HIGH | Use XNNPACK | R-ORT-01 |
| Kokoro English-only | MEDIUM | LOW | tts-rs fallback | R-TTS-01 |
| Thermal throttling on mobile | MEDIUM | MEDIUM | Adaptive model switching | R-THR-01 |
| Model supply chain attacks | HIGH | LOW | Signed manifest verification | R-SEC-01 |

---

## Decisions

Architectural decisions resolved during design review:

| ID | Decision | Choice | Rationale | Impact |
|----|----------|--------|-----------|--------|
| D1 | **Model Download UX** | Immediate | R-UX-01 recommends upfront download for offline-first UX | First launch downloads all tier-appropriate models before use |
| D2 | **Bluetooth Audio** | HFP 1.6 supported | Enables 16kHz audio quality over Bluetooth headsets | Audio pipeline must handle route changes and codec negotiation |
| D3 | **Speaker Diarization** | Configurable | Schema ready with `speaker_id` column | Implementation phase to select algorithm (pyannote, whisper clustering, or sherpa-onnx) |
| D4 | **Multi-language TTS** | tts-rs fallback | Kokoro is English-only; tts-rs provides platform-native TTS | Fallback triggers on non-English text detection or user preference |

### Decision Details

**D1: Immediate Model Download**
- On first launch: profile device capabilities → determine tier → download all models for tier
- Progress UI required during download
- Block transcription features until at least one STT model ready
- Background download for TTS model (non-blocking)

**D2: Bluetooth HFP 1.6 Support**
- Negotiate HFP 1.6 codec for 16kHz sample rate (vs 8kHz in older profiles)
- Handle audio route changes gracefully (R-AUD-01 notes stuttering on Bluetooth toggle)
- Implement reconnection logic with stream rebuild
- Fallback to device microphone on Bluetooth disconnection

**D3: Configurable Speaker Diarization**
- Storage layer ready: `segments.speaker_id` column exists
- Algorithm selection deferred to implementation phase
- Options to evaluate: pyannote (accuracy), whisper clustering (integrated), sherpa-onnx (mobile-friendly)
- User-configurable: enable/disable per session

**D4: tts-rs Fallback for Multi-language**
- Primary: Kokoro TTS for English text
- Fallback triggers:
  - Non-English language detected in text
  - User explicitly selects platform voice
  - Kokoro model fails to load
- tts-rs uses platform-native synthesis (AVSpeechSynthesizer on iOS, Android TTS)

---

## References

### Research Documents
- [001-mobile-platform-constraints.md](research/001-mobile-platform-constraints.md) — iOS/Android feasibility
- [003-translation-tts-engines.md](research/003-translation-tts-engines.md) — TTS selection
- [004-realtime-performance.md](research/004-realtime-performance.md) — Latency analysis
- [005-cross-platform-distribution.md](research/005-cross-platform-distribution.md) — Build strategy
- [006-architecture-decisions.md](research/006-architecture-decisions.md) — Core decisions
- [R-WRS-01-whisper-rs-benchmarks.md](research/R-WRS-01-whisper-rs-benchmarks.md) — STT benchmarks
- [R-TTS-01-alternative-tts-engines.md](research/R-TTS-01-alternative-tts-engines.md) — TTS evaluation
- [R-ORT-01-ort-mobile-builds.md](research/R-ORT-01-ort-mobile-builds.md) — ONNX mobile
- [R-AUD-01-cpal-mobile-audio.md](research/R-AUD-01-cpal-mobile-audio.md) — Audio capture
- [R-ERR-01-error-handling-patterns.md](research/R-ERR-01-error-handling-patterns.md) — Error design
- [R-DB-01-sqlite-schema.md](research/R-DB-01-sqlite-schema.md) — Database schema
- [R-THR-01-thermal-profiling.md](research/R-THR-01-thermal-profiling.md) — Thermal management
- [R-SEC-01-model-verification.md](research/R-SEC-01-model-verification.md) — Security
- [R-LIC-01-license-audit.md](research/R-LIC-01-license-audit.md) — Licensing

### External References
- [whisper-rs (Codeberg)](https://codeberg.org/tazz4843/whisper-rs)
- [Kokoro TTS](https://github.com/hexgrad/kokoro)
- [ort crate](https://ort.pyke.io/)
- [cpal](https://github.com/RustAudio/cpal)
- [UniFFI](https://github.com/mozilla/uniffi-rs)

---

*This document serves as the source of truth for Lexicon Local Server architecture. Update as design evolves.*
