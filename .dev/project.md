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

## Dependencies

    - sherpa-onnx
    - SQLite

## Constraints

    - ability to install and deploy on Android, iOS, Windows, Mac and Linux

## Feasibility Considerations

### Mobile Platform Constraints (HIGH IMPACT)

- **iOS CGO restrictions**: Does Go's CGO (required for whisper.cpp bindings) work reliably on iOS? What are the build toolchain requirements and App Store compliance implications?
- **Android native library loading**: How do whisper.cpp Go bindings behave when cross-compiled for Android ARM architectures? What's the NDK integration story?
- **Mobile memory limits**: Whisper models range from ~40MB (tiny) to ~3GB (large). What models are viable on mobile devices with typical 2-6GB RAM constraints while OS and apps compete for memory?
- **Background audio processing**: iOS and Android have strict background execution limits. Can real-time STT operate reliably when app is backgrounded?

### whisper.cpp Go Bindings Maturity (HIGH IMPACT)

- **Binding completeness**: Do the official Go bindings expose all features needed (streaming, voice activity detection, language detection)? What's missing?
- **Streaming/real-time support**: Does the binding support incremental audio processing, or only batch file processing? Real-time STT requires chunked streaming.
- **Thread safety**: Are the bindings safe for concurrent use across multiple client connections?
- **Build complexity**: whisper.cpp requires BLAS/OpenBLAS/Accelerate/CUDA. What's the cross-platform build matrix complexity?

### Translation & TTS Components (MEDIUM-HIGH IMPACT)

- **Translation engine selection**: Project mentions translation but no engine specified. Options (LibreTranslate, Argos, cloud APIs) have vastly different offline/size/quality tradeoffs. Which is viable for local deployment?
- **TTS engine selection**: No TTS dependency specified. Piper, eSpeak, Coqui have different quality/size/platform support profiles. What's the target?
- **Component coherence**: Are there proven Go bindings or integration patterns for chosen translation/TTS engines, or does this require additional CGO work?

### Real-Time Performance (MEDIUM IMPACT)

- **Latency requirements**: What's the acceptable STT latency for "real-time"? Whisper has inherent 30-second chunk processing. Is word-level streaming needed?
- **Hardware acceleration**: whisper.cpp supports CUDA, Metal, OpenCL. What's the Go binding story for GPU acceleration on each target platform?
- **Concurrent client capacity**: For N simultaneous clients, what's the compute/memory scaling? Model loading per-client vs shared?

### Cross-Platform Build & Distribution (MEDIUM IMPACT)

- **Mobile app wrapper strategy**: Go produces native binaries. What wraps them for iOS (Swift/ObjC) and Android (Kotlin/Java)? gomobile? Manual FFI?
- **App store distribution**: iOS requires signed IPAs, Android requires signed APKs/AABs. What's the CI/CD pipeline for 5 platforms?
- **Update mechanism**: How do model updates and server updates reach deployed mobile instances?

### Unspecified Architecture Questions

- **Client-server split**: Is this a local server that mobile apps connect to on localhost, or embedded directly in mobile apps, or a remote server mobile apps call? Architecture implications differ vastly.
- **Offline vs connected**: Must all processing work fully offline, or is cloud fallback acceptable?
- **Multi-language model strategy**: Whisper multilingual models are larger. How are language-specific models managed/downloaded?
