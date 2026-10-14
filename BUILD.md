# Building Lexicon Local Server

This document provides comprehensive build instructions for all supported platforms.

## Prerequisites

### All Platforms

- **Rust toolchain**: 1.85+ (edition 2024)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update stable
  ```

- **Git**: For cloning the repository

### Platform-Specific Dependencies

#### macOS

```bash
# Install Xcode command line tools
xcode-select --install

# Install Homebrew dependencies
brew install cmake llvm pkg-config

# For iOS builds
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

#### Linux (Ubuntu/Debian)

```bash
# Build essentials
sudo apt update
sudo apt install -y build-essential cmake pkg-config

# Audio dependencies (for cpal)
sudo apt install -y libasound2-dev

# SSL for downloads
sudo apt install -y libssl-dev
```

#### Windows

**Option 1: Visual Studio Build Tools (Recommended)**

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. Select "Desktop development with C++" workload
3. Ensure Windows 10/11 SDK is selected

**Required for all Windows builds:**

```powershell
# Install LLVM (for bindgen)
winget install LLVM.LLVM

# Set environment variable
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
```

---

## Building

### Debug Build (Development)

```bash
# Clone repository
git clone https://github.com/varianceenjoyer/Lexicon-Local-Server.git
cd Lexicon-Local-Server

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace
```

### Release Build

```bash
cargo build --workspace --release
```

Binaries are output to `target/release/`:
- `lexicon` - CLI tool
- `lexicon-server` - HTTP/WebSocket server

### Feature Flags

```bash
# Core only (no audio, no inference)
cargo build -p lexicon-core --no-default-features

# STT only
cargo build -p lexicon-core --no-default-features --features stt

# Full build (default)
cargo build -p lexicon-core --features "stt,tts,storage,audio"
```

---

## Platform-Specific Builds

### macOS Universal Binary

```bash
# Add targets
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Build for each architecture
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Create universal binary
lipo -create \
  target/aarch64-apple-darwin/release/lexicon \
  target/x86_64-apple-darwin/release/lexicon \
  -output target/release/lexicon-universal
```

### iOS

```bash
# Install iOS targets
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

# Build static library for device
cargo build -p lexicon-ffi --release --target aarch64-apple-ios

# Build for simulator
cargo build -p lexicon-ffi --release --target aarch64-apple-ios-sim
```

### Android

```bash
# Install Android NDK and set environment
export ANDROID_NDK_HOME=/path/to/android-ndk

# Install targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# Install cargo-ndk
cargo install cargo-ndk

# Build
cargo ndk -t arm64-v8a -o ./bindings/android/jniLibs build -p lexicon-ffi --release
```

### Windows Native

```powershell
# Open Developer Command Prompt or set environment
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
cargo build --workspace --release
```

---

## GPU Acceleration

### CUDA (NVIDIA GPUs)

```bash
export CUDA_PATH=/usr/local/cuda
cargo build --release --features cuda
```

### Metal (Apple GPUs)

Metal support is enabled by default on Apple platforms.

---

## Running Tests

```bash
# All tests
cargo test --workspace

# With logging
RUST_LOG=debug cargo test --workspace -- --nocapture

# Specific test
cargo test test_database_lifecycle
```

---

## Troubleshooting

### `linker 'cc' not found`

```bash
# Linux
sudo apt install build-essential

# macOS
xcode-select --install
```

### `whisper-rs-sys` build failure

Ensure CMake is installed:
```bash
# macOS
brew install cmake

# Ubuntu
sudo apt install cmake

# Windows
winget install Kitware.CMake
```

### `clang-c/Index.h` not found

```bash
# macOS
brew install llvm
export LIBCLANG_PATH=$(brew --prefix llvm)/lib

# Linux
sudo apt install libclang-dev
```

---

## See Also

- [README.md](README.md) - Project overview
- [MANUAL.md](MANUAL.md) - User documentation
