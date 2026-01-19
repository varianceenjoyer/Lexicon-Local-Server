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

**Deliverable:** Audio capture, VAD, resampling, and Bluetooth support

**Scope:**
- `AudioSource` trait for platform abstraction
- cpal-based audio capture (44.1/48kHz input)
- webrtc-vad integration for speech detection
- rubato resampling to 16kHz mono
- Ring buffer for chunk accumulation
- Audio route change handling (stutter mitigation per R-AUD-01)
- **Bluetooth HFP 1.6 support** (Decision D2):
  - Codec negotiation for 16kHz sample rate
  - Graceful handling of Bluetooth connect/disconnect
  - Automatic fallback to device microphone on disconnection
  - Route change detection and stream rebuild

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
- [ ] Bluetooth HFP 1.6 negotiates 16kHz when available
- [ ] Route changes handled without crashes

**Architecture Refs:** Audio Pipeline, Decision D2 (Bluetooth HFP 1.6)

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

**Deliverable:** Model download, verification, registry, and thermal-aware selection

**Scope:**
- Model manifest parsing (signed JSON)
- Minisign signature verification
- SHA-256 hash verification (download + load)
- Download progress reporting
- Local cache management
- **Immediate download flow** (Decision D1):
  - Device capability profiling at first launch
  - Tier determination (mobile low/mid, desktop, server)
  - Automatic download of all tier-appropriate models
  - Block STT features until at least one model ready
  - Background download for TTS model (non-blocking)
- **Thermal-aware model switching**:
  - Interface for receiving thermal state from platform layer
  - Automatic downgrade (base → tiny) when thermal headroom < 0.7
  - Upgrade back to preferred model when cooled

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
    pub thermal_headroom: Option<f32>,  // 0.0-1.0, None if unavailable
}

pub enum ThermalState {
    Nominal,
    Fair,
    Serious,
    Critical,
}
```

**Completion Criteria:**
- [ ] Manifest signature verified before parsing
- [ ] Download streams with SHA-256 hashing
- [ ] Verification fails on hash mismatch
- [ ] Device profiling selects appropriate tier models
- [ ] First launch downloads all tier models before allowing transcription
- [ ] Thermal state triggers model downgrade at headroom < 0.7

**Architecture Refs:** Model Verification, Decisions D1 (Immediate Download), Thermal Management

---

### Layer 2: Engines (Depends on Layer 1)

These packages depend on Foundation layer completion. They can be developed in parallel with each other.

**Dependencies:**
- WP-STT requires: WP-CORE, WP-AUDIO, WP-MODEL
- WP-TTS requires: WP-CORE, WP-MODEL

---

#### WP-STT: Speech-to-Text Engine

**Deliverable:** whisper-rs integration with streaming and diarization support

**Scope:**
- whisper-rs context management
- VAD-driven chunk processing (5-10s segments)
- Quantized model loading (q5_1)
- Translation mode (X→English)
- Confidence scores extraction
- GPU acceleration (Metal, CUDA, Vulkan)
- **Speaker diarization** (Decision D3):
  - Configurable per-session (enable/disable)
  - Speaker embedding extraction
  - Clustering for speaker identification
  - `speaker_id` assignment to segments
  - Algorithm options: whisper-based clustering (MVP), sherpa-onnx (future)

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
    pub speaker_id: Option<u32>,  // Decision D3: diarization
}

pub struct TranscriptionConfig {
    pub enable_diarization: bool,
    pub max_speakers: Option<u32>,
}
```

**Completion Criteria:**
- [ ] Transcribes audio with <800ms latency (base model)
- [ ] Translation mode produces English output
- [ ] GPU acceleration functional on supported platforms
- [ ] Handles chunk boundaries without word splits
- [ ] Diarization assigns consistent speaker IDs when enabled

**Architecture Refs:** STT Engine, Decision D3 (Speaker Diarization)

---

#### WP-TTS: Text-to-Speech Engine

**Deliverable:** Kokoro TTS with platform fallback

**Scope:**
- ort crate ONNX runtime integration
- Kokoro model loading (~350MB)
- Streaming audio output
- Voice selection API
- **tts-rs fallback triggers** (Decision D4):
  - Non-English language detected in text (automatic)
  - User explicitly selects platform voice (preference)
  - Kokoro model fails to load (error recovery)
  - Platform-native synthesis: AVSpeechSynthesizer (iOS), Android TTS, SAPI (Windows)

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

pub enum TtsEngine {
    Kokoro,
    Platform,  // tts-rs
}

