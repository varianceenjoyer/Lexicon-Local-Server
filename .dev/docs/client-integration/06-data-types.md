# Data Types Reference

## Overview

This document provides a complete reference for all data types used across Lexicon's APIs. Types are consistent across all integration methods (UniFFI, Flutter, HTTP).

## Configuration Types

### LexiconFfiConfig / LexiconConfig

Main configuration for initializing the Lexicon context.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `data_dir` | string | Yes | - | Path to data directory for cache/config |
| `model_dir` | string | No | `{data_dir}/models` | Path to model storage directory |
| `log_level` | string | No | `"info"` | Logging level: `error`, `warn`, `info`, `debug`, `trace` |

**Swift:**
```swift
let config = LexiconFfiConfig(
    dataDir: "/path/to/data",
    modelDir: "/path/to/models",
    logLevel: "info"
)
```

**Kotlin:**
```kotlin
val config = LexiconFfiConfig(
    dataDir = "/path/to/data",
    modelDir = "/path/to/models",
    logLevel = "info"
)
```

**Dart:**
```dart
final config = LexiconConfig(
    dataDir: '/path/to/data',
    modelDir: '/path/to/models',
    logLevel: 'info',
);
```

### AudioFfiConfig / AudioConfig

Audio capture and processing configuration.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `sample_rate` | u32 | No | `16000` | Sample rate in Hz |
| `chunk_ms` | u32 | No | `100` | Chunk duration in milliseconds |
| `vad_enabled` | bool | No | `true` | Enable voice activity detection |
| `vad_mode` | u8 | No | `2` | VAD aggressiveness (0-3, higher = more aggressive) |

### TranscriptionFfiConfig / TranscriptionConfig

Configuration for transcription requests.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `language` | string? | No | `null` (auto) | Language code (ISO 639-1) |
| `translate` | bool | No | `false` | Translate to English |
| `num_threads` | u32 | No | `4` | Number of CPU threads |
| `initial_prompt` | string? | No | `null` | Context prompt for transcription |
| `temperature` | f32 | No | `0.0` | Sampling temperature (0.0 = greedy) |

**Supported Languages:**

| Code | Language | Code | Language |
|------|----------|------|----------|
| `en` | English | `ja` | Japanese |
| `es` | Spanish | `ko` | Korean |
| `fr` | French | `pt` | Portuguese |
| `de` | German | `ru` | Russian |
| `it` | Italian | `zh` | Chinese |
| `nl` | Dutch | `ar` | Arabic |

Auto-detection works well for most languages when `language` is `null`.

### TtsFfiConfig / TtsConfig

Configuration for text-to-speech synthesis.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `language` | string? | No | `null` | Language/locale code (e.g., "en-US") |
| `voice_id` | string? | No | `null` | Specific voice ID |
| `rate` | f32 | No | `1.0` | Speech rate (0.5 - 2.0) |
| `pitch` | f32 | No | `0.0` | Pitch adjustment |
| `volume` | f32 | No | `1.0` | Volume (0.0 - 1.0) |
| `use_kokoro` | bool | No | `true` | Use Kokoro engine (vs platform native) |

## Result Types

### TranscriptionSegment

A single transcribed segment.

| Field | Type | Description |
|-------|------|-------------|
| `start_ms` | i64 | Start time in milliseconds |
| `end_ms` | i64 | End time in milliseconds |
| `text` | string | Transcribed text |
| `confidence` | f32? | Confidence score (0.0 - 1.0) |
| `language` | string? | Detected language for this segment |
| `speaker_id` | u32? | Speaker identifier (if diarization enabled) |

**Example (JSON):**
```json
{
  "start_ms": 0,
  "end_ms": 2500,
  "text": "Hello, world!",
  "confidence": 0.95,
  "language": "en",
  "speaker_id": null
}
```

### TranscriptionFfiResult / TranscriptionResult

Complete transcription result.

