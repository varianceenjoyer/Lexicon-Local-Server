# HTTP API Reference

## Overview

The Lexicon HTTP server provides REST endpoints and WebSocket streaming for desktop and web clients. The server is built with Axum and supports concurrent connections.

## Starting the Server

### CLI

```bash
# Start server with default config (127.0.0.1:8765)
lexicon serve

# With custom host and port
lexicon serve --host 0.0.0.0 --port 3000
```

### Programmatic (Rust)

For more advanced configuration (API keys, rate limiting, model directory), use the library directly:

```rust
use lexicon_server::{Server, ServerConfig, AuthConfig, RateLimitConfig};
use lexicon_core::LexiconConfig;

let core_config = LexiconConfig::builder()
    .data_dir("/path/to/data")
    .model_dir("/path/to/models")
    .build()?;

let server_config = ServerConfig::new("127.0.0.1", 3000)
    .with_core_config(core_config)
    .with_api_key("your-secret-key")  // Enable authentication
    .with_rate_limit(RateLimitConfig::default());  // Enable rate limiting

let server = Server::new(server_config)?;
server.run().await?;
```

## Base URL

Default (port 8765):
```
http://localhost:8765
```

With custom port (`--port 3000`):
```
http://localhost:3000
```

Note: Examples in this document use port 3000 for illustration.

## Authentication

Authentication is **optional** and disabled by default for local use.

### Enabling Authentication

The CLI does not currently support authentication flags. To enable authentication, configure the server programmatically:

```rust
use lexicon_server::{Server, ServerConfig, AuthConfig};

// Simple: single API key
let config = ServerConfig::default()
    .with_api_key("your-secret-key");

// Advanced: multiple keys with full AuthConfig control
let config = ServerConfig::default()
    .with_auth(AuthConfig::with_api_key("key1").add_api_key("key2"));
```

### Using API Keys

When authentication is enabled, include the `X-API-Key` header:

```bash
curl -H "X-API-Key: your-secret-key" http://localhost:3000/api/v1/models
```

```javascript
fetch('/api/v1/models', {
  headers: {
    'X-API-Key': 'your-secret-key'
  }
});
```

### Bypassed Endpoints

The following endpoints do not require authentication:
- `GET /health` - Always accessible for health checks

### Error Responses

**401 Unauthorized (missing key):**
```json
{
  "error": "unauthorized",
  "message": "API key required. Provide via X-API-Key header."
}
```

**401 Unauthorized (invalid key):**
```json
{
  "error": "unauthorized",
  "message": "Invalid or missing API key"
}
```

## Endpoints

### Health Check

```http
GET /health
```

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "stt_ready": true,
  "tts_ready": true,
  "db_ready": true
}
```

**Note:** The `stt_ready`, `tts_ready`, and `db_ready` fields are only present when their respective features are enabled in the server build. They may be omitted entirely if the feature is disabled.

### Models

#### List Models

```http
GET /api/v1/models
```

**Response:**
```json
[
  {
    "id": "whisper-tiny",
    "name": "Whisper Tiny",
    "size_bytes": 75000000,
    "downloaded": false,
    "path": null
  },
  {
    "id": "whisper-base",
    "name": "Whisper Base",
    "size_bytes": 142000000,
    "downloaded": true,
    "path": "/path/to/models/whisper-base.bin"
  }
]
```

#### Get Model Info

```http
GET /api/v1/models/:model_id
```

**Response:**
```json
{
  "id": "whisper-base",
  "name": "whisper-base",
  "size_bytes": 142000000,
  "downloaded": true,
  "path": "/path/to/models/whisper-base.bin"
}
```

**Note:** `size_bytes` may be `null` if the model is not downloaded or file metadata cannot be read.

#### Download Model

```http
POST /api/v1/models/download
Content-Type: application/json

