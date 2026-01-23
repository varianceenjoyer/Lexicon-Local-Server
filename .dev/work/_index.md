# Work Packages Index

> **Status**: Active | **Last Updated**: 2026-01-23 (WP-CONFIG complete, WP-HASH-VERIFY scaffolding)
>
> This index tracks all actionable work packages for Lexicon Local Server.

---

## Overview

Work packages are self-contained units of implementation work with:
- Clear scope and deliverables
- Interface contracts
- Acceptance criteria
- Dependencies mapped

---

## Work Package Status

### Ready for Implementation (Beta Phase)

| ID | Name | Priority | Phase | Handoff |
|----|------|----------|-------|---------|
| [WP-BLE](WP-BLE.md) | Bluetooth HFP 1.6 Support | MEDIUM | Beta | [HO-BLE-001](../handoffs/HO-BLE-001.md) |

### Blocked (Beta/Release Phase)

| ID | Name | Priority | Blocked By | Notes |
|----|------|----------|------------|-------|
| [WP-FFI-TEST](WP-FFI-TEST.md) | Mobile FFI Binding Tests | HIGH | macOS/Android SDK | Swift/Kotlin unit tests |
| [WP-DIARIZATION](WP-DIARIZATION.md) | Speaker Diarization (Neural) | MEDIUM | Release phase | ECAPA-TDNN integration |
| [WP-IOS](WP-IOS.md) | iOS Platform Integration | HIGH | WP-FFI, macOS required | XCFramework + Swift |
| [WP-ANDROID](WP-ANDROID.md) | Android Platform Integration | HIGH | WP-FFI, NDK required | AAR + Kotlin |

### In Progress

| ID | Name | Owner | Started | Notes |
|----|------|-------|---------|-------|
| [WP-HASH-VERIFY](WP-HASH-VERIFY.md) | Model Hash Verification | - | 2026-01-23 | Scaffolding complete - awaiting hash values |
| [WP-TTS-QUALITY](WP-TTS-QUALITY.md) | TTS Quality Validation | - | 2026-01-23 | Test infrastructure complete - awaiting human eval |

### Completed (MVP/Early Beta)

| ID | Name | Completed | Notes |
|----|------|-----------|-------|
| WP-CORE | Core Library Scaffolding | 2026-01-19 | Cargo workspace, config, errors |
| WP-AUDIO | Audio Pipeline | 2026-01-19 | cpal + VAD + resampling |
| WP-AUDIOCONV | Audio Format Conversion | 2026-01-19 | MP3/OGG/FLAC via symphonia |
| WP-DB | Storage Layer | 2026-01-19 | SQLite + FTS5 search |
| WP-MODEL | Model Manager | 2026-01-19 | Download + verification |
| WP-STT | Speech-to-Text Engine | 2026-01-19 | whisper-rs integration |
| WP-TTS | Text-to-Speech Engine | 2026-01-19 | Kokoro + tts-rs fallback |
| WP-SERVER | HTTP/WebSocket Server | 2026-01-19 | Axum REST + WebSocket |
| WP-CLI | Command-Line Interface | 2026-01-19 | All commands implemented |
| WP-FFI | Mobile FFI Bindings | 2026-01-19 | UniFFI Swift/Kotlin gen |
| WP-WIN | Windows Native Build | 2026-01-20 | MSVC + CI pipeline |
| WP-BUILD | CI/CD Pipeline | 2026-01-20 | GitHub Actions workflows |
| [WP-CONFIG](WP-CONFIG.md) | Local Configuration Management | 2026-01-23 | System profiling + CLI config commands |

---

## Phase Mapping

### MVP Phase (Complete)
Core transcription capability on desktop:
- WP-CORE, WP-AUDIO, WP-AUDIOCONV, WP-DB, WP-MODEL, WP-STT, WP-CLI

### Beta Phase (Current)
Enhanced features and security:
- **WP-CONFIG** - System profiling and model recommendations (COMPLETE)
- **WP-HASH-VERIFY** - Secure fallback downloads (scaffolding complete)
- **WP-BLE** - Bluetooth audio quality
- **WP-TTS-QUALITY** - TTS quality validation
- **WP-FFI-TEST** - Mobile binding tests (blocked)
- WP-TTS, WP-SERVER, WP-FFI (complete)

