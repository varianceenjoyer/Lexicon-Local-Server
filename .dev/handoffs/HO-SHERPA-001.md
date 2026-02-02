# Handoff: WP-SHERPA-STT Streaming STT Integration

> **ID**: HO-SHERPA-001 | **Work Package**: WP-SHERPA-STT
>
> **Created**: 2026-02-02 | **Status**: 🟢 READY FOR IMPLEMENTATION

---

## Summary

Integrate sherpa-onnx as an alternative STT engine using the `sherpa-rs` Rust crate. This replaces the batch-oriented Whisper model with a streaming-native architecture to reduce transcription latency from ~3250ms to <500ms.

**Complexity**: MEDIUM (well-documented crate, clear interface contracts)

**Expected Impact**: 6-10x latency improvement for STT inference

---

## Background

### Why This Change?

Whisper is fundamentally a batch model:
- Always processes 30-second chunks (padded)
- 1 second of audio → 30 seconds of compute
- Current best: ~3250ms on Samsung ARM64

sherpa-onnx uses streaming architectures:
- Processes audio incrementally as it arrives
- Compute scales with actual audio length
- Benchmark: 160ms latency on similar hardware

### Research Sources

- [sherpa-onnx GitHub](https://github.com/k2-fsa/sherpa-onnx) - Main project
- [sherpa-rs crate](https://crates.io/crates/sherpa-rs) - Rust bindings
- [Streaming models](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/) - Available models

---

## Quick Start

1. **Read the work package**: [.dev/work/WP-SHERPA-STT.md](../work/WP-SHERPA-STT.md)
2. **Review current STT**: `crates/lexicon-core/src/stt/whisper.rs`
3. **Start with Phase 1**: Add sherpa-rs dependency and test compilation
4. **Test on device** after each phase

---

## Implementation Order

### Phase 1: Dependency Integration (1-2 hours)

**Goal**: Verify sherpa-rs builds for Android ARM64

1. Add to `Lexicon-Local-Server/Cargo.toml`:
```toml
[workspace.dependencies]
sherpa-rs = { version = "0.1", optional = true }
```

2. Add feature flag to `crates/lexicon-core/Cargo.toml`:
```toml
[features]
stt = ["dep:whisper-rs"]
stt-sherpa = ["dep:sherpa-rs"]

[dependencies]
sherpa-rs = { workspace = true, optional = true }
```

3. Test build:
```bash
cargo build --features stt-sherpa --target aarch64-linux-android
```

**Checkpoint**: Build succeeds, native libs generated

### Phase 2: Core Implementation (2-3 hours)

**Goal**: Create SherpaTranscriber with Transcriber trait

1. Create `crates/lexicon-core/src/stt/sherpa.rs`
2. Implement `Transcriber` trait (see work package for interface)
3. Add streaming callback support
4. Export from `mod.rs` under `#[cfg(feature = "stt-sherpa")]`

**Key Code Pattern**:
```rust
use sherpa_rs::{
    OnlineRecognizer, OnlineRecognizerConfig,
    OnlineStream, OnlineModelConfig
};

pub struct SherpaTranscriber {
    recognizer: Option<OnlineRecognizer>,
    model_dir: Option<String>,
}

impl Transcriber for SherpaTranscriber {
    fn transcribe(&self, samples: &[f32], config: &TranscriptionConfig)
        -> Result<TranscriptionResult>
    {
        let recognizer = self.recognizer.as_ref()
            .ok_or(LexiconError::Inference(InferenceError::ModelNotLoaded))?;

        let mut stream = recognizer.create_stream()?;
        stream.accept_waveform(16000, samples);

        while recognizer.is_ready(&stream) {
            recognizer.decode(&mut stream);
        }

        let result = recognizer.get_result(&stream);
        // Convert to TranscriptionResult...
    }
}
```

**Checkpoint**: Unit tests pass, basic transcription works

### Phase 3: FFI Integration (1-2 hours)

**Goal**: Expose streaming API to Flutter

1. Add streaming types to `crates/lexicon-flutter/src/types.rs`
2. Add streaming functions to `crates/lexicon-flutter/src/api.rs`
3. Regenerate flutter_rust_bridge bindings

**Checkpoint**: Dart can call streaming transcription

### Phase 4: Build Integration (1-2 hours)

**Goal**: Bundle model and native libs in APK

1. Download streaming Zipformer model (~60MB):
```bash
./scripts/download_sherpa_model.sh
```

2. Update `build/build_mobile_apk.sh`:
   - Copy sherpa models to assets
   - Verify native libs in jniLibs

3. Test full build:
```bash
./build/build_mobile_apk.sh arm64 --features stt-sherpa
```

**Checkpoint**: APK builds, installs, model loads

### Phase 5: Benchmarking (1 hour)

**Goal**: Verify latency improvement

1. Deploy to Samsung RFCR8069FBK
2. Run same test audio through both engines
3. Compare:
   - FFI latency (target: <500ms vs ~3250ms)
   - WER accuracy (target: within 5% of Whisper)
   - Memory usage

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` (workspace) | Modify | Add sherpa-rs dependency |
| `crates/lexicon-core/Cargo.toml` | Modify | Add stt-sherpa feature |
| `crates/lexicon-core/src/stt/sherpa.rs` | Create | SherpaTranscriber implementation |
| `crates/lexicon-core/src/stt/mod.rs` | Modify | Export sherpa module |
| `crates/lexicon-flutter/src/types.rs` | Modify | Add streaming types |
| `crates/lexicon-flutter/src/api.rs` | Modify | Add streaming FFI functions |
| `build/build_mobile_apk.sh` | Modify | Handle sherpa model/libs |
| `scripts/download_sherpa_model.sh` | Create | Model download script |

---

## Model Download

**Recommended Model**: `sherpa-onnx-streaming-zipformer-en-20M-2023-06-26`

```bash
# Download to Lexicon-Mobile/assets/models/sherpa/
MODEL_URL="https://huggingface.co/csukuangfj/sherpa-onnx-streaming-zipformer-en-20M-2023-06-26/resolve/main"
mkdir -p Lexicon-Mobile/assets/models/sherpa
wget -O Lexicon-Mobile/assets/models/sherpa/encoder-epoch-99-avg-1.int8.onnx "$MODEL_URL/encoder-epoch-99-avg-1.int8.onnx"
wget -O Lexicon-Mobile/assets/models/sherpa/decoder-epoch-99-avg-1.onnx "$MODEL_URL/decoder-epoch-99-avg-1.onnx"
wget -O Lexicon-Mobile/assets/models/sherpa/joiner-epoch-99-avg-1.onnx "$MODEL_URL/joiner-epoch-99-avg-1.onnx"
wget -O Lexicon-Mobile/assets/models/sherpa/tokens.txt "$MODEL_URL/tokens.txt"
```

---

## Acceptance Criteria

- [ ] `cargo build --features stt-sherpa` succeeds
- [ ] SherpaTranscriber implements Transcriber trait
- [ ] Unit tests pass (same test cases as WhisperTranscriber)
- [ ] APK builds with sherpa model bundled
- [ ] FFI latency <500ms on test device
- [ ] WER within 5% of Whisper baseline
- [ ] Feature flag allows switching between engines

---

## Rollback

If issues arise:
1. `--features stt` still uses Whisper
2. No breaking changes to existing API
3. sherpa-rs is optional dependency

---

## Questions?

- sherpa-rs API: https://docs.rs/sherpa-rs
- sherpa-onnx docs: https://k2-fsa.github.io/sherpa/onnx/
- Model selection: https://k2-fsa.github.io/sherpa/onnx/pretrained_models/
