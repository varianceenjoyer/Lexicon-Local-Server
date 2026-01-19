# Research Plan: Architecture Decisions

**ID:** 006
**Status:** Complete (pending stakeholder decisions on B1-B3)
**Impact:** HIGH (foundational)
**Created:** 2026-01-19
**Last Updated:** 2026-01-19

---

## Overview

This research addresses fundamental architecture questions that must be resolved before detailed implementation planning. These decisions significantly impact all other feasibility areas.

---

## Research Questions

### RQ-1: Client-Server Deployment Model

**Problem Statement:**
Is this a local server that mobile apps connect to on localhost, embedded directly in mobile apps, or a remote server that mobile apps call? Architecture implications differ vastly.

**Deployment Models:**

| Model | Description | Pros | Cons |
|-------|-------------|------|------|
| **Embedded** | Go code runs within mobile app | Fully offline, no networking | Memory/CPU on device, app size |
| **Local Server** | Go server on localhost, app connects via HTTP/WS | Clean separation, web interface | Two processes, port conflicts |
| **Remote Server** | Cloud-hosted Go server | Powerful hardware, shared models | Requires internet, latency, costs |
| **Hybrid** | Embedded with remote fallback | Best of both | Complexity |

**Decision Factors:**
- [ ] What is the primary use case? (Personal device? Shared server? Both?)
- [ ] Is internet connectivity reliably available?
- [ ] What devices will run the server? (Powerful laptop? Raspberry Pi? Phone?)
- [ ] Is multi-user access required?
- [ ] What are the privacy requirements? (Data leaving device?)

**Areas of Inquiry:**
- [ ] Review project mission for deployment hints
- [ ] Identify target user personas
- [ ] Evaluate each model against constraints (cross-platform, offline)
- [ ] Consider "Lexicon clients" reference—what are the clients?

**Success Criteria:**
- Decision: Primary deployment model(s)
- Documented: Rationale for decision
- Identified: Implications for other research areas

---

### RQ-2: Offline vs Connected Operation

**Problem Statement:**
Must all processing work fully offline, or is cloud fallback acceptable for certain features?

**Operational Modes:**

| Mode | STT | Translation | TTS | Models |
|------|-----|-------------|-----|--------|
| **Fully Offline** | Local Whisper | Local engine | Local engine | Pre-downloaded |
| **Offline-First** | Local Whisper | Local, cloud fallback | Local engine | Download on demand |
| **Connected Required** | Cloud API | Cloud API | Cloud API | Remote |
| **Hybrid per Feature** | Local | Cloud | Local | Mixed |

**Decision Factors:**
- [ ] Privacy requirements—can audio/text leave the device?
- [ ] Target environments—are they always connected?
- [ ] Quality tradeoffs—local vs cloud accuracy
- [ ] Cost considerations—cloud API usage
- [ ] Use cases requiring offline (airplane, rural, privacy-sensitive)

**Areas of Inquiry:**
- [ ] Survey user expectations for offline capability
- [ ] Evaluate local vs cloud quality gap for each component
- [ ] Consider graceful degradation strategies
- [ ] Research data residency/privacy regulations if applicable

**Success Criteria:**
- Decision: Offline requirement per component
- Documented: Fallback strategy if applicable
- Identified: Offline-only feature set

---

### RQ-3: Multi-Language Model Strategy

**Problem Statement:**
Whisper multilingual models are larger than English-only. How are language-specific models managed, downloaded, and selected?

**Model Options:**

| Model | Languages | Size | Use Case |
|-------|-----------|------|----------|
| whisper-tiny.en | English only | ~40MB | Low resource, English only |
| whisper-tiny | Multilingual | ~40MB | Low resource, any language |
| whisper-base.en | English only | ~140MB | Balanced, English only |
| whisper-base | Multilingual | ~140MB | Balanced, any language |
| whisper-small | Multilingual | ~460MB | Better accuracy |
| whisper-medium | Multilingual | ~1.5GB | High accuracy |
| whisper-large-v3 | Multilingual | ~3GB | Best accuracy |

