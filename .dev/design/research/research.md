# Research

## Status Summary (Updated 2026-01-23)

**Decision: Rust Implementation**

The project will be implemented using Rust libraries instead of Go. This decision was made based on:
- Superior FFI story for mobile platforms (C ABI, no CGO complexity)
- Better cross-compilation support
- Native whisper-rs bindings to whisper.cpp
- Strong ecosystem for audio processing

### Initial Feasibility (Complete)

| Plan | Status | Key Finding |
|------|--------|-------------|
| 001-mobile-platform-constraints | ✅ Complete | Rust + whisper-rs feasible on iOS/Android with memory constraints |
| 003-translation-tts-engines | ✅ Complete | Rust crates available for TTS (Piper via ONNX); translation via Whisper X→EN |
| 004-realtime-performance | ✅ Complete | RTF <1.0 achievable; target <800ms for conversational latency |
| 005-cross-platform-distribution | ✅ Complete | Rust cross-compilation straightforward; cargo-mobile2 for mobile |
| 006-architecture-decisions | ✅ Complete | Hybrid architecture: embedded Rust lib + optional remote server |

### Implementation Research (Complete)

| Task ID | Task | Category | Output | Status |
|---------|------|----------|--------|--------|
| R-SEC-01 | Model file verification patterns | Security | Best practices doc | ✅ Complete |
| R-WRS-01 | Benchmark whisper-rs (latency, memory, stability) | Performance | Benchmark report | ✅ Research Complete |
| R-ORT-01 | ort crate mobile builds (iOS/Android) | Feasibility | Build guide + gotchas | ✅ Complete |
| R-AUD-01 | cpal audio capture on mobile platforms | Feasibility | Integration patterns | ✅ Complete |
| R-THR-01 | Thermal behavior of sustained inference | Performance | Throttling thresholds | ✅ Complete |
| R-ERR-01 | Rust error handling for streaming pipelines | Architecture | Error handling guide | ✅ Complete |
| R-BLE-01 | Bluetooth audio latency and disconnection | Edge cases | Compatibility matrix | ✅ Complete |
| R-BLE-02 | Desktop HFP 1.6 codec negotiation | Audio | Implementation guide | ✅ Complete |
| R-TTS-01 | Alternative TTS engines (non-GPL) | Legal | TTS engine recommendation | ✅ Complete |
| R-LIC-01 | License audit (whisper-rs, ort, cpal, Piper) | Legal | License compatibility report | ✅ Complete |
| R-UX-01 | Offline-first UX for model management | Design | UX recommendations | ✅ Complete |
| R-DB-01 | SQLite schema for transcription storage | Architecture | Schema design doc | ✅ Complete |
| R-DIA-01 | Speaker diarization algorithms | Algorithm | Upgrade path doc | ✅ Complete |
| R-SYS-01 | System profiling for model selection | Configuration | Implementation guide | ✅ Complete |

---

## Ongoing

*All implementation research complete!*

## Complete

- [x] 001 - Mobile Platform Constraints (Adapted for Rust)
- [x] 003 - Translation & TTS Engine Selection (Adapted for Rust)
- [x] 004 - Real-Time Performance (Adapted for Rust)
- [x] 005 - Cross-Platform Build & Distribution (Adapted for Rust)
- [x] 006 - Architecture Decisions (Adapted for Rust)
- [x] [R-LIC-01](R-LIC-01-license-audit.md) - License Audit ✅ **Resolved: Kokoro TTS selected (Apache 2.0) to avoid espeak-ng GPL**
- [x] [R-WRS-01](R-WRS-01-whisper-rs-benchmarks.md) - whisper-rs Benchmarking ✅ **GO recommendation** (empirical testing pending)
- [x] [R-SEC-01](R-SEC-01-model-verification.md) - Model File Verification Patterns ✅
- [x] [R-DB-01](R-DB-01-sqlite-schema.md) - SQLite Schema Design — FTS5 patterns, 100KB blob threshold, export formats
- [x] [R-ORT-01](R-ORT-01-ort-mobile-builds.md) - ort Mobile Builds ✅ **CONDITIONAL GO** (tract fallback available, NNAPI deprecated)
- [x] [R-ERR-01](R-ERR-01-error-handling-patterns.md) - Error Handling Patterns — thiserror for lib, UniFFI for FFI, tracing for logging
- [x] [R-AUD-01](R-AUD-01-cpal-mobile-audio.md) - cpal Mobile Audio ✅ **GO** (CoreAudio/AAudio, API 26+ Android, resampling needed)
- [x] [R-THR-01](R-THR-01-thermal-profiling.md) - Thermal Profiling ✅ **GO** (ANE/NPU 85% power reduction, adaptive throttling strategy)
- [x] [R-UX-01](R-UX-01-offline-first-ux.md) - Offline-First UX — Immediate download recommended, Material Design patterns
- [x] [R-BLE-01](R-BLE-01-bluetooth-audio.md) - Bluetooth Audio ✅ **SUPPORT WITH CAVEATS** (HFP 1.6 for 16kHz, LE Audio preferred)
- [x] [R-BLE-02](R-BLE-02-desktop-hfp-negotiation.md) - Desktop HFP Negotiation ✅ **CONDITIONAL** (macOS/Windows auto, Linux BlueZ control)
- [x] [R-TTS-01](R-TTS-01-alternative-tts-engines.md) - Alternative TTS Engines ✅ **Kokoro TTS recommended** (Apache 2.0, Misaki G2P avoids GPL)
- [x] [R-DIA-01](R-DIA-01-speaker-diarization.md) - Speaker Diarization ✅ **ECAPA-TDNN recommended** for Release phase upgrade
- [x] [R-SYS-01](R-SYS-01-system-profiling.md) - System Profiling ✅ **sysinfo + platform GPU detection** for WP-CONFIG model recommendations

## Archived

- 002-whisper-go-bindings.md - Removed (Go-specific, replaced by Rust approach)
