# Lexicon Client Integration Documentation

This directory contains reference documentation for integrating Lexicon Local Server into client applications.

## Document Index

| Document | Platform | Description |
|----------|----------|-------------|
| [01-overview.md](01-overview.md) | All | Architecture overview and integration approaches |
| [02-swift-ios.md](02-swift-ios.md) | iOS/macOS | Swift/UniFFI integration guide |
| [03-kotlin-android.md](03-kotlin-android.md) | Android | Kotlin/UniFFI integration guide |
| [04-flutter.md](04-flutter.md) | Cross-platform | Flutter/Dart integration guide |
| [05-http-api.md](05-http-api.md) | Desktop/Web | HTTP REST + WebSocket API reference |
| [06-data-types.md](06-data-types.md) | All | Complete data type reference |
| [07-error-handling.md](07-error-handling.md) | All | Error codes and handling strategies |
| [08-audio-formats.md](08-audio-formats.md) | All | Audio format requirements and conversion |

## Quick Start by Platform

### iOS (Swift)

```swift
import Lexicon

// Initialize
try initLexicon(dataDir: documentsDir)

// Create transcriber
let transcriber = try Transcriber()
try transcriber.loadModel(modelPath: modelPath)

// Transcribe
let config = TranscriptionFfiConfig(language: "en", translate: false, ...)
let result = try transcriber.transcribe(audioData: data, config: config)
```

### Android (Kotlin)

```kotlin
import uniffi.lexicon.*

// Initialize (can throw LexiconFfiException)
initLexicon(dataDir = context.filesDir.absolutePath)

// Create transcriber (can throw LexiconFfiException)
val transcriber = Transcriber()
transcriber.loadModel(modelPath)

// Transcribe
val config = TranscriptionFfiConfig(language = "en", translate = false, ...)
val result = transcriber.transcribe(audioData, config)
```

### Flutter (Dart)

```dart
import 'package:lexicon/lexicon.dart';

// Initialize (sync call)
initLexicon(dataDir: appDir);

// Create transcriber (async - returns Result)
final transcriber = await Transcriber.newInstance();
await transcriber.loadModel(modelPath: modelPath);

// Transcribe
final config = TranscriptionConfig(language: 'en');
final result = await transcriber.transcribe(audioData: audioData, config: config);
```

### HTTP API (Desktop/Web)

```bash
# Health check
curl http://localhost:3000/health

# Transcribe audio
curl -X POST http://localhost:3000/api/v1/transcribe \
  -H "Content-Type: application/json" \
  -d '{"audio": "<base64-encoded-pcm>", "language": "en"}'
```

## Integration Approaches

Lexicon supports three integration approaches:

1. **Native FFI (iOS/Android)** - Direct library linking via UniFFI bindings
2. **Flutter Rust Bridge** - Cross-platform mobile via flutter_rust_bridge
3. **HTTP Server** - REST API + WebSocket streaming for desktop/web

Choose based on your platform and requirements. See [01-overview.md](01-overview.md) for detailed comparison.
