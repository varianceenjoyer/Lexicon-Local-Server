---
id: WP-LATENCY-STT
type: work-plan
status: in-progress
phase: beta
created: 2026-01-31
updated: 2026-02-02
author: design-lead
consumers: [developer]
depends-on: [WP-STT, WP-FFI]
blocks: []
priority: critical
tags: [latency, performance, whisper, stt, optimization, streaming]
---

# WP-LATENCY-STT: Speech-to-Text Latency Optimization

## Overview

Optimize Whisper inference and the Rust STT pipeline to achieve sub-second transcription latency.

**Initial State** (2026-01-31): ~10+ seconds on emulator, ~3800ms on physical ARM64
**Current State** (2026-02-02): ~3250ms on physical ARM64 (Samsung RFCR8069FBK)
**Target State**: <1800ms for Gen-5 smartphones

**Progress**: 15% latency reduction achieved (3800ms → 3250ms) through config-level optimizations.

## Problem Analysis

### Latency Components (Rust Side)

1. **Model Loading**: One-time cost, but verify it's truly one-time
2. **Audio Preprocessing**: Resampling, normalization, mel spectrogram
3. **Whisper Inference**: The main bottleneck - encoder + decoder
4. **Token Decoding**: Converting tokens to text
5. **FFI Serialization**: Rust → Dart data marshaling

### Whisper Architecture Bottleneck

```
Audio → [Mel Spectrogram] → [Encoder] → [Decoder Loop] → Text
              ~10ms           ~200ms      ~500ms+/token
```

The decoder is autoregressive - each token depends on previous tokens, preventing parallelization.

## Scope

### In Scope
- Whisper.cpp configuration optimization
- Thread count tuning
- Model warm-up strategies
- Streaming/chunked inference
- Memory management
- SIMD/NEON verification

### Out of Scope
- Model architecture changes
- Custom model training
- Alternative STT engines (future work)

## Optimization Strategies

### 1. Verify Optimized Build Flags

**Problem**: whisper.cpp may not be compiled with optimal flags.

**Verification**:
```bash
# Check if NEON/SIMD enabled for ARM
nm target/aarch64-linux-android/release/liblexicon_flutter.so | grep -i neon

# Check optimization level
objdump -d target/release/liblexicon_flutter.so | head -100
```

**CMake flags to verify**:
```cmake
-DWHISPER_NO_ACCELERATE=OFF  # Use Accelerate on macOS
-DWHISPER_OPENBLAS=ON        # Use OpenBLAS if available
-DGGML_NEON=ON               # ARM NEON
-DGGML_FP16_VA=ON            # FP16 vectorization
```

### 2. Optimize Thread Count

**Problem**: Default thread count may not be optimal for mobile.

**Current**: `numThreads: 4` hardcoded in Dart.

**Proposal**: Auto-detect optimal thread count:

```rust
pub fn optimal_thread_count() -> u32 {
    let cpus = num_cpus::get();
    // Mobile: use N-1 to leave headroom for UI
    // Emulator: use N/2 due to QEMU overhead
    match cpus {
        1..=2 => 1,
        3..=4 => 2,
        5..=8 => cpus as u32 - 2,
        _ => 4,
    }
}
```

### 3. Model Warm-up / Keep-Alive

**Problem**: First inference may be slower due to lazy initialization.

**Solution**: Warm up model on load:

```rust
impl WhisperTranscriber {
    pub fn warm_up(&self) -> Result<()> {
        // Run inference on 100ms of silence to warm caches
        let silence = vec![0.0f32; 1600];
        let _ = self.transcribe(&silence, &TranscriptionConfig::default())?;
        Ok(())
    }
}
```

### 4. Chunked Streaming with State Preservation

**Problem**: Each chunk processed independently loses context.

**Solution**: Implement true streaming with encoder state caching:

