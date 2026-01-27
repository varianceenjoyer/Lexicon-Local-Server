# Work Packages Index

> **Status**: Active | **Last Updated**: 2026-01-27
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

### Blocked (Beta/Release Phase)

| ID | Name | Priority | Blocked By | Notes |
|----|------|----------|------------|-------|
| [WP-FFI-TEST](WP-FFI-TEST.md) | Mobile FFI Binding Tests | HIGH | macOS/Android SDK | Swift/Kotlin unit tests |
| [WP-DIARIZATION](WP-DIARIZATION.md) | Speaker Diarization (Neural) | MEDIUM | Release phase | ECAPA-TDNN integration |
| [WP-IOS](WP-IOS.md) | iOS Platform Integration | HIGH | WP-FFI, macOS required | XCFramework + Swift |
| [WP-ANDROID](WP-ANDROID.md) | Android Platform Integration | HIGH | Device testing | STT+TTS working on emulator |

### Ready for Implementation

(None - all high priority items in progress or complete)

### In Progress

| ID | Name | Owner | Started | Notes |
|----|------|-------|---------|-------|
| [WP-BLE](WP-BLE.md) | Bluetooth HFP 1.6 Support | - | 2026-01-23 | Scaffolding complete - awaiting hardware test |
| [WP-AUDIO-SIM](WP-AUDIO-SIM.md) | Simulated Audio Device | - | 2026-01-27 | Core complete - testing remaining |

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
| [WP-TUI](WP-TUI.md) | Diagnostic TUI | 2026-01-25 | Hardware verified on Windows and bash |
| [WP-TTS-QUALITY](WP-TTS-QUALITY.md) | TTS Quality Validation | 2026-01-25 | MOS 3.0, shipped as known limitation |
| [WP-HASH-VERIFY](WP-HASH-VERIFY.md) | Model Hash Verification | 2026-01-27 | Fallback hash verification integrated |

---

## Phase Mapping

### MVP Phase (Complete)
Core transcription capability on desktop:
- WP-CORE, WP-AUDIO, WP-AUDIOCONV, WP-DB, WP-MODEL, WP-STT, WP-CLI

### Beta Phase (Current)
Enhanced features and security:
- **WP-CONFIG** - System profiling and model recommendations (COMPLETE)
- **WP-TUI** - Diagnostic TUI for real-time transcription (COMPLETE)
- **WP-HASH-VERIFY** - Secure fallback downloads (COMPLETE)
- **WP-AUDIO-SIM** - Simulated audio for Android testing (core complete, testing pending)
- **WP-BLE** - Bluetooth audio quality (scaffolding complete, awaiting hardware)
- **WP-TTS-QUALITY** - TTS quality validation (COMPLETE - MOS 3.0)
- **WP-FFI-TEST** - Mobile binding tests (blocked on SDK)
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
2. **WP-HASH-VERIFY** - Security requirement (COMPLETE)
3. **WP-TUI** - Diagnostic TUI for real-time transcription (COMPLETE)
4. **WP-TTS-QUALITY** - TTS quality validation (COMPLETE - MOS 3.0)

### Should Complete for Beta
5. **WP-BLE** - Audio quality improvement (scaffolding complete)
6. **WP-AUDIO-SIM** - Simulated audio for Android testing

### Must Complete for Release
7. **WP-IOS** - iOS platform support
8. **WP-ANDROID** - Android platform support

### Nice to Have for Release
9. **WP-DIARIZATION** - Multi-speaker support

---

## Quick Links

### Work Packages (Beta) - In Progress
- [WP-AUDIO-SIM](WP-AUDIO-SIM.md) - Simulated audio device (core complete, testing pending)

### Work Packages (Beta) - In Progress
- [WP-BLE](WP-BLE.md) - Bluetooth HFP support (awaiting hardware)
- [WP-FFI-TEST](WP-FFI-TEST.md) - Mobile binding tests (blocked on SDK)

### Work Packages (Beta) - Complete
- [WP-CONFIG](WP-CONFIG.md) - Configuration management
- [WP-TUI](WP-TUI.md) - Diagnostic TUI (verified working)
- [WP-TTS-QUALITY](WP-TTS-QUALITY.md) - TTS quality validation (MOS 3.0)
- [WP-HASH-VERIFY](WP-HASH-VERIFY.md) - Model hash verification (fallback integrated)

### Work Packages (Release)
- [WP-DIARIZATION](WP-DIARIZATION.md) - Speaker diarization
- [WP-IOS](WP-IOS.md) - iOS integration
- [WP-ANDROID](WP-ANDROID.md) - Android integration

### Handoffs (Awaiting Human Action)
- [HO-BLE-001](../handoffs/HO-BLE-001.md) - Bluetooth HFP 1.6 support (awaiting hardware)

### Handoffs (Complete)
- [HO-CONFIG-001](../handoffs/HO-CONFIG-001.md) - System profiling and CLI config
- [HO-TUI-001](../handoffs/HO-TUI-001.md) - Diagnostic TUI (verified working)
- [HO-HASH-001](../handoffs/HO-HASH-001.md) - Fallback hash verification (complete)
- [HO-TTS-QUALITY-001](../handoffs/HO-TTS-QUALITY-001.md) - TTS quality validation (MOS 3.0)

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
  WP-HASH-VERIFY ───────────┤
  WP-BLE ───────────────────┼──▶ Beta Release
  WP-TTS-QUALITY ───────────┤
  WP-TUI ───────────────────┤
  WP-AUDIO-SIM ─────────────┘ (Android TUI testing)

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

*Last updated: 2026-01-27. Beta at 90% completion. WP-HASH-VERIFY complete. WP-AUDIO-SIM core implementation complete. See [.dev/human.md](../human.md) for action items.*
