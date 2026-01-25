# Cross-Compiling Lexicon TUI for Android

This guide explains how to cross-compile the Lexicon CLI/TUI for running directly on Android devices (Termux).

## Overview

The Lexicon TUI can be cross-compiled from Windows, Linux, or macOS to run natively on Android. The resulting binary runs in Termux, providing a full terminal-based transcription interface on your Android device.

## Prerequisites

### 1. Android NDK

Install Android NDK r26 or later:

**Option A: Via Android Studio (Recommended)**
1. Open Android Studio
2. Go to Settings → Appearance & Behavior → System Settings → Android SDK
3. Select "SDK Tools" tab
4. Check "NDK (Side by side)"
5. Click Apply

**Option B: Standalone Download**
1. Download from: https://developer.android.com/ndk/downloads
2. Extract to a location (e.g., `C:\android-ndk-r26d` on Windows)

### 2. Set Environment Variable

**Windows (PowerShell):**
```powershell
# Temporary (current session)
$env:ANDROID_NDK_HOME = "C:\Users\YourName\AppData\Local\Android\Sdk\ndk\26.1.10909125"

# Permanent (run as admin or via System Properties)
[Environment]::SetEnvironmentVariable("ANDROID_NDK_HOME", "C:\Users\YourName\AppData\Local\Android\Sdk\ndk\26.1.10909125", "User")
```

**Linux/macOS:**
```bash
# Add to ~/.bashrc or ~/.zshrc
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/26.1.10909125"
```

### 3. Rust Target

Install the Android target for Rust:

```bash
rustup target add aarch64-linux-android
```

### 4. Build Dependencies

The same dependencies as native builds:
- CMake (for whisper-rs)
- LLVM/Clang (for bindgen - set `LIBCLANG_PATH`)

## Building

### Windows (PowerShell)

```powershell
# Debug build
.\scripts\android\build-android-tui.ps1

# Release build (recommended for deployment)
.\scripts\android\build-android-tui.ps1 -Release

# Build and push to connected device
.\scripts\android\build-android-tui.ps1 -Release -Push

# Show all options
.\scripts\android\build-android-tui.ps1 -Help
```

### Linux/macOS (Bash)

```bash
# Make script executable
chmod +x scripts/android/build-android-tui.sh

# Debug build
./scripts/android/build-android-tui.sh

# Release build
./scripts/android/build-android-tui.sh --release

# Build and push to device
./scripts/android/build-android-tui.sh --release --push
```

## Output

The compiled binary is located at:
- Debug: `target/aarch64-linux-android/debug/lexicon`
- Release: `target/aarch64-linux-android/release/lexicon`

## Deploying to Device

### Via ADB

1. Enable USB debugging on your Android device
2. Connect via USB
3. Push the binary:

```bash
adb push target/aarch64-linux-android/release/lexicon /data/local/tmp/
```

### In Termux

1. Copy to Termux home directory:
```bash
cp /data/local/tmp/lexicon ~/
chmod +x ~/lexicon
```

2. Install audio dependencies (for microphone access):
```bash
pkg install pulseaudio
pulseaudio --start
```

3. Run the TUI:
```bash
./lexicon tui
```

## Features

The Android build includes:
- **STT (Speech-to-Text)**: Real-time transcription via Whisper
- **Storage**: SQLite database for saved transcriptions
- **Audio**: Microphone input via PulseAudio in Termux

The clipboard feature is disabled on Android as it's not supported in Termux.

## Troubleshooting

### "clang not found"

The Android NDK is not properly configured. Verify:
```bash
# Check NDK path exists
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/
```

### "libclang not found"

Set LIBCLANG_PATH to your LLVM installation:
```powershell
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
```

### Audio not working in Termux

Ensure PulseAudio is running:
```bash
pkg install pulseaudio
pulseaudio --start
pactl list short sinks
```

Grant Termux microphone permission in Android settings.

### Binary crashes on device

Check architecture matches:
```bash
# On device
uname -m  # Should be aarch64 for ARM64

# Check binary type
file target/aarch64-linux-android/release/lexicon
# Should show: ELF 64-bit LSB ... ARM aarch64
```

## Build Targets

| Target | Architecture | Use Case |
|--------|--------------|----------|
| arm64 (default) | aarch64-linux-android | Modern Android phones/tablets |
| x86_64 | x86_64-linux-android | Android emulators, some Chromebooks |
| arm32 | armv7-linux-androideabi | Older 32-bit devices |
| x86 | i686-linux-android | Older emulators |

## See Also

- [Termux Installation Guide](../../.dev/docs/android-termux/01-termux-installation.md)
- [Development Environment Setup](../../.dev/docs/android-termux/03-dev-environment.md)
- [Build and Deployment](../../.dev/docs/android-termux/04-build-deployment.md)
