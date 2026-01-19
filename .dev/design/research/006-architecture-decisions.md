# Research Plan: Architecture Decisions

**ID:** 006
**Status:** Complete
**Impact:** HIGH (foundational)
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Adapted for:** Rust implementation

---

## Overview

This research addresses fundamental architecture questions resolved before detailed implementation planning. The decision to use Rust significantly simplifies many of these concerns.

---

## Key Decisions

### Decision: Rust Implementation

**Rationale:**
1. **Superior FFI:** Rust produces native C ABI libraries without runtime overhead
2. **No CGO complexity:** Unlike Go, Rust cross-compilation is straightforward
3. **Memory safety:** Compile-time guarantees without garbage collection
4. **Performance:** Near-C++ speed, critical for real-time audio processing
5. **Mobile support:** First-class iOS/Android cross-compilation via cargo

---

## Research Findings

### Deployment Model Analysis

| Model | Rust Feasibility | Memory Impact | Network Dependency | Complexity |
|-------|------------------|---------------|-------------------|------------|
| **Embedded (static lib)** | ✅ Excellent | High (model in app) | None | Low |
| **Local HTTP Server** | ✅ Excellent | Moderate | localhost only | Low |
| **Remote Server** | ✅ Excellent | Low (client-side) | Required | Low |
| **Hybrid** | ✅ Achievable | Variable | Optional | Medium |

**Rust advantages over Go for embedded:**
- No runtime to embed (no Go scheduler, no GC)
- Static libraries link cleanly into Swift/Kotlin
- Predictable memory usage
- UniFFI generates idiomatic bindings

---

### Recommended Architecture: Hybrid Tiered

```
                          ┌─────────────────────────────────────┐
                          │        Lexicon Core Library         │
                          │            (Rust crate)             │
                          └─────────────────────────────────────┘
                                           │
              ┌────────────────────────────┼────────────────────────────┐
              ▼                            ▼                            ▼
    ┌─────────────────┐          ┌─────────────────┐          ┌─────────────────┐
    │  Desktop/Server │          │   Mobile Apps   │          │   Web Clients   │
    │  (Full Binary)  │          │ (Embedded Lib)  │          │  (via Server)   │
    └─────────────────┘          └─────────────────┘          └─────────────────┘
    • Rust binary               • Static lib (iOS)            • WebSocket to server
    • All model sizes           • Shared lib (Android)        • No local processing
    • Full offline              • tiny/base models            • Requires server
    • Local web UI              • Offline capable
    • Axum/Actix server         • UniFFI bindings
```

**Tier Details:**

| Tier | Platform | Implementation | Models | Offline |
|------|----------|----------------|--------|---------|
| **Tier 1** | Mobile | Embedded Rust lib | tiny/base q5_1 | ✅ Full |
| **Tier 2** | Desktop | Rust binary + web UI | base/small | ✅ Full |
| **Tier 3** | Server | Axum HTTP/WS server | medium/large-v3 | N/A |

---

### Offline Requirements Analysis

| Component | Offline Feasible? | Rust Crate | Notes |
|-----------|-------------------|------------|-------|
| **STT** | ✅ Yes | whisper-rs | Full offline with local models |
| **Translation (X→EN)** | ✅ Yes | whisper-rs | Built into STT engine |
| **Translation (X→Y)** | ⚠️ Partial | reqwest + server | LibreTranslate server needed |
| **TTS** | ✅ Yes | ort (Piper models) | 30+ languages offline |
| **VAD** | ✅ Yes | webrtc-vad | Lightweight |

**Recommendation: Offline-First with Cloud Enhancement**

| Feature | Offline | Online Enhancement |
|---------|---------|-------------------|
| STT | tiny/base models | large-v3 via remote server |
| Translation | X→English via whisper-rs | X→Y via LibreTranslate/Cloud |
| TTS | Piper voices | Cloud voices (optional) |

---

### Multi-Language Model Strategy

