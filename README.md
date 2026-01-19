# Lexicon Local Server

A local-first speech-to-text (STT), text-to-speech (TTS), and translation server. Runs entirely on your machine with no cloud dependencies.

## Features

- **Speech-to-Text**: Whisper-based transcription with support for 99 languages
- **Text-to-Speech**: Neural TTS (Kokoro) with platform fallback
- **Real-time Streaming**: WebSocket APIs for live audio processing
- **REST API**: HTTP endpoints for batch processing and management
- **CLI Tools**: Full-featured command-line interface
- **Privacy-First**: All processing happens locally

## Quick Start

```bash
# Build from source
cargo build --release

# Download a model
lexicon models download base

# Transcribe an audio file
lexicon transcribe recording.wav --output transcript.srt

# Start the HTTP server
lexicon serve
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `lexicon transcribe <file>` | Transcribe audio to text |
| `lexicon serve` | Start HTTP/WebSocket server |
| `lexicon models list` | List available models |
| `lexicon models download <id>` | Download a model |
| `lexicon sessions list` | List transcription sessions |
| `lexicon export <id>` | Export session to file |

## API Endpoints

When running `lexicon serve`:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/v1/models` | GET | List models |
| `/api/v1/transcribe` | POST | Batch transcription |
| `/api/v1/sessions` | GET | List sessions |
| `/api/v1/stt/stream` | WS | Real-time STT |
| `/api/v1/tts/stream` | WS | Real-time TTS |

## Documentation

See [MANUAL.md](MANUAL.md) for complete documentation including:
- Installation instructions
- CLI command reference
- HTTP API documentation
- WebSocket protocol details
- Configuration options
- Troubleshooting guide

## Project Structure

```
crates/
├── lexicon-core/    # Core library (STT, TTS, storage, models)
├── lexicon-cli/     # Command-line interface
└── lexicon-server/  # HTTP/WebSocket server
```

## Requirements

- Rust 1.85+
- CMake (for whisper-rs)
- ONNX Runtime (for Kokoro TTS)

## License

MIT License - See [LICENSE](LICENSE) for details.
