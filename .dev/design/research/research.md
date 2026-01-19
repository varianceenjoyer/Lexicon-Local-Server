# Research

## Status Summary (Updated 2026-01-19)

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

### Implementation Research (Ongoing)

| Task ID | Task | Category | Output |
|---------|------|----------|--------|
| R-SEC-01 | Model file verification patterns | Security | Best practices doc |
| R-WRS-01 | Benchmark whisper-rs (latency, memory, stability) | Performance | Benchmark report |
| R-ORT-01 | ort crate mobile builds (iOS/Android) | Feasibility | Build guide + gotchas |
| R-AUD-01 | cpal audio capture on mobile platforms | Feasibility | Integration patterns |
| R-THR-01 | Thermal behavior of sustained inference | Performance | Throttling thresholds |
| R-ERR-01 | Rust error handling for streaming pipelines | Architecture | Error handling guide |
| R-BLE-01 | Bluetooth audio latency and disconnection | Edge cases | Compatibility matrix |
| R-LIC-01 | License audit (whisper-rs, ort, cpal, Piper) | Legal | License compatibility report |
| R-UX-01 | Offline-first UX for model management | Design | UX recommendations |
| R-DB-01 | SQLite schema for transcription storage | Architecture | Schema design doc |

---

## Ongoing

- [ ] [R-SEC-01](R-SEC-01-model-verification.md) - Model File Verification Patterns (Security)
- [ ] [R-WRS-01](R-WRS-01-whisper-rs-benchmarks.md) - whisper-rs Benchmarking (Performance)
- [ ] [R-ORT-01](R-ORT-01-ort-mobile-builds.md) - ort Crate Mobile Builds (Feasibility)
- [ ] [R-AUD-01](R-AUD-01-cpal-mobile-audio.md) - cpal Audio Capture on Mobile (Feasibility)
- [ ] [R-THR-01](R-THR-01-thermal-profiling.md) - Thermal Behavior Profiling (Performance)
- [ ] [R-ERR-01](R-ERR-01-error-handling-patterns.md) - Rust Error Handling Patterns (Architecture)
- [ ] [R-BLE-01](R-BLE-01-bluetooth-audio.md) - Bluetooth Audio Latency (Edge Cases)
- [ ] [R-LIC-01](R-LIC-01-license-audit.md) - License Audit (Legal)
- [ ] [R-UX-01](R-UX-01-offline-first-ux.md) - Offline-First UX Patterns (Design)
- [ ] [R-DB-01](R-DB-01-sqlite-schema.md) - SQLite Schema Design (Architecture)

## Complete

- [x] 001 - Mobile Platform Constraints (Adapted for Rust)
- [x] 003 - Translation & TTS Engine Selection (Adapted for Rust)
- [x] 004 - Real-Time Performance (Adapted for Rust)
- [x] 005 - Cross-Platform Build & Distribution (Adapted for Rust)
- [x] 006 - Architecture Decisions (Adapted for Rust)

## Archived

- 002-whisper-go-bindings.md - Removed (Go-specific, replaced by Rust approach)