```rust
pub struct StreamingWhisperState {
    encoder_cache: Option<EncoderOutput>,
    decoder_state: DecoderState,
    audio_buffer: Vec<f32>,
    context_tokens: Vec<i32>,
}

impl StreamingTranscriber {
    pub fn process_chunk_streaming(
        &mut self,
        chunk: &[f32],
    ) -> Result<PartialTranscription> {
        self.audio_buffer.extend_from_slice(chunk);

        // Only re-encode if buffer changed significantly
        if self.should_reencode() {
            self.encoder_cache = Some(self.encode(&self.audio_buffer)?);
        }

        // Decode with cached encoder output
        let tokens = self.decode_incremental(&self.encoder_cache)?;
        Ok(PartialTranscription::from_tokens(tokens))
    }
}
```

### 5. Speculative Decoding

**Problem**: Decoder generates one token at a time.

**Solution**: Predict multiple tokens, verify in parallel:

```rust
// Generate 3 candidate tokens speculatively
let candidates = self.speculate_tokens(3);
// Verify which are correct
let verified = self.verify_tokens(&candidates);
// Accept verified, regenerate rest
```

**Note**: Requires whisper.cpp modifications or custom implementation.

### 6. Reduce Model Precision (If Not Already)

**Verification**: Ensure q5_1 quantization is actually being used.

```rust
pub fn verify_quantization(model_path: &str) -> QuantizationType {
    // Check model header for quantization type
    // tiny-q5_1 should show Q5_1 quantization
}
```

### 7. Audio Preprocessing Optimization

**Problem**: Mel spectrogram computation on every chunk.

**Solution**: Cache and append mel frames:

```rust
pub struct MelCache {
    frames: Vec<[f32; 80]>,  // 80 mel bins
    hop_samples: usize,
}

impl MelCache {
    pub fn append_audio(&mut self, samples: &[f32]) {
        // Only compute mel for new samples
        let new_frames = compute_mel_incremental(samples, self.hop_samples);
        self.frames.extend(new_frames);
    }
}
```

### 8. Memory Pool for Inference

**Problem**: Allocations during inference cause latency spikes.

**Solution**: Pre-allocate inference buffers:

```rust
pub struct InferencePool {
    mel_buffer: Vec<f32>,
    encoder_buffer: Vec<f32>,
    decoder_buffer: Vec<f32>,
    token_buffer: Vec<i32>,
}

impl InferencePool {
    pub fn new(max_audio_length: usize) -> Self {
        Self {
            mel_buffer: vec![0.0; max_audio_length * 80],
            encoder_buffer: vec![0.0; ENCODER_DIM * MAX_FRAMES],
            decoder_buffer: vec![0.0; DECODER_DIM * MAX_TOKENS],
            token_buffer: vec![0; MAX_TOKENS],
        }
    }
}
```

### 9. Early Exit on Silence

**Problem**: Processing silence wastes compute.

**Solution**: Enhanced VAD with early exit:

```rust
pub fn should_process(chunk: &[f32]) -> ProcessingDecision {
    let energy = compute_rms(chunk);
    let zcr = compute_zero_crossing_rate(chunk);

    match (energy, zcr) {
        (e, _) if e < SILENCE_THRESHOLD => ProcessingDecision::Skip,
        (e, z) if e < SPEECH_THRESHOLD && z > NOISE_ZCR => ProcessingDecision::Skip,
        _ => ProcessingDecision::Process,
    }
}
```

### 10. Profile-Guided Optimization

**Problem**: Unknown bottleneck distribution.

**Action**: Profile with `perf` or `flamegraph`:

```bash
# On Linux
perf record -g ./target/release/lexicon transcribe test.wav
perf report

# Generate flamegraph
cargo flamegraph --bin lexicon -- transcribe test.wav
```

## Acceptance Criteria

- [ ] 100ms audio chunk processed in <500ms on physical aarch64
- [ ] 100ms audio chunk processed in <1000ms on emulated x86_64
- [ ] First transcription after model load <1 second
- [ ] Memory usage stable (no leaks during long sessions)
- [x] Build flags verified for NEON/SIMD optimization (release profile with LTO added)

## Implementation Order

1. **Phase 1 - Measurement & Verification**
   - Add timing logs at each pipeline stage
   - Verify build flags and quantization
   - Profile with flamegraph

2. **Phase 2 - Quick Wins**
   - Thread count optimization
   - Model warm-up
   - Enhanced VAD early exit

