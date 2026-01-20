# Performance Benchmarks

This document describes the performance benchmark infrastructure for Lexicon Local Server and documents target metrics.

## Overview

Lexicon uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for statistical benchmarking. Benchmarks measure:

- **Audio Pipeline**: VAD, resampling, audio chunk processing
- **STT Latency**: Whisper transcription performance
- **TTS Latency**: Text-to-speech synthesis performance

## Running Benchmarks

### All Benchmarks

```bash
# Run all benchmarks
cargo bench -p lexicon-core

# With specific features
cargo bench -p lexicon-core --features "audio,stt,tts"
```

### Individual Benchmark Suites

```bash
# Audio pipeline benchmarks (VAD, resampling)
cargo bench -p lexicon-core --bench audio_pipeline

# STT transcription benchmarks (requires model)
cargo bench -p lexicon-core --bench stt_latency --features stt

# TTS synthesis benchmarks
cargo bench -p lexicon-core --bench tts_latency --features tts
```

### Viewing Reports

Criterion generates HTML reports in `target/criterion/`. Open `target/criterion/report/index.html` to view detailed results with graphs.

---

## Target Metrics

### Audio Pipeline

| Component | Metric | Target | Notes |
|-----------|--------|--------|-------|
| VAD | 1s audio processing | < 100ms p95 | 16kHz mono input |
| Resampler | 44.1kHz → 16kHz, 1s | < 50ms | Mono channel |
| Resampler | 48kHz → 16kHz, 1s | < 50ms | Mono channel |
| AudioChunk | Creation (16k samples) | < 1ms | Memory allocation |

### Speech-to-Text (STT)

| Model | Audio Duration | Target Latency | Notes |
|-------|----------------|----------------|-------|
| Whisper Tiny | 5s | < 400ms p95 | English, CPU |
| Whisper Base | 5s | < 800ms p95 | English, CPU |
| Whisper Small | 5s | < 2000ms p95 | Multilingual, CPU |

**Factors affecting STT performance:**
- Model size (tiny < base < small < medium < large)
- Audio duration (linear scaling)
- Language detection vs. specified language
- CPU vs. GPU acceleration

### Text-to-Speech (TTS)

| Engine | Text Length | Target Latency | Notes |
|--------|-------------|----------------|-------|
| Platform (espeak-ng) | Short (< 20 chars) | < 200ms | Linux |
| Platform (say) | Short (< 20 chars) | < 300ms | macOS |
| Kokoro | Short (< 20 chars) | < 500ms | Neural TTS |
| Platform | Medium (< 100 chars) | < 500ms | Any platform |
| Kokoro | Medium (< 100 chars) | < 1000ms | Neural TTS |

**First chunk latency** (streaming):
- Target: < 200ms for first audio chunk
- Critical for perceived responsiveness

---

## Benchmark Details

### Audio Pipeline (`audio_pipeline.rs`)

**VAD Benchmarks:**
- `VAD/silence/{10,100,1000}ms` - Processing silent audio
- `VAD/speech/{10,100,1000}ms` - Processing speech-like audio
- `VAD_Modes/mode/{0,1,2,3}` - Different aggressiveness levels

**Resampler Benchmarks:**
- `Resampler/44.1kHz_to_16kHz/{100,500,1000,5000}ms`
- `Resampler/48kHz_to_16kHz/{100,500,1000}ms`

**AudioChunk Benchmarks:**
- `AudioChunk/create/{480,1600,16000,80000}` samples

### STT Latency (`stt_latency.rs`)

**Prerequisites:**
- A Whisper model must be downloaded
- Model locations checked: `~/.lexicon/models/`, `~/.local/share/lexicon/models/`

**Benchmarks:**
- `STT_Transcription/duration/{1,3,5,10}s` - Different audio lengths
- `STT_Language/lang/{en,auto}` - Language specification impact

### TTS Latency (`tts_latency.rs`)

**Benchmarks:**
- `TTS_Config/default` - Config creation overhead
- `TTS_Config/with_options` - Config with custom options
- `TTS_AudioBuffer/create/{2400,24000,240000}` - Buffer creation
- `TTS_AudioBuffer/duration_ms` - Duration calculation
- `TTS_AudioBuffer/to_i16_24k_samples` - Format conversion
- `TTS_Streaming/channel_create` - Streaming channel setup
- `TTS_Streaming/send_chunk/{480,2400,4800}` - Chunk transmission
- `TTS_Streaming/receive_10_chunks` - Receiving workflow
- `TTS_Platform/synthesize/{short,medium,long}` - Platform TTS synthesis

---

## CI Integration

Benchmarks can be run in CI for regression detection:

```yaml
# .github/workflows/benchmark.yml
benchmark:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install dependencies
      run: sudo apt-get install -y libasound2-dev espeak-ng

    - name: Run benchmarks
      run: cargo bench -p lexicon-core --bench audio_pipeline -- --noplot

    - name: Upload results
      uses: actions/upload-artifact@v4
      with:
        name: benchmark-results
        path: target/criterion
```

For benchmark comparison between commits, consider using [Bencher](https://bencher.dev/) or [Criterion Compare](https://github.com/BurntSushi/critcmp).

---

## Interpreting Results

Criterion provides statistical analysis:

- **Mean**: Average execution time
- **Median**: Middle value (less affected by outliers)
- **Std Dev**: Variation in measurements
- **p95/p99**: 95th/99th percentile (worst-case latency)

**Example output:**
```
VAD/speech/1000ms   time:   [12.5 ms 12.8 ms 13.1 ms]
                    thrpt:  [76.3 Kelem/s 78.1 Kelem/s 80.0 Kelem/s]
```

**Performance regression detection:**
- Criterion compares with previous runs
- Reports `+X%` or `-X%` change
- Alerts if change exceeds threshold

---

## Hardware Considerations

Benchmark results vary significantly based on hardware:

| Component | Impact on STT | Impact on TTS | Impact on Audio |
|-----------|---------------|---------------|-----------------|
| CPU cores | High (parallel) | Medium | Low |
| CPU speed | High | High | Medium |
| RAM | Medium (model) | Low | Low |
| GPU (CUDA) | Very High | High (Kokoro) | None |
| Storage | Low | Low | None |

**Recommended test hardware:**
- Development: Any modern laptop/desktop
- CI: GitHub-hosted runners (2-core, 7GB RAM)
- Production baseline: 4-core CPU, 8GB RAM

---

## Adding New Benchmarks

1. Add benchmark file to `crates/lexicon-core/benches/`
2. Register in `Cargo.toml`:
   ```toml
   [[bench]]
   name = "my_benchmark"
   harness = false
   ```
3. Follow the pattern:
   ```rust
   use criterion::{criterion_group, criterion_main, Criterion};

   fn my_benchmark(c: &mut Criterion) {
       c.bench_function("my_function", |b| {
           b.iter(|| {
               // Code to benchmark
           });
       });
   }

   criterion_group!(benches, my_benchmark);
   criterion_main!(benches);
   ```

---

## References

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Whisper Performance](https://github.com/openai/whisper#available-models-and-languages)
