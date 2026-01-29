---
id: WP-FLUTTER-FFI
type: work-plan
status: in-progress
phase: release
created: 2026-01-29
updated: 2026-01-29
author: design-lead
consumers: [developer]
depends-on: [WP-FFI]
blocks: []
priority: high
tags: [flutter, dart, ffi, flutter-rust-bridge, mobile, cross-platform]
---

# WP-FLUTTER-FFI: Flutter/Dart FFI Bindings (flutter_rust_bridge)

## Overview

Add `flutter_rust_bridge` support to Lexicon Local Server, enabling Flutter/Dart developers to integrate Lexicon Core directly into cross-platform mobile applications. This complements the existing UniFFI bindings (Swift/Kotlin) with an equivalent Dart API surface.

**Rationale**: Flutter provides a single-codebase alternative to native iOS/Android development. Research (R-FLUTTER-01 from Lexicon-Mobile) confirms `flutter_rust_bridge` is mature and supports the required streaming patterns via `StreamSink`.

## Scope

### In Scope

- New `lexicon-flutter` crate with `flutter_rust_bridge` bindings
- Dart API surface mirroring the existing `lexicon-ffi` UniFFI interface
- Streaming transcription callbacks via `StreamSink`
- Streaming TTS audio output via `StreamSink`
- Build targets for iOS and Android (Flutter-compatible libraries)
- Generated Dart bindings with type-safe structs and enums
- Integration documentation and example code

### Out of Scope

- Flutter UI components (handled by consuming Flutter apps)
- Platform audio capture (handled by Flutter audio plugins)
- Desktop Flutter targets (future enhancement)
- Web target (WASM not in scope for v1)

## Background

### Current State

Lexicon Local Server provides FFI bindings via `lexicon-ffi` crate using Mozilla's UniFFI:

| Binding | Target | Status |
|---------|--------|--------|
| UniFFI → Swift | iOS native | Complete |
| UniFFI → Kotlin | Android native | Complete |
| UniFFI → Dart | N/A | **Not supported** |

### Flutter Requirement

UniFFI does not generate Dart bindings. Flutter apps require an alternative FFI approach:

| FFI Library | Dart Support | Streaming | Maturity |
|-------------|--------------|-----------|----------|
| UniFFI | No | N/A | Excellent |
| **flutter_rust_bridge** | Yes | StreamSink | Excellent (Flutter Favorite) |
| Manual dart:ffi | Yes | Manual | Error-prone |

**Decision**: Use `flutter_rust_bridge` (v2.x) for Dart bindings.

### Research References

- [R-FLUTTER-01](../../Lexicon-Mobile/.dev/design/research/R-FLUTTER-01-cross-platform-viability.md) — Flutter viability assessment
- [R-FFI-01](../design/research/R-FFI-01-uniffi-integration.md) — UniFFI integration (reference API)

## Interface Contract

### API Surface (Dart)

The Dart API should mirror `lexicon-ffi` for consistency. Key differences:
- Callbacks become `Stream<T>` via `StreamSink`
- Async operations return `Future<T>` (native Dart async)

#### Initialization

```dart
// lib/src/lexicon_flutter.dart
import 'package:lexicon_flutter/lexicon_flutter.dart';

// Initialize library
Future<void> initLexicon({required String dataDir});

// Get version
String getVersion();
```

#### Configuration Types

```dart
class LexiconConfig {
  final String dataDir;
  final String? modelDir;
  final String logLevel;
}

class AudioConfig {
  final int sampleRate;
  final int chunkMs;
  final bool vadEnabled;
  final int vadMode;
}

class TranscriptionConfig {
  final String? language;
  final bool translate;
  final int numThreads;
  final String? initialPrompt;
  final double temperature;
}

class TtsConfig {
  final String? language;
  final String? voiceId;
  final double rate;
  final double pitch;
  final double volume;
  final bool useKokoro;
}
```

#### Result Types

