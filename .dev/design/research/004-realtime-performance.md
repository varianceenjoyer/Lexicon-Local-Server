# Research Plan: Real-Time Performance

**ID:** 004
**Status:** Complete
**Impact:** MEDIUM
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Adapted for:** Rust implementation

---

## Overview

This research addresses the latency, throughput, and hardware acceleration requirements for achieving "real-time" speech processing across the STT→Translation→TTS pipeline using Rust.

---

## Key Findings

### Human Latency Expectations

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

### Rust Performance Characteristics

**whisper-rs Performance:**

| Aspect | Characteristic | Notes |
|--------|----------------|-------|
| Native speed | ✅ Near C++ performance | Direct whisper.cpp bindings |
| Memory safety | ✅ Rust guarantees | No Go GC overhead |
| Threading | ✅ Fearless concurrency | Send/Sync traits |
| FFI overhead | Minimal | Direct C bindings via bindgen |

**Comparison to Go bindings:**
- No garbage collection pauses
- No CGO overhead (was up to 45x slower in Go)
- Direct control over memory allocation
- Better SIMD utilization

---

### GPU Acceleration

| Platform | Accelerator | whisper.cpp Flag | Expected Speedup |
|----------|-------------|------------------|------------------|
| macOS | Metal | Default | 2-3x |
| macOS | Core ML | `-DWHISPER_COREML=1` | **3x+** (Apple Neural Engine) |
| Windows/Linux | CUDA | `-DGGML_CUDA=1` | 5-20x (depends on GPU) |
| Linux | Vulkan | `-DGGML_VULKAN=1` | 2-5x |
| Linux | OpenVINO | `-DWHISPER_OPENVINO=1` | Significant |
| Android | NNAPI | Via ONNX EP | Device-specific |
| iOS | CoreML | Via ONNX EP | Device-specific |

**Rust GPU Support:**
- whisper-rs inherits whisper.cpp GPU support via build flags
- `ort` crate supports CUDA, CoreML, DirectML execution providers
- Feature flags control GPU backend compilation

---

### Quantization Impact

| Metric | Improvement |
|--------|-------------|
| Model size | **45% reduction** |
| Latency | **19% reduction** |
| Accuracy (WER) | Maintained |

Quantization (INT8, Q5_1, Q4_0) enables mobile deployment without significant quality loss.

---

### Concurrency Model (Rust)

| Approach | Throughput | Memory | Complexity |
|----------|------------|--------|------------|
| Single context + async queue | 1 request at a time | Low | Simple |
| Arc<Mutex<Context>> pool | N contexts × throughput | Medium | Medium |
| Tokio task per request | True parallelism | High | Medium |
| Actix/Axum workers | Horizontal scaling | High | Low (framework) |

**Rust advantages:**
- `Send + Sync` traits prevent data races at compile time
- No thread safety crashes (unlike whisper.cpp Go bindings #918)
- Tokio async runtime for efficient I/O
- Zero-cost abstractions for concurrent patterns

---

### End-to-End Pipeline Analysis

```
Audio → [VAD] → [STT] → [Translation] → [TTS] → Audio Out
         ~50ms   ~1-3s     ~100-300ms    ~200-500ms
```

**Estimated total latency:** 1.5-4 seconds (speech-to-speech)

| Stage | Latency Range | Rust Implementation |
|-------|---------------|---------------------|
| VAD | 50-100ms | webrtc-vad or silero-vad via ort |
| STT (streaming) | 1-3 seconds | whisper-rs with chunk streaming |
| Translation | ~0ms (Whisper) / 100-300ms (separate) | whisper-rs translate mode / reqwest |
| TTS | 200-500ms | Piper via ort crate |
| Audio buffering | 100-200ms | cpal + ringbuffer |

**Pipeline optimization strategies:**
1. **Async pipeline:** Use tokio channels between stages
2. **Streaming TTS:** Output audio chunks as they're generated
3. **VAD-driven chunking:** Process speech segments, not fixed intervals
4. **Speculative execution:** Begin processing before speech ends

---

## Recommendations

### R1: Define Latency Tiers

| Use Case | Target Latency | Model Choice |
|----------|----------------|--------------|
| Live conversation | <800ms | Tiny/base, streaming |
| Transcription | <5 seconds | Base/small, chunked |
| Offline processing | N/A | Medium+ for quality |

### R2: Use Tokio Async Runtime

For the server component:
```rust
use tokio::sync::mpsc;
use whisper_rs::WhisperContext;

// Pipeline stages as async tasks connected by channels
async fn stt_stage(rx: mpsc::Receiver<AudioChunk>, tx: mpsc::Sender<Text>) { ... }
async fn tts_stage(rx: mpsc::Receiver<Text>, tx: mpsc::Sender<AudioChunk>) { ... }
```

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

1. **Single-user local:** Single context, async I/O
2. **Multi-user server:** Worker pool with `Arc<Mutex<Vec<Context>>>`
3. **High-scale:** Horizontal scaling behind load balancer (Actix/Axum)

### R5: Pipeline Optimization

1. Enable VAD to skip silence (webrtc-vad crate)
2. Use 5-10 second audio chunks
3. Stream TTS output via cpal (don't wait for complete synthesis)
4. If X→English only: skip separate translation (whisper-rs handles it)

---

## References

- whisper-rs: https://github.com/tazz4843/whisper-rs
- whisper.cpp: https://github.com/ggml-org/whisper.cpp
- tokio: https://tokio.rs
- cpal: https://crates.io/crates/cpal
- ort: https://crates.io/crates/ort
- webrtc-vad: https://crates.io/crates/webrtc-vad
- WhisperFlow paper: https://arxiv.org/pdf/2412.11272
- Picovoice latency guide: https://picovoice.ai/blog/latency-in-speech-recognition/
- Human conversational latency: https://arxiv.org/html/2404.16053v1

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial Go-based research |
| 2026-01-19 | Adapted for Rust implementation; replaced Go concurrency patterns with Rust/Tokio approaches |