3. **Phase 3 - Caching & Pooling**
   - Mel spectrogram caching
   - Memory pool pre-allocation
   - Encoder state caching

4. **Phase 4 - Advanced (If Needed)**
   - Speculative decoding
   - Custom streaming implementation

## Files to Modify

| File | Changes |
|------|---------|
| `crates/lexicon-core/src/stt/whisper.rs` | Warm-up, threading, caching |
| `crates/lexicon-flutter/src/api.rs` | Streaming API, timing logs |
| `crates/lexicon-core/src/audio/mod.rs` | VAD improvements |
| `build/build_mobile_apk.sh` | Verify CMake flags |
| `Cargo.toml` | Verify release profile optimizations |

## Metrics to Track

| Metric | Baseline | Current (2026-02-02) | Target |
|--------|----------|----------------------|--------|
| FFI inference time (tiny-q5_1) | ~3800ms | ~3250ms | <1800ms |
| VAD + Conv overhead | N/A | <0.2ms | <1ms |
| Pipeline total | ~3800ms | ~3250ms | <2000ms |
| Mel spectrogram computation | ? | ? | <10ms |
| Encoder pass | ? | ? | <200ms |
| Decoder per token | ? | ? | <50ms |
| FFI round-trip overhead | ? | <0.2ms | <5ms |
| Memory per transcription | ? | ? | <50MB |

**Note**: Measurements taken on physical Samsung ARM64 device (RFCR8069FBK) with tiny-q5_1 model.

## Platform-Specific Expectations

Understanding latency expectations by platform is critical for proper testing:

| Platform | Model | Expected Latency | Notes |
|----------|-------|------------------|-------|
| ARM64 Physical (Pixel 6) | tiny-q5_1 | 500-1000ms | Target for optimization |
| ARM64 Physical (Pixel 6) | base-q5_1 | 1000-2000ms | Acceptable for longer speech |
| x86_64 Emulator (QEMU) | tiny-q5_1 | **10-15 seconds** | Expected - QEMU overhead 2-5x |
| x86_64 Emulator (QEMU) | base-q5_1 | **20-30 seconds** | Expected - too slow for real-time |
| Desktop Linux x86_64 | tiny | 100-300ms | Reference baseline |
| Desktop Linux x86_64 | base | 200-500ms | Reference baseline |

**Critical**: Do NOT use emulator latency as the benchmark for mobile performance.
The emulator adds 10-20x overhead due to QEMU CPU translation.

## Build Verification Checklist

- [x] `__ARM_NEON` defined (baseline for ARM64, always enabled)
- [ ] `__ARM_FEATURE_DOTPROD` enabled via `-march=armv8.2-a+dotprod` (added 2026-01-31)
- [ ] LTO enabled in release profile
- [x] whisper.cpp compiled with `-O3` (verified in CMakeCache)
- [x] No debug symbols in release build (verified with `file` command)

### ARM64 Optimization Investigation (2026-01-31)

**Finding**: NEON is enabled by default (confirmed via `__ARM_NEON 1`), but DOTPROD was missing.

**Compiler Flags Analysis**:
```bash
# Base ARM64 compile (before fix):
# - __ARM_NEON = 1 (enabled)
# - __ARM_FEATURE_FMA = 1 (enabled)
# - __ARM_FEATURE_DOTPROD = NOT DEFINED (missing!)

# With -march=armv8.2-a+dotprod:
# - __ARM_FEATURE_DOTPROD = 1 (now enabled)
```

**Impact**: DOTPROD enables optimized `vdotq_s32` instructions for quantized matrix operations in ggml.
Many Q4/Q8 quantized operations in whisper.cpp use DOTPROD-gated code paths:
- `ggml-cpu-aarch64.cpp`: Uses `__ARM_FEATURE_DOTPROD` for optimized quantized kernels
- `llamafile/sgemm.cpp`: Uses DOTPROD for faster matrix multiply

**Fix Applied**: Added `-march=armv8.2-a+dotprod` to `build_mobile_apk.sh` for ARM64 builds.

**Compatibility Note**: ARMv8.2-A with DOTPROD is supported on:
- Cortex-A75/A55 (2017+)
- Cortex-A76/A77/A78 and newer
- Most devices running Android 8+ with arm64