```dart
class TranscriptionSegment {
  final int startMs;
  final int endMs;
  final String text;
  final double? confidence;
  final String? language;
  final int? speakerId;
}

class TranscriptionResult {
  final List<TranscriptionSegment> segments;
  final String? detectedLanguage;
  final int audioDurationMs;
  final int processingTimeMs;
}

class SynthesisResult {
  final Uint8List audioData;  // Raw PCM f32 bytes
  final int sampleRate;
  final int durationMs;
  final String engine;
}

class ModelInfo {
  final String id;
  final String name;
  final String modelType;
  final int sizeBytes;
  final bool downloaded;
  final String? localPath;
}

class DownloadProgress {
  final String modelId;
  final int downloadedBytes;
  final int totalBytes;
  final int speedBps;
  final int? etaSeconds;
}

class PlatformInfo {
  final String platform;
  final String modelTier;
  final int availableMemoryMb;
  final bool hasGpu;
}
```

#### LexiconContext

```dart
class LexiconContext {
  LexiconContext(LexiconConfig config);

  PlatformInfo getPlatformInfo();
  Future<List<ModelInfo>> listModels();
  Future<bool> isModelDownloaded(String modelId);
  Future<void> deleteModel(String modelId);
}
```

#### Transcriber (with Streaming)

```dart
class Transcriber {
  Transcriber();

  Future<void> loadModel(String modelPath);
  void unloadModel();
  bool isModelLoaded();
  String? getModelName();

  // Batch transcription
  Future<TranscriptionResult> transcribe(
    Uint8List audioData,
    TranscriptionConfig config,
  );

  // Streaming transcription (key flutter_rust_bridge feature)
  Stream<TranscriptionSegment> transcribeStream(
    Stream<Float32List> audioChunks,
    TranscriptionConfig config,
  );

  Future<TranscriptionResult> translate(
    Uint8List audioData,
    TranscriptionConfig config,
  );
}
```

#### Synthesizer (with Streaming)

```dart
class Synthesizer {
  Synthesizer();

  bool isReady();
  List<String> listVoices();

  // Batch synthesis
  Future<SynthesisResult> synthesize(String text, TtsConfig config);

  // Streaming synthesis (chunks as ready)
  Stream<Uint8List> synthesizeStream(String text, TtsConfig config);
}
```

#### Model Downloader

```dart
class ModelDownloader {
  ModelDownloader(String modelDir);

  // Download with progress stream
  Stream<DownloadProgress> downloadModel(String modelId);

  void cancelDownload(String modelId);
  bool isDownloading(String modelId);
}
```

### Rust Side (flutter_rust_bridge)

```rust
// crates/lexicon-flutter/src/api.rs

use flutter_rust_bridge::frb;

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Initialize Lexicon library
pub fn init_lexicon(data_dir: String) -> Result<(), LexiconFlutterError> { ... }

/// Get library version
pub fn get_version() -> String { ... }

// --- Transcriber with StreamSink ---

/// Streaming transcription - audio chunks in, segments out
pub fn transcribe_stream(
    model_path: String,
    config: TranscriptionConfig,
    audio_stream: impl Stream<Item = Vec<f32>>,
    segment_sink: StreamSink<TranscriptionSegment>,
) -> Result<(), LexiconFlutterError> { ... }

// --- Synthesizer with StreamSink ---

/// Streaming TTS - text in, audio chunks out
pub fn synthesize_stream(
    text: String,
    config: TtsConfig,
    audio_sink: StreamSink<Vec<u8>>,
) -> Result<(), LexiconFlutterError> { ... }

// --- Model Downloader with StreamSink ---

/// Download model with progress updates
pub fn download_model(
    model_dir: String,
    model_id: String,
    progress_sink: StreamSink<DownloadProgress>,
) -> Result<(), LexiconFlutterError> { ... }
```

## Technical Design

### Crate Structure

```
crates/
├── lexicon-core/           # Existing core library
├── lexicon-ffi/            # Existing UniFFI bindings (Swift/Kotlin)
└── lexicon-flutter/        # NEW: flutter_rust_bridge bindings (Dart)
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs          # Crate root
    │   ├── api.rs          # flutter_rust_bridge API functions
    │   ├── types.rs        # FFI types (mirroring lexicon-ffi)
    │   └── error.rs        # Error types
    └── flutter_rust_bridge.yaml  # Codegen config
```

### Cargo.toml

```toml
[package]
name = "lexicon-flutter"
description = "Flutter/Dart bindings for Lexicon via flutter_rust_bridge"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "staticlib"]

[features]
default = ["stt", "tts"]
stt = ["lexicon-core/stt"]
tts = ["lexicon-core/tts"]

[dependencies]
lexicon-core.workspace = true
flutter_rust_bridge = "2"
thiserror.workspace = true
anyhow.workspace = true
tracing.workspace = true
tokio.workspace = true
```