{
  "model_id": "whisper-base"
}
```

**Note:** This endpoint is synchronous and blocks until the download completes.

**Response (already exists):**
```json
{
  "status": "exists",
  "model_id": "whisper-base",
  "message": "Model already downloaded"
}
```

**Response (download complete):**
```json
{
  "status": "complete",
  "model_id": "whisper-base",
  "message": "Model downloaded successfully"
}
```

#### Delete Model

```http
DELETE /api/v1/models/:model_id
```

**Response:**
```json
{
  "status": "deleted",
  "model_id": "whisper-base"
}
```

**Error Response (404):**
```json
{
  "error": "Model not found: whisper-base",
  "code": "NOT_FOUND"
}
```

### Transcription

#### Batch Transcribe

```http
POST /api/v1/transcribe
Content-Type: application/json

{
  "audio": "<base64-encoded-16kHz-mono-f32-PCM>",
  "language": "en",
  "translate": false,
  "model": "base"
}
```

**Request Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `audio` | string | Yes | Base64-encoded audio (16kHz mono f32 PCM) |
| `language` | string | No | Language code (auto-detect if omitted) |
| `translate` | boolean | No | Translate to English (default: false) |
| `model` | string | No | Model to use (default: "base") |

**Response:**
```json
{
  "segments": [
    {
      "start_ms": 0,
      "end_ms": 2500,
      "text": "Hello, world!",
      "confidence": 0.95
    },
    {
      "start_ms": 2500,
      "end_ms": 5000,
      "text": "This is a test.",
      "confidence": 0.92
    }
  ],
  "detected_language": "en",
  "duration_ms": 5000,
  "processing_time_ms": 1234
}
```

**Error Response (400):**
```json
{
  "error": "Invalid base64 audio: invalid padding",
  "code": "BAD_REQUEST"
}
```

**Error Response (503):**
```json
{
  "error": "Model not available: whisper-base",
  "code": "SERVICE_UNAVAILABLE"
}
```

### Sessions (Storage Feature)

#### List Sessions

```http
GET /api/v1/sessions
```

**Response:**
```json
[
  {
    "id": "1",
    "name": "Meeting Notes",
    "language": "en",
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T11:45:00Z"
  }
]
```

#### Get Session

```http
GET /api/v1/sessions/:session_id
```

**Response:**
```json
{
  "id": 1,
  "name": "Meeting Notes",
  "language": "en",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T11:45:00Z",
  "duration_ms": 300000,
  "segment_count": 42,
  "segments": [
    {
      "id": 1,
      "start_ms": 0,
      "end_ms": 2500,
      "text": "Hello everyone.",
      "confidence": 0.95,
      "language": "en",
      "speaker_id": null
    }
  ]
}
```

#### Delete Session

```http
DELETE /api/v1/sessions/:session_id
```

**Response:**
```json
{
  "status": "deleted",
  "session_id": 1
}
```

#### Export Session

```http
GET /api/v1/sessions/:session_id/export?format=srt
```

**Query Parameters:**

| Parameter | Values | Description |
|-----------|--------|-------------|
| `format` | `srt`, `json` | Export format (default: srt) |

**Response (SRT format):**
```
1
00:00:00,000 --> 00:00:02,500
Hello everyone.

2
00:00:02,500 --> 00:00:05,000
Welcome to the meeting.
```

**Response (JSON format):**
```json
{
  "session_id": 1,
  "name": "Meeting Notes",
  "language": "en",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T11:45:00Z",
  "segments": [...]
}
```

## WebSocket Streaming

### STT Streaming

```
WebSocket: ws://localhost:3000/api/v1/stt/stream
```

#### Connection Flow

1. Connect to WebSocket
2. Receive `ready` message
3. Send audio chunks as binary messages
4. Receive transcription segments as JSON
5. Close connection when done

#### Server Messages

**Ready:**
```json
{"type": "ready", "message": "STT stream ready"}
```

**Segment:**
```json
{
  "type": "segment",
  "start_ms": 0,
  "end_ms": 2500,
  "text": "Hello, world!",
  "confidence": 0.95
}
```

**Error:**
```json
{
  "type": "error",
  "message": "No model loaded"
}
```

**Model Loaded:**
```json
{
  "type": "model_loaded",
  "model": "whisper-base"
}
```

#### Client Messages

**Audio Data:** Send binary message with raw f32 PCM samples.

**Load Model:**
```json
{
  "type": "load_model",
  "model": "whisper-base"
}
```

#### JavaScript Example

```javascript
const ws = new WebSocket('ws://localhost:3000/api/v1/stt/stream');