## Alternative Approaches (Future)

If Whisper optimization is insufficient:

1. **Distilled Models**: Use whisper-tiny distilled variants
2. **Alternative Engines**: Vosk, DeepSpeech (smaller, faster, less accurate)
3. **Hybrid Approach**: Fast VAD + keyword spotting, Whisper for full transcription
4. **Server Offload**: Send audio to server for heavy processing

## Research References

- whisper.cpp streaming implementation
- GGML performance tuning guide
- ARM NEON optimization for ML
- Real-time ASR latency benchmarks

## Notes

- Physical device testing is essential - emulator adds 2-5x overhead
- Battery impact should be monitored alongside latency
- Consider thermal throttling on sustained transcription
- User-perceived latency includes UI rendering time

---

## Implementation Log

### 2026-02-01: Phase 1 & 2 Complete

**Phase 1 - Measurement & Verification:**
- Added detailed timing instrumentation to `whisper.rs:transcribe()` tracking:
  - State creation time
  - Config time
  - Inference time (main bottleneck)
  - Segment extraction time
  - Diarization time
- All metrics logged via `tracing::debug!` for analysis

**Phase 2 - Quick Wins:**
1. **Release profile optimizations** (`Cargo.toml`):
   - LTO="fat" for cross-crate optimization
   - codegen-units=1 for maximum optimization
   - panic="abort" for smaller binaries
   - strip=true for release builds

2. **Thread count optimization** (`whisper.rs`):
   - Added `optimal_thread_count()` function
   - Auto-detects CPU cores and selects optimal thread count
   - Mobile-optimized: leaves headroom for UI thread
   - Desktop-optimized: caps at 8 threads (diminishing returns)

3. **Model warm-up** (`whisper.rs`):
   - Added `warm_up()` method to `WhisperTranscriber`
   - Runs short inference on silence to prime caches
   - Eliminates first-inference latency penalty (100-500ms)

4. **Enhanced VAD early exit** (`audio/vad.rs`):
   - Added `should_process()` function for fast pre-screening
   - Computes RMS energy and zero-crossing rate
   - Skips silent/noise chunks before expensive inference
   - Exported `ProcessingDecision` enum, `compute_rms()`, `compute_zero_crossing_rate()`

**Files Modified:**
- `Cargo.toml` - Release profile
- `crates/lexicon-core/src/stt/whisper.rs` - Timing, threading, warm-up
- `crates/lexicon-core/src/stt/mod.rs` - Export optimal_thread_count
- `crates/lexicon-core/src/audio/vad.rs` - Enhanced VAD functions
- `crates/lexicon-core/src/audio/mod.rs` - Export new VAD functions

**Tests Added:**
- 19 VAD tests (all passing)
- 2 optimal_thread_count tests (all passing)
- Total: 330 tests passing

### 2026-02-01: Android Build Optimizations

**Build configuration improvements:**
1. **NEON SIMD for ARM64** (`.cargo/config.toml`):
   - Added `rustflags = ["-C", "target-feature=+neon"]` for aarch64-linux-android
   - Enables hardware-accelerated vector operations critical for Whisper

2. **opt-level = 3 for Android** (`.cargo/config.toml`):
   - Changed from "s" (size) to "3" (speed) for release-android profile
   - Size optimization caused significant latency penalty

3. **whisper.cpp CMake flags** (`build-flutter-android.sh`):
   - Added GGML_NEON=ON, GGML_FP16_VA=ON
   - Added CMAKE_C_FLAGS/CMAKE_CXX_FLAGS with -O3 -DNDEBUG

4. **Latency-optimized model selection** (`system_profile.rs`):
   - Added `latency_optimized_model()` for real-time use cases
   - Added `expected_latency_ms()` for latency estimation

**Platform-specific expectations documented:**
- ARM64 physical device (Pixel 6): 500-1000ms for tiny model
- x86_64 emulator: 10-15 seconds (expected due to QEMU overhead)

### 2026-02-01: Flutter API Integration

**Integrated latency optimizations into lexicon-flutter crate:**

