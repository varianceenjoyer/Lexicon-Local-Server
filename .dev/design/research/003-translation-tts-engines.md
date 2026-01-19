# Research Plan: Translation & TTS Engine Selection

**ID:** 003
**Status:** Complete
**Impact:** MEDIUM-HIGH
**Created:** 2026-01-19
**Last Updated:** 2026-01-19

---

## Overview

The project requires translation and text-to-speech capabilities but no specific engines are specified. This research evaluates options across offline capability, quality, size, platform support, and Go integration feasibility.

---

## Research Questions

### RQ-1: Translation Engine Selection

**Problem Statement:**
Multiple translation engine options exist with vastly different tradeoffs. Which is viable for local deployment across desktop and mobile platforms?

**Candidates to Evaluate:**

| Engine | Type | Offline | Approx Size | Quality |
|--------|------|---------|-------------|---------|
| LibreTranslate | Local API | Yes | ~1-5GB/lang | Good |
| Argos Translate | Python lib | Yes | ~100-500MB/lang | Good |
| NLLB (Meta) | ML model | Yes | 600MB-3GB | Excellent |
| OpenNMT | Toolkit | Yes | Varies | Good |
| Cloud APIs (Google, DeepL) | Remote | No | N/A | Excellent |
| Whisper (translation mode) | ML model | Yes | Same as STT | Good |

**Areas of Inquiry:**
- [ ] What languages are required? (Drives model selection)
- [ ] Is fully offline operation mandatory?
- [ ] What is acceptable translation latency?
- [ ] Can Whisper's built-in translation mode suffice (X→English only)?
- [ ] What are the memory/storage constraints per platform?

**Go Integration Options:**
- [ ] HTTP API wrapper (LibreTranslate, cloud)
- [ ] CGO bindings to C/C++ models
- [ ] Subprocess/IPC to Python-based engines
- [ ] Pure Go implementations (if any exist)

**Success Criteria:**
- Decision matrix comparing viable options
- Identified: Go integration strategy for chosen engine
- Documented: Size/quality/platform tradeoffs

---

### RQ-2: TTS Engine Selection

**Problem Statement:**
Multiple TTS engines exist with different quality, size, and platform support profiles. Which is appropriate for local deployment?

**Candidates to Evaluate:**

| Engine | Type | Offline | Voice Quality | Size/Voice |
|--------|------|---------|---------------|------------|
| Piper | ONNX | Yes | Good-Excellent | 20-100MB |
| eSpeak-ng | Formant | Yes | Robotic | <10MB |
| Coqui TTS | ML | Yes | Excellent | 100MB-1GB |
| Mimic 3 | ONNX | Yes | Good | 50-200MB |
| System TTS | OS API | N/A | Varies | N/A |
| Cloud (Google, AWS) | Remote | No | Excellent | N/A |

**Areas of Inquiry:**
- [ ] What voice quality level is acceptable?
- [ ] How many languages/voices are needed?
- [ ] Is real-time synthesis required (streaming audio output)?
- [ ] What are the CPU/memory requirements per engine?
- [ ] Can system TTS APIs be used as fallback?

**Go Integration Options:**
- [ ] CGO bindings (eSpeak-ng has C API)
- [ ] ONNX runtime in Go (Piper, Mimic 3)
- [ ] HTTP API wrapper (Coqui server mode)
- [ ] System TTS APIs via platform-specific code

**Success Criteria:**
- Decision matrix comparing viable options
- Identified: Go integration strategy for chosen engine
- Documented: Voice quality vs size tradeoffs

---

### RQ-3: Component Coherence

**Problem Statement:**
Do proven Go bindings or integration patterns exist for the chosen engines, or does this require significant custom CGO work?

**Areas of Inquiry:**
- [ ] Are there existing Go bindings for candidate engines?
- [ ] What is the maintenance status of any Go bindings?
- [ ] Is subprocess/IPC a viable integration pattern?
- [ ] What is the complexity of custom CGO binding work?
- [ ] Are ONNX-based engines easier to integrate (via go-onnxruntime)?

