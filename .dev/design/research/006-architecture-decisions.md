# Research Plan: Architecture Decisions

**ID:** 006
**Status:** Ongoing
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
_Pending stakeholder input and research_

### Offline Requirements
_Pending clarification_

### Model Strategy
_Pending requirements_

### Web Interface
_Pending design decisions_

---

## Blockers Identified

**B1: Stakeholder Decisions Required**
Several questions in this research require product/stakeholder decisions rather than pure technical research:
- Primary deployment model
- Offline requirements
- Target languages

These should be escalated for decision before detailed technical work proceeds.

---

## Recommendations

_Pending completion of research tasks_

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

- Project mission statement: .dev/project.md
- Related: 001-mobile-platform-constraints.md
- Related: 005-cross-platform-distribution.md

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