| Field | Type | Description |
|-------|------|-------------|
| `segments` | [TranscriptionSegment] | List of transcribed segments |
| `detected_language` | string? | Overall detected language |
| `audio_duration_ms` | i64 | Input audio duration |
| `processing_time_ms` | i64 | Time taken to process |

**Example (JSON):**
```json
{
  "segments": [
    {"start_ms": 0, "end_ms": 2500, "text": "Hello.", "confidence": 0.95},
    {"start_ms": 2500, "end_ms": 5000, "text": "World.", "confidence": 0.92}
  ],
  "detected_language": "en",
  "audio_duration_ms": 5000,
  "processing_time_ms": 1234
}
```

### SynthesisFfiResult / SynthesisResult

TTS synthesis result.

| Field | Type | Description |
|-------|------|-------------|
| `audio_data` | bytes | Raw f32 PCM audio samples |
| `sample_rate` | u32 | Output sample rate (typically 24000 for Kokoro, varies for platform TTS) |
| `duration_ms` | i64 | Audio duration in milliseconds |
| `engine` | string | Engine used ("Kokoro" or "Platform") |

## Model Types

### ModelFfiInfo / ModelInfo

Information about an available model.

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Model identifier (e.g., "whisper-base") |
| `name` | string | Human-readable name |
| `model_type` | string | Model type ("stt", "tts") |
| `size_bytes` | u64 | Size in bytes |
| `downloaded` | bool | Whether model is downloaded |
| `local_path` | string? | Path to downloaded model file |

**Available Models:**

| ID | Name | Type | Size |
|----|------|------|------|
| `whisper-tiny` | Whisper Tiny | stt | 75 MB |
| `whisper-base` | Whisper Base | stt | 142 MB |
| `whisper-small` | Whisper Small | stt | 466 MB |
| `whisper-medium` | Whisper Medium | stt | 1.5 GB |
| `whisper-large` | Whisper Large | stt | 2.9 GB |

### DownloadProgress

Progress information during model download.

| Field | Type | Description |
|-------|------|-------------|
| `model_id` | string | Model being downloaded |
| `downloaded_bytes` | u64 | Bytes downloaded so far |
| `total_bytes` | u64 | Total bytes to download |
| `speed_bps` | u64 | Download speed in bytes/second |
| `eta_seconds` | u64? | Estimated time remaining |

### PlatformFfiInfo / PlatformInfo

Platform and device information.

| Field | Type | Description |
|-------|------|-------------|
| `platform` | string | Platform name (see below) |
| `model_tier` | string | Recommended model tier |
| `available_memory_mb` | u64 | Available RAM in MB |
| `has_gpu` | bool | Whether GPU acceleration is available |

**Platform Values:**
- `MacOsArm64` - Apple Silicon Mac
- `MacOsX86` - Intel Mac
- `Windows` - Windows PC
- `LinuxX86` - Linux x86_64
- `LinuxArm64` - Linux ARM64
- `Ios` - iOS device
- `IosSimulator` - iOS Simulator
- `AndroidArm64` - Android ARM64
- `AndroidArm32` - Android ARM32
- `Unknown` - Unknown platform

**Model Tier Values:**
- `MobileLow` - Recommended: tiny (low-end mobile, <2GB RAM)
- `MobileMid` - Recommended: tiny/base (mid-range mobile, 2-4GB RAM)
- `Desktop` - Recommended: base/small (desktop devices)
- `Server` - Recommended: medium/large (high-end with GPU, 16GB+ RAM)

## Streaming Types (Flutter)

### AudioChunk

Input chunk for streaming transcription.

| Field | Type | Description |
|-------|------|-------------|
| `data` | bytes | Raw f32 PCM audio data |
| `timestamp_ms` | i64 | Timestamp from start of stream |
| `is_final` | bool | Whether this is the last chunk |

### TranscriptionEvent

Event from streaming transcription.

