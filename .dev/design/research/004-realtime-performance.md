# Research Plan: Real-Time Performance

**ID:** 004
**Status:** Ongoing
**Impact:** MEDIUM
**Created:** 2026-01-19
**Last Updated:** 2026-01-19

---

## Overview

This research addresses the latency, throughput, and hardware acceleration requirements for achieving "real-time" speech processing across the STT→Translation→TTS pipeline.

---

## Research Questions

### RQ-1: Latency Requirements & Whisper Characteristics

**Problem Statement:**
"Real-time" STT has different meanings. Whisper inherently processes audio in chunks (up to 30 seconds). What latency is acceptable, and how can it be minimized?

**Areas of Inquiry:**
- [ ] What is the user-perceived latency requirement? (<1s? <3s? <5s?)
- [ ] What is Whisper's minimum chunk size for accurate transcription?
- [ ] Is word-level streaming required, or is segment-level acceptable?
- [ ] What is the end-to-end latency target for STT→Translate→TTS pipeline?
- [ ] How does model size affect inference latency?

**Benchmarks Needed:**
- [ ] Tiny model latency vs base vs small on CPU
- [ ] Impact of chunk size on accuracy
- [ ] Cumulative pipeline latency measurement

**Success Criteria:**
- Defined: Latency target with rationale
- Documented: Whisper chunking strategy
- Baseline: Latency measurements per model size

---

### RQ-2: Hardware Acceleration

**Problem Statement:**
whisper.cpp supports CUDA, Metal, and OpenCL. What is the Go binding support for GPU acceleration on each target platform?

**Acceleration Options by Platform:**

| Platform | GPU Option | Expected Speedup |
|----------|-----------|------------------|
| Windows | CUDA (NVIDIA) | 5-20x |
| Windows | DirectML? | TBD |
| macOS | Metal | 5-10x |
| Linux | CUDA | 5-20x |
| Linux | OpenCL | 2-5x |
| iOS | Metal/CoreML | TBD |
| Android | OpenCL/NNAPI | TBD |

**Areas of Inquiry:**
- [ ] Do whisper.cpp Go bindings expose GPU acceleration?
- [ ] What is the CPU-only performance baseline?
- [ ] Is GPU memory a constraint for model loading?
- [ ] Can GPU/CPU processing happen concurrently (for multiple clients)?
- [ ] Are quantized models GPU-compatible?

**Success Criteria:**
- Documented: GPU support status per platform
- Baseline: CPU vs GPU inference benchmarks
- Recommendation: Minimum hardware requirements

---

### RQ-3: Concurrent Client Capacity

**Problem Statement:**
For N simultaneous clients, what is the compute/memory scaling? Should models be loaded per-client or shared?

**Areas of Inquiry:**
- [ ] Is the whisper.cpp context stateful or stateless?
- [ ] Can a single model serve multiple concurrent requests?
- [ ] What is the memory overhead per additional client?
- [ ] Is request queuing needed, or can requests process in parallel?
- [ ] What is the maximum concurrent capacity on reference hardware?

**Scaling Models to Evaluate:**
1. **Single context, queued requests** - Sequential processing
2. **Context pool** - Pre-allocated contexts, limited parallelism
3. **Context per request** - Maximum parallelism, high memory
4. **Distributed** - Multiple server instances behind load balancer

**Success Criteria:**
- Documented: Concurrency model recommendation
- Baseline: Clients per server capacity estimate
- Identified: Scaling strategy for high load

---

### RQ-4: End-to-End Pipeline Performance

**Problem Statement:**
The full pipeline is STT → Translation → TTS. Each component adds latency. What is the achievable end-to-end performance?

**Pipeline Stages:**

```
Audio In → [Chunking] → [STT] → [Translation] → [TTS] → Audio Out
              ~0ms       ?ms        ?ms           ?ms
```

**Areas of Inquiry:**
- [ ] Can pipeline stages be parallelized (streaming)?
- [ ] What buffering strategy minimizes perceived latency?
- [ ] Is speculative execution viable (start TTS before translation completes)?
- [ ] What is the audio output latency requirement?

**Success Criteria:**
- Documented: Per-stage latency budget
- Recommendation: Pipeline optimization strategy
- Identified: Bottleneck components

---

## Discovery Tasks

### Phase 1: Baseline Measurements
- [ ] **T1.1:** Benchmark whisper.cpp tiny/base/small on reference CPU
- [ ] **T1.2:** Measure minimum viable chunk size for accuracy
- [ ] **T1.3:** Profile memory usage during inference
- [ ] **T1.4:** Test concurrent inference with multiple contexts

### Phase 2: Acceleration Testing
- [ ] **T2.1:** Build whisper.cpp with CUDA on Windows/Linux
- [ ] **T2.2:** Build whisper.cpp with Metal on macOS
- [ ] **T2.3:** Benchmark GPU vs CPU inference
- [ ] **T2.4:** Test GPU memory limits with larger models

### Phase 3: Pipeline Integration
- [ ] **T3.1:** Build end-to-end prototype (STT only first)
- [ ] **T3.2:** Add translation stage and measure overhead
- [ ] **T3.3:** Add TTS stage and measure total latency
- [ ] **T3.4:** Optimize buffering and streaming strategy

---

## Findings

### Latency Baselines
| Model | CPU Time (1s audio) | GPU Time | Platform |
|-------|---------------------|----------|----------|
| _Pending_ | | | |

### Concurrency Testing
_Pending research_

### Pipeline Analysis
_Pending research_

---

## Blockers Identified

_None yet_

---

## Recommendations

_Pending completion of research tasks_

---

## References

- whisper.cpp benchmarks: https://github.com/ggerganov/whisper.cpp#benchmarks
- GGML quantization: https://github.com/ggerganov/ggml
- Audio latency best practices: TBD

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