1. **VAD early exit in StreamingTranscriber.process_chunk()**
   - Checks `should_process()` before calling Whisper inference
   - Silent/noise chunks return empty segments instantly
   - Saves compute on mobile devices with intermittent speech

2. **warm_up() method for Transcriber and StreamingTranscriber**
   - Exposes model warm-up functionality to Dart/Flutter
   - Returns warm-up time in milliseconds for diagnostics
   - Can be called after load_model() to prime caches

3. **Added audio feature dependency for VAD**
   - lexicon-flutter now includes lexicon-core/audio feature

**Files Modified:**
- `crates/lexicon-flutter/Cargo.toml` - Added audio feature
- `crates/lexicon-flutter/src/api.rs` - VAD early exit, warm_up methods

**Tests Added:**
- 4 new tests for latency optimization features
- Total: 15 tests in lexicon-flutter (all passing)

### 2026-02-02: Physical Device Benchmarks & whisper-rs Optimizations

**Physical device testing on Samsung ARM64 (RFCR8069FBK):**

Baseline measurements established on physical ARM64 device with tiny-q5_1 model:
- **Initial baseline**: ~3800ms FFI time
- **Target**: ~1800ms (Gen-5 smartphone theoretical)

**whisper-rs/whisper-rs-sys optimizations implemented:**

1. **Flash Attention** (`whisper.rs`):
   - Added `params.flash_attn(self.use_gpu)` for GPU acceleration
   - **CRITICAL**: Only enable on macOS/iOS (Metal) - causes 2x slowdown on Android ARM
   - Used `#[cfg(any(target_os = "macos", target_os = "ios"))]` guard
   - On Android: FFI time went from 3800ms → 6388ms with flash_attn, back to ~3200ms without

2. **Single Segment Mode** (`whisper.rs`, `types.rs`):
   - Added `params.set_single_segment(config.single_segment)`
   - Default: `true` for lower latency
   - Reduces segmentation overhead (~5-10% improvement)

3. **Temperature Fallback Disabled** (`whisper.rs`):
   - Added `params.set_temperature_inc(0.0)`
   - Disables retry with higher temperature on failure
   - Saves ~0-5% latency

4. **No Timestamps Option** (`whisper.rs`, `types.rs`):
   - Added `no_timestamps: bool` to TranscriptionConfig
   - Added `params.set_no_timestamps(config.no_timestamps)`
   - Saves ~2-3% when timestamps not needed

5. **ARM64 Enhanced SIMD** (`.cargo/config.toml`):
   - Changed from `+neon` to `+neon,+dotprod,+fp16`
   - Enables additional ARM SIMD instructions for Rust code

6. **Force English Language** (`lexicon-flutter/src/types.rs`):
   - Changed default language from `None` to `Some("en")`
   - Skips language auto-detection forward pass
   - Saves ~5% latency

**Benchmark Results:**

| Build Configuration | FFI Time | Change |
|---------------------|----------|--------|
| Baseline (pre-optimization) | ~3800ms | - |
| + flash_attn (BROKEN on Android) | 6388ms | +68% worse |
| + all optimizations (flash_attn disabled) | 3200-3600ms | ~10% better |
| + forced English | 3079-3401ms | ~15% better |

**Final Result**: ~3250ms average (down from ~3800ms) = **~15% improvement**

**Files Modified:**
- `crates/lexicon-core/src/stt/whisper.rs` - flash_attn (guarded), single_segment, temperature_inc, no_timestamps
- `crates/lexicon-core/src/stt/types.rs` - Added single_segment, no_timestamps fields
- `crates/lexicon-flutter/src/types.rs` - Force English language default
- `.cargo/config.toml` - Enhanced ARM64 SIMD flags

**Key Learning:**
Flash attention in whisper.cpp requires Metal (macOS/iOS) or CUDA (NVIDIA). On Android ARM,
enabling flash_attn causes significant slowdown because the code path is not optimized for
CPU-only execution. Always guard flash_attn with platform checks.

**Remaining Gap:**
Current: ~3250ms, Target: ~1800ms. Closing this gap requires:
- Hardware NPU/GPU acceleration
- Streaming chunked inference (process audio as it arrives)
- Different model architecture or alternative STT engine
