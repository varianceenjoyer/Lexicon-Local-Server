# Lexicon Local Server — Design Plan

> **Status**: Draft v1.0 | **Last Updated**: 2026-01-19
>
> This document translates the system architecture into actionable work packages. All implementation work should reference this plan.

---

## Executive Summary

This plan decomposes Lexicon Local Server into **11 work packages** organized in **4 layers**. The layered structure enables parallel development within each layer while maintaining clear dependencies between layers.

**Delivery Strategy:**
- **MVP**: Desktop STT transcription with SQLite persistence
- **Beta**: Add TTS, mobile FFI bindings, streaming APIs
- **Release**: Full platform coverage, production hardening

**Timeline Independence:** Work packages are defined by deliverables and dependencies, not time estimates. Teams should track completion criteria, not calendars.

---

## Work Package Definitions

### Layer 1: Foundation (Parallel Development)

These packages have no internal dependencies and can be developed concurrently by separate developers.

---

#### WP-CORE: Core Library Scaffolding

**Deliverable:** Cargo workspace with shared types, configuration, and error handling

**Scope:**
- Cargo workspace structure (`lexicon-core`, `lexicon-server`, `lexicon-cli`)
- `LexiconConfig` struct with platform detection
- `LexiconError` hierarchy (thiserror)
- Logging infrastructure (tracing)
- Feature flags for optional components

**Interface Contract:**
```rust
// lexicon-core/src/lib.rs
pub struct LexiconConfig {
    pub data_dir: PathBuf,
    pub model_dir: PathBuf,
    pub log_level: tracing::Level,
}

pub struct LexiconContext {
    config: LexiconConfig,
    // Components added as layers complete
}

impl LexiconContext {
    pub fn new(config: LexiconConfig) -> Result<Self, LexiconError>;
}
```

**Completion Criteria:**
- [ ] Workspace compiles with `cargo build --workspace`
- [ ] Error types documented with examples
- [ ] Config loads from file and environment
- [ ] Tracing outputs structured logs

**Architecture Refs:** Error Handling (lines 237-258)

---

#### WP-AUDIO: Audio Pipeline

**Deliverable:** Audio capture, VAD, and resampling module

**Scope:**
- `AudioSource` trait for platform abstraction
- cpal-based audio capture (44.1/48kHz input)
- webrtc-vad integration for speech detection
- rubato resampling to 16kHz mono
- Ring buffer for chunk accumulation
- Audio route change handling

**Interface Contract:**
```rust
// lexicon-core/src/audio/mod.rs
pub trait AudioSource: Send {
    fn start(&mut self) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
    fn read_chunk(&mut self) -> Result<AudioChunk, AudioError>;
}

pub struct AudioChunk {
    pub samples: Vec<f32>,      // 16kHz mono
    pub has_speech: bool,       // VAD result
    pub timestamp_ms: i64,
}

pub struct CpalAudioSource { ... }
impl AudioSource for CpalAudioSource { ... }
```

**Completion Criteria:**
- [ ] Captures audio on macOS, Windows, Linux
- [ ] VAD correctly identifies speech segments
- [ ] Output is 16kHz mono f32 samples
- [ ] Handles device disconnection gracefully

**Architecture Refs:** Audio Pipeline (lines 165-185)

---

#### WP-DB: Storage Layer

**Deliverable:** SQLite database with FTS5 search

**Scope:**
- Schema: sessions, segments, models, settings, audio_files
- FTS5 virtual table for segment search
- Millisecond timestamp precision
- WAL mode configuration
- Blob storage with 100KB threshold
- Migration framework

**Interface Contract:**
```rust
// lexicon-core/src/storage/mod.rs
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, DatabaseError>;
    pub fn create_session(&self, metadata: SessionMetadata) -> Result<SessionId, DatabaseError>;
    pub fn insert_segment(&self, session: SessionId, segment: Segment) -> Result<SegmentId, DatabaseError>;
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, DatabaseError>;
    pub fn export_srt(&self, session: SessionId) -> Result<String, DatabaseError>;
}
```

