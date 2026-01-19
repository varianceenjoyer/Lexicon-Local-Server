# Research Plan: Real-Time Performance

**ID:** 004
**Status:** Complete
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

### Human Latency Expectations

**Research Date:** 2026-01-19
**Sources:** [Picovoice](https://picovoice.ai/blog/latency-in-speech-recognition/), [Phonely](https://www.phonely.ai/blogs/performance-reliability-in-voice-ai-why-sub-second-latency-matters), [arXiv](https://arxiv.org/html/2404.16053v1)

| Metric | Value | Notes |
|--------|-------|-------|
| Human turn-taking gap | ~239ms (English) | One standard deviation: 519ms |
| Human reaction time | ~220ms | Baseline expectation |
| **Ideal response** | **300-500ms** | Feels natural |
| **Acceptable response** | **500-800ms** | Still usable |
| **Unacceptable** | **>800ms** | Feels unnatural, breaks flow |

**Key Insight:** For conversational AI, users have some tolerance for longer delays than human-to-human conversation, but exceeding 800ms total response time significantly degrades the experience.

---

### Whisper Streaming Latency

**Sources:** [WhisperFlow](https://arxiv.org/pdf/2412.11272), [whisper_streaming](https://github.com/ufal/whisper_streaming)

| Implementation | Per-Word Latency | Notes |
|----------------|------------------|-------|
| Standard Whisper | 30 seconds max | Processes complete chunks |
| Whisper-Streaming | **3.3 seconds** | High quality, long-form |
| WhisperFlow | **0.5-1.0 seconds** | 1.6x-4.7x improvement |

**Minimum chunk sizes:**
- 1-2 seconds minimum for reasonable accuracy
- 5-10 seconds optimal for latency/accuracy tradeoff
- VAD (Voice Activity Detection) reduces unnecessary processing

---

### sherpa-onnx Performance

**Sources:** [sherpa-onnx GitHub](https://github.com/k2-fsa/sherpa-onnx), [Issue #2910](https://github.com/k2-fsa/sherpa-onnx/issues/2910)

| Platform | Model | Provider | RTF | Notes |
|----------|-------|----------|-----|-------|
| macOS M2 | FireRedASR | CPU (2 threads) | **0.427** | 31.6s to process 74s audio |
| macOS M2 | FireRedASR | CoreML | **0.470** | Slightly slower than CPU |
| Cortex-M7 | 64KB model | CMSIS-NN | N/A | 30ms inference |
| RISC-V K210 | Keyword spotting | - | N/A | 0.2W power |

**RTF (Real-Time Factor):** <1.0 means faster than real-time. An RTF of 0.427 means 1 second of audio processes in 0.427 seconds.

**Key Insight:** sherpa-onnx achieves 2.3x real-time on CPU for small models. For streaming STT, this leaves headroom for translation and TTS stages.

---

### GPU Acceleration

**Sources:** [whisper.cpp](https://github.com/ggml-org/whisper.cpp), [ONNX Runtime Mobile](https://onnxruntime.ai/docs/tutorials/mobile/)

| Platform | Accelerator | Build Flag | Expected Speedup |
|----------|-------------|------------|------------------|
| macOS | Metal | Default | 2-3x |
| macOS | Core ML | `-DWHISPER_COREML=1` | **3x+** (Apple Neural Engine) |
| Windows/Linux | CUDA | `-DGGML_CUDA=1` | 5-20x (depends on GPU) |
| Linux | Vulkan | `-DGGML_VULKAN=1` | 2-5x |
| Linux | OpenVINO | `-DWHISPER_OPENVINO=1` | Significant |
| Android | NNAPI | ONNX EP | Device-specific |
| iOS | CoreML | ONNX EP | Device-specific |

**Mobile GPU Notes:**
- ONNX Runtime supports NNAPI (Android) and CoreML (iOS) execution providers
- Performance is device and model specific
- Quantized models perform well on CPU, may not benefit from GPU
- Start with CPU/XNNPACK; only use NNAPI/CoreML if CPU doesn't meet needs

---

### Quantization Impact

**Source:** [arXiv](https://arxiv.org/html/2503.09905v1)

| Metric | Improvement |
|--------|-------------|
| Model size | **45% reduction** |
| Latency | **19% reduction** |
| Accuracy (WER) | Maintained |

Quantization (INT8, Q5_1, Q4_0) enables mobile deployment without significant quality loss.

---

### Concurrency Model

**Source:** Research findings from 002-whisper-go-bindings.md

| Approach | Throughput | Memory | Complexity |
|----------|------------|--------|------------|
| Single context + queue | 1 request at a time | Low | Simple |
| Context pool + mutex | N contexts × throughput | Medium (N × model) | Medium |
| Worker processes | True parallelism | High (model per process) | High |
| Distributed (load balancer) | Horizontal scaling | High | High |

**Critical:** whisper.cpp has thread safety issues (#918, #341). Do NOT share contexts across goroutines. Use request serialization or separate processes.

**sherpa-onnx advantage:** Designed for streaming with built-in concurrency support.

---

### End-to-End Pipeline Analysis

```
Audio → [VAD] → [STT] → [Translation] → [TTS] → Audio Out
         ~50ms   ~1-3s     ~100-300ms    ~200-500ms
```

**Estimated total latency:** 1.5-4 seconds (speech-to-speech)

| Stage | Latency Range | Notes |
|-------|---------------|-------|
| VAD | 50-100ms | Lightweight, continuous |
| STT (streaming) | 1-3 seconds | Depends on chunk size |
| Translation | 100-300ms | Whisper X→EN: ~0ms (same model) |
| TTS | 200-500ms | Piper/sherpa-onnx |
| Audio buffering | 100-200ms | Playback latency |

**Optimization strategies:**
1. **Pipeline parallelism:** Start TTS while translation completes
2. **Streaming TTS:** Output audio chunks as they're generated
3. **VAD-driven chunking:** Process speech segments, not fixed intervals
4. **Speculative execution:** Begin processing before speech ends

---

## Blockers Identified

**B1: Latency Target Undefined**
"Real-time" is not defined in project requirements. For conversational use, target <800ms total; for transcription, 3-5 second lag is acceptable.

**B2: Mobile GPU Inconsistency**
NNAPI and CoreML performance varies significantly by device. Some operations fall back to CPU, negating benefits. Testing on target devices is required.

---

## Recommendations

### R1: Define Latency Tiers

| Use Case | Target Latency | Model Choice |
|----------|----------------|--------------|
| Live conversation | <800ms | Tiny/base, streaming |
| Transcription | <5 seconds | Base/small, chunked |
| Offline processing | N/A | Medium+ for quality |

### R2: Use sherpa-onnx for Streaming

sherpa-onnx provides:
- Built-in streaming support
- Pre-built Go bindings
- Integrated VAD for efficient chunking
- Same library for STT + TTS (reduces pipeline overhead)

### R3: Target Hardware Requirements

**Minimum (mobile):**
- 2GB RAM (tiny/base quantized models)
- ARM64 CPU with NEON
- RTF <1.0 achievable on most modern phones

**Recommended (desktop):**
- 8GB RAM
- GPU with Metal/CUDA for 3-10x speedup
- Can run small/medium models

### R4: Concurrency Strategy

1. **Single-user local:** Single context, no concurrency needed
2. **Multi-user server:** Context pool with N workers (N = CPU cores / 2)
3. **High-scale:** Horizontal scaling with load balancer

### R5: Pipeline Optimization

1. Enable VAD to skip silence
2. Use 5-10 second audio chunks
3. Stream TTS output (don't wait for complete synthesis)
4. If X→English only: skip separate translation (Whisper handles it)

---

## References

- [x] whisper.cpp benchmarks: https://github.com/ggml-org/whisper.cpp
- [x] WhisperFlow paper: https://arxiv.org/pdf/2412.11272
- [x] whisper_streaming: https://github.com/ufal/whisper_streaming
- [x] sherpa-onnx: https://github.com/k2-fsa/sherpa-onnx
- [x] Picovoice latency guide: https://picovoice.ai/blog/latency-in-speech-recognition/
- [x] ONNX Runtime mobile: https://onnxruntime.ai/docs/tutorials/mobile/
- [x] Quantization research: https://arxiv.org/html/2503.09905v1
- [x] Human conversational latency: https://arxiv.org/html/2404.16053v1

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Research complete: Latency requirements, GPU acceleration, concurrency models, pipeline analysis documented |
