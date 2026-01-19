# Research

## Status Summary (Updated 2026-01-19)

**Decision: Rust Implementation**

The project will be implemented using Rust libraries instead of Go. This decision was made based on:
- Superior FFI story for mobile platforms (C ABI, no CGO complexity)
- Better cross-compilation support
- Native whisper-rs bindings to whisper.cpp
- Strong ecosystem for audio processing

| Plan | Status | Key Finding |
|------|--------|-------------|
| 001-mobile-platform-constraints | ✅ Complete | Rust + whisper-rs feasible on iOS/Android with memory constraints |
| 003-translation-tts-engines | ✅ Complete | Rust crates available for TTS (Piper via ONNX); translation via Whisper X→EN |
| 004-realtime-performance | ✅ Complete | RTF <1.0 achievable; target <800ms for conversational latency |
| 005-cross-platform-distribution | ✅ Complete | Rust cross-compilation straightforward; cargo-mobile2 for mobile |
| 006-architecture-decisions | ✅ Complete | Hybrid architecture: embedded Rust lib + optional remote server |

## Complete

- [x] 001 - Mobile Platform Constraints (Adapted for Rust)
- [x] 003 - Translation & TTS Engine Selection (Adapted for Rust)
- [x] 004 - Real-Time Performance (Adapted for Rust)
- [x] 005 - Cross-Platform Build & Distribution (Adapted for Rust)
- [x] 006 - Architecture Decisions (Adapted for Rust)

## Archived

- 002-whisper-go-bindings.md - Removed (Go-specific, replaced by Rust approach)

## Remaining Research Items

None - initial feasibility research complete. Implementation planning can begin.