| Model | File Size | Runtime Memory | Mobile | Desktop | Server |
|-------|-----------|----------------|--------|---------|--------|
| tiny.en (q5_1) | ~15MB | ~50MB | ✅ | ✅ | ✅ |
| tiny (multilingual) | ~15MB | ~50MB | ✅ | ✅ | ✅ |
| base.en (q5_1) | ~26MB | ~100MB | ✅ | ✅ | ✅ |
| base (multilingual) | ~26MB | ~100MB | ✅ | ✅ | ✅ |
| small | ~460MB | ~1GB+ | ⚠️ High-end | ✅ | ✅ |
| medium | ~1.5GB | ~2.6GB | ❌ | ✅ | ✅ |
| large-v3 | ~3GB | ~3.3GB+ | ❌ | ⚠️ 8GB+ | ✅ |

**Download Strategy:**
1. No models bundled (minimize app size)
2. Device profiling determines recommended model
3. On-demand download with progress UI
4. Store in platform cache directory
5. Version tracking in SQLite

---

### Web Interface Architecture

**Communication Protocol:**

| Protocol | Use Case | Rust Crate |
|----------|----------|------------|
| WebSocket | Real-time audio streaming | tokio-tungstenite |
| REST | Config, model management | axum |
| SSE | Progress updates | axum |

**Recommended Stack:**

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Web framework | Axum | Async, type-safe, ergonomic |
| WebSocket | tokio-tungstenite | Async WebSocket support |
| Serialization | serde + serde_json | Standard Rust |
| Static files | tower-http | Serve web UI |
| Templates | Askama or static SPA | Type-safe templates |

**Architecture:**
```
┌─────────────────────────────────────────────────────────────┐
│                    Web Interface (SPA)                       │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ Audio Input │→ │ WebSocket    │→ │ Results Display     │ │
│  │ (MediaRecorder)│ │ Client     │  │ (Transcription/TTS) │ │
│  └─────────────┘  └──────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            ↕ WSS
┌─────────────────────────────────────────────────────────────┐
│                    Rust Server (Axum)                        │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ WebSocket   │→ │ Audio Buffer │→ │ whisper-rs          │ │
│  │ Handler     │  │ + VAD        │  │ Processing          │ │
│  └─────────────┘  └──────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

**API Endpoints:**

WebSocket:
- `ws://host/api/v1/stt/stream` - Real-time STT
- `ws://host/api/v1/tts/stream` - Streaming TTS output

REST:
- `GET /api/v1/models` - List available models
- `POST /api/v1/models/download` - Trigger model download
- `GET /api/v1/languages` - Supported language list
- `POST /api/v1/transcribe` - Batch transcription

---

## Recommendations

### R1: Use Rust Throughout ⭐ (DECIDED)

Single language for all components:
- Core library (embedded on mobile)
- Desktop application
- Server application
- CLI tools

Benefits: Shared code, no FFI between components, unified tooling.

### R2: Hybrid Architecture

Match capabilities to platform constraints:
- **Mobile:** Embedded library, small models, full offline
- **Desktop:** Full binary, medium models, local web UI
- **Server:** Large models, multi-client, GPU acceleration

### R3: Axum for Server Component

```rust
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/v1/models", get(list_models))
        .route("/api/v1/stt/stream", get(stt_websocket))
        .nest_service("/", ServeDir::new("static"));

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### R4: Offline-First Design

- All core functionality works without network
- Cloud features are enhancements, not requirements
- User controls data leaving device

### R5: On-Demand Model Downloads

- Profile device on first launch
- Recommend appropriate model tier
- Background download with progress
- Graceful degradation if download fails

---

## Crate Dependencies (Core)

| Crate | Purpose | Version |
|-------|---------|---------|
| `whisper-rs` | STT engine | Latest |
| `ort` | ONNX Runtime (TTS) | Latest |
| `cpal` | Audio I/O | Latest |
| `tokio` | Async runtime | 1.x |
| `axum` | Web framework | 0.7.x |
| `serde` | Serialization | 1.x |
| `reqwest` | HTTP client | 0.11.x |
| `rusqlite` | Local storage | 0.31.x |

---

## References

- whisper-rs: https://github.com/tazz4843/whisper-rs
- ort: https://crates.io/crates/ort
- axum: https://github.com/tokio-rs/axum
- tokio: https://tokio.rs
- UniFFI: https://github.com/mozilla/uniffi-rs
- cpal: https://crates.io/crates/cpal
- cargo-mobile2: https://github.com/niceda/cargo-mobile2

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial Go-based research |
| 2026-01-19 | Adapted for Rust implementation; replaced Go/sherpa-onnx architecture with Rust-native approach |
