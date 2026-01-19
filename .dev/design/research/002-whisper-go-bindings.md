# Research Plan: whisper.cpp Go Bindings Maturity

**ID:** 002
**Status:** Complete
**Impact:** HIGH
**Created:** 2026-01-19
**Last Updated:** 2026-01-19

---

## Overview

This research assesses the maturity, completeness, and production-readiness of the official whisper.cpp Go bindings for use in a real-time transcription server.

---

## Research Questions

### RQ-1: Binding Completeness

**Problem Statement:**
The official Go bindings may not expose all whisper.cpp features needed for production use. What features are available, what's missing, and what would require custom binding work?

**Areas of Inquiry:**
- [ ] What API surface does the Go binding expose?
- [ ] Is streaming/incremental audio processing supported?
- [ ] Is Voice Activity Detection (VAD) exposed?
- [ ] Is language detection/auto-detection available?
- [ ] Are timestamps (word-level, segment-level) accessible?
- [ ] Can model parameters be configured (temperature, beam size)?
- [ ] Is there support for different output formats (text, SRT, VTT)?

**Existing Solutions to Evaluate:**
- Official whisper.cpp Go bindings: `github.com/ggerganov/whisper.cpp/bindings/go`
- Alternative Go bindings (if any)
- Direct CGO wrapping of whisper.cpp C API

**Success Criteria:**
- Documented: Complete API inventory of Go bindings
- Identified: Feature gaps requiring custom work
- Decision: Use official bindings vs fork/custom

---

### RQ-2: Streaming/Real-Time Support

**Problem Statement:**
Real-time STT requires processing audio chunks incrementally, not waiting for complete files. Whisper inherently processes 30-second windows. Does the Go binding support streaming patterns?

**Areas of Inquiry:**
- [ ] Does the binding support feeding audio in chunks?
- [ ] Is there a streaming API or only batch file processing?
- [ ] What is the minimum audio buffer size for processing?
- [ ] Can partial results be obtained before chunk completion?
- [ ] How does the binding handle continuous audio streams?

**Existing Solutions to Evaluate:**
- whisper.cpp streaming example (C/C++)
- whisper_full_with_state API usage
- Existing streaming implementations in other languages

**Success Criteria:**
- Confirmed: Streaming capability exists or can be implemented
- Documented: Recommended audio buffer strategy
- Identified: Latency characteristics of streaming approach

---

### RQ-3: Thread Safety

**Problem Statement:**
A server handling multiple concurrent clients needs thread-safe bindings. Are the Go bindings safe for concurrent use, or does the underlying C code have global state issues?

**Areas of Inquiry:**
- [ ] Is the whisper context thread-safe?
- [ ] Can multiple contexts run concurrently?
- [ ] Are there global locks or shared state in whisper.cpp?
- [ ] What is the recommended concurrency pattern (context-per-request, pool)?
- [ ] How does Go's goroutine model interact with CGO threading?

**Existing Solutions to Evaluate:**
- whisper.cpp thread safety documentation
- Go binding source code review for mutexes/locks
- CGO threading best practices

**Success Criteria:**
- Documented: Thread safety guarantees
- Recommendation: Concurrency model for server use
- Identified: Any serialization bottlenecks

---

### RQ-4: Build Complexity

**Problem Statement:**
whisper.cpp performance depends on optimized math libraries (BLAS, Accelerate, CUDA). What is the build matrix complexity for cross-platform deployment?

**Areas of Inquiry:**
- [ ] What are the required dependencies per platform?
  - [ ] Windows: OpenBLAS? Intel MKL?
  - [ ] macOS: Accelerate framework?
  - [ ] Linux: OpenBLAS? CUDA?
  - [ ] iOS: Accelerate?
  - [ ] Android: ?
- [ ] How are dependencies specified in Go build process?
- [ ] Are prebuilt binaries available?
- [ ] What is the performance impact of missing optimizations?
- [ ] Does the binding support CUDA/Metal acceleration?

