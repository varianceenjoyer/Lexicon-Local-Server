---
# PROJECT CONFIGURATION
# Replace the placeholder below with your actual project name
# This value is referenced as [projectName] throughout the workflow
projectName: Lexicon Local Server
---

# [projectName]

## Project Mission

    Offers Lexicon clients a web interface for real-time STT, translation, and TTS.
    
## Project Description

    Configurable speech-to-text transcription, translation, and text-to-speech server.
    Provisions compute resources and enables compute intensive audio processing.
    Uses an SQLite database for configurable persistence of audio and/or transcription data.

    Implemented as a Rust core library exposing a C ABI, enabling:
    - Direct embedding into mobile apps (iOS/Android) via native FFI
    - Optional localhost HTTP server mode for desktop platforms
    - Consistent cross-platform behavior with platform-specific thin wrappers

## Implementation Language

    Rust

    Rationale:
    - C ABI compatibility for seamless iOS (Swift) and Android (Kotlin/JNI) integration
    - No garbage collector—deterministic memory management for real-time audio processing
    - Smaller binary size compared to Go (no runtime overhead)
    - Strong ecosystem for audio/ML (whisper-rs, ort for ONNX Runtime)
    - Memory safety without sacrificing performance

## Dependencies

    - whisper-rs (Rust bindings for whisper.cpp)
    - ort (ONNX Runtime for Rust) or sherpa-onnx bindings
    - SQLite (via rusqlite)
    - Platform-specific audio capture (cpal or platform APIs)

## Constraints

    - ability to install and deploy on Android, iOS, Windows, Mac and Linux

## Feasibility Considerations

### Mobile Platform Constraints (MEDIUM IMPACT - mitigated by Rust)

- **iOS integration**: Rust compiles to static libraries with C ABI, providing first-class Xcode/Swift integration. No subprocess spawning needed—library is linked directly into the app.
- **Android integration**: Rust cross-compiles to Android NDK targets. JNI bindings via `jni` crate or C ABI with Kotlin/Java native calls.
- **Mobile memory limits**: Whisper models range from ~40MB (tiny) to ~3GB (large). What models are viable on mobile devices with typical 2-6GB RAM constraints while OS and apps compete for memory?
- **Background audio processing**: iOS and Android have strict background execution limits. Can real-time STT operate reliably when app is backgrounded? (OS-level constraint)

### Whisper Integration (MEDIUM IMPACT)

- **whisper-rs maturity**: Evaluate whisper-rs bindings for feature completeness (streaming, VAD, language detection).
- **Streaming/real-time support**: Does whisper-rs support incremental audio processing? Real-time STT requires chunked streaming.
- **Build complexity**: whisper.cpp requires BLAS/OpenBLAS/Accelerate/Metal/CUDA. Rust build.rs can manage this, but cross-platform build matrix needs attention.

### Translation & TTS Components (MEDIUM-HIGH IMPACT)

- **Translation engine selection**: Project mentions translation but no engine specified. Options (LibreTranslate, Argos, cloud APIs) have vastly different offline/size/quality tradeoffs. Which is viable for local deployment?
- **TTS engine selection**: No TTS dependency specified. Piper, eSpeak, Coqui have different quality/size/platform support profiles. What's the target?
- **Rust ecosystem**: Evaluate Rust bindings or C FFI paths for chosen translation/TTS engines.

### Real-Time Performance (MEDIUM IMPACT)

- **Latency requirements**: What's the acceptable STT latency for "real-time"? Whisper has inherent 30-second chunk processing. Is word-level streaming needed?
- **Hardware acceleration**: whisper.cpp supports CUDA, Metal, OpenCL. Rust bindings should expose these. Verify Metal support for iOS/macOS.
- **Concurrent client capacity**: For embedded use (single client per app instance), this is less critical. Desktop server mode may need concurrency consideration.

### Cross-Platform Build & Distribution (MEDIUM IMPACT)

- **Build targets**: Rust supports all required targets (aarch64-apple-ios, aarch64-linux-android, x86_64-pc-windows-msvc, etc.). CI can use cargo + cross-compilation toolchains.
- **Platform wrappers**:
  - iOS: Swift package wrapping static library via C headers (cbindgen generates headers)
  - Android: JNI crate or C ABI with Kotlin
  - Desktop: Native binary with optional HTTP server (axum, actix-web)
- **App store distribution**: iOS requires signed IPAs, Android requires signed APKs/AABs. What's the CI/CD pipeline for 5 platforms?
- **Update mechanism**: How do model updates and library updates reach deployed mobile instances?

### Architecture Decisions (RESOLVED)

- **Client-server split**: Embedded architecture—core library is linked directly into each client app. On mobile, called via FFI. On desktop, can optionally expose localhost HTTP server for UI separation.
- **Offline vs connected**: Fully offline. All STT, translation, and TTS processing runs locally on-device with no cloud dependencies. This ensures privacy, eliminates network latency, and enables use in disconnected environments.
- **Multi-language model strategy**: Whisper multilingual models are larger. How are language-specific models managed/downloaded?
