# Lexicon Local Server - Build Guide

## Prerequisites

### System Requirements

- **Operating System**: Ubuntu 24.04+ (tested), macOS, Windows with WSL2
- **Rust**: 1.85+ (required for edition = "2024")

### System Dependencies (Ubuntu/Debian)

Install the following packages before building:

```bash
# Build essentials
sudo apt-get update
sudo apt-get install -y build-essential pkg-config

# Audio capture (ALSA for cpal)
sudo apt-get install -y libasound2-dev

# Bindgen requirements (for whisper-rs-sys, speech-dispatcher-sys)
sudo apt-get install -y libclang-dev

# Speech dispatcher (for tts-rs fallback)
sudo apt-get install -y libspeechd-dev

# CMake (for whisper-rs-sys)
sudo apt-get install -y cmake
```

### Installing Rust

If Rust is not installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
```

Verify installation:

```bash
rustc --version  # Should be 1.85+
cargo --version
```

## Building the Project

### Layer 1 Only (Foundation)

Build with only storage and audio features (no STT/TTS engines):

```bash
cargo build -p lexicon-core --no-default-features --features "storage,audio"
```

### Full Build (All Features)

Build the entire workspace with all features:

```bash
cargo build --workspace --all-features
```

### Running Tests

```bash
cargo test --workspace --all-features
```

## Feature Flags

The `lexicon-core` crate supports the following feature flags:

| Feature   | Description                              | Dependencies              |
|-----------|------------------------------------------|---------------------------|
| `storage` | SQLite database with FTS5 search         | rusqlite                  |
| `audio`   | Audio capture, VAD, resampling           | cpal, webrtc-vad, rubato  |
| `stt`     | Speech-to-text (Whisper)                 | whisper-rs                |
| `tts`     | Text-to-speech (Kokoro + fallback)       | ort, tts                  |
| `server`  | HTTP server components                   | axum, tower               |
| `ffi`     | C FFI / UniFFI bindings                  | (future)                  |

Default features: `stt`, `tts`, `storage`, `audio`

## Dependency Summary

| System Package     | Crate          | Purpose                        |
|--------------------|----------------|--------------------------------|
| libasound2-dev     | cpal           | ALSA audio capture on Linux    |
| libclang-dev       | bindgen        | Generate FFI bindings          |
| libspeechd-dev     | tts            | Speech dispatcher fallback     |
| cmake              | whisper-rs-sys | Build whisper.cpp              |

## Troubleshooting

### "Unable to find libclang"

Install libclang development files:

```bash
sudo apt-get install -y libclang-dev
```

### "cmake not found"

```bash
sudo apt-get install -y cmake
```

### Audio device not found

Ensure ALSA is installed and configured:

```bash
sudo apt-get install -y libasound2-dev alsa-utils
arecord -l  # List audio devices
```

### whisper-rs-sys build fails

Ensure cmake and C++ compiler are installed:

```bash
sudo apt-get install -y cmake build-essential
```