| Variant | Fields | Description |
|---------|--------|-------------|
| `Segment` | TranscriptionSegment | A transcribed segment |
| `Partial` | text, timestamp_ms | Interim result (may change) |
| `Complete` | TranscriptionResult | Final result |
| `Error` | string | Error message |

### TtsAudioChunk

Output chunk from streaming TTS.

| Field | Type | Description |
|-------|------|-------------|
| `data` | bytes | Raw f32 PCM audio data |
| `sample_rate` | u32 | Sample rate of this chunk |
| `sequence` | u32 | Chunk sequence number (0-indexed) |
| `is_final` | bool | Whether this is the last chunk |

### DownloadEvent

Event from model download.

| Variant | Fields | Description |
|---------|--------|-------------|
| `Progress` | DownloadProgress | Progress update |
| `Complete` | model_id, path | Download completed |
| `Error` | model_id, error | Download failed |
| `Cancelled` | model_id | Download was cancelled |

## Callback Interfaces (UniFFI)

### DownloadCallback

Implement this interface to receive download progress.

| Method | Parameters | Description |
|--------|------------|-------------|
| `on_progress` | DownloadProgress | Called periodically with progress |
| `on_complete` | model_id: string | Called when download completes |
| `on_error` | model_id: string, error: string | Called on failure |
| `on_cancelled` | model_id: string | Called when cancelled |

### TranscriptionCallback

Implement this interface for streaming transcription callbacks.

| Method | Parameters | Description |
|--------|------------|-------------|
| `on_segment` | TranscriptionSegment | Called for each segment |
| `on_final` | TranscriptionFfiResult | Called with final result |
| `on_error` | error: string | Called on failure |

## Type Mappings by Platform

### Primitive Types

| Rust | Swift | Kotlin | Dart | JSON |
|------|-------|--------|------|------|
| `i32` | `Int32` | `Int` | `int` | number |
| `i64` | `Int64` | `Long` | `int` | number |
| `u32` | `UInt32` | `UInt` | `int` | number |
| `u64` | `UInt64` | `ULong` | `int` | number |
| `f32` | `Float` | `Float` | `double` | number |
| `f64` | `Double` | `Double` | `double` | number |
| `bool` | `Bool` | `Boolean` | `bool` | boolean |
| `String` | `String` | `String` | `String` | string |
| `bytes` | `Data` | `ByteArray` | `Uint8List` | base64 string |
| `Option<T>` | `T?` | `T?` | `T?` | T or null |
| `Vec<T>` | `[T]` | `List<T>` | `List<T>` | array |

### Audio Data Type

Audio samples are always represented as:
- **Rust:** `Vec<u8>` (packed f32 bytes)
- **Swift:** `Data`
- **Kotlin:** `ByteArray`
- **Dart:** `Uint8List`
- **JSON:** Base64-encoded string

The bytes represent little-endian f32 (32-bit float) samples.

## JSON Schemas

### TranscriptionRequest

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["audio"],
  "properties": {
    "audio": {
      "type": "string",
      "description": "Base64-encoded audio (16kHz mono f32 PCM)"
    },
    "language": {
      "type": "string",
      "description": "ISO 639-1 language code"
    },
    "translate": {
      "type": "boolean",
      "default": false
    },
    "model": {
      "type": "string",
      "default": "base"
    }
  }
}
```

### TranscriptionResponse

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["segments", "duration_ms", "processing_time_ms"],
  "properties": {
    "segments": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["start_ms", "end_ms", "text"],
        "properties": {
          "start_ms": {"type": "integer"},
          "end_ms": {"type": "integer"},
          "text": {"type": "string"},
          "confidence": {"type": "number"},
          "language": {"type": "string"},
          "speaker_id": {"type": "integer"}
        }
      }
    },
    "detected_language": {"type": "string"},
    "duration_ms": {"type": "integer"},
    "processing_time_ms": {"type": "integer"}
  }
}
```