### Release Phase (Planned)
Mobile platform deployment:
- **WP-IOS** - iOS XCFramework + Swift package
- **WP-ANDROID** - Android AAR + Kotlin bindings
- **WP-DIARIZATION** - Neural speaker diarization

---

## Priority Matrix

### Must Complete for Beta
1. **WP-CONFIG** - Enables automatic model selection (COMPLETE)
2. **WP-HASH-VERIFY** - Security requirement

### Should Complete for Beta
3. **WP-BLE** - Audio quality improvement

### Must Complete for Release
4. **WP-IOS** - iOS platform support
5. **WP-ANDROID** - Android platform support

### Nice to Have for Release
6. **WP-DIARIZATION** - Multi-speaker support

---

## Quick Links

### Work Packages (Beta) - Active
- [WP-HASH-VERIFY](WP-HASH-VERIFY.md) - Model verification (HIGH priority)
- [WP-BLE](WP-BLE.md) - Bluetooth HFP support
- [WP-TTS-QUALITY](WP-TTS-QUALITY.md) - TTS quality validation
- [WP-FFI-TEST](WP-FFI-TEST.md) - Mobile binding tests (blocked)

### Work Packages (Beta) - Complete
- [WP-CONFIG](WP-CONFIG.md) - Configuration management

### Work Packages (Release)
- [WP-DIARIZATION](WP-DIARIZATION.md) - Speaker diarization
- [WP-IOS](WP-IOS.md) - iOS integration
- [WP-ANDROID](WP-ANDROID.md) - Android integration

### Handoffs (Ready to Implement)
- [HO-HASH-001](../handoffs/HO-HASH-001.md) - Fallback hash verification (HIGH priority)
- [HO-BLE-001](../handoffs/HO-BLE-001.md) - Bluetooth HFP 1.6 support
- [HO-TTS-QUALITY-001](../handoffs/HO-TTS-QUALITY-001.md) - TTS quality validation

### Handoffs (Complete)
- [HO-CONFIG-001](../handoffs/HO-CONFIG-001.md) - System profiling and CLI config

---

## Creating New Work Packages

Work packages should follow the template:

```markdown
# WP-XXX: Title

## Overview
Brief description

## Scope
What's included/excluded

## Interface Contract
Code signatures and types

## Acceptance Criteria
Measurable completion criteria

## Dependencies
What this needs/blocks

## Files
Key files to modify/create

## Research References
Related R-XXX documents
```

---

## Dependency Graph

```
Layer Architecture:
Layer 1 (Parallel)
├── WP-CORE ─────────┐
├── WP-AUDIO ────────┤
├── WP-AUDIOCONV ────┤
├── WP-DB ───────────┼──▶ Layer 2 (Parallel)
└── WP-MODEL ────────┤    ├── WP-STT ──────┐
                     │    └── WP-TTS ──────┼──▶ Layer 3 (Parallel)
                     │                     │    ├── WP-FFI ────┐
                     └─────────────────────┘    ├── WP-SERVER  │
                                                └── WP-CLI ────┼──▶ Layer 4 (Parallel)
                                                               │    ├── WP-IOS
                                                               │    ├── WP-ANDROID
                                                               │    ├── WP-WIN
                                                               │    └── WP-BUILD

Beta Phase Dependencies:
  WP-CONFIG ────────────────┐
  WP-HASH-VERIFY ───────────┼──▶ Beta Release
  WP-BLE ───────────────────┘
  WP-TTS-QUALITY ───────────┘

Release Phase Dependencies:
  WP-FFI (complete) ─────────┬──▶ WP-IOS ─────────┐
                             └──▶ WP-ANDROID ─────┼──▶ Release
  WP-DIARIZATION ────────────────────────────────┘
```

---

## Additional Work Items

### Roadblocks (Technical Debt)
See [roadblocks.md](roadblocks.md) for tracked technical issues that need resolution.

### Cuttlefish (Android Emulation)
See [../docs/android-emulation/](../docs/android-emulation/) for Android emulation setup documentation.

---

*Last updated: 2026-01-23. WP-CONFIG complete, WP-HASH-VERIFY scaffolding complete (awaiting hash values).*
