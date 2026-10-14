# Lexicon Local Server - User Manual

Lexicon is a local speech-to-text (STT), text-to-speech (TTS), and translation server. It runs entirely on your machine with no cloud dependencies, providing privacy-focused speech processing via CLI tools and HTTP/WebSocket APIs.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [CLI Commands](#cli-commands)
4. [HTTP Server](#http-server)
5. [WebSocket Streaming](#websocket-streaming)
6. [Configuration](#configuration)
7. [Supported Models](#supported-models)
8. [Troubleshooting](#troubleshooting)

---

## Installation

### Building from Source

Requirements:
- Rust 1.85 or later
- CMake (for whisper-rs compilation)
- ONNX Runtime (for Kokoro TTS)

```bash
# Clone the repository
git clone https://github.com/varianceenjoyer/Lexicon-Local-Server
cd Lexicon-Local-Server

# Build in release mode
cargo build --release

# The binary is at target/release/lexicon
```

### Verify Installation

```bash
./target/release/lexicon --version
./target/release/lexicon --help
```

---

## Quick Start

### 1. Download a Model

Before transcribing audio, download a Whisper model:

```bash
# Download the base model (142 MB, good balance of speed/accuracy)
lexicon models download base

# Or download a smaller/faster model
lexicon models download tiny

# Or a larger/more accurate model
lexicon models download medium
```

### 2. Transcribe an Audio File

```bash
# Basic transcription (outputs SRT subtitles)
lexicon transcribe recording.wav

# Output as plain text
lexicon transcribe recording.wav --format txt

# Save to file
lexicon transcribe recording.wav --output transcript.srt

# Use a specific model
lexicon transcribe recording.wav --model medium

# Specify language (skip auto-detection)
lexicon transcribe recording.wav --language en

# Translate non-English audio to English
lexicon transcribe spanish_audio.wav --translate
```

### 3. Start the HTTP Server

```bash
# Start server on default port (8765)
lexicon serve

# Specify host and port
lexicon serve --host 0.0.0.0 --port 9000
```

---

## CLI Commands

### `lexicon transcribe`

Transcribe audio files to text.

```
USAGE:
    lexicon transcribe [OPTIONS] <FILES>...

ARGUMENTS:
    <FILES>...    Audio file(s) to transcribe (WAV, PCM, RAW)

OPTIONS:
    -f, --format <FORMAT>      Output format: srt, txt, json [default: srt]
    -o, --output <FILE>        Output file path (stdout if not specified)
    -m, --model <MODEL>        Model to use: tiny, base, small, medium, large [default: base]
    -l, --language <LANG>      Language code (e.g., en, es, fr). Auto-detect if not specified
        --translate            Translate non-English audio to English
    -v, --verbose              Increase logging verbosity (use -vv for debug)
```

**Examples:**

```bash
# Transcribe multiple files
lexicon transcribe meeting1.wav meeting2.wav --output all_meetings.srt

# JSON output with timestamps and confidence scores
lexicon transcribe interview.wav --format json --output interview.json

# Verbose mode to see progress details
lexicon transcribe -vv long_recording.wav
```

### `lexicon serve`

Start the HTTP/WebSocket server.

```
USAGE:
    lexicon serve [OPTIONS]

OPTIONS:
        --host <HOST>    Host to bind to [default: 127.0.0.1]
    -p, --port <PORT>    Port to listen on [default: 8765]
    -v, --verbose        Increase logging verbosity
```

**Examples:**

```bash
# Start on localhost only
lexicon serve

# Allow connections from other machines
lexicon serve --host 0.0.0.0

# Custom port
lexicon serve --port 3000
```

### `lexicon models`

Manage speech recognition models.

```
SUBCOMMANDS:
    list        List available models
    download    Download a model
    delete      Delete a cached model
    info        Show model information
```

**Examples:**

```bash
# List all models and their status
lexicon models list

# List only downloaded models
lexicon models list --downloaded

# Download the small model
lexicon models download small

# Show model details
lexicon models info base

# Delete a model (with confirmation)
lexicon models delete large

# Force delete without confirmation
lexicon models delete large --force
```

### `lexicon sessions`

Manage transcription sessions stored in the database.

```
SUBCOMMANDS:
    list      List all sessions
    show      Show session details
    delete    Delete a session
```

**Examples:**

```bash
# List all sessions
lexicon sessions list

# Show details of session 42
lexicon sessions show 42

# Delete session 42
lexicon sessions delete 42
```

### `lexicon export`

Export a session to a file.

```
USAGE:
    lexicon export <SESSION_ID> [OPTIONS]

ARGUMENTS:
    <SESSION_ID>    Session ID to export

OPTIONS:
    -f, --format <FORMAT>    Output format: srt, txt, json [default: srt]
    -o, --output <FILE>      Output file path (stdout if not specified)
```

**Examples:**

```bash
# Export as SRT subtitles
lexicon export 42 --format srt --output transcript.srt

# Export as plain text
lexicon export 42 --format txt

# Export as JSON with full metadata
lexicon export 42 --format json --output session_42.json
```

### `lexicon tui`

Launch the diagnostic TUI for real-time transcription with visual audio level meter.

```
USAGE:
    lexicon tui [OPTIONS]

OPTIONS:
        --device <DEVICE>           Pre-select audio device by name
        --no-mouse                  Disable mouse support
        --model <MODEL>             Path to Whisper model file
        --language <LANGUAGE>       Language code (e.g., "en", "auto") [default: en]
        --translate                 Translate non-English speech to English
        --simulated-audio <PATH>    Path to audio file for simulated input
        --simulated-loop            Loop simulated audio playback continuously
```

**Examples:**

```bash
# Launch TUI with default settings
lexicon tui

# Pre-select a specific microphone
lexicon tui --device "Blue Yeti"

# Use a specific model
lexicon tui --model ~/.local/share/lexicon/models/whisper-small.bin

# Test with simulated audio (useful for headless environments)
lexicon tui --simulated-audio test_audio.wav --simulated-loop
```

**TUI Keyboard Shortcuts:**

| Key | Action |
|-----|--------|
| Ctrl+T | Toggle recording |
| Ctrl+S | Save transcription to database |
| Ctrl+L | List saved transcriptions |
| Tab | Switch between panels |
| q / Ctrl+C | Quit |

### `lexicon config`

Manage configuration and system profiling.

```
SUBCOMMANDS:
    show       Show current configuration
    set        Set a configuration value
    reset      Reset configuration to defaults
    profile    Profile system and show hardware information
    recommend  Show recommended model based on system profile
    path       Show configuration file path
```

**Examples:**

```bash
# Show current configuration
lexicon config show

# Profile your system
lexicon config profile

# Get model recommendation based on your hardware
lexicon config recommend

# Show configuration file path
lexicon config path

# Set default model
lexicon config set stt.default_model whisper-base-q5_1
```

---

## HTTP Server

When running `lexicon serve`, the following REST API endpoints are available:

### Health Check

```
GET /health
```

Returns server status and component availability.

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

### Models

#### List Models
```
GET /api/v1/models
```

Returns available Whisper models with download status.

**Response:**
```json
[
  {
    "id": "whisper-base",
    "name": "Whisper Base",
    "size_bytes": 142000000,
    "downloaded": true,
    "path": "/home/user/.local/share/lexicon/models/whisper-base.bin"
  }
]
```

#### Download Model
```
POST /api/v1/models/download
Content-Type: application/json

{
  "model_id": "whisper-small"
}
```

#### Get Model Info
```
GET /api/v1/models/:model_id
```

#### Delete Model
```
DELETE /api/v1/models/:model_id
```

### Transcription

#### Batch Transcribe
```
POST /api/v1/transcribe
Content-Type: application/json

{
  "audio": "<base64-encoded 16kHz mono f32 PCM>",
  "language": "en",
  "translate": false,
  "model": "base"
}
```

**Response:**
```json
{
  "segments": [
    {
      "start_ms": 0,
      "end_ms": 2500,
      "text": "Hello, world.",
      "confidence": 0.95
    }
  ],
  "detected_language": "en",
  "duration_ms": 5000,
  "processing_time_ms": 1200
}
```

### Sessions

#### List Sessions
```
GET /api/v1/sessions
```

#### Get Session
```
GET /api/v1/sessions/:id
```

#### Delete Session
```
DELETE /api/v1/sessions/:id
```

#### Export Session
```
GET /api/v1/sessions/:id/export?format=srt
```

Supported formats: `srt`, `json`

---

## WebSocket Streaming

### Real-time Speech-to-Text

Connect to: `ws://localhost:8765/api/v1/stt/stream`

#### Protocol

1. **Server sends ready message:**
```json
{"type": "ready", "message": "STT stream ready"}
```

2. **Client sends audio data:**
   - Binary frames containing 16kHz mono f32 PCM samples
   - Each frame should be ~100-500ms of audio

3. **Server sends transcription segments:**
```json
{
  "type": "segment",
  "start_ms": 1000,
  "end_ms": 2500,
  "text": "Hello world",
  "confidence": 0.92
}
```

4. **Client can load a model:**
```json
{"type": "load_model", "model": "small"}
```

5. **Server confirms model load:**
```json
{"type": "model_loaded", "model": "small"}
```

#### JavaScript Example

```javascript
const ws = new WebSocket('ws://localhost:8765/api/v1/stt/stream');

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === 'segment') {
    console.log(`[${msg.start_ms}-${msg.end_ms}] ${msg.text}`);
  }
};

// Send audio data (16kHz mono f32 PCM)
function sendAudio(float32Array) {
  ws.send(float32Array.buffer);
}
```

### Real-time Text-to-Speech

Connect to: `ws://localhost:8765/api/v1/tts/stream`

#### Protocol

1. **Server sends ready message:**
```json
{"type": "ready", "message": "TTS stream ready"}
```

2. **Client sends synthesis request:**
```json
{"type": "synthesize", "text": "Hello, how are you?"}
```

3. **Server sends audio metadata:**
```json
{
  "type": "audio_start",
  "sample_rate": 24000,
  "duration_ms": 1500,
  "engine": "Kokoro"
}
```

4. **Server sends audio data:**
   - Binary frame containing f32 PCM samples

5. **Server signals completion:**
```json
{"type": "audio_end"}
```

#### JavaScript Example

```javascript
const ws = new WebSocket('ws://localhost:8765/api/v1/tts/stream');
let audioContext = new AudioContext();

ws.onmessage = async (event) => {
  if (typeof event.data === 'string') {
    const msg = JSON.parse(event.data);
    if (msg.type === 'audio_start') {
      console.log(`Receiving ${msg.duration_ms}ms of audio at ${msg.sample_rate}Hz`);
    }
  } else {
    // Binary audio data
    const float32Array = new Float32Array(event.data);
    // Play the audio...
  }
};

// Request speech synthesis
ws.send(JSON.stringify({type: 'synthesize', text: 'Hello world'}));
```

---

## Configuration

### Configuration File

Lexicon looks for configuration in these locations (in order):
1. Path specified by `--config` flag
2. `$XDG_CONFIG_HOME/lexicon/config.toml`
3. `~/.config/lexicon/config.toml`

**Example `config.toml`:**

```toml
# Logging level: error, warn, info, debug, trace
log_level = "info"

[server]
host = "127.0.0.1"
port = 8765
cors_enabled = true

[stt]
default_model = "whisper-base-q5_1"
diarization_enabled = false
max_speakers = 4
gpu_enabled = true

[tts]
preferred_engine = "kokoro"  # or "platform"
fallback_enabled = true
```

### Environment Variables

Configuration can be overridden with environment variables prefixed with `LEXICON_`:

```bash
export LEXICON_LOG_LEVEL=debug
export LEXICON_SERVER_PORT=9000
export LEXICON_STT_DEFAULT_MODEL=whisper-small
```

### Data Directories

Lexicon stores data in platform-specific locations:

| Platform | Data Directory | Models Directory |
|----------|----------------|------------------|
| Linux    | `~/.local/share/lexicon` | `~/.local/share/lexicon/models` |
| macOS    | `~/Library/Application Support/lexicon` | Same + `/models` |
| Windows  | `%APPDATA%\lexicon` | Same + `\models` |

---

## Supported Models

### Whisper Models

| Model | Size | Parameters | Relative Speed | Use Case |
|-------|------|------------|----------------|----------|
| tiny | 75 MB | 39M | ~10x | Quick drafts, low-resource devices |
| base | 142 MB | 74M | ~7x | **Recommended default** |
| small | 466 MB | 244M | ~4x | Better accuracy |
| medium | 1.5 GB | 769M | ~2x | High accuracy |
| large | 2.9 GB | 1550M | ~1x | Best accuracy |

All models support 99 languages with automatic language detection.

### TTS Engines

| Engine | Quality | Languages | Requirements |
|--------|---------|-----------|--------------|
| Kokoro | High (neural) | English | ONNX Runtime |
| Platform | Medium | Many | OS-native TTS |

Kokoro is preferred for English; platform TTS is used as fallback for other languages or when Kokoro fails.

---

## Troubleshooting

### Model Not Found

```
Error: Model 'base' not found at /home/user/.local/share/lexicon/models/whisper-base.bin
Run 'lexicon models download base' to download it first.
```

**Solution:** Download the model with `lexicon models download base`

### Audio Format Not Supported

```
Error: Unsupported audio format: mp3. Supported formats: wav, pcm, raw
```

**Solution:** Convert your audio to WAV format:
```bash
ffmpeg -i input.mp3 -ar 16000 -ac 1 output.wav
```

### GPU Acceleration Not Working

If transcription is slower than expected, GPU acceleration may not be enabled.

**Linux (CUDA):**
- Ensure CUDA toolkit is installed
- Rebuild with CUDA support enabled

**macOS (Metal):**
- Metal acceleration is automatic on Apple Silicon
- Intel Macs may fall back to CPU

### Server Won't Start

```
Error: Address already in use
```

**Solution:** Another process is using port 8765. Either:
- Stop the other process
- Use a different port: `lexicon serve --port 9000`

### Database Errors

```
Error: Database not found
```

**Solution:** The database is created automatically when you first use sessions. Run a transcription through the server or ensure the data directory exists and is writable.

### WebSocket Connection Refused

Ensure the server is running and you're connecting to the correct URL:
- STT: `ws://localhost:8765/api/v1/stt/stream`
- TTS: `ws://localhost:8765/api/v1/tts/stream`

For HTTPS/WSS, you'll need to configure TLS certificates (not covered in this manual).

---

## License

MIT License - See LICENSE file for details.