**Existing Go Projects to Evaluate:**
- [ ] go-piper (if exists)
- [ ] go-espeak (if exists)
- [ ] onnxruntime-go
- [ ] Projects using translation/TTS in Go

**Success Criteria:**
- Documented: Existing Go integration options
- Estimated: Custom binding work if needed
- Recommendation: Integration architecture

---

### RQ-4: Whisper Translation Mode Viability

**Problem Statement:**
Whisper can translate speech in many languages directly to English text. If English is the primary target language, this could eliminate a separate translation engine.

**Areas of Inquiry:**
- [ ] What source languages does Whisper support for translation?
- [ ] What is the quality compared to dedicated translation?
- [ ] Can Whisper translate to languages other than English?
- [ ] What is the latency impact of translation mode?
- [ ] Is this exposed in the Go bindings?

**Success Criteria:**
- Documented: Whisper translation capabilities
- Recommendation: Use Whisper translation vs separate engine

---

## Discovery Tasks

### Phase 1: Requirements Clarification
- [ ] **T1.1:** Define target language pairs for translation
- [ ] **T1.2:** Define acceptable quality level for TTS
- [ ] **T1.3:** Clarify offline requirement (hard vs preferred)
- [ ] **T1.4:** Establish size budget per platform

### Phase 2: Engine Evaluation
- [ ] **T2.1:** Evaluate LibreTranslate (self-hosted) for size, quality, speed
- [ ] **T2.2:** Evaluate Argos Translate integration options
- [ ] **T2.3:** Evaluate Piper TTS quality and ONNX integration
- [ ] **T2.4:** Evaluate Whisper translation mode quality
- [ ] **T2.5:** Test eSpeak-ng as minimal fallback TTS

### Phase 3: Go Integration Research
- [ ] **T3.1:** Search for existing Go bindings for all candidates
- [ ] **T3.2:** Evaluate onnxruntime-go maturity
- [ ] **T3.3:** Prototype HTTP API wrapper approach
- [ ] **T3.4:** Assess CGO binding complexity for top candidates

---

## Findings

### Translation Engine Comparison

**Research Date:** 2026-01-19