**Existing Solutions to Evaluate:**
- whisper.cpp CMake build configuration
- Go binding build instructions
- CI/CD examples in whisper.cpp repo

**Success Criteria:**
- Documented: Build requirements per target platform
- Documented: Dependency acquisition strategy
- Identified: GPU acceleration options per platform

---

## Discovery Tasks

### Phase 1: Source Analysis
- [ ] **T1.1:** Clone and review whisper.cpp Go bindings source code
- [ ] **T1.2:** Inventory all exported Go functions and types
- [ ] **T1.3:** Map Go API to underlying whisper.cpp C API
- [ ] **T1.4:** Review open issues/PRs in whisper.cpp repo related to Go bindings

### Phase 2: Documentation & Community
- [ ] **T2.1:** Review official whisper.cpp documentation for Go
- [ ] **T2.2:** Search for community usage examples and tutorials
- [ ] **T2.3:** Identify projects using whisper.cpp Go bindings in production
- [ ] **T2.4:** Review streaming implementation discussions

### Phase 3: Technical Validation
- [ ] **T3.1:** Build Go bindings on Windows with OpenBLAS
- [ ] **T3.2:** Build Go bindings on macOS with Accelerate
- [ ] **T3.3:** Implement minimal streaming test
- [ ] **T3.4:** Benchmark concurrent context performance

---

## Findings

### API Surface