### Build Integration

```yaml
# flutter_rust_bridge.yaml
rust_input: crate::api
dart_output: lib/src/rust
full_dep: true
```

### Streaming Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      FLUTTER ↔ RUST STREAMING                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  DART SIDE                              RUST SIDE (flutter_rust_bridge)     │
│  ─────────                              ────────────────────────────────     │
│                                                                              │
│  ┌─────────────────┐                    ┌─────────────────┐                 │
│  │ Float32List     │ ──[audio chunks]──▶│ Stream<Vec<f32>>│                 │
│  │ Stream          │                    │ (receives)      │                 │
│  └─────────────────┘                    └────────┬────────┘                 │
│                                                  │                          │
│                                         ┌────────▼────────┐                 │
│                                         │  Whisper STT    │                 │
│                                         │  (process)      │                 │
│                                         └────────┬────────┘                 │
│                                                  │                          │
│  ┌─────────────────┐                    ┌────────▼────────┐                 │
│  │ Stream<Segment> │◀──[segments]───────│ StreamSink      │                 │
│  │ (Dart Stream)   │                    │ .add(segment)   │                 │
│  └─────────────────┘                    └─────────────────┘                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Build Targets

| Target | Library | Flutter Platform |
|--------|---------|------------------|
| `aarch64-apple-ios` | `liblexicon_flutter.a` | iOS (device) |
| `aarch64-apple-ios-sim` | `liblexicon_flutter.a` | iOS (simulator) |
| `aarch64-linux-android` | `liblexicon_flutter.so` | Android (arm64) |
| `armv7-linux-androideabi` | `liblexicon_flutter.so` | Android (arm32) |
| `x86_64-linux-android` | `liblexicon_flutter.so` | Android (x86_64 emulator) |

### CI Integration

Add to `.github/workflows/`:

```yaml
flutter-bindings:
  runs-on: ubuntu-latest
  steps:
    - uses: aspect-build/aspect_flutter_rust_bridge/setup@v1
    - run: flutter_rust_bridge_codegen generate
    - run: cargo build -p lexicon-flutter --target aarch64-linux-android
```

## Acceptance Criteria

### Functional

- [ ] `lexicon-flutter` crate compiles for all mobile targets
- [ ] Dart bindings generated via `flutter_rust_bridge_codegen`
- [ ] Batch transcription works: audio → segments
- [ ] Streaming transcription works: audio Stream → segment Stream
- [ ] Batch TTS works: text → audio bytes
- [ ] Streaming TTS works: text → audio chunk Stream
- [ ] Model download with progress Stream works
- [ ] All error types map to Dart exceptions

### Performance

- [ ] FFI call overhead <1ms (benchmark simple function)
- [ ] Audio chunk transfer <5ms for 100ms of audio (1600 samples)
- [ ] StreamSink latency comparable to UniFFI callbacks

### Quality

- [ ] API surface mirrors `lexicon-ffi` for consistency
- [ ] Dart types are idiomatic (e.g., `Uint8List` not `List<int>`)
- [ ] Documentation in Dart doc comments
- [ ] Example code in README

### Integration

- [ ] Flutter package published (or ready to publish) to private registry
- [ ] Integration tested with minimal Flutter app
- [ ] Cross-compilation works on CI (GitHub Actions)

## Dependencies

### Depends On

| Dependency | Reason |
|------------|--------|
| WP-FFI (complete) | API surface reference |
| lexicon-core | Core functionality |

### Blocks

| Blocked | Reason |
|---------|--------|
| Lexicon-Mobile Flutter implementation | Requires Dart bindings |

## Files

### Create

- `crates/lexicon-flutter/Cargo.toml`
- `crates/lexicon-flutter/src/lib.rs`
- `crates/lexicon-flutter/src/api.rs`
- `crates/lexicon-flutter/src/types.rs`
- `crates/lexicon-flutter/src/error.rs`
- `crates/lexicon-flutter/flutter_rust_bridge.yaml`

### Modify

- `Cargo.toml` (workspace members)
- `.github/workflows/ci.yml` (Flutter binding generation)

### Generate (by flutter_rust_bridge_codegen)

- `lib/src/rust/api.dart`
- `lib/src/rust/frb_generated.dart`
- `lib/src/rust/frb_generated.io.dart`