pub struct TtsConfig {
    pub preferred_engine: TtsEngine,
    pub fallback_enabled: bool,
}
```

**Completion Criteria:**
- [ ] Synthesizes English text with Kokoro
- [ ] Streaming output begins within 200ms
- [ ] Fallback triggers on non-English detection
- [ ] Fallback triggers on user preference for platform voice
- [ ] Fallback triggers on Kokoro model load failure
- [ ] Platform TTS works on all desktop platforms

**Architecture Refs:** TTS Engine, Decision D4 (Multi-language TTS fallback)

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
- **Thermal monitoring** (supports WP-MODEL thermal-aware switching):
  - `ProcessInfo.thermalState` monitoring
  - Thermal state change notifications
  - Expose thermal headroom to Rust core via FFI
  - User warning at "serious" thermal state

**Deliverables:**
- `Lexicon.xcframework` (universal binary)
- `LexiconSwift` Swift Package
- Example iOS app

**Completion Criteria:**
- [ ] Framework builds for device and simulator
- [ ] Swift API is ergonomic and documented
- [ ] Background transcription works
- [ ] ANE acceleration verified on device
- [ ] Thermal state changes trigger model switching
- [ ] User warned at serious/critical thermal state

**Architecture Refs:** iOS platform, Thermal Management, R-THR-01

---

#### WP-ANDROID: Android Platform Integration

**Deliverable:** Android shared library and Kotlin bindings

**Scope:**
- Shared library build (aarch64-linux-android, armv7)
- JNI/Kotlin bindings
- Foreground service for background audio
- Permission handling
- XNNPACK acceleration (NNAPI deprecated per R-ORT-01)
- **Thermal monitoring** (supports WP-MODEL thermal-aware switching):
  - `PowerManager.getThermalHeadroom()` monitoring (API 29+)
  - Thermal status callback registration
  - Expose thermal headroom to Rust core via JNI
  - User warning when headroom < 0.3

**Deliverables:**
- `liblexicon.so` (arm64-v8a, armeabi-v7a)
- `lexicon-android` AAR package
- Example Android app

**Completion Criteria:**
- [ ] Library loads on API 26+ devices
- [ ] Kotlin API matches Swift API semantics
- [ ] Foreground service keeps transcription active
- [ ] XNNPACK acceleration verified
- [ ] Thermal headroom monitoring works on API 29+
- [ ] Model downgrade triggered when headroom < 0.7

**Architecture Refs:** Android platform, Thermal Management, R-THR-01

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

**Goal:** Desktop STT transcription with persistence and immediate model download

**Included Packages:**
| Package | Scope |
|---------|-------|
| WP-CORE | Full |
| WP-AUDIO | Desktop only (no Bluetooth, no mobile routes) |
| WP-DB | Full |
| WP-MODEL | Full including device profiling and immediate download (Decision D1) |
| WP-STT | Core transcription (diarization deferred to Beta) |
| WP-CLI | `transcribe` and `models` commands only |

**Not Included:**
- TTS (WP-TTS)
- Server (WP-SERVER)
- Mobile (WP-FFI, WP-IOS, WP-ANDROID)
- CI/CD (WP-BUILD)
- Speaker diarization (deferred to Beta)
- Bluetooth audio (deferred to Beta)

**MVP Deliverables:**
1. `lexicon transcribe audio.mp3` produces SRT output
2. First launch profiles device and downloads tier-appropriate models
3. Sessions persist to SQLite with FTS5 search
4. Works on macOS, Windows, Linux

**Exit Criteria:**
- [ ] First launch downloads models before allowing transcription (D1)
- [ ] Transcribe 10-minute audio file successfully
- [ ] Search finds text across sessions
- [ ] Export produces valid SRT

---

### Phase 2: Beta (User Experience)

**Goal:** Full feature set with streaming, mobile bindings, and enhanced audio

**Added Packages:**
| Package | Scope |
|---------|-------|
| WP-TTS | Full including fallback triggers (D4) |
| WP-SERVER | Full |
| WP-FFI | Full |
| WP-AUDIO | Add Bluetooth HFP 1.6 (D2), mobile audio routes |
| WP-STT | Add speaker diarization (D3) |
| WP-CLI | Add `serve` command |

**Not Included:**
- Platform packages (WP-IOS, WP-ANDROID)
- CI/CD (WP-BUILD)

**Beta Deliverables:**
1. HTTP server with WebSocket streaming
2. TTS synthesis with Kokoro + tts-rs fallback
3. UniFFI bindings compile
4. Real-time STT latency <800ms
5. Speaker diarization assigns IDs when enabled
6. Bluetooth audio works at 16kHz

**Exit Criteria:**
- [ ] WebSocket client receives streaming segments
- [ ] TTS produces natural-sounding speech
- [ ] TTS falls back to platform voice for non-English (D4)
- [ ] Swift/Kotlin bindings pass unit tests
- [ ] Diarization correctly identifies 2+ speakers (D3)
- [ ] Bluetooth HFP 1.6 verified at 16kHz (D2)
- [ ] Latency benchmarks documented

---

### Phase 3: Release (Production Ready)

**Goal:** Full platform coverage with production hardening and thermal management

**Added Packages:**
| Package | Scope |
|---------|-------|
| WP-IOS | Full including thermal monitoring |
| WP-ANDROID | Full including thermal monitoring |
| WP-BUILD | Full |

**Release Deliverables:**
1. iOS framework on TestFlight
2. Android AAR on Maven
3. Desktop binaries on GitHub Releases
4. Signed model manifests

**Exit Criteria:**
- [ ] iOS app transcribes on device
- [ ] Android app transcribes on device
- [ ] Thermal throttling triggers model downgrade on both platforms
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
| Thermal throttling on mobile | MEDIUM | Thermal API monitoring, adaptive model switching | WP-MODEL, WP-IOS, WP-ANDROID |
| Bluetooth audio quality | MEDIUM | HFP 1.6 negotiation, fallback to device mic | WP-AUDIO |
| Diarization accuracy | MEDIUM | Configurable, multiple algorithm options | WP-STT |

---

## References

- [architecture.md](architecture.md) — System architecture (authoritative)
- [project.md](../project.md) — Project constraints

---

*This document serves as the source of truth for Lexicon Local Server implementation. Update as development progresses.*
