---
type: index
updated: 2026-01-22
---

# Research Index

> Technical research findings organized by category. All research complete.

---

## Summary

**Decision: Rust Implementation**

The project uses Rust libraries instead of Go, based on:
- Superior FFI story for mobile platforms (C ABI, no CGO complexity)
- Better cross-compilation support
- Native whisper-rs bindings to whisper.cpp
- Strong ecosystem for audio processing

---

## By Category

### Audio
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-AUD-01](R-AUD-01-cpal-mobile-audio.md) | cpal Mobile Audio | ✅ GO | CoreAudio/AAudio, API 26+ Android, resampling needed |

### Bluetooth
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-BLE-01](R-BLE-01-bluetooth-audio.md) | Bluetooth Audio | ✅ CAVEATS | HFP 1.6 for 16kHz, LE Audio preferred |
| [R-BLE-02](R-BLE-02-desktop-hfp-negotiation.md) | Desktop HFP Negotiation | ✅ CONDITIONAL | macOS/Windows auto, Linux BlueZ control |

### Database
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-DB-01](R-DB-01-sqlite-schema.md) | SQLite Schema Design | ✅ Complete | FTS5 patterns, 100KB blob threshold, export formats |

### Algorithm
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-DIA-01](R-DIA-01-speaker-diarization.md) | Speaker Diarization | ✅ Complete | ECAPA-TDNN recommended for Release phase |

### Architecture
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-ERR-01](R-ERR-01-error-handling-patterns.md) | Error Handling Patterns | ✅ Complete | thiserror for lib, UniFFI for FFI, tracing for logging |

### Legal
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-LIC-01](R-LIC-01-license-audit.md) | License Audit | ✅ Resolved | Kokoro TTS selected (Apache 2.0) to avoid espeak-ng GPL |

### Feasibility
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-ORT-01](R-ORT-01-ort-mobile-builds.md) | ort Mobile Builds | ✅ CONDITIONAL | tract fallback available, NNAPI deprecated |

### Security
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-SEC-01](R-SEC-01-model-verification.md) | Model Verification | ✅ Complete | Best practices for model file verification |

### Configuration
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-SYS-01](R-SYS-01-system-profiling.md) | System Profiling | ✅ Complete | sysinfo + platform GPU detection for WP-CONFIG |

### Performance
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-THR-01](R-THR-01-thermal-profiling.md) | Thermal Profiling | ✅ GO | ANE/NPU 85% power reduction, adaptive throttling |
| [R-WRS-01](R-WRS-01-whisper-rs-benchmarks.md) | whisper-rs Benchmarks | ✅ GO | Empirical testing pending |

### TTS
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-TTS-01](R-TTS-01-alternative-tts-engines.md) | Alternative TTS Engines | ✅ Complete | Kokoro TTS recommended (Apache 2.0, Misaki G2P) |

### UX
| ID | Title | Status | Key Finding |
|----|-------|--------|-------------|
| [R-UX-01](R-UX-01-offline-first-ux.md) | Offline-First UX | ✅ Complete | Immediate download recommended, Material Design |

---

## Initial Feasibility Research

| Plan | Status | Key Finding |
|------|--------|-------------|
| [001-mobile-platform-constraints](001-mobile-platform-constraints.md) | ✅ Complete | Rust + whisper-rs feasible on iOS/Android |
| [003-translation-tts-engines](003-translation-tts-engines.md) | ✅ Complete | Rust crates available for TTS |
| [004-realtime-performance](004-realtime-performance.md) | ✅ Complete | RTF <1.0 achievable; <800ms latency target |
| [005-cross-platform-distribution](005-cross-platform-distribution.md) | ✅ Complete | Rust cross-compilation straightforward |
| [006-architecture-decisions](006-architecture-decisions.md) | ✅ Complete | Hybrid architecture selected |

---

## Archived

- 002-whisper-go-bindings.md — Removed (Go-specific, replaced by Rust approach)

---

## Research Template

New research should use this format:

```markdown
---
id: R-XXX-NN
type: research
status: complete  # in-progress | complete | superseded
created: YYYY-MM-DD
updated: YYYY-MM-DD
author: design-lead
category: Category
tags: []
---

# R-XXX-NN: Title

## Summary
One-paragraph summary of findings.

## Context
Why this research was needed.

## Findings
Detailed findings...

## Recommendation
GO | NO-GO | CONDITIONAL with rationale.

## References
- Links to external resources
```
