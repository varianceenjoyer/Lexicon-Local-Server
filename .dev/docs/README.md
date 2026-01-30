# Lexicon Documentation

Developer documentation for Lexicon Local Server.

## Documentation Index

### Client Integration

Reference documentation for integrating Lexicon into client applications.

| Document | Description |
|----------|-------------|
| [client-integration/](client-integration/README.md) | **Client API Documentation** |
| [client-integration/01-overview.md](client-integration/01-overview.md) | Architecture and integration approaches |
| [client-integration/02-swift-ios.md](client-integration/02-swift-ios.md) | Swift/iOS integration guide |
| [client-integration/03-kotlin-android.md](client-integration/03-kotlin-android.md) | Kotlin/Android integration guide |
| [client-integration/04-flutter.md](client-integration/04-flutter.md) | Flutter/Dart integration guide |
| [client-integration/05-http-api.md](client-integration/05-http-api.md) | HTTP REST + WebSocket API |
| [client-integration/06-data-types.md](client-integration/06-data-types.md) | Complete data type reference |
| [client-integration/07-error-handling.md](client-integration/07-error-handling.md) | Error handling strategies |
| [client-integration/08-audio-formats.md](client-integration/08-audio-formats.md) | Audio format requirements |

### Platform-Specific

| Directory | Description |
|-----------|-------------|
| [android-emulation/](android-emulation/README.md) | Android emulator setup and testing |
| [android-termux/](android-termux/README.md) | Termux-based Android development |

## Quick Links

### For Client Developers

- **iOS/Swift**: Start with [02-swift-ios.md](client-integration/02-swift-ios.md)
- **Android/Kotlin**: Start with [03-kotlin-android.md](client-integration/03-kotlin-android.md)
- **Flutter**: Start with [04-flutter.md](client-integration/04-flutter.md)
- **Web/Desktop**: Start with [05-http-api.md](client-integration/05-http-api.md)

### For All Developers

- **Architecture**: [01-overview.md](client-integration/01-overview.md)
- **Data Types**: [06-data-types.md](client-integration/06-data-types.md)
- **Audio Format**: [08-audio-formats.md](client-integration/08-audio-formats.md)

## API Surface Summary

Lexicon provides three integration approaches:

### 1. Native FFI (UniFFI)

Direct library linking for iOS (Swift) and Android (Kotlin).

**Main Objects:**
- `LexiconContext` - Model management and platform info
- `Transcriber` - Speech-to-text transcription
- `Synthesizer` - Text-to-speech synthesis
- `ModelDownloader` - Async model downloading

### 2. Flutter Rust Bridge

Cross-platform mobile via flutter_rust_bridge.

**Main Objects:**
- `Transcriber` - Batch transcription
- `StreamingTranscriber` - Real-time streaming
- `Synthesizer` - Batch TTS
- `StreamingSynthesizer` - Chunked TTS
- `ModelDownloader` - Async downloads with events

### 3. HTTP Server

REST API + WebSocket streaming.

**Endpoints:**
- `GET /health` - Health check
- `GET /api/v1/models` - List models
- `POST /api/v1/transcribe` - Batch transcription
- `WS /api/v1/stt/stream` - Streaming STT
- `WS /api/v1/tts/stream` - Streaming TTS

## Audio Requirements

All interfaces require:
- **Sample Rate**: 16,000 Hz
- **Channels**: Mono
- **Format**: 32-bit float PCM (-1.0 to 1.0)
- **Byte Order**: Little-endian

See [08-audio-formats.md](client-integration/08-audio-formats.md) for conversion utilities.
