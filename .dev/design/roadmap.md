---
id: roadmap
type: roadmap
status: active
updated: 2026-01-22
author: design-lead
---

# Lexicon Local Server — Roadmap

> **Current Phase**: MVP → Beta Transition
>
> **Progress**: Layers 1-3 substantially complete. Phase 1 MVP ready for testing.

---

## Executive Summary

Lexicon Local Server decomposes into **14 work packages** organized in **4 layers**. The layered structure enables parallel development within each layer while maintaining clear dependencies between layers.

**Delivery Strategy:**
- **MVP**: Desktop STT transcription with SQLite persistence
- **Beta**: Add TTS, mobile FFI bindings, streaming APIs
- **Release**: Full platform coverage, production hardening

**Timeline Independence:** Work packages are defined by deliverables and dependencies, not time estimates. Teams should track completion criteria, not calendars.

---

## Phased Delivery

### Phase 1: MVP (Core Functionality) ✅

**Goal:** Desktop STT transcription with persistence and immediate model download

**Included Packages:**

| Package | Scope | Status |
|---------|-------|--------|
| WP-CORE | Full | ✅ Complete |
| WP-AUDIO | Desktop only (no Bluetooth, no mobile routes) | ✅ Complete |
| WP-DB | Full | ✅ Complete |
| WP-MODEL | Full including device profiling and immediate download | ✅ Complete |
| WP-STT | Core transcription (diarization deferred to Beta) | ✅ Complete |
| WP-CLI | `transcribe` and `models` commands only | ✅ Complete |

**Not Included:**
- TTS (WP-TTS) — deferred to Beta
- Server (WP-SERVER) — deferred to Beta
- Mobile (WP-FFI, WP-IOS, WP-ANDROID) — deferred to Beta/Release
- CI/CD (WP-BUILD) — deferred to Release
- Speaker diarization — deferred to Beta
- Bluetooth audio — deferred to Beta

**MVP Deliverables:**
1. ✅ `lexicon transcribe audio.mp3` produces SRT output
2. ✅ First launch profiles device and downloads tier-appropriate models
3. ✅ Sessions persist to SQLite with FTS5 search
4. ✅ Works on macOS, Windows, Linux

**Exit Criteria:**
- [x] First launch downloads models before allowing transcription (D1)
- [x] Transcribe 10-minute audio file successfully
- [x] Search finds text across sessions
- [x] Export produces valid SRT

---

### Phase 2: Beta (User Experience) 🔄

**Goal:** Full feature set with streaming, mobile bindings, and enhanced audio

**Added Packages:**

| Package | Scope | Status |
|---------|-------|--------|
| WP-TTS | Full including fallback triggers (D4) | 🔄 In Progress |
| WP-SERVER | Full | ✅ Complete |
| WP-FFI | Full | 🔄 In Progress |
| WP-AUDIO | Add Bluetooth HFP 1.6 (D2), mobile audio routes | 🔄 In Progress |
| WP-STT | Add speaker diarization (D3) | 🔄 In Progress |
| WP-CLI | Add `serve` command | ✅ Complete |
| **WP-CONFIG** | Configuration management (NEW) | ⬜ Ready |

**Beta Deliverables:**
1. ✅ HTTP server with WebSocket streaming
2. 🔄 TTS synthesis with Kokoro + tts-rs fallback
3. 🔄 UniFFI bindings compile
4. ✅ Real-time STT latency <800ms
5. 🔄 Speaker diarization assigns IDs when enabled
6. 🔄 Bluetooth audio works at 16kHz
7. 🔄 Hash verification for fallback model downloads

**Exit Criteria:**
- [x] WebSocket client receives streaming segments
- [ ] TTS produces natural-sounding speech
- [x] TTS falls back to platform voice for non-English (D4)
- [ ] Swift/Kotlin bindings pass unit tests
- [ ] Diarization correctly identifies 2+ speakers (D3)
- [ ] Bluetooth HFP 1.6 verified at 16kHz (D2)
- [x] Latency benchmarks documented

---

### Phase 3: Release (Production Ready) ⬜

**Goal:** Full platform coverage with production hardening and thermal management

**Added Packages:**

| Package | Scope | Status |
|---------|-------|--------|
| WP-IOS | Full including thermal monitoring | ⬜ Ready |
| WP-ANDROID | Full including thermal monitoring | ⬜ Ready |
| WP-WIN | Full including GPU acceleration documentation | ✅ Complete |
| WP-BUILD | Full | ✅ Complete |

**Release Deliverables:**
1. iOS framework on TestFlight
2. Android AAR on Maven
3. Desktop binaries on GitHub Releases (including Windows native)
4. Windows portable zip distribution
5. Signed model manifests

**Exit Criteria:**
- [ ] iOS app transcribes on device
- [ ] Android app transcribes on device
- [x] Windows native build works without WSL
- [ ] Thermal throttling triggers model downgrade on mobile platforms
- [ ] All platforms pass integration tests
- [ ] Security audit completed

---

## Dependency Graph

```
Layer 1 (Parallel) ─ Foundation
├── WP-CORE ─────────┐
├── WP-AUDIO ────────┤
├── WP-AUDIOCONV ────┤
├── WP-DB ───────────┼──▶ Layer 2 (Parallel) ─ Engines
└── WP-MODEL ────────┤    ├── WP-STT ──────┐
                     │    └── WP-TTS ──────┼──▶ Layer 3 (Parallel) ─ Integration
                     │                     │    ├── WP-FFI ────┐
                     └─────────────────────┘    ├── WP-SERVER  │
                                                └── WP-CLI ────┼──▶ Layer 4 (Parallel) ─ Platform
                                                               │    ├── WP-IOS
                                                               │    ├── WP-ANDROID
                                                               │    ├── WP-WIN
                                                               │    └── WP-BUILD
                                                               ▼
```

---

## Key Decisions

| ID | Decision | Rationale | Phase |
|----|----------|-----------|-------|
| D1 | Immediate model download | Better UX than lazy loading | MVP |
| D2 | Bluetooth HFP 1.6 | 16kHz sample rate for quality | Beta |
| D3 | Configurable diarization | Per-session enable/disable | Beta |
| D4 | TTS platform fallback | Multi-language support | Beta |

*Full ADRs to be migrated to [decisions/](decisions/)*

---

## Current Priorities

1. **WP-CONFIG** — High priority MVP enhancement for first-run experience
2. **WP-AUDIO** — Complete Bluetooth HFP 1.6 support
3. **WP-STT** — Verify diarization with 2+ speakers
4. **WP-FFI** — Platform compilation verification

---

## References

- [Work Packages](../work/_index.md) — Detailed work package specifications
- [Architecture](architecture.md) — System design
- [Research](research/_index.md) — Technical findings