**Strategy Options:**
1. **Single bundled model** - Ship one model for all users
2. **Language packs** - Download models per language/quality tier
3. **On-demand download** - Download on first use of a language
4. **User selection** - Let users choose model size/quality
5. **Automatic selection** - Based on device capability

**Areas of Inquiry:**
- [ ] What languages must be supported at launch?
- [ ] Can models be downloaded incrementally?
- [ ] How is model storage managed on mobile (limited space)?
- [ ] Is model selection per-session or persistent?
- [ ] How do translation/TTS models interact with this strategy?

**Success Criteria:**
- Decision: Model distribution strategy
- Defined: Launch language set
- Documented: Model download and storage approach

---

### RQ-4: Web Interface Design

**Problem Statement:**
The project mission mentions "web interface for real-time STT, translation, and TTS." What is the scope and architecture of this interface?

**Interface Options:**

| Type | Technology | Hosting | Interaction Model |
|------|------------|---------|-------------------|
| **Embedded web UI** | Go templates + vanilla JS | Served by Go server | Traditional web |
| **SPA** | React/Vue/Svelte | Static files served by Go | WebSocket/REST API |
| **PWA** | SPA + service worker | Installable, offline UI | WebSocket/REST API |
| **Native mobile UI** | Swift/Kotlin | Bundled in app | Direct Go binding calls |

**Areas of Inquiry:**
- [ ] Is the web interface primary or secondary to native apps?
- [ ] What browsers/platforms must be supported?
- [ ] Is offline web interface functionality needed?
- [ ] What is the real-time communication mechanism (WebSocket, SSE)?
- [ ] Are there accessibility requirements?

**Success Criteria:**
- Decision: Web interface technology stack
- Defined: API contract between UI and server
- Documented: Real-time communication architecture

---

## Discovery Tasks

### Phase 1: Stakeholder Clarification
- [ ] **T1.1:** Define primary deployment scenario(s)
- [ ] **T1.2:** Identify target user personas
- [ ] **T1.3:** Clarify offline requirements
- [ ] **T1.4:** Define launch language set

### Phase 2: Architecture Spike
- [ ] **T2.1:** Prototype embedded Go in mobile app
- [ ] **T2.2:** Prototype local server with web interface
- [ ] **T2.3:** Evaluate communication patterns (REST vs WebSocket)
- [ ] **T2.4:** Test model download and caching

### Phase 3: Decision Documentation
- [ ] **T3.1:** Document chosen deployment model with rationale
- [ ] **T3.2:** Document offline/connected strategy
- [ ] **T3.3:** Document model management approach
- [ ] **T3.4:** Document web interface architecture

---

## Findings

### Deployment Model Analysis

