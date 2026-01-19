# Research Plan: Translation & TTS Engine Selection

**ID:** 003
**Status:** Ongoing
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
| Engine | Go Integration | Platform Support | Notes |
|--------|----------------|------------------|-------|
| _Pending_ | | | |

### TTS Engine Comparison
| Engine | Go Integration | Platform Support | Notes |
|--------|----------------|------------------|-------|
| _Pending_ | | | |

### Whisper Translation
_Pending research_

---

## Blockers Identified

_None yet_

---

## Recommendations

_Pending completion of research tasks_

---

## References

- LibreTranslate: https://libretranslate.com
- Argos Translate: https://github.com/argosopentech/argos-translate
- Piper TTS: https://github.com/rhasspy/piper
- eSpeak-ng: https://github.com/espeak-ng/espeak-ng
- Coqui TTS: https://github.com/coqui-ai/TTS
- ONNX Runtime Go: https://github.com/yalue/onnxruntime_go

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
