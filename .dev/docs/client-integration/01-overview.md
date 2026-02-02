# Lexicon Integration Overview

## Architecture

Lexicon Local Server is a Rust core library providing:
- **Speech-to-Text (STT)** - Whisper or sherpa-onnx transcription
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
│  ┌──────────────────────┐  ┌──────────┐  ┌──────────────────┐   │
│  │         STT          │  │   TTS    │  │     Storage      │   │
│  │ whisper | sherpa-onnx│  │ (kokoro) │  │    (SQLite)      │   │
│  └──────────────────────┘  └──────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## STT Engine Comparison

Lexicon supports two STT engines with different performance characteristics:

| Feature | Whisper | sherpa-onnx (Zipformer) |
|---------|---------|-------------------------|
| Architecture | Encoder-decoder (batch) | Transducer (streaming-capable) |
| Latency | ~3250ms per 3s audio | ~50-300ms per 3s audio |
| Model Size | 75MB - 2.9GB | ~60-320MB |
| Languages | 99+ languages | English (more available) |
| Translation | Yes (to English) | No |
| Best For | Accuracy, translation | Low-latency, real-time |

### When to Use Each Engine

**Use Whisper (`Transcriber`) when:**
- You need translation to English
- You're transcribing non-English languages
- Accuracy is more important than latency
- Processing pre-recorded audio (not real-time)

**Use sherpa-onnx (`SherpaTranscriber`) when:**
- Low latency is critical (<500ms)
- Building real-time/live transcription
- English-only is acceptable
- Running on mobile devices where battery matters

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
2. **Set model directory** - Where models are stored
3. **Download model** (if needed) - Fetch from HuggingFace/GitHub
4. **Load model** - Load into memory for inference
5. **Process audio** - Transcribe/synthesize

```
init_lexicon(data_dir)
    │
    ├── Create Transcriber (choose one)
    │       │
    │       ├── Transcriber (Whisper)
    │       │       ├── load_model("whisper-base.bin")
    │       │       └── transcribe(audio)
    │       │
    │       └── SherpaTranscriber (low-latency)
    │               ├── load_model("models/sherpa/")  ← directory, not file
    │               └── transcribe(audio)
    │
    ├── Create Synthesizer
    │       │
    │       └── synthesize(text)
    │
    └── Create ModelDownloader (optional)
            │
            └── download_model(id, callback)
```

### Model Format Differences

| Engine | Model Format | Example Path |
|--------|--------------|--------------|
| Whisper | Single `.bin` file | `models/whisper-base.bin` |
| sherpa-onnx | Directory with multiple files | `models/sherpa/zipformer-en/` |

**sherpa-onnx model directory structure:**
```
models/sherpa/zipformer-en/
├── encoder-epoch-99-avg-1.int8.onnx  (54MB)
├── decoder-epoch-99-avg-1.onnx       (5MB)
├── joiner-epoch-99-avg-1.onnx        (1MB)
└── tokens.txt
```

## Thread Safety

- **Transcriber**: Thread-safe. Internal mutex protects model state.
- **Synthesizer**: Thread-safe. Internal mutex protects TTS state.
- **ModelDownloader**: Thread-safe. Uses Tokio runtime internally.
- **LexiconContext**: Thread-safe. Read-heavy operations.

All objects can be safely shared across threads (wrapped in Arc/similar as needed).

## Memory Considerations

### Model Sizes (Approximate)

**Whisper Models:**

| Model | Size on Disk | RAM Usage |
|-------|--------------|-----------|
| whisper-tiny | 75 MB | ~200 MB |
| whisper-base | 142 MB | ~400 MB |
| whisper-small | 466 MB | ~1.0 GB |
| whisper-medium | 1.5 GB | ~2.5 GB |
| whisper-large | 2.9 GB | ~4.5 GB |

**sherpa-onnx Models:**

| Model | Size on Disk | RAM Usage | RTF* |
|-------|--------------|-----------|------|
| zipformer-en-20M (int8) | ~60 MB | ~150 MB | 0.05 |
| zipformer-gigaspeech (int8) | ~320 MB | ~400 MB | 0.05 |

*RTF = Real-Time Factor. RTF of 0.05 means 1 second of audio processes in 50ms.

### Mobile Recommendations

**For latency-sensitive apps (real-time):**
- Use sherpa-onnx Zipformer models
- ~60MB model with ~150MB RAM, 50ms latency per second of audio

**For accuracy/translation:**
- **Low-end devices (2-4GB RAM):** whisper-tiny or whisper-base
- **Mid-range devices (4-6GB RAM):** whisper-base or whisper-small
- **High-end devices (6GB+ RAM):** whisper-small or whisper-medium

Use `get_platform_info()` to query device capabilities at runtime.

## Next Steps

- [02-swift-ios.md](02-swift-ios.md) - iOS/Swift integration
- [03-kotlin-android.md](03-kotlin-android.md) - Android/Kotlin integration
- [04-flutter.md](04-flutter.md) - Flutter/Dart integration
- [05-http-api.md](05-http-api.md) - HTTP API reference