**Research Date:** 2026-01-19
**Sources:** [sherpa-onnx GitHub](https://github.com/k2-fsa/sherpa-onnx), [gomobile docs](https://pkg.go.dev/golang.org/x/mobile/cmd/gomobile), [Go Mobile Wiki](https://go.dev/wiki/Mobile)

#### Detailed Model Comparison

| Model | Technical Feasibility | Memory Impact | Network Dependency | Complexity |
|-------|----------------------|---------------|-------------------|------------|
| **Embedded (gomobile)** | ⚠️ CGO issues on iOS | High (model in app) | None | High |
| **Embedded (sherpa-onnx)** | ✅ Pre-built bindings | High (model in app) | None | Medium |
| **Local HTTP Server** | ✅ Standard Go | Moderate | localhost only | Low |
| **Remote Server** | ✅ Standard Go | Low (client-side) | Required | Low |
| **Hybrid** | ✅ Achievable | Variable | Optional | Medium-High |

#### Embedded Model Analysis

**With gomobile + whisper.cpp (originally planned):**
- iOS: CGO-dependent packages have known issues ([GitHub #35426](https://github.com/golang/go/issues/35426))
- gomobile bind doesn't handle complex CGO scenarios reliably
- Type limitations require bridge packages with struct wrappers
- Performance concerns: Go bindings up to 45x slower than C++ (Research 002 findings)

**With sherpa-onnx (recommended alternative):**
- Pre-built Go bindings for Windows, Linux, macOS
- Native iOS/Android support (Swift/Kotlin wrappers)
- No CGO cross-compilation complexity
- Unified STT + TTS + VAD in single package
- [Go package for macOS/iOS available](https://pkg.go.dev/github.com/JatosDetking/sherpa-onnx-go-macOS-iOS)

**Key Constraint:** Mobile memory limit (iOS ~2GB hard limit) restricts embedded deployments to tiny/base quantized models only (per Research 001).

#### Local HTTP Server Analysis

**Architecture Pattern:**
```
[Mobile App UI] ←HTTP/WebSocket→ [Go Server on localhost] ←→ [Whisper/TTS]
```

**iOS Considerations:**
- iOS Simulator shares host network; `localhost` works directly
- On-device: App would need to bundle and run server process
- [iOS network restrictions](https://dev.to/sskinfinite/understanding-localhost-networking-in-ios-simulator-vs-android-emulator-5ekb) complicate this pattern

**Android Considerations:**
- Emulator requires `10.0.2.2` or `adb reverse` to reach localhost
- On-device: Foreground service could host local server

**Viability Assessment:** Local HTTP server within mobile app is **complex and non-standard**. This pattern works well for desktop but adds unnecessary complexity on mobile.

#### Remote Server Analysis

**Architecture Pattern:**
```
[Mobile/Web Client] ←HTTPS/WSS→ [Remote Go Server] ←→ [Whisper/TTS]
```

**Advantages:**
- Powerful hardware available (GPU acceleration)
- Large models accessible (medium/large Whisper)
- Standard web architecture, well-understood
- Single codebase serves all clients

**Disadvantages:**
- Requires internet connectivity
- Latency dependent on network
- Ongoing server costs
- Privacy considerations (audio leaves device)

#### Hybrid Model Analysis

**Recommended Tiered Architecture:**

| Tier | Use Case | Implementation | Models |
|------|----------|----------------|--------|
| **Tier 1: Offline Mobile** | No connectivity | sherpa-onnx embedded | tiny/base q5_1 |
| **Tier 2: Desktop/Local Server** | Privacy-sensitive, LAN | Go HTTP server | base/small |
| **Tier 3: Remote Server** | Maximum quality | Cloud-hosted Go server | medium/large-v3 |

**Key Insight:** The project constraint "ability to install and deploy on Android, iOS, Windows, Mac and Linux" doesn't mandate identical architecture on all platforms. A hybrid approach matches capabilities to platform constraints.

---

### Offline Requirements Analysis

**Research Date:** 2026-01-19
**Sources:** [Simform Offline Architecture](https://www.simform.com/blog/offline-mobile-app-architectures-ios-android/), Research 003 Findings

#### Component-by-Component Assessment

| Component | Offline Feasible? | Engine | Notes |
|-----------|-------------------|--------|-------|
| **STT** | ✅ Yes | sherpa-onnx / whisper.cpp | Full offline with local models |
| **Translation (X→English)** | ✅ Yes | Whisper translate mode | Built into STT engine |
| **Translation (X→Y)** | ⚠️ Partial | LibreTranslate self-hosted | Requires server; no pure Go option |
| **TTS** | ✅ Yes | sherpa-onnx Piper | 30+ languages, offline |
| **VAD** | ✅ Yes | sherpa-onnx | Built-in |

#### Offline Architecture Requirements

Per [Simform research](https://www.simform.com/blog/offline-mobile-app-architectures-ios-android/), offline mobile architectures require:
1. **Local data storage** - SQLite (already a project dependency)
2. **Cache management** - For model files and user data
3. **Sync strategies** - For optional cloud connectivity
4. **Graceful degradation** - When features require connectivity

#### Recommendation: Offline-First with Cloud Enhancement

| Feature | Offline | Online Enhancement |
|---------|---------|-------------------|
| STT | tiny/base models | large-v3 via remote server |
| Translation | X→English via Whisper | X→Y via LibreTranslate/Cloud API |
| TTS | Piper voices | Cloud voices (optional) |

This satisfies the cross-platform deployment constraint while providing best-of-both-worlds experience.

---

### Multi-Language Model Strategy

**Research Date:** 2026-01-19
**Sources:** Research 001 Memory Analysis, [sherpa-onnx Model Zoo](https://k2-fsa.github.io/sherpa/onnx/tts/index.html)

#### Model Size vs Platform Matrix

| Model | File Size | Runtime Memory | Desktop | Mobile | Server |
|-------|-----------|----------------|---------|--------|--------|
| tiny.en (q5_1) | ~15MB | ~50MB | ✅ | ✅ | ✅ |
| tiny (multilingual, q5_1) | ~15MB | ~50MB | ✅ | ✅ | ✅ |
| base.en (q5_1) | ~26MB | ~100MB | ✅ | ✅ | ✅ |
| base (multilingual, q5_1) | ~26MB | ~100MB | ✅ | ✅ | ✅ |
| small | ~460MB | ~1GB+ | ✅ | ⚠️ High-end only | ✅ |
| medium | ~1.5GB | ~2.6GB | ✅ | ❌ | ✅ |
| large-v3 | ~3GB | ~3.3GB+ | ⚠️ 8GB+ RAM | ❌ | ✅ |

#### Recommended Strategy: On-Demand Download with Device Profiling

```
[App Launch] → [Profile Device] → [Suggest Optimal Model] → [Download if Needed]
```

**Device Profiling Criteria:**
- Available RAM
- Storage capacity
- Platform (iOS vs Android vs Desktop)
- User language preferences

**Download Strategy:**
1. **Bundle:** None (minimize app store binary size)
2. **First Run:** Download base.q5_1 multilingual (~26MB)
3. **Optional:** User can download larger models on desktop
4. **Storage Location:** Platform-specific cache directory

**Version Management:**
- Model version tracked in SQLite metadata table
- Server provides model manifest with version + checksum
- Delta updates not viable for GGML format; full model replacement

#### TTS Voice Strategy (via sherpa-onnx Piper)

| Language | Voice Size | Quality |
|----------|------------|---------|
| English (US) | ~50MB | High |
| German | ~50MB | High |
| Spanish | ~50MB | High |
| (30+ languages available) | 20-100MB each | Good-High |

**Recommendation:** Download voices on-demand based on user's TTS target language settings.

---

### Web Interface Architecture

**Research Date:** 2026-01-19
**Sources:** [Deepgram Streaming API](https://deepgram.com/learn/streaming-speech-recognition-api), [What PWA Can Do Today](https://whatpwacando.today/speech-recognition/), [MDN PWA Guide](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps)

#### Communication Protocol Analysis

| Protocol | Latency | Bidirectional | Use Case |
|----------|---------|---------------|----------|
| **REST API** | High (handshake per request) | No | Batch transcription, config |
| **WebSocket** | Low (~300ms) | Yes | **Real-time audio streaming** |
| **WebRTC** | Lowest (~150ms) | Yes | Peer-to-peer, complex setup |
| **SSE** | Medium | Server→Client only | Progress updates |

**Key Finding:** For real-time STT, [WebSockets are essential](https://deepgram.com/learn/streaming-speech-recognition-api). REST APIs are only suitable for batch processing. Audio should be chunked in 100-200ms segments for optimal latency.

#### Web Interface Options Evaluated

| Option | Offline UI | Offline STT | Complexity | Mobile UX |
|--------|------------|-------------|------------|-----------|
| **Embedded (Go templates)** | ✅ | Requires server | Low | Basic |
| **SPA + WebSocket** | ✅ (cached) | Requires server | Medium | Good |
| **PWA + WebSocket** | ✅ (service worker) | ⚠️ Requires server | Medium | Excellent |
| **PWA + Local STT** | ✅ | ❌ No browser support | High | Excellent |

**Critical PWA Limitation:**
Per [whatpwacando.today](https://whatpwacando.today/speech-recognition/), browser-native speech recognition (Web Speech API) **requires internet connectivity** - no browser vendor ships offline-capable speech recognition. For true offline STT, the server must be local or embedded.

#### Recommended Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Web Interface (SPA/PWA)                   │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ Audio Input │→ │ WebSocket    │→ │ Results Display     │ │
│  │ (MediaRecorder)│ │ Client     │  │ (Transcription/TTS) │ │
│  └─────────────┘  └──────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            ↕ WSS
┌─────────────────────────────────────────────────────────────┐
│                    Go Server                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ WebSocket   │→ │ Audio Buffer │→ │ sherpa-onnx/whisper │ │
│  │ Handler     │  │ (VAD chunks) │  │ Processing          │ │
│  └─────────────┘  └──────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

**Technology Recommendations:**

| Component | Recommendation | Rationale |
|-----------|----------------|-----------|
| UI Framework | Vanilla JS or Preact | Minimal bundle, fast load |
| Real-time Comm | WebSocket (gorilla/websocket) | Low latency, bidirectional |
| Audio Capture | MediaRecorder API | Browser standard |
| Audio Format | PCM 16kHz mono | Whisper optimal input |
| Styling | Tailwind or minimal CSS | Responsive, maintainable |

#### API Contract (Draft)

**WebSocket Endpoints:**
- `ws://host/api/v1/stt/stream` - Real-time STT
- `ws://host/api/v1/tts/stream` - Streaming TTS output

**REST Endpoints:**
- `GET /api/v1/models` - List available models
- `POST /api/v1/models/download` - Trigger model download
- `GET /api/v1/languages` - Supported language list
- `POST /api/v1/transcribe` - Batch transcription (file upload)

---

## Blockers Identified

**B1: Stakeholder Decisions Required**
The following questions require product/stakeholder input (not purely technical):

| Decision | Options | Recommended Default | Impact |
|----------|---------|---------------------|--------|
| Primary deployment model | Embedded / Local Server / Remote / Hybrid | **Hybrid** (see rationale below) | All other research |
| Offline requirement | Hard requirement / Preferred / Optional | **Preferred** (offline-first) | 003, 004 |
| Target languages (STT) | English-only / Top 10 / All Whisper languages | **Multilingual (99 languages)** | Model downloads |
| Target languages (TTS) | Subset / All Piper voices | **On-demand download** | Storage budget |
| Web vs Native priority | Web-first / Native-first | **Web-first** | 005 |

**B2: "Lexicon Clients" Undefined**
The project mission references "Lexicon clients" but doesn't define what these are. This affects:
- Whether the server is a standalone product or backend for existing apps
- Integration requirements and API stability concerns
- Authentication/multi-user requirements

**B3: Privacy Requirements Unspecified**
Key question: Can audio data leave the user's device?
- If **yes**: Remote server architecture is viable
- If **no**: Only embedded or local server options are acceptable

This directly impacts whether cloud fallback for translation/higher-quality models is permissible.

---

## Recommendations

### R1: Adopt Hybrid Architecture ⭐ (STRONG)

Based on the research findings across 001-003 and this document, a hybrid architecture best satisfies project constraints:

```
                          ┌─────────────────────────────────────┐
                          │        Lexicon Local Server         │
                          │         (Go + sherpa-onnx)          │
                          └─────────────────────────────────────┘
                                           │
              ┌────────────────────────────┼────────────────────────────┐
              ▼                            ▼                            ▼
    ┌─────────────────┐          ┌─────────────────┐          ┌─────────────────┐
    │  Desktop/Server │          │   Mobile Apps   │          │   Web Clients   │
    │  (Full Server)  │          │ (Native + API)  │          │  (SPA via API)  │
    └─────────────────┘          └─────────────────┘          └─────────────────┘
    • Go binary                  • sherpa-onnx native         • WebSocket to server
    • All model sizes            • tiny/base models           • No local processing
    • Full offline               • Offline capable            • Requires server
    • Local web UI               • Remote server fallback
```

**Rationale:**
1. **Satisfies all platform constraints** - Works on all 5 target platforms
2. **Avoids CGO mobile complexity** - sherpa-onnx handles native bindings
3. **Respects memory limits** - Mobile uses appropriate model sizes
4. **Provides quality tiers** - Server can run large-v3 for best quality
5. **Maintains offline option** - Mobile and desktop can work without network

### R2: Use sherpa-onnx as Primary Engine ⭐ (STRONG)

Consolidating findings from Research 001, 002, and 003:

| whisper.cpp Go Bindings | sherpa-onnx |
|-------------------------|-------------|
| ❌ CGO cross-compilation complexity | ✅ Pre-built for all platforms |
| ❌ 45x performance degradation possible | ✅ Optimized native code |
| ❌ Thread safety crashes (#918) | ✅ Designed for concurrent use |
| ❌ STT only; separate TTS needed | ✅ STT + TTS + VAD unified |
| ❌ Build complexity (cmake + libs) | ✅ Single package import |

**Exception:** If specific whisper.cpp features not in sherpa-onnx are required (e.g., word-level timestamps), use [mutablelogic/go-whisper](https://github.com/mutablelogic/go-whisper) for server deployments only.

### R3: Implement Offline-First with Cloud Enhancement

| Scenario | Behavior |
|----------|----------|
| **Offline** | STT (tiny/base), TTS (Piper), Translation (X→English via Whisper) |
| **LAN Server** | STT (small+), TTS (all voices), Translation (X→English) |
| **Online** | STT (large-v3), TTS (all), Translation (X→any via LibreTranslate/Cloud) |

User settings should control:
- Preferred processing location (local/remote)
- Quality vs latency tradeoff
- Data privacy preferences

### R4: Web Interface as Primary UI

**Recommendation:** Build the web interface first, native mobile UI second.

**Rationale:**
1. Web UI works across all 5 platforms immediately
2. Faster iteration during development
3. Single codebase for UI
4. Mobile apps can embed WebView initially, go native later
5. Aligns with project mission ("web interface for real-time STT")

### R5: Model Download Strategy

**On First Launch:**
1. No models bundled (minimal app size)
2. Detect device capabilities
3. Prompt user to download recommended model
4. Default: `base-q5_1` multilingual (~26MB)

**Model Management:**
- Store in platform-appropriate cache directory
- Track versions in SQLite
- Allow user to download additional models/voices
- Implement background download with progress UI

---

## Dependencies

This research plan is a **dependency for all other research plans**. Decisions made here significantly impact:

| Decision | Impacts |
|----------|---------|
| Deployment model | 001 (Mobile), 002 (Bindings), 005 (Distribution) |
| Offline requirement | 003 (Translation/TTS), 004 (Performance) |
| Model strategy | 001 (Mobile memory), 004 (Performance) |
| Web interface | 002 (API design), 004 (Latency) |

---

## References

**Project Documentation:**
- [x] Project mission statement: .dev/project.md
- [x] Related: 001-mobile-platform-constraints.md
- [x] Related: 002-whisper-go-bindings.md
- [x] Related: 003-translation-tts-engines.md
- [x] Related: 005-cross-platform-distribution.md

**Technical References:**
- [x] sherpa-onnx: https://github.com/k2-fsa/sherpa-onnx
- [x] sherpa-onnx Go bindings: https://pkg.go.dev/github.com/k2-fsa/sherpa-onnx/scripts/go
- [x] sherpa-onnx macOS/iOS: https://pkg.go.dev/github.com/JatosDetking/sherpa-onnx-go-macOS-iOS
- [x] gomobile: https://pkg.go.dev/golang.org/x/mobile/cmd/gomobile
- [x] Go Mobile Wiki: https://go.dev/wiki/Mobile
- [x] gomobile CGO issue: https://github.com/golang/go/issues/35426
- [x] mutablelogic/go-whisper: https://github.com/mutablelogic/go-whisper

**Architecture & Communication:**
- [x] Deepgram Streaming API: https://deepgram.com/learn/streaming-speech-recognition-api
- [x] OpenAI Realtime API vs WebRTC: https://skywork.ai/blog/openai-realtime-api-vs-webrtc-2025-which-to-choose/
- [x] PWA Speech Recognition: https://whatpwacando.today/speech-recognition/
- [x] MDN PWA Guide: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps
- [x] Offline Mobile Architecture: https://www.simform.com/blog/offline-mobile-app-architectures-ios-android/
- [x] iOS/Android localhost networking: https://dev.to/sskinfinite/understanding-localhost-networking-in-ios-simulator-vs-android-emulator-5ekb

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Research complete: Deployment model, offline requirements, model strategy, web interface all analyzed with recommendations |
