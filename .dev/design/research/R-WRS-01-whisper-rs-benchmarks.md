# Research Plan: whisper-rs Benchmarking

**ID:** R-WRS-01
**Status:** Ongoing
**Category:** Performance
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Benchmark report

---

## Problem Statement

whisper-rs is the proposed Rust binding for whisper.cpp. Before committing to this dependency, we need empirical performance data on:
- Transcription latency (time to first word, total time)
- Memory consumption (peak, steady-state)
- Stability under sustained load
- Accuracy preservation across quantization levels

**Engineering Challenge:** Establish baseline performance metrics and identify potential bottlenecks or limitations.

---

## Research Questions

### RQ1: What is the baseline latency for whisper-rs?

**Metrics to capture:**
- [ ] Cold start time (model loading)
- [ ] Time to first transcription result
- [ ] Total transcription time for 10s, 30s, 60s audio
- [ ] Real-time factor (RTF) across model sizes
- [ ] Memory allocation patterns

**Test matrix:**
| Model | macOS (M1/M2) | Linux (x86_64) | Quantized |
|-------|---------------|----------------|-----------|
| tiny.en | [ ] | [ ] | [ ] q5_1 |
| base.en | [ ] | [ ] | [ ] q5_1 |
| small | [ ] | [ ] | [ ] |

### RQ2: What is the memory profile during inference?

**Areas to investigate:**
- [ ] Peak memory during model load
- [ ] Steady-state memory during inference
- [ ] Memory release after inference
- [ ] Memory behavior with concurrent requests

### RQ3: How stable is whisper-rs under sustained load?

**Areas to investigate:**
- [ ] 1-hour continuous transcription test
- [ ] Memory leak detection (valgrind/heaptrack)
- [ ] Thread safety under concurrent calls
- [ ] Graceful handling of malformed audio

### RQ4: How does quantization affect accuracy and performance?

**Areas to investigate:**
- [ ] Word Error Rate (WER) comparison: base vs base-q5_1
- [ ] Latency comparison: base vs base-q5_1
- [ ] Memory comparison: base vs base-q5_1

---

## Known Benchmarks & Prior Art

| Source | Findings | Notes |
|--------|----------|-------|
| whisper.cpp repo | RTF benchmarks by hardware | Focus on C++, not Rust bindings |
| whisper-rs issues | Anecdotal performance reports | User-reported, not systematic |
| Picovoice comparison | Competitor benchmarks | Marketing-focused |

---

## Technical Setup

### Hardware Requirements
- macOS: Apple Silicon (M1/M2/M3) for Metal acceleration
- Linux: x86_64 with AVX2/AVX-512
- Optional: NVIDIA GPU for CUDA testing

### Software Requirements
```toml
[dependencies]
whisper-rs = "0.11"  # or latest
criterion = "0.5"     # benchmarking framework
```

### Test Audio
- LibriSpeech test-clean samples (standard benchmark corpus)
- Varied lengths: 5s, 10s, 30s, 60s
- Multiple speakers and accents

---

## Agent Tasks

### Task 1: Set up benchmark environment
- [ ] Create Rust benchmark project with criterion
- [ ] Download test audio files (LibriSpeech subset)
- [ ] Download model files (tiny.en, base.en, base-q5_1)
- [ ] Document hardware specifications

### Task 2: Implement latency benchmarks
- [ ] Model load time benchmark
- [ ] First-word latency benchmark
- [ ] Full transcription time benchmark
- [ ] RTF calculation

### Task 3: Implement memory benchmarks
- [ ] Peak memory measurement
- [ ] Memory over time profiling
- [ ] Memory leak detection (1-hour test)

### Task 4: Implement stability tests
- [ ] Sustained load test (continuous transcription)
- [ ] Concurrent request test (thread safety)
- [ ] Error handling test (malformed input)

### Task 5: Compile results
- [ ] Generate benchmark report with charts
- [ ] Compare to whisper.cpp C++ benchmarks
- [ ] Identify any performance gaps or concerns
- [ ] Recommendations for production use

---

## Benchmark Template

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

fn bench_model_load(c: &mut Criterion) {
    c.bench_function("load_base_en", |b| {
        b.iter(|| {
            WhisperContext::new_with_params(
                "models/ggml-base.en.bin",
                WhisperContextParameters::default()
            ).unwrap()
        })
    });
}

fn bench_transcribe_10s(c: &mut Criterion) {
    let ctx = WhisperContext::new_with_params(
        "models/ggml-base.en.bin",
        WhisperContextParameters::default()
    ).unwrap();

    let audio = load_audio("test_10s.wav");

    c.bench_function("transcribe_10s_base_en", |b| {
        b.iter(|| {
            let mut state = ctx.create_state().unwrap();
            let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            state.full(params, &audio).unwrap()
        })
    });
}
```

---

## Success Criteria

- [ ] Benchmark results for all models in test matrix
- [ ] Memory profile documented with peaks and steady-state
- [ ] Stability confirmed for 1-hour sustained operation
- [ ] No memory leaks detected
- [ ] RTF < 1.0 confirmed for target models
- [ ] Clear go/no-go recommendation for whisper-rs

---

## Dependencies

- Model files from whisper.cpp releases
- LibriSpeech test corpus

---

## References

- whisper-rs: https://github.com/tazz4843/whisper-rs
- whisper.cpp benchmarks: https://github.com/ggerganov/whisper.cpp#benchmarks
- Criterion.rs: https://bheisler.github.io/criterion.rs/book/
- LibriSpeech: https://www.openslr.org/12
- Valgrind: https://valgrind.org/
- heaptrack: https://github.com/KDE/heaptrack

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