ws.onopen = () => {
  console.log('Connected to STT stream');
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);

  switch (msg.type) {
    case 'ready':
      console.log('STT ready');
      startRecording();
      break;
    case 'segment':
      console.log(`[${msg.start_ms}-${msg.end_ms}] ${msg.text}`);
      appendTranscription(msg.text);
      break;
    case 'error':
      console.error('STT error:', msg.message);
      break;
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('Disconnected from STT stream');
};

// Send audio data
function sendAudioChunk(float32Array) {
  const buffer = float32Array.buffer;
  ws.send(buffer);
}

// Load a model
function loadModel(modelName) {
  ws.send(JSON.stringify({
    type: 'load_model',
    model: modelName
  }));
}
```

### TTS Streaming

```
WebSocket: ws://localhost:3000/api/v1/tts/stream
```

#### Connection Flow

1. Connect to WebSocket
2. Receive `ready` message
3. Send synthesis request as JSON
4. Receive `audio_start` metadata
5. Receive audio data as binary message
6. Receive `audio_end` completion
7. Repeat or close connection

#### Server Messages

**Ready:**
```json
{"type": "ready", "message": "TTS stream ready"}
```

**Audio Start:**
```json
{
  "type": "audio_start",
  "sample_rate": 24000,
  "duration_ms": 1500,
  "engine": "Kokoro"
}
```

**Audio Data:** Binary message with raw f32 PCM samples.

**Audio End:**
```json
{"type": "audio_end"}
```

**Error:**
```json
{
  "type": "error",
  "message": "TTS not initialized"
}
```

#### Client Messages

**Synthesize:**
```json
{
  "type": "synthesize",
  "text": "Hello, world!"
}
```

#### JavaScript Example

```javascript
const ws = new WebSocket('ws://localhost:3000/api/v1/tts/stream');

let audioContext;
let audioQueue = [];

ws.onopen = () => {
  console.log('Connected to TTS stream');
  audioContext = new AudioContext();
};

ws.onmessage = async (event) => {
  if (typeof event.data === 'string') {
    const msg = JSON.parse(event.data);

    switch (msg.type) {
      case 'ready':
        console.log('TTS ready');
        break;
      case 'audio_start':
        console.log(`Receiving audio: ${msg.duration_ms}ms at ${msg.sample_rate}Hz`);
        break;
      case 'audio_end':
        console.log('Audio complete');
        playQueuedAudio();
        break;
      case 'error':
        console.error('TTS error:', msg.message);
        break;
    }
  } else {
    // Binary audio data
    const arrayBuffer = await event.data.arrayBuffer();
    audioQueue.push(arrayBuffer);
  }
};

function synthesize(text) {
  audioQueue = [];
  ws.send(JSON.stringify({
    type: 'synthesize',
    text: text
  }));
}

