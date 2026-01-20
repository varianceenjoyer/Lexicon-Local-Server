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

## Windows Native Build

### Prerequisites (Windows)

Windows native builds require the following tools:

**Option 1: Using winget (recommended)**

```powershell
# Visual Studio Build Tools with C++ workload
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

# LLVM/Clang for bindgen
winget install LLVM.LLVM

# CMake for whisper-rs-sys
winget install Kitware.CMake

# Rust
winget install Rustlang.Rustup
```

**Option 2: Using Chocolatey**

```powershell
choco install visualstudio2022buildtools --package-parameters "--add Microsoft.VisualStudio.Workload.VCTools"
choco install llvm cmake
```

### Environment Variables (Windows)

Set these environment variables (add to your PowerShell profile or System Environment Variables):

```powershell
# Set LIBCLANG_PATH for bindgen
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"

# Ensure CMake is in PATH
$env:PATH += ";C:\Program Files\CMake\bin"
```

To make permanent, add to your PowerShell profile (`$PROFILE`):

```powershell
notepad $PROFILE
# Add the environment variable lines above
```

### Building on Windows

```powershell
# Build entire workspace
cargo build --workspace --all-features

# Build release version
cargo build --workspace --all-features --release

# Run tests
cargo test --workspace --all-features
```

### Windows-Specific Troubleshooting

#### "Unable to find libclang"

1. Verify LLVM is installed: `where clang`
2. Set LIBCLANG_PATH: `$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"`
3. Restart your terminal

#### "cmake not found"

1. Install CMake: `winget install Kitware.CMake`
2. Add to PATH: `$env:PATH += ";C:\Program Files\CMake\bin"`
3. Restart your terminal

#### "link.exe not found" / MSVC errors

Install Visual Studio Build Tools with C++ workload:

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Then open a new "Developer PowerShell for VS 2022" terminal.

#### Long path issues

Enable long paths in Windows:

```powershell
# Run as Administrator
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

#### Windows Defender false positives

During development, Windows Defender may flag the build directory. Add an exclusion:

```powershell
# Run as Administrator
Add-MpPreference -ExclusionPath "C:\path\to\Lexicon-Local-Server\target"
```

## macOS Build

### Prerequisites (macOS)

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew dependencies
brew install cmake llvm

# Set LLVM path for bindgen
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
```

### Building on macOS

```bash
cargo build --workspace --all-features
cargo test --workspace --all-features
```

### macOS-Specific Notes

- **Metal acceleration**: Automatically detected and used for GPU inference
- **Apple Silicon**: Native arm64 builds are fully supported
- **x86 Macs**: Rosetta 2 emulation works but native x86 builds are recommended

## Cross-Platform Build Scripts

Build scripts are available in the `scripts/` directory:

| Script | Platform | Purpose |
|--------|----------|---------|
| `build.sh` | Linux/macOS | Build with all features |
| `build-windows.ps1` | Windows | Build with all features |
| `test.sh` | Linux/macOS | Run full test suite |
| `test-windows.ps1` | Windows | Run full test suite |

### Using Build Scripts

**Linux/macOS:**
```bash
./scripts/build.sh        # Build
./scripts/test.sh         # Test
./scripts/build.sh release # Release build
```

**Windows:**
```powershell
.\scripts\build-windows.ps1        # Build
.\scripts\test-windows.ps1         # Test
.\scripts\build-windows.ps1 -Release # Release build
```