**Completion Criteria:**
- [ ] Schema matches architecture spec
- [ ] FTS5 search returns ranked results
- [ ] WAL mode enabled for concurrent reads
- [ ] Export produces valid SRT format

**Architecture Refs:** Storage Layer (lines 217-234), R-DB-01

---

#### WP-MODEL: Model Manager

**Deliverable:** Model download, verification, and registry

**Scope:**
- Model manifest parsing (signed JSON)
- Minisign signature verification
- SHA-256 hash verification (download + load)
- Download progress reporting
- Local cache management
- Device capability profiling for model selection

**Interface Contract:**
```rust
// lexicon-core/src/models/mod.rs
pub struct ModelRegistry {
    cache_dir: PathBuf,
    manifest: ModelManifest,
}

impl ModelRegistry {
    pub fn new(cache_dir: &Path) -> Result<Self, ModelError>;
    pub fn fetch_manifest(&mut self) -> Result<(), ModelError>;
    pub fn download_model(&self, model_id: &str, progress: impl Fn(u64, u64)) -> Result<PathBuf, ModelError>;
    pub fn verify_model(&self, model_id: &str) -> Result<bool, ModelError>;
    pub fn get_model_path(&self, model_id: &str) -> Option<PathBuf>;
    pub fn recommend_models(&self, device_profile: &DeviceProfile) -> Vec<ModelId>;
}

pub struct DeviceProfile {
    pub available_memory_mb: u64,
    pub has_gpu: bool,
    pub platform: Platform,
}
```

**Completion Criteria:**
- [ ] Manifest signature verified before parsing
- [ ] Download streams with SHA-256 hashing
- [ ] Verification fails on hash mismatch
- [ ] Device profiling selects appropriate models

**Architecture Refs:** Model Verification (lines 296-329), R-SEC-01

---

### Layer 2: Engines (Depends on Layer 1)

These packages depend on Foundation layer completion. They can be developed in parallel with each other.

**Dependencies:**
- WP-STT requires: WP-CORE, WP-AUDIO, WP-MODEL
- WP-TTS requires: WP-CORE, WP-MODEL

---

#### WP-STT: Speech-to-Text Engine

**Deliverable:** whisper-rs integration with streaming support

**Scope:**
- whisper-rs context management
- VAD-driven chunk processing (5-10s segments)
- Quantized model loading (q5_1)
- Translation mode (X→English)
- Confidence scores extraction
- GPU acceleration (Metal, CUDA, Vulkan)

**Interface Contract:**
```rust
// lexicon-core/src/stt/mod.rs
pub trait Transcriber: Send {
    fn load_model(&mut self, path: &Path) -> Result<(), InferenceError>;
    fn transcribe(&self, audio: &AudioChunk) -> Result<Vec<Segment>, InferenceError>;
    fn translate(&self, audio: &AudioChunk) -> Result<Vec<Segment>, InferenceError>;
}

pub struct WhisperTranscriber {
    ctx: whisper_rs::WhisperContext,
}

impl Transcriber for WhisperTranscriber { ... }

pub struct Segment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f32>,
    pub language: Option<String>,
}
```

**Completion Criteria:**
- [ ] Transcribes audio with <800ms latency (base model)
- [ ] Translation mode produces English output
- [ ] GPU acceleration functional on supported platforms
- [ ] Handles chunk boundaries without word splits

**Architecture Refs:** STT Engine (lines 187-204), R-WRS-01

---

#### WP-TTS: Text-to-Speech Engine

**Deliverable:** Kokoro TTS with platform fallback

**Scope:**
- ort crate ONNX runtime integration
- Kokoro model loading (~350MB)
- Streaming audio output
- Language detection for fallback trigger
- tts-rs fallback for non-English
- Voice selection API

**Interface Contract:**
```rust
// lexicon-core/src/tts/mod.rs
pub trait Synthesizer: Send {
    fn load_model(&mut self, path: &Path) -> Result<(), InferenceError>;
    fn synthesize(&self, text: &str) -> Result<AudioBuffer, InferenceError>;
    fn synthesize_streaming(&self, text: &str) -> Result<impl Iterator<Item = AudioChunk>, InferenceError>;
}

pub struct KokoroSynthesizer {
    session: ort::Session,
}

impl Synthesizer for KokoroSynthesizer { ... }

pub struct PlatformSynthesizer { ... }  // tts-rs fallback
impl Synthesizer for PlatformSynthesizer { ... }
```