async function playQueuedAudio() {
  const totalLength = audioQueue.reduce((sum, buf) => sum + buf.byteLength, 0);
  const combined = new Float32Array(totalLength / 4);

  let offset = 0;
  for (const buf of audioQueue) {
    const samples = new Float32Array(buf);
    combined.set(samples, offset);
    offset += samples.length;
  }

  const audioBuffer = audioContext.createBuffer(1, combined.length, 24000);
  audioBuffer.copyToChannel(combined, 0);

  const source = audioContext.createBufferSource();
  source.buffer = audioBuffer;
  source.connect(audioContext.destination);
  source.start();
}
```

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `BAD_REQUEST` | 400 | Invalid request format or parameters |
| `NOT_FOUND` | 404 | Resource not found (model, session, etc.) |
| `INTERNAL_ERROR` | 500 | Server-side error |
| `SERVICE_UNAVAILABLE` | 503 | Feature not enabled or resource not ready |

## Audio Format for API

All audio data must be:

- **Sample Rate:** 16,000 Hz
- **Channels:** 1 (mono)
- **Format:** 32-bit float PCM
- **Encoding:** For REST API, Base64 encode the raw bytes

### Preparing Audio (JavaScript)

```javascript
async function prepareAudioForAPI(audioFile) {
  const audioContext = new AudioContext({ sampleRate: 16000 });
  const arrayBuffer = await audioFile.arrayBuffer();
  const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

  // Get mono channel
  const channelData = audioBuffer.getChannelData(0);

  // Resample if needed
  const targetLength = Math.floor(channelData.length * 16000 / audioBuffer.sampleRate);
  const resampled = new Float32Array(targetLength);

  for (let i = 0; i < targetLength; i++) {
    const srcIndex = Math.floor(i * audioBuffer.sampleRate / 16000);
    resampled[i] = channelData[Math.min(srcIndex, channelData.length - 1)];
  }

  // Convert to bytes and base64
  const bytes = new Uint8Array(resampled.buffer);
  const base64 = btoa(String.fromCharCode(...bytes));

  return base64;
}

// Usage
const audioBase64 = await prepareAudioForAPI(file);
const response = await fetch('/api/v1/transcribe', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    audio: audioBase64,
    language: 'en'
  })
});
```

### Preparing Audio (Python)

```python
import base64
import numpy as np
from scipy.io import wavfile
from scipy.signal import resample

def prepare_audio_for_api(filepath: str) -> str:
    # Read audio file
    sample_rate, audio = wavfile.read(filepath)

    # Convert to float32 (-1.0 to 1.0)
    if audio.dtype == np.int16:
        audio = audio.astype(np.float32) / 32768.0
    elif audio.dtype == np.int32:
        audio = audio.astype(np.float32) / 2147483648.0

    # Convert to mono if stereo
    if len(audio.shape) > 1:
        audio = np.mean(audio, axis=1)

    # Resample to 16kHz
    if sample_rate != 16000:
        num_samples = int(len(audio) * 16000 / sample_rate)
        audio = resample(audio, num_samples)

    # Encode to base64
    audio_bytes = audio.astype(np.float32).tobytes()
    return base64.b64encode(audio_bytes).decode('utf-8')

# Usage
import requests

audio_base64 = prepare_audio_for_api('recording.wav')
response = requests.post('http://localhost:3000/api/v1/transcribe', json={
    'audio': audio_base64,
    'language': 'en'
})
result = response.json()
print(result['segments'])
```

## Rate Limiting

The server includes per-IP rate limiting (enabled by default).

### Default Limits

| Endpoint Type | Requests/Minute |
|---------------|-----------------|
| General (models, sessions, health) | 100 |
| Heavy (transcribe, TTS synthesis) | 20 |
| WebSocket connections | 10 |

### Rate Limit Headers

Responses include rate limit information:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 30
```

Note: `X-RateLimit-Reset` contains **seconds until reset**, not a Unix timestamp.

### Rate Limit Exceeded

**429 Too Many Requests:**
```json
{
  "error": "Rate limit exceeded",
  "code": "RATE_LIMIT_EXCEEDED",
  "retry_after_secs": 30
}
```

The response also includes a `Retry-After` HTTP header with the same value.

### Configuration

```rust
// Programmatic configuration
let config = ServerConfig::default()
    .with_rate_limit(RateLimitConfig {
        enabled: true,
        default_rpm: 200,    // General endpoints
        heavy_rpm: 50,       // Transcribe/TTS
        websocket_rpm: 20,   // WebSocket connections
        burst_multiplier: 2.0,
        ..Default::default()
    });

// Disable rate limiting
let config = ServerConfig::default().without_rate_limit();
```

## CORS

CORS is enabled by default in permissive mode (allows all origins). For production:

```rust
// Strict CORS
let config = ServerConfig::default().with_strict_cors();
```

Permissive mode allows:
- Any origin (`*`)
- Any method
- Any headers
