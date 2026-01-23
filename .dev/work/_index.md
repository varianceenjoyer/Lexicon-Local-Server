---
type: index
updated: 2026-01-22
---

# Work Packages Index

> Active work packages for implementation. Agents should scan this first to find relevant work.

---

## Summary

| Layer | Package | Status | Phase | Priority | Consumers |
|-------|---------|--------|-------|----------|-----------|
| 1 | [WP-CORE](WP-CORE.md) | complete | mvp | - | developer |
| 1 | [WP-AUDIO](WP-AUDIO.md) | in-progress | beta | high | developer |
| 1 | [WP-AUDIOCONV](WP-AUDIOCONV.md) | complete | mvp | - | developer |
| 1 | [WP-DB](WP-DB.md) | complete | mvp | - | developer |
| 1 | [WP-MODEL](WP-MODEL.md) | complete | mvp | - | developer |
| 2 | [WP-STT](WP-STT.md) | in-progress | beta | high | developer |
| 2 | [WP-TTS](WP-TTS.md) | in-progress | beta | medium | developer |
| 3 | [WP-FFI](WP-FFI.md) | in-progress | beta | high | developer |
| 3 | [WP-SERVER](WP-SERVER.md) | complete | beta | - | developer |
| 3 | [WP-CLI](WP-CLI.md) | complete | mvp | - | developer |
| 4 | [WP-IOS](WP-IOS.md) | ready | release | medium | developer |
| 4 | [WP-ANDROID](WP-ANDROID.md) | ready | release | medium | developer |
| 4 | [WP-WIN](WP-WIN.md) | complete | release | - | developer |
| 4 | [WP-BUILD](WP-BUILD.md) | complete | release | - | developer |
| - | [WP-CONFIG](WP-CONFIG.md) | ready | mvp | high | developer |

---

## By Status

### In Progress
- [WP-AUDIO](WP-AUDIO.md) — Bluetooth HFP 1.6 support remaining
- [WP-STT](WP-STT.md) — Diarization verification pending
- [WP-TTS](WP-TTS.md) — Natural speech quality verification
- [WP-FFI](WP-FFI.md) — Platform compilation pending

### Ready for Implementation
- [WP-CONFIG](WP-CONFIG.md) — Local configuration management (HIGH priority)
- [WP-IOS](WP-IOS.md) — iOS platform integration
- [WP-ANDROID](WP-ANDROID.md) — Android platform integration

### Blocked
*None currently*

---

## By Layer

### Layer 1: Foundation (No Dependencies)
These can be developed in parallel with no interdependencies.

| Package | Deliverable | Status |
|---------|-------------|--------|
| WP-CORE | Cargo workspace, config, errors, logging | ✅ Complete |
| WP-AUDIO | Audio capture, VAD, resampling, Bluetooth | 🔄 In Progress |
| WP-AUDIOCONV | MP3/OGG/FLAC to WAV conversion | ✅ Complete |
| WP-DB | SQLite with FTS5 search | ✅ Complete |
| WP-MODEL | Model download, verification, selection | ✅ Complete |

### Layer 2: Engines (Depends on Layer 1)
| Package | Deliverable | Dependencies | Status |
|---------|-------------|--------------|--------|
| WP-STT | whisper-rs integration, streaming | WP-CORE, WP-AUDIO, WP-MODEL | 🔄 In Progress |
| WP-TTS | Kokoro TTS with platform fallback | WP-CORE, WP-MODEL | 🔄 In Progress |

### Layer 3: Integration (Depends on Layer 2)
| Package | Deliverable | Dependencies | Status |
|---------|-------------|--------------|--------|
| WP-FFI | UniFFI bindings for Swift/Kotlin | WP-STT, WP-TTS, WP-DB | 🔄 In Progress |
| WP-SERVER | Axum HTTP/WebSocket server | WP-STT, WP-TTS, WP-DB | ✅ Complete |
| WP-CLI | Command-line interface | WP-STT, WP-TTS, WP-DB | ✅ Complete |

### Layer 4: Platform (Depends on Layer 3)
| Package | Deliverable | Dependencies | Status |
|---------|-------------|--------------|--------|
| WP-IOS | iOS static library + Swift package | WP-FFI | ⬜ Ready |
| WP-ANDROID | Android SO + Kotlin bindings | WP-FFI | ⬜ Ready |
| WP-WIN | Windows native build | All | ✅ Complete |
| WP-BUILD | CI/CD pipeline | All | ✅ Complete |

---

## Dependency Graph

```
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
                                                               ▼
```

---

## Additional Work Items

### Roadblocks (Technical Debt)
See [roadblocks.md](roadblocks.md) for tracked technical issues that need resolution.

### Cuttlefish (Android Emulation)
See [../docs/android-emulation/](../docs/android-emulation/) for Android emulation setup documentation.

---

## Recently Updated

1. [WP-CONFIG](WP-CONFIG.md) — 2026-01-22 (new)
2. [WP-AUDIO](WP-AUDIO.md) — 2026-01-20
3. [WP-STT](WP-STT.md) — 2026-01-20