**Completion Criteria:**
- [ ] Synthesizes English text with Kokoro
- [ ] Streaming output begins within 200ms
- [ ] Fallback triggers on non-English detection
- [ ] Platform TTS works on all desktop platforms

**Architecture Refs:** TTS Engine (lines 206-215), R-TTS-01

---

### Layer 3: Integration (Depends on Layer 2)

These packages integrate engines into deployable forms. They can be developed in parallel.

**Dependencies:**
- WP-FFI requires: WP-STT, WP-TTS, WP-DB
- WP-SERVER requires: WP-STT, WP-TTS, WP-DB
- WP-CLI requires: WP-STT, WP-TTS, WP-DB

---

#### WP-FFI: Mobile FFI Bindings

**Deliverable:** UniFFI bindings for Swift and Kotlin

**Scope:**
- UniFFI UDL definition
- Swift package generation
- Kotlin bindings generation
- Async operation support
- Error mapping to platform exceptions
- Memory management (Arc, prevent leaks)

**Interface Contract:**
```
// lexicon-core/src/ffi.udl
namespace lexicon {
    [Throws=LexiconError]
    LexiconContext create_context(LexiconConfig config);
};

interface LexiconContext {
    [Throws=LexiconError]
    void load_model(string model_id);

    [Throws=LexiconError]
    TranscriptionSession start_session();
};

interface TranscriptionSession {
    [Throws=LexiconError]
    sequence<Segment> process_audio(sequence<float> samples);

    [Throws=LexiconError]
    void finish();
};
```

**Completion Criteria:**
- [ ] Swift package compiles in Xcode
- [ ] Kotlin bindings compile with Gradle
- [ ] Async callbacks work correctly
- [ ] No memory leaks under Instruments/LeakCanary

**Architecture Refs:** FFI Error Handling (lines 254-258), UniFFI tooling (line 353)

---

#### WP-SERVER: HTTP/WebSocket Server

**Deliverable:** Axum-based server with REST and WebSocket APIs

**Scope:**
- Axum router setup
- WebSocket streaming endpoints
- REST API for models, sessions, export
- Authentication (optional, configurable)
- CORS configuration
- Rate limiting
- Static file serving for web UI

**Interface Contract:**
```
WebSocket Endpoints:
  ws://host/api/v1/stt/stream    - Real-time STT (audio in, segments out)
  ws://host/api/v1/tts/stream    - Streaming TTS (text in, audio out)

REST Endpoints:
  GET  /api/v1/models            - List available models
  POST /api/v1/models/download   - Download model
  POST /api/v1/transcribe        - Batch transcription
  GET  /api/v1/sessions          - List sessions
  GET  /api/v1/sessions/{id}     - Get session details
  GET  /api/v1/sessions/{id}/export?format=srt - Export transcription
  DELETE /api/v1/sessions/{id}   - Delete session
```

**Completion Criteria:**
- [ ] WebSocket STT streams segments in real-time
- [ ] REST endpoints return valid JSON
- [ ] Server handles concurrent connections
- [ ] Graceful shutdown preserves data

**Architecture Refs:** Server API (lines 401-412)

---

#### WP-CLI: Command-Line Interface

**Deliverable:** Desktop CLI for transcription and server management

**Scope:**
- `lexicon transcribe <file>` - Batch transcription
- `lexicon serve` - Start HTTP server
- `lexicon models list|download|delete` - Model management
- `lexicon export <session> --format srt` - Export
- Progress indicators
- Configuration file support

**Interface Contract:**
```
lexicon 0.1.0
Real-time STT, translation, and TTS

USAGE:
    lexicon <COMMAND>

COMMANDS:
    transcribe  Transcribe audio file(s)
    serve       Start the HTTP server
    models      Manage models
    sessions    Manage transcription sessions
    export      Export session to file
    help        Print help information

OPTIONS:
    -c, --config <FILE>    Configuration file path
    -v, --verbose          Increase logging verbosity
    -h, --help             Print help
    -V, --version          Print version
```

