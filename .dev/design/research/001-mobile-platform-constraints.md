# Research: Mobile Platform Constraints

**ID:** 001
**Status:** Complete
**Impact:** HIGH
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Adapted for:** Rust implementation

---

## Overview

This research addresses the feasibility of deploying a Rust-based STT/TTS library with native FFI on iOS and Android platforms.

---

## Key Findings

### iOS Integration

**Status: FEASIBLE**

Rust compiles to static libraries with C ABI, providing first-class Xcode/Swift integration:

1. **Build process:**
   - Compile with `cargo build --target aarch64-apple-ios`
   - Use `cbindgen` to generate C headers
   - Link static library into Swift via bridging header or Swift Package

2. **No CGO complexity:**
   - Unlike Go's gomobile/CGO issues, Rust's C ABI is straightforward
   - No subprocess spawning needed—library is linked directly into the app

3. **whisper.cpp iOS native support:**
   - [SwiftWhisper](https://swiftpackageindex.com/exPHAT/SwiftWhisper) - native Swift wrapper
   - [whisper.spm](https://github.com/ggerganov/whisper.spm) - official Swift Package Manager support
   - whisper-rs wraps the same whisper.cpp, so iOS optimizations (Metal, Core ML) are available

---

### Android Integration

**Status: FEASIBLE**

1. **NDK cross-compilation:**
   - Rust cross-compiles to Android NDK targets (aarch64-linux-android, armv7-linux-androideabi)
   - JNI bindings via `jni` crate or C ABI with Kotlin/Java native calls
   - NDK 25+ recommended for whisper.cpp (C++17 support, ARM optimizations)

2. **Android 2025 requirements:**
   - Android 16KB page size: add `-C link-arg=-Wl,-z,max-page-size=16384` to rustflags
   - Target API 21+ recommended for NDK 28 compatibility

3. **whisper.cpp Android:**
   - ARM Neon SIMD optimizations available
   - Recommended flags: `-march=armv8.2-a+fp16` for fp16_va support

---

### Mobile Memory Constraints

**Status: CONSTRAINED - Model selection critical**

1. **iOS memory limit:** ~2GB hard limit for apps

2. **Runtime memory requirements (not just file size):**

   | Model | File Size | Context Memory (Total ~2x) | Mobile Viable? |
   |-------|-----------|----------------------------|----------------|
   | tiny/tiny.en | ~40MB | ~125MB (~250MB total) | Yes |
   | base/base.en | ~140MB | **~506MB** (~1GB total) | Marginal |
   | small | ~460MB | ~1GB+ | Risky |
   | medium | ~1.5GB | **~2610MB** | No |
   | large-v3 | ~3GB | ~3336MB+ | No |

3. **Quantization significantly helps:**
   - Q5_0 base.en: ~26MB file (reduced from ~140MB)
   - 19% latency reduction, 45% size reduction, preserved accuracy
   - Quantized models require less runtime memory

4. **Recommended mobile models:**
   - **Primary:** `ggml-base-q5_1.bin` (~26MB file, good accuracy)
   - **Low-resource:** `ggml-tiny.en-q5_1.bin` (English only)
   - Avoid medium/large on mobile entirely

---

### Background Audio Processing

**Status: FEASIBLE WITH PLATFORM-SPECIFIC IMPLEMENTATION**

#### iOS Background Audio

1. **Possible with proper entitlements:**
   - Enable "Audio, AirPlay, and Picture in Picture" background mode
   - Use `AVAudioSession.Category.playAndRecord`
   - Audio continues when app backgrounded or device locked

2. **AVAudioSession configuration required:**
   - Category determines eligible audio routes
   - `.playAndRecord` enables simultaneous input/output
   - Route changes require handling via `routeChangeNotification`

3. **Limitations:**
   - System may interrupt audio for phone calls, alarms
   - Battery/thermal throttling may affect long sessions

#### Android Background Audio

1. **Requires foreground service:**
   ```xml
   <service android:foregroundServiceType="microphone" />
   <uses-permission android:name="android.permission.FOREGROUND_SERVICE_MICROPHONE" />
   ```

2. **Persistent notification mandatory:**
   - User must see notification while recording
   - Can record indefinitely while service alive
   - Service dying means losing recording capability

3. **Android 14+ requirements:**
   - System enforces capability checks for foreground service types
   - Must have microphone permission granted before starting service
   - Service must be started while app is in foreground

---

## Recommendations

### R1: Model Strategy for Mobile

| Device Tier | Model | Size | Notes |
|-------------|-------|------|-------|
| Low-end (2GB RAM) | tiny.en q5_1 | ~15MB | English only |
| Mid-range (4GB RAM) | base q5_1 | ~26MB | Multilingual |
| High-end (6GB+ RAM) | base | ~140MB | Best accuracy for mobile |

Never deploy medium/large models on mobile (iOS 2GB hard limit).

### R2: Platform Wrapper Strategy

**iOS:**
- Use `cbindgen` to generate C headers from Rust
- Create Swift Package wrapping the static library
- Leverage Metal/Core ML acceleration via whisper.cpp

**Android:**
- Use `jni` crate for JNI bindings or C ABI with Kotlin
- Cross-compile with Android NDK toolchain
- Include arm64-v8a (primary) and armeabi-v7a (legacy support)

---

## References

- whisper.cpp: https://github.com/ggerganov/whisper.cpp
- whisper-rs: https://github.com/tazz4843/whisper-rs
- SwiftWhisper: https://github.com/exPHAT/SwiftWhisper
- whisper.spm: https://github.com/ggerganov/whisper.spm
- Apple Background Execution: https://developer.apple.com/documentation/backgroundtasks
- Android Foreground Services: https://developer.android.com/develop/background-work/services/fgs/declare
- jni crate: https://crates.io/crates/jni
- cbindgen: https://github.com/mozilla/cbindgen

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial Go-based research |
| 2026-01-19 | Adapted for Rust implementation; removed Go-specific content |