| Engine | Go Integration | Platform Support | Offline | Notes |
|--------|----------------|------------------|---------|-------|
| **Whisper (translation)** | ✅ Native (via STT bindings) | All | Yes | X→English only, no separate engine needed |
| **LibreTranslate** | ✅ HTTP client ([SnakeSel/libretranslate](https://github.com/SnakeSel/libretranslate)) | Server-only | Yes | Requires self-hosted Python server |
| **Argos Translate** | ⚠️ Subprocess only | Python required | Yes | Powers LibreTranslate, no direct Go bindings |
| **NLLB (Meta)** | ❌ None | Python/C++ | Yes | Would require custom ONNX integration |
| **Cloud APIs** | ✅ HTTP client | N/A | No | Google, DeepL, Azure - excellent quality |

**Key Finding:** No pure Go offline translation library exists. All offline options require either:
1. Using Whisper's built-in translation (limited to X→English)
2. Running a Python-based server (LibreTranslate/Argos) with Go HTTP client
3. Cloud API fallback

### TTS Engine Comparison

**Research Date:** 2026-01-19

| Engine | Go Integration | Platform Support | Quality | Size | Notes |
|--------|----------------|------------------|---------|------|-------|
| **sherpa-onnx + Piper** | ✅ Native Go bindings | All 5 platforms | Good-Excellent | 20-100MB/voice | **Recommended** - pre-built, 30+ languages |
| **Piper (standalone)** | ⚠️ ONNX runtime needed | Desktop | Good-Excellent | 20-100MB/voice | [piper-onnx](https://github.com/thewh1teagle/piper-onnx) Python wrapper exists |
| **eSpeak-ng** | ⚠️ CGO required | All | Robotic | <10MB | C API available, low quality |
| **Coqui TTS** | ❌ HTTP only | Server | Excellent | 100MB-1GB | Python server mode |
| **System TTS** | ⚠️ Platform-specific | Per-platform | Varies | N/A | macOS: NSSpeechSynthesizer, Windows: SAPI |

### sherpa-onnx Discovery ⭐

**Critical Finding:** [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) provides a unified solution for STT + TTS + VAD:

| Feature | Support |
|---------|---------|
| Go bindings | ✅ Pre-built for Windows, Linux, macOS, iOS |
| Android | ✅ Native support |
| iOS | ✅ Native support |
| Streaming STT | ✅ Both streaming and non-streaming |
| TTS (Piper) | ✅ VITS models, 30+ languages |
| VAD | ✅ Built-in |
| Offline | ✅ Full offline operation |
| Models | Zipformer, Paraformer, Whisper, Piper |

**Go API Examples Available:**
- `go-api-examples/real-time-speech-recognition-from-microphone`
- `go-api-examples/streaming-decode-files`
- `go-api-examples/non-streaming-tts`

### Whisper Translation

**Status:** ✅ Viable for X→English translation

- **Source languages:** All 99 languages Whisper supports
- **Target language:** English only (Whisper limitation)
- **Go bindings:** `SetTranslate(true)` method available
- **Latency:** Same as transcription (no additional overhead)
- **Quality:** Good - competitive with dedicated translation for many language pairs

**Limitation:** Cannot translate TO non-English languages. For multilingual output, a separate translation engine is required.

---

## Blockers Identified

**B1: No Pure Go Offline Translation**
All offline translation options require either Python dependencies or are limited to English output (Whisper). If multilingual translation output is required offline, this needs architectural decision.

**B2: Translation Target Languages Unclear**
The project requirements don't specify required translation directions. If only X→English is needed, Whisper suffices. If bidirectional translation is needed, LibreTranslate server or cloud fallback is required.

---

## Recommendations

**R1: Use sherpa-onnx for TTS** ⭐
- Pre-built Go bindings eliminate build complexity
- Piper TTS models provide good quality across 30+ languages
- Same library handles STT, eliminating multiple dependencies
- Native mobile support (critical for project constraints)

**R2: Use Whisper Translation for X→English**
- Already available through STT integration
- No additional dependencies
- Good quality for most language pairs

**R3: For Multilingual Translation Output**
If translation to non-English languages is required:
- **Option A:** Self-hosted LibreTranslate server + Go HTTP client (offline capable)
- **Option B:** Cloud API fallback (Google Translate, DeepL) when online
- **Option C:** Defer multilingual output to future version

**R4: Architecture Recommendation**
Consider a **tiered approach**:
1. **Core (offline):** sherpa-onnx for STT + TTS + Whisper translation (X→English)
2. **Extended (server):** LibreTranslate for multilingual translation
3. **Fallback (online):** Cloud APIs for highest quality when available

---

## References

- [x] LibreTranslate: https://libretranslate.com
- [x] LibreTranslate Go client: https://github.com/SnakeSel/libretranslate
- [x] Argos Translate: https://github.com/argosopentech/argos-translate
- [x] Piper TTS: https://github.com/rhasspy/piper
- [x] piper-onnx (Python): https://github.com/thewh1teagle/piper-onnx
- [x] eSpeak-ng: https://github.com/espeak-ng/espeak-ng
- [ ] Coqui TTS: https://github.com/coqui-ai/TTS (not evaluated in depth)
- [x] ONNX Runtime Go: https://github.com/yalue/onnxruntime_go
- [x] sherpa-onnx (recommended): https://github.com/k2-fsa/sherpa-onnx
- [x] sherpa-onnx TTS docs: https://k2-fsa.github.io/sherpa/onnx/tts/index.html

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Research complete: sherpa-onnx identified as unified STT+TTS solution, translation options documented |