**Completion Criteria:**
- [ ] All commands execute successfully
- [ ] Progress bars show during long operations
- [ ] Exit codes follow Unix conventions
- [ ] Shell completion scripts generated

---

### Layer 4: Platform (Depends on Layer 3)

These packages create platform-specific distributions. They can be developed in parallel.

**Dependencies:**
- WP-IOS requires: WP-FFI
- WP-ANDROID requires: WP-FFI
- WP-BUILD requires: All other packages

---

#### WP-IOS: iOS Platform Integration

**Deliverable:** iOS static library and Swift package

**Scope:**
- Static library build (aarch64-apple-ios)
- Simulator build (aarch64-apple-ios-sim)
- Swift package with C headers
- Background audio configuration
- Microphone permission handling
- CoreML/ANE acceleration

**Deliverables:**
- `Lexicon.xcframework` (universal binary)
- `LexiconSwift` Swift Package
- Example iOS app

**Completion Criteria:**
- [ ] Framework builds for device and simulator
- [ ] Swift API is ergonomic and documented
- [ ] Background transcription works
- [ ] ANE acceleration verified on device

**Architecture Refs:** iOS platform (line 126), Build targets (lines 338-339)

---

#### WP-ANDROID: Android Platform Integration

**Deliverable:** Android shared library and Kotlin bindings

**Scope:**
- Shared library build (aarch64-linux-android, armv7)
- JNI/Kotlin bindings
- Foreground service for background audio
- Permission handling
- XNNPACK acceleration

**Deliverables:**
- `liblexicon.so` (arm64-v8a, armeabi-v7a)
- `lexicon-android` AAR package
- Example Android app

**Completion Criteria:**
- [ ] Library loads on API 26+ devices
- [ ] Kotlin API matches Swift API semantics
- [ ] Foreground service keeps transcription active
- [ ] XNNPACK acceleration verified

**Architecture Refs:** Android platform (line 127), Build targets (lines 340-341)

---

#### WP-BUILD: CI/CD Pipeline

**Deliverable:** Automated build, test, and release pipeline

**Scope:**
- GitHub Actions workflows
- Cross-compilation for all targets
- Automated testing matrix
- Release artifact generation
- Code signing (iOS, Android)
- Model artifact publishing

**Workflows:**
- `ci.yml` - Build and test on PR
- `release.yml` - Build releases on tag
- `models.yml` - Sign and publish model manifests

**Completion Criteria:**
- [ ] All platforms build in CI
- [ ] Tests pass on all platforms
- [ ] Release creates signed artifacts
- [ ] Model manifest signing automated

**Architecture Refs:** Cross-Platform Build (lines 332-367)

---

## Phased Delivery

### Phase 1: MVP (Core Functionality)

**Goal:** Desktop STT transcription with persistence

**Included Packages:**
| Package | Scope Reduction |
|---------|-----------------|
| WP-CORE | Full |
| WP-AUDIO | Desktop only (no mobile routes) |
| WP-DB | Full |
| WP-MODEL | Download only (no device profiling) |
| WP-STT | Full |
| WP-CLI | `transcribe` and `models` commands only |

**Not Included:**
- TTS (WP-TTS)
- Server (WP-SERVER)
- Mobile (WP-FFI, WP-IOS, WP-ANDROID)
- CI/CD (WP-BUILD)

**MVP Deliverables:**
1. `lexicon transcribe audio.mp3` produces SRT output
2. Models download and verify correctly
3. Sessions persist to SQLite
4. Works on macOS, Windows, Linux

**Exit Criteria:**
- [ ] Transcribe 10-minute audio file successfully
- [ ] Search finds text across sessions
- [ ] Export produces valid SRT

---

### Phase 2: Beta (User Experience)

**Goal:** Full feature set with streaming and mobile

**Added Packages:**
| Package | Scope |
|---------|-------|
| WP-TTS | Full |
| WP-SERVER | Full |
| WP-FFI | Full |
| WP-AUDIO | Add mobile audio routes |
| WP-MODEL | Add device profiling |
| WP-CLI | Add `serve` command |

