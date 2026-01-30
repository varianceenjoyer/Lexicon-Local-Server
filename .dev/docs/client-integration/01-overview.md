# Lexicon Integration Overview

## Architecture

Lexicon Local Server is a Rust core library providing:
- **Speech-to-Text (STT)** - Whisper-based transcription
- **Text-to-Speech (TTS)** - Kokoro + platform native synthesis
- **Translation** - Audio-to-English translation via Whisper

The library exposes a C ABI, enabling integration via:

```
┌─────────────────────────────────────────────────────────────────┐
│                      Client Applications                         │
├─────────────────┬─────────────────┬─────────────────────────────┤
│   iOS (Swift)   │ Android (Kotlin)│     Flutter (Dart)          │
│   via UniFFI    │   via UniFFI    │ via flutter_rust_bridge     │
├─────────────────┴─────────────────┴─────────────────────────────┤
│                    lexicon-ffi / lexicon-flutter                 │
├─────────────────────────────────────────────────────────────────┤
│                        lexicon-core (Rust)                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐ │
│  │   STT    │  │   TTS    │  │  Models  │  │     Storage      │ │
│  │(whisper) │  │ (kokoro) │  │ Registry │  │    (SQLite)      │ │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

For desktop platforms, an optional HTTP server mode is available:

```
┌──────────────────────────┐
│   Web/Desktop Clients    │
│   (HTTP + WebSocket)     │
├──────────────────────────┤
│   lexicon-server (Axum)  │
├──────────────────────────┤
│     lexicon-core         │
└──────────────────────────┘
```

## Integration Approaches Comparison

| Approach | Platforms | Latency | Binary Size | Complexity |
|----------|-----------|---------|-------------|------------|
| UniFFI (Native) | iOS, Android | Lowest | ~15-30MB | Medium |
| Flutter Bridge | iOS, Android, Desktop | Low | ~20-40MB | Medium |
| HTTP Server | All | Higher | N/A (separate) | Low |

### UniFFI (Recommended for Native Apps)

Best for native iOS (Swift) and Android (Kotlin) apps.

**Pros:**
- Lowest latency (direct function calls)
- Type-safe generated bindings
- Full async/callback support
- No network overhead

**Cons:**
- Platform-specific integration code
- Larger app bundle size

**Use when:** Building dedicated iOS/Android apps with real-time requirements.

### Flutter Rust Bridge

Best for cross-platform mobile apps using Flutter.

**Pros:**
- Single codebase for iOS + Android + Desktop
- Streaming support via Dart Streams
- Opaque handles for stateful objects

**Cons:**
- Additional abstraction layer
- Flutter-specific tooling required

**Use when:** Building cross-platform apps with Flutter.

### HTTP Server

Best for desktop apps, web clients, or when network separation is needed.

**Pros:**
- Language/platform agnostic
- WebSocket streaming support
- Familiar REST API patterns
- Can serve multiple clients

**Cons:**
- Network latency overhead
- Requires running server process
- Additional deployment complexity

**Use when:** Desktop apps, web interfaces, or multi-client scenarios.

## Feature Availability

| Feature | UniFFI | Flutter | HTTP |
|---------|--------|---------|------|
| Batch Transcription | Yes | Yes | Yes |
| Streaming Transcription | Via callback | Via chunks | WebSocket |
| Batch TTS | Yes | Yes | Yes |
| Streaming TTS | Via callback | Via chunks | WebSocket |
| Model Download | Async + cancel | Async + cancel | Yes |
| Progress Callbacks | Yes | Yes | SSE/polling |
| Session Storage | No (use core) | No (use core) | Yes |

## Audio Format Requirements

All interfaces expect the same audio format:

- **Sample Rate:** 16,000 Hz (16 kHz)
- **Channels:** Mono (1 channel)
- **Format:** 32-bit float PCM (f32le)
- **Byte Order:** Little-endian

Audio bytes are packed as consecutive f32 samples:
```
[sample0: 4 bytes][sample1: 4 bytes][sample2: 4 bytes]...
```

See [08-audio-formats.md](08-audio-formats.md) for conversion utilities.

## Initialization Flow

All platforms follow the same initialization pattern:

1. **Set data directory** - Where Lexicon stores configuration and cache
2. **Set model directory** - Where Whisper models are stored
3. **Download model** (if needed) - Fetch from Hugging Face
4. **Load model** - Load into memory for inference
5. **Process audio** - Transcribe/synthesize

```
init_lexicon(data_dir)
    │
    ├── Create Transcriber/Synthesizer
    │       │
    │       ├── load_model(path)
    │       │       │
    │       │       └── Ready for inference
    │       │
    │       └── transcribe(audio) / synthesize(text)
    │
    └── Create ModelDownloader (optional)
            │
            └── download_model(id, callback)
```

## Thread Safety

- **Transcriber**: Thread-safe. Internal mutex protects model state.
- **Synthesizer**: Thread-safe. Internal mutex protects TTS state.
- **ModelDownloader**: Thread-safe. Uses Tokio runtime internally.
- **LexiconContext**: Thread-safe. Read-heavy operations.

All objects can be safely shared across threads (wrapped in Arc/similar as needed).

## Memory Considerations

### Model Sizes (Approximate)

| Model | Size on Disk | RAM Usage |
|-------|--------------|-----------|
| whisper-tiny | 75 MB | ~200 MB |
| whisper-base | 142 MB | ~400 MB |
| whisper-small | 466 MB | ~1.0 GB |
| whisper-medium | 1.5 GB | ~2.5 GB |
| whisper-large | 2.9 GB | ~4.5 GB |

### Mobile Recommendations

- **Low-end devices (2-4GB RAM):** whisper-tiny or whisper-base
- **Mid-range devices (4-6GB RAM):** whisper-base or whisper-small
- **High-end devices (6GB+ RAM):** whisper-small or whisper-medium

Use `get_platform_info()` to query device capabilities at runtime.

## Next Steps

- [02-swift-ios.md](02-swift-ios.md) - iOS/Swift integration
- [03-kotlin-android.md](03-kotlin-android.md) - Android/Kotlin integration
- [04-flutter.md](04-flutter.md) - Flutter/Dart integration
- [05-http-api.md](05-http-api.md) - HTTP API reference
