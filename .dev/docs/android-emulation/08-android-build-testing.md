# Android Build Testing with Cuttlefish

> **Status:** Verified
>
> **Last Updated:** 2026-01-25
>
> **Purpose:** Document the Android build process and emulator testing workflow.

---

## Overview

This document describes how to build Lexicon FFI for Android and test it on the Cuttlefish emulator.

---

## Prerequisites

1. **Android NDK r26d** installed at `/home/dev/android-ndk-r26d`
2. **cargo-ndk** installed: `cargo install cargo-ndk`
3. **Rust Android targets** installed:
   ```bash
   rustup target add aarch64-linux-android x86_64-linux-android
   ```
4. **Cuttlefish emulator** running (see [03-running-cuttlefish.md](03-running-cuttlefish.md))

---

## Building for Android

### Quick Build (Debug)

```bash
# x86_64 for Cuttlefish emulator
bash scripts/android/build-android.sh x86_64

# ARM64 for real devices
bash scripts/android/build-android.sh arm64
```

### Release Build

```bash
bash scripts/android/build-android.sh x86_64 --release
bash scripts/android/build-android.sh arm64 --release
```

### Build Output

```
target/x86_64-linux-android/release/liblexicon_ffi.so  # ~3.4 MB
target/aarch64-linux-android/release/liblexicon_ffi.so # ~2.9 MB
```

---

## Current Build Status

| Feature | Status | Notes |
|---------|--------|-------|
| Core FFI | Working | Initialization, context management, platform detection |
| STT (whisper-rs) | **Working** | Requires patched whisper-rs-sys (see below) |
| TTS (ort) | Blocked | ort-sys has no Android x86_64 pre-built binaries |

### whisper-rs-sys Patch

The STT feature requires a patch to whisper-rs-sys to pass `ANDROID_*` env vars to CMake.

Patch location: `~/.cargo/registry/src/.../whisper-rs-sys-0.13.1/build.rs`

```rust
// Change from:
let is_cmake_flag = key.starts_with("CMAKE_");
if is_whisper_flag || is_cmake_flag {

// Change to:
let is_cmake_flag = key.starts_with("CMAKE_");
let is_android_flag = key.starts_with("ANDROID_");
if is_whisper_flag || is_cmake_flag || is_android_flag {
```

### STT Build Requirements

STT builds require `libc++_shared.so` from the NDK to be bundled:
```
$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/<arch>/libc++_shared.so
```

### Build Sizes (Release)

| Target | Core Only | With STT |
|--------|-----------|----------|
| arm64-v8a | 2.9 MB | 3.7 MB |
| x86_64 | 3.4 MB | 4.3 MB |

---

## Testing on Cuttlefish

### 1. Verify Emulator is Running

```bash
adb devices
# Should show: 0.0.0.0:6520    device

adb shell getprop ro.build.version.release
# Should show: 13 (or your API version)
```

### 2. Push Library to Emulator

```bash
adb push target/x86_64-linux-android/release/liblexicon_ffi.so /data/local/tmp/
```

### 3. Verify Library Loads

```bash
# Check library dependencies
adb shell "readelf -d /data/local/tmp/liblexicon_ffi.so | head -10"

# Should only depend on: libdl.so, libm.so, libc.so
```

### 4. Run Native Tests

Use the NDK to compile and run test binaries:

```bash
# Compile test
/home/dev/android-ndk-r26d/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android26-clang \
    -o /tmp/test_lexicon test_lexicon.c -ldl

# Push and run
adb push /tmp/test_lexicon /data/local/tmp/
adb shell "chmod +x /data/local/tmp/test_lexicon && /data/local/tmp/test_lexicon"
```

---

## Verified Functionality

The following FFI functions have been verified on Cuttlefish:

| Function | Status | Notes |
|----------|--------|-------|
| `get_version()` | Working | Returns "0.1.0" |
| `init_lexicon(data_dir)` | Working | Creates directories |
| `LexiconContext::new(config)` | Working | Creates context |
| `LexiconContext::get_platform_info()` | Working | Returns AndroidArm64, MobileMid tier |
| `LexiconContext::list_models()` | Working | Returns model list |
| `LexiconContext::is_model_downloaded(id)` | Working | Checks model files |
| `Transcriber::new()` | Working | Creates transcriber instance |
| `Transcriber::load_model()` | Working | Loads Whisper model (~3s for base) |
| `Transcriber::transcribe()` | Working* | *Slow on emulator, too slow to complete |
| `Synthesizer::new()` | Working | Creates stub (TTS disabled) |

### Transcription Performance Note

Whisper inference on the Cuttlefish emulator is extremely slow (CPU-only, emulated x86_64):
- Model loading: ~3 seconds
- Inference: >5 minutes for 3.7s audio (did not complete)

On a real ARM64 device with NEON optimizations, performance would be much better.
Consider using whisper-tiny model (~75MB) for testing.

---

## Android Project Structure

The Android project is located at `android/lexicon-android/`:

```
android/
├── settings.gradle.kts
├── build.gradle.kts
├── gradle.properties
└── lexicon-android/
    ├── build.gradle.kts
    ├── proguard-rules.pro
    ├── consumer-rules.pro
    └── src/main/
        ├── AndroidManifest.xml
        ├── jniLibs/
        │   ├── arm64-v8a/liblexicon_ffi.so
        │   └── x86_64/liblexicon_ffi.so
        └── kotlin/
            ├── com/lexicon/core/Lexicon.kt
            └── uniffi/lexicon/lexicon.kt
```

---

## Next Steps

1. ~~**Resolve whisper-rs-sys cross-compilation**~~ - DONE (patched)

2. **Resolve ort-sys Android builds** (for TTS feature)
   - Option A: Build ONNX Runtime for Android from source
   - Option B: Use pre-built Android ONNX Runtime AAR

3. **Complete Android project**
   - Add Gradle wrapper
   - Create example app with transcription demo
   - Add instrumentation tests

4. **Upstream whisper-rs-sys patch**
   - Submit PR to add ANDROID_* env var passthrough

---

## References

- [WP-ANDROID](../../work/WP-ANDROID.md) - Work package
- [02-cuttlefish-installation.md](02-cuttlefish-installation.md) - Cuttlefish setup
- [build-android.sh](../../../scripts/android/build-android.sh) - Build script