**Not Included:**
- Platform packages (WP-IOS, WP-ANDROID)
- CI/CD (WP-BUILD)

**Beta Deliverables:**
1. HTTP server with WebSocket streaming
2. TTS synthesis with Kokoro
3. UniFFI bindings compile
4. Real-time STT latency <800ms

**Exit Criteria:**
- [ ] WebSocket client receives streaming segments
- [ ] TTS produces natural-sounding speech
- [ ] Swift/Kotlin bindings pass unit tests
- [ ] Latency benchmarks documented

---

### Phase 3: Release (Production Ready)

**Goal:** Full platform coverage with production hardening

**Added Packages:**
| Package | Scope |
|---------|-------|
| WP-IOS | Full |
| WP-ANDROID | Full |
| WP-BUILD | Full |

**Release Deliverables:**
1. iOS framework on TestFlight
2. Android AAR on Maven
3. Desktop binaries on GitHub Releases
4. Signed model manifests

**Exit Criteria:**
- [ ] iOS app transcribes on device
- [ ] Android app transcribes on device
- [ ] All platforms pass integration tests
- [ ] Security audit completed

---

## Parallel Development Strategy

### Dependency Graph

```
Layer 1 (Parallel)
├── WP-CORE ─────────┐
├── WP-AUDIO ────────┤
├── WP-DB ───────────┼──▶ Layer 2 (Parallel)
└── WP-MODEL ────────┤    ├── WP-STT ──────┐
                     │    └── WP-TTS ──────┼──▶ Layer 3 (Parallel)
                     │                     │    ├── WP-FFI ────┐
                     └─────────────────────┘    ├── WP-SERVER  │
                                                └── WP-CLI     │
                                                               ▼
                                               Layer 4 (Parallel)
                                               ├── WP-IOS
                                               ├── WP-ANDROID
                                               └── WP-BUILD
```

### Concurrent Workstreams

**Maximum Parallelism:** 4 developers can work simultaneously during Layer 1

| Developer | Layer 1 | Layer 2 | Layer 3 | Layer 4 |
|-----------|---------|---------|---------|---------|
| Dev A | WP-CORE | WP-STT | WP-SERVER | WP-BUILD |
| Dev B | WP-AUDIO | WP-STT | WP-CLI | — |
| Dev C | WP-DB | WP-TTS | WP-FFI | WP-IOS |
| Dev D | WP-MODEL | WP-TTS | WP-FFI | WP-ANDROID |

### Interface Contracts

Work packages communicate through defined traits and structs. Developers can work against interfaces before implementations exist:

| Interface | Defined In | Used By |
|-----------|-----------|---------|
| `AudioSource` | WP-AUDIO | WP-STT |
| `Transcriber` | WP-STT | WP-FFI, WP-SERVER, WP-CLI |
| `Synthesizer` | WP-TTS | WP-FFI, WP-SERVER, WP-CLI |
| `Database` | WP-DB | WP-FFI, WP-SERVER, WP-CLI |
| `ModelRegistry` | WP-MODEL | WP-STT, WP-TTS |

### Mock Implementations

Each interface should have a mock implementation for testing:

```rust
// For WP-STT development before WP-AUDIO completes
pub struct MockAudioSource {
    samples: Vec<f32>,
}

// For WP-SERVER development before WP-STT completes
pub struct MockTranscriber {
    response: Vec<Segment>,
}
```

---

## Risk Mitigations

| Risk | Impact | Mitigation | Owner |
|------|--------|------------|-------|
| whisper-rs build complexity | HIGH | Document build matrix, provide Docker images | WP-STT |
| ort mobile cross-compilation | MEDIUM | tract fallback, pre-built binaries | WP-TTS |
| UniFFI async limitations | MEDIUM | Manual async wrapper if needed | WP-FFI |
| Thermal throttling on mobile | MEDIUM | Test early with real devices | WP-IOS, WP-ANDROID |

---

## References

- [architecture.md](architecture.md) — System architecture (authoritative)
- [project.md](../project.md) — Project constraints

---

*This document serves as the source of truth for Lexicon Local Server implementation. Update as development progresses.*