**Research Date:** 2026-01-19
**Source:** [Official Go Package Documentation](https://pkg.go.dev/github.com/ggerganov/whisper.cpp/bindings/go/pkg/whisper)

The official whisper.cpp Go bindings (v0.0.0-20260116121605) expose a comprehensive API:

**Core Interfaces:**
- `Model` - Load models, check multilingual support, enumerate languages
- `Context` - Main processing interface with full configuration options

**Key Exported Methods (Context):**

| Category | Methods |
|----------|---------|
| Language | `SetLanguage()`, `Language()`, `DetectedLanguage()`, `SetTranslate()` |
| Threading | `SetThreads(uint)`, `SetAudioCtx(uint)` |
| Processing | `Process()`, `NextSegment()` |
| VAD | `SetVAD()`, `SetVADThreshold()`, `SetVADMinSpeechMs()`, etc. |
| Model Params | `SetBeamSize()`, `SetTemperature()`, `SetEntropyThold()`, `SetInitialPrompt()` |
| Timestamps | `SetTokenTimestamps()`, `SetTokenThreshold()` |
| Segmentation | `SetSplitOnWord()`, `SetMaxSegmentLength()`, `SetMaxTokensPerSegment()` |

**Data Structures:**
- `Segment` - Contains Num, Start/End duration, Text, Tokens
- `Token` - Contains Id, Text, Probability, Start/End duration

**Checklist Updates:**
- [x] What API surface does the Go binding expose? **Comprehensive - see above**
- [x] Is streaming/incremental audio processing supported? **Yes, via callbacks**
- [x] Is Voice Activity Detection (VAD) exposed? **Yes, full VAD configuration**
- [x] Is language detection/auto-detection available? **Yes, SetLanguage("auto")**
- [x] Are timestamps (word-level, segment-level) accessible? **Yes, Token-level timestamps**
- [x] Can model parameters be configured (temperature, beam size)? **Yes**
- [ ] Is there support for different output formats (text, SRT, VTT)? **Not directly - requires application-level formatting**

### Streaming Capability

**Status:** ✅ Supported via callbacks

The Go bindings support real-time streaming through:

1. **Callback-based streaming:**
   - `SegmentCallback func(Segment)` - Called for each transcribed segment
   - `ProgressCallback func(int)` - Progress reporting during processing
   - `EncoderBeginCallback func() bool` - Control flow (return false to stop)

2. **Sequential retrieval:**
   - `NextSegment()` returns segments one at a time (returns `io.EOF` at end)
   - Recent update allows `NextSegment()` across multiple `Process()` calls

**Latency Characteristics:**
- Minimum viable chunk: ~1-2 seconds for reasonable accuracy
- Typical real-time latency: **2-5 seconds** (inherent to chunked processing)
- Whisper processes up to 30-second windows

**2025 State-of-Art Streaming Solutions (Python-based):**
- **SimulStreaming** - Ultra-low latency using AlignAtt policy (SOTA 2025)
- **WhisperLiveKit** - Intelligent buffering with incremental processing
- **whisper-flow** - Incremental chunk processing framework

These would require Python subprocess or adaptation for Go integration.

**Alternative with HTTP Streaming:**
[mutablelogic/go-whisper](https://github.com/mutablelogic/go-whisper) provides:
- OpenAI-compatible HTTP API
- Streaming transcription endpoint (`?stream=true`)
- Model management
- GPU acceleration (CUDA, Vulkan, Metal)

### Thread Safety

**Status:** ⚠️ CRITICAL - Known Crash Bugs with Concurrent Contexts

Per whisper.h documentation:
> "The interface is thread-safe as long as the same whisper_context is not used by multiple threads concurrently."

**Critical Issues Discovered:**

| Issue | Problem | Status |
|-------|---------|--------|
| [#918](https://github.com/ggml-org/whisper.cpp/issues/918) | Crashes on concurrent usage: `GGML_ASSERT: ggml.c:8386: ne02 == ne12` | **Unresolved** |
| [#341](https://github.com/ggml-org/whisper.cpp/issues/341) | Assertion failures with multiple threads on same model | Workaround exists |

**Workaround via whisper_full_parallel():**
The underlying C API provides `whisper_full_parallel()` which:
- Creates separate contexts sharing the same loaded model
- Allocates independent key-value memory tensors per context
- Allows parallel processing without crashes

**However:** The Go bindings do NOT expose this parallel API. Code comment notes: "We don't do parallel processing at the moment."

**Recommended Concurrency Models for Server:**

| Approach | Pros | Cons |
|----------|------|------|
| Single context, request queue | Simple, no crashes | Sequential, higher latency |
| Context pool + mutex | Bounded parallelism | Complex coordination |
| Separate worker processes | True isolation | High memory (model per process) |

**Bottom Line:** Use request serialization or separate processes. Do NOT attempt concurrent goroutines with shared model/context.

### Build Matrix

| Platform | Math Library | GPU Support | Build Status | Notes |
|----------|--------------|-------------|--------------|-------|
| Windows | OpenBLAS | CUDA, Vulkan | ✅ Feasible | Requires MinGW or MSVC |
| macOS | Accelerate | Metal (native) | ✅ Feasible | Zero extra deps |
| Linux | OpenBLAS | CUDA, Vulkan, OpenVINO | ✅ Feasible | Straightforward |
| iOS | Accelerate | Metal, Core ML | ⚠️ Complex | XCFramework available |
| Android | NDK | OpenCL (limited) | ⚠️ Complex | See examples/whisper.android |

**GPU Acceleration Build Commands:**

```bash
# NVIDIA CUDA
cmake -B build -DGGML_CUDA=1
cmake --build build -j --config Release

# Apple Metal (default on macOS, no flag needed)
cmake -B build
cmake --build build -j --config Release

# Core ML (Apple Neural Engine)
cmake -B build -DWHISPER_COREML=1
cmake --build build -j --config Release

# Vulkan (cross-platform GPU)
cmake -B build -DGGML_VULKAN=1
cmake --build build -j --config Release

# OpenBLAS (CPU acceleration)
cmake -B build -DGGML_BLAS=1
cmake --build build -j --config Release
```

**Go Binding Integration Requirements:**
1. Compile `libwhisper.a` static library using cmake
2. Set `C_INCLUDE_PATH` to whisper.h location
3. Set `LIBRARY_PATH` to compiled library location
4. Enable CGO: `CGO_ENABLED=1`
5. Allow specific flags: `export CGO_CFLAGS_ALLOW="-mfma|-mf16c"`

**Required Libraries (linking):**
```
-ldl -lm -lpthread -lcommon -lcommon-ggml -lggml -lggml-base -lggml-blas -lggml-cpu -lwhisper -lstdc++
```

**Prebuilt Options:**
- XCFramework available for iOS/macOS (no source compilation needed)
- No official prebuilt Go bindings (must compile libwhisper.a)

---

## Blockers Identified

**B1: Severe Performance Degradation**
Testing revealed Go bindings can be **up to 45x slower** than equivalent C++ binaries ([GitHub Discussion #312](https://github.com/ggml-org/whisper.cpp/discussions/312)). Callback overhead is a major factor - one benchmark showed removing progress callbacks reduced processing time from 62 minutes to 14 minutes.

**B2: Mobile Build Complexity**
The whisper.cpp Go bindings require building libwhisper.a and linking multiple GGML libraries. Mobile cross-compilation via gomobile adds significant complexity. Users report "undefined references to ggml methods" issues when linking.

**B3: Build System Migration**
whisper.cpp v1.7.3+ deprecated Makefile builds in favor of CMake, causing migration issues for existing Go binding users.

**B4: Alternative Discovery - sherpa-onnx**
During research, [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) was discovered as a potentially superior alternative:
- Go bindings with **pre-built libraries** for Windows, Linux, macOS, iOS
- Native mobile support (Android, iOS) without CGO build complexity
- STT + TTS + VAD in one unified package
- Streaming support built-in
- Multiple model options (Zipformer, Paraformer, Whisper)
- 12 programming language bindings

---

## Recommendations

**R1: Evaluate sherpa-onnx as Primary STT/TTS Solution** ⭐
Strongly recommended due to:
- Eliminates CGO build complexity for mobile platforms
- Provides unified STT + TTS solution (addresses 003 research as well)
- Pre-built Go packages for all 5 target platforms
- Maintains offline operation capability
- Active development with recent releases

**R2: If whisper.cpp bindings are specifically required:**
- Use [mutablelogic/go-whisper](https://github.com/mutablelogic/go-whisper) for server deployments (includes HTTP streaming API)
- Implement context pooling for concurrent clients
- Desktop-first approach; defer mobile until architecture is validated
- Plan significant effort for mobile cross-compilation

**R3: Streaming Architecture**
- Use callback-based streaming (`SegmentCallback`) for real-time transcription
- Buffer audio in ~5-10 second chunks for optimal latency/accuracy tradeoff
- Enable VAD to detect speech boundaries and reduce unnecessary processing

---

## References

- [x] whisper.cpp Go bindings: https://github.com/ggerganov/whisper.cpp/tree/master/bindings/go
- [x] whisper.cpp Go package docs: https://pkg.go.dev/github.com/ggerganov/whisper.cpp/bindings/go/pkg/whisper
- [x] whisper.cpp main repo: https://github.com/ggerganov/whisper.cpp
- [x] CGO documentation: https://pkg.go.dev/cmd/cgo
- [x] mutablelogic/go-whisper (alternative): https://github.com/mutablelogic/go-whisper
- [x] sherpa-onnx (recommended alternative): https://github.com/k2-fsa/sherpa-onnx
- [x] Thread safety issue #918: https://github.com/ggml-org/whisper.cpp/issues/918
- [x] Thread safety issue #341: https://github.com/ggml-org/whisper.cpp/issues/341
- [x] WhisperLiveKit (streaming): https://skywork.ai/blog/whisperlivekit-the-ultimate-solution-for-real-time-speech-recognition/
- [x] whisper-flow (streaming): https://github.com/dimastatz/whisper-flow

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Research complete: API documented, streaming confirmed, sherpa-onnx identified as alternative |