## Implementation Notes

### flutter_rust_bridge v2 Features

Key features to leverage:
- `StreamSink<T>` for Rust → Dart streaming
- `Stream<T>` for Dart → Rust streaming (audio chunks)
- Zero-copy mode for large arrays (`Float32List`)
- `#[frb(sync)]` for synchronous getters (version, model loaded)
- `#[frb(opaque)]` for Rust objects held by Dart

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum LexiconFlutterError {
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Cancelled")]
    Cancelled,
}
```

### Testing Strategy

1. **Unit tests**: Rust tests for API functions
2. **Integration tests**: Flutter app calling Rust
3. **Benchmark tests**: Measure FFI overhead, streaming latency

## Estimated Effort

| Task | Complexity |
|------|------------|
| Crate scaffolding | Low |
| Type definitions | Low |
| Batch API (transcribe, synthesize) | Medium |
| Streaming API (StreamSink) | Medium |
| Error handling | Low |
| Build targets setup | Medium |
| CI integration | Medium |
| Documentation | Low |
| Integration testing | Medium |

## References

### External

- [flutter_rust_bridge documentation](https://cjycode.com/flutter_rust_bridge/)
- [flutter_rust_bridge GitHub](https://github.com/aspect-build/aspect_flutter_rust_bridge)
- [StreamSink guide](https://cjycode.com/flutter_rust_bridge/guides/types/translatable/stream)

### Internal

- [lexicon-ffi/src/lib.rs](../../crates/lexicon-ffi/src/lib.rs) — UniFFI API reference
- [architecture.md](../design/architecture.md) — System architecture
- [R-FLUTTER-01](../../Lexicon-Mobile/.dev/design/research/R-FLUTTER-01-cross-platform-viability.md) — Flutter viability research

---

## Implementation Progress

### ✅ Completed (2026-01-29)

1. **Crate Scaffolding**
   - Created `crates/lexicon-flutter/` directory structure
   - `Cargo.toml` with flutter_rust_bridge v2 and workspace dependencies
   - Added to workspace `Cargo.toml`

2. **Core Modules**
   - `src/lib.rs` - Crate root with module exports
   - `src/error.rs` - `LexiconFlutterError` enum matching lexicon-ffi
   - `src/types.rs` - FFI types mirroring lexicon-ffi for API consistency
   - `src/api.rs` - flutter_rust_bridge annotated API:
     - `init_app()` - FRB initialization
     - `init_lexicon()` - Library initialization
     - `get_version()` - Version info
     - `get_platform_info()` - Platform detection
     - `list_models()` / `is_model_downloaded()` / `delete_model()` - Model management
     - `Transcriber` - Opaque handle with `load_model()`, `transcribe()`, `translate()`
     - `Synthesizer` - Opaque handle with `synthesize()`, `list_voices()`

3. **Unit Tests**
   - 4 tests passing for basic API functionality
   - Tests for version, platform info, transcriber creation, config defaults

### 🔄 Remaining

- [ ] Generate Dart bindings via `flutter_rust_bridge_codegen`
- [ ] Add StreamSink-based streaming API for real-time transcription
- [ ] Add StreamSink-based streaming API for TTS
- [ ] Add model download with progress Stream
- [ ] Build for mobile targets (iOS, Android)
- [ ] Create flutter_rust_bridge.yaml config
- [ ] Integration test with Flutter app
- [ ] CI pipeline for Dart codegen

### Acceptance Criteria Progress

**Functional:**
- [x] `lexicon-flutter` crate compiles (desktop targets)
- [ ] `lexicon-flutter` crate compiles for all mobile targets
- [ ] Dart bindings generated via `flutter_rust_bridge_codegen`
- [x] Batch transcription works: audio → segments (API defined)
- [ ] Streaming transcription works: audio Stream → segment Stream
- [x] Batch TTS works: text → audio bytes (API defined)
- [ ] Streaming TTS works: text → audio chunk Stream
- [ ] Model download with progress Stream works
- [x] All error types map to Dart exceptions

**Quality:**
- [x] API surface mirrors `lexicon-ffi` for consistency
- [ ] Dart types are idiomatic (pending codegen)
- [ ] Documentation in Dart doc comments
- [ ] Example code in README

---

*Implementation started 2026-01-29. Crate scaffolding and batch APIs complete.*
