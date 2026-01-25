---
id: roadmap
type: roadmap
status: active
updated: 2026-01-25
author: design-lead
---

# Lexicon Local Server — Roadmap

> **Current Phase**: Beta (70% Complete)
>
> **Progress**: MVP complete. All Beta scaffolding done. Awaiting human/hardware validation.

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
| WP-TTS | Full including fallback triggers (D4) | ✅ Complete |
| WP-SERVER | Full | ✅ Complete |
| WP-FFI | Full | ✅ Complete (tests blocked) |
| WP-AUDIO | Add Bluetooth HFP 1.6 (D2), mobile audio routes | 🟡 Scaffolding complete |
| WP-STT | Add speaker diarization (D3) | 🔄 In Progress |
| WP-CLI | Add `serve` command | ✅ Complete |
| **WP-CONFIG** | Configuration management | ✅ Complete |
| **WP-TUI** | Diagnostic TUI | 🟡 Feature complete |
| **WP-HASH-VERIFY** | Model hash verification | 🟡 Scaffolding complete |
| **WP-TTS-QUALITY** | TTS quality validation | 🟡 Infrastructure complete |

**Beta Deliverables:**
1. ✅ HTTP server with WebSocket streaming
2. ✅ TTS synthesis with Kokoro + tts-rs fallback
3. ✅ UniFFI bindings compile (tests blocked on SDK)
4. ✅ Real-time STT latency <800ms
5. 🔄 Speaker diarization assigns IDs when enabled
6. 🟡 Bluetooth audio works at 16kHz (scaffolding complete)
7. 🟡 Hash verification for fallback model downloads (awaiting hashes)
8. 🟡 Diagnostic TUI for real-time transcription (feature complete)
9. 🟡 TTS quality validation (infrastructure complete)

**Exit Criteria:**
- [x] WebSocket client receives streaming segments
- [ ] TTS produces natural-sounding speech (MOS >= 3.5)
- [x] TTS falls back to platform voice for non-English (D4)
- [ ] Swift/Kotlin bindings pass unit tests (blocked on SDK)
- [ ] Diarization correctly identifies 2+ speakers (D3)
- [ ] Bluetooth HFP 1.6 verified at 16kHz (D2) (awaiting hardware)
- [x] Latency benchmarks documented
- [ ] Model hash verification working (awaiting hash values)

---

### Phase 3: Release (Production Ready) ⬜

**Goal:** Full platform coverage with production hardening and thermal management

**Added Packages:**

| Package | Scope | Status |
|---------|-------|--------|
| WP-IOS | Full including thermal monitoring | ⬜ Ready |
| WP-ANDROID | Full including thermal monitoring | 🔄 Partial (STT+TTS working, device testing needed) |
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

### Awaiting Human Action (High Priority)
1. **WP-HASH-VERIFY** — Acquire model SHA-256 hashes (security requirement)
2. **WP-TUI** — Hardware test with Whisper model
3. **WP-TTS-QUALITY** — Human listening evaluation (MOS scoring)
4. **WP-BLE** — Physical Bluetooth hardware testing

### Code Work Remaining
5. **WP-STT** — Verify diarization with 2+ speakers
6. **WP-FFI-TEST** — Swift/Kotlin unit tests (blocked on macOS/Android SDK)

---

## References

- [Work Packages](../work/_index.md) — Detailed work package specifications
- [Architecture](architecture.md) — System design
- [Research](research/_index.md) — Technical findings
