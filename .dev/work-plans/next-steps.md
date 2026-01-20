# Next Steps Work Plans

> **Created**: 2026-01-20 | **Phase**: MVP → Beta Transition
>
> Actionable work plans based on unchecked items in design-plan.md

---

## Summary

| Priority | Work Plan | Phase | Effort | Status |
|----------|-----------|-------|--------|--------|
| 1 | MVP Validation | MVP | Low | 🟡 In Progress |
| 2 | Benchmark Validation | Beta | Low | 🟡 In Progress |
| 3 | TTS Phoneme Tokenization | Beta | High | ⬜ Not Started |
| 4 | Server Rate Limiting | Beta | Medium | ✅ Complete |
| 5 | FFI Download Cancellation | Beta | Medium | ⬜ Not Started |
| 6 | Bluetooth HFP 1.6 | Beta | High | ⬜ Not Started |
| 7 | Security Audit Prep | Release | Medium | ⬜ Not Started |

---

## Work Plan 1: MVP Validation

**Priority:** CRITICAL | **Effort:** Low | **Phase:** MVP Exit

### Objective
Validate remaining MVP exit criterion: "Transcribe 10-minute audio file successfully"

### Tasks

- [ ] Obtain or generate 10-minute test audio file (English speech)
- [ ] Run `cargo run -p lexicon-cli -- transcribe <audio_file>`
- [ ] Verify output produces valid SRT format
- [ ] Measure transcription time and memory usage
- [ ] Document results in test report

### Acceptance Criteria
- Transcription completes without error
- Output is valid SRT with correct timestamps
- Memory usage stays under 2GB (desktop target)

### Files
- `crates/lexicon-cli/src/main.rs` - CLI entry point
- `tests/fixtures/audio/` - Test audio location

---

## Work Plan 2: Benchmark Validation

**Priority:** HIGH | **Effort:** Low | **Phase:** Beta

### Objective
Validate the benchmark infrastructure added in commit e888adc

### Tasks

- [ ] Run `cargo bench` to verify benchmarks execute
- [ ] Review benchmark output for STT latency targets
- [ ] Review benchmark output for TTS latency targets
- [ ] Review benchmark output for audio pipeline
- [ ] Create BENCHMARKS.md documenting methodology and baseline results
- [ ] Add benchmark CI job (optional, runs on schedule)

### Acceptance Criteria
- `cargo bench` completes without errors
- STT latency benchmarks report p95 values
- TTS latency benchmarks report first-chunk time
- Results documented in BENCHMARKS.md

### Files
- `crates/lexicon-core/benches/audio_pipeline.rs`
- `crates/lexicon-core/benches/stt_latency.rs`
- `crates/lexicon-core/benches/tts_latency.rs`
- `BENCHMARKS.md` (to create)

---

## Work Plan 3: TTS Phoneme Tokenization (WP-ROADBLOCK-03)

**Priority:** HIGH | **Effort:** High | **Phase:** Beta Quality

### Problem
Kokoro TTS tokenizer uses simplified ASCII character mapping instead of proper phoneme conversion, resulting in poor synthesis quality.

**Location:** `crates/lexicon-core/src/tts/kokoro.rs:60-87`

**Current Implementation:**
```rust
fn tokenize_text(&self, text: &str) -> Vec<i64> {
    // Simple ASCII mapping - real implementation would use phonemes
    let token = match ch {
        'a'..='z' => (ch as i64) - ('a' as i64) + 1,
        // ...
    };
}
```

### Tasks

- [ ] Research Kokoro model's expected input format (IPA phonemes vs custom tokens)
- [ ] Evaluate G2P options:
  - [ ] Option A: `espeakng-sys` crate for phoneme generation
  - [ ] Option B: CMU Pronouncing Dictionary lookup
  - [ ] Option C: Pre-trained G2P neural model
- [ ] Implement chosen G2P solution
- [ ] Add phoneme tokenization unit tests
- [ ] Benchmark phoneme conversion latency (target: <50ms per sentence)
- [ ] Manual QA: verify natural-sounding speech output

### Acceptance Criteria
- Kokoro produces natural-sounding speech for common English sentences
- Phoneme tokenization is tested with known inputs
- Performance impact documented (target: <50ms for typical sentence)
- Fallback to platform TTS works if phoneme conversion fails

### Files
- `crates/lexicon-core/src/tts/kokoro.rs` - Main implementation
- `crates/lexicon-core/src/tts/mod.rs` - Module exports
- `crates/lexicon-core/Cargo.toml` - Add G2P dependency

### Research Links
- Kokoro model: https://github.com/hexgrad/kokoro
- espeakng-sys: https://crates.io/crates/espeakng-sys
- CMU Dict: http://www.speech.cs.cmu.edu/cgi-bin/cmudict

---

## Work Plan 4: Server Rate Limiting (WP-ROADBLOCK-14)

**Priority:** MEDIUM | **Effort:** Medium | **Phase:** Beta

### Problem
No rate limiting exists in the server. API is vulnerable to abuse.

**Current State:** Only uses `tower_http` for CORS and tracing, no rate limiting middleware.

### Tasks

- [x] Add `tower-governor` or `tower` rate limiting to Cargo.toml
- [x] Create rate limiting middleware configuration
- [x] Apply middleware to API routes
- [x] Implement configurable limits:
  - [x] Per-IP request rate (default: 100 req/min)
  - [x] Per-endpoint limits (transcribe: lower, models: higher)
  - [x] Burst allowance configuration
- [x] Add rate limit response headers:
  - [x] `X-RateLimit-Limit`
  - [x] `X-RateLimit-Remaining`
  - [x] `X-RateLimit-Reset`
- [x] Return 429 Too Many Requests when exceeded
- [x] Add rate limit configuration to `ServerConfig`
- [x] Write integration tests for rate limiting

### Acceptance Criteria (All Complete ✅)
- Rate limiting middleware integrated with axum router
- Per-IP limits enforced
- Rate limit headers present in responses
- 429 responses returned when limits exceeded
- Limits configurable via ServerConfig

### Files
- `crates/lexicon-server/Cargo.toml` - Add dependency
- `crates/lexicon-server/src/lib.rs` - Add middleware
- `crates/lexicon-server/src/routes.rs` - Apply to routes
- `crates/lexicon-server/tests/integration_tests.rs` - Add tests

### Implementation Notes
```rust
// Example using tower-governor
use tower_governor::{GovernorLayer, GovernorConfigBuilder};

let governor_conf = GovernorConfigBuilder::default()
    .per_second(2)
    .burst_size(10)
    .finish()
    .unwrap();

let app = Router::new()
    .route("/api/v1/transcribe", post(transcribe))
    .layer(GovernorLayer { config: governor_conf });
```

---

## Work Plan 5: FFI Download Cancellation (WP-ROADBLOCK-04)

**Priority:** MEDIUM | **Effort:** Medium | **Phase:** Beta (Mobile UX)

### Problem
Model download cancellation is not implemented. Mobile users cannot abort large downloads (up to 3GB).

**Location:** `crates/lexicon-ffi/src/lib.rs:699-703`

**Current Implementation:**
```rust
pub fn cancel_download(&self, _model_id: String) {
    // TODO: Implement download cancellation
    tracing::warn!("Download cancellation not yet implemented");
}
```

### Tasks

- [ ] Add cancellation token type (e.g., `tokio_util::sync::CancellationToken`)
- [ ] Modify `ModelRegistry::download_model()` to accept cancellation token
- [ ] Store active download handles in `ModelDownloader` struct
- [ ] Implement `cancel_download()` to:
  - [ ] Find active download by model_id
  - [ ] Trigger cancellation token
  - [ ] Clean up partial download file
- [ ] Update progress callback to check cancellation state
- [ ] Handle cancellation gracefully (return appropriate error type)
- [ ] Test re-download after cancellation works correctly

### Acceptance Criteria
- `cancel_download()` aborts an active download within 1 second
- Partial files are cleaned up on cancellation
- Cancellation fires callback with `DownloadCancelled` error
- Re-downloading after cancellation starts fresh

### Files
- `crates/lexicon-core/src/models/registry.rs` - Add cancellation support
- `crates/lexicon-ffi/src/lib.rs` - Implement cancel_download
- `crates/lexicon-core/Cargo.toml` - Add tokio-util if needed

---

## Work Plan 6: Bluetooth HFP 1.6 Support (WP-ROADBLOCK-06)

**Priority:** LOW | **Effort:** High | **Phase:** Beta (Audio Quality)

### Problem
Audio pipeline handles route changes but doesn't actively negotiate HFP codec selection for 16kHz audio quality.

**Design Plan Reference:** Decision D2

### Tasks

- [ ] Research platform-specific HFP negotiation:
  - [ ] macOS: CoreBluetooth/AVAudioSession codec selection
  - [ ] Windows: Windows.Devices.Bluetooth.Rfcomm
  - [ ] Linux: BlueZ HFP profile / PulseAudio
- [ ] Implement HFP codec preference (16kHz wideband over 8kHz narrowband)
- [ ] Add audio route change handler that rebuilds stream with optimal codec
- [ ] Implement graceful fallback to 8kHz with automatic resampling
- [ ] Test with at least 2 different Bluetooth headsets
- [ ] Document audio quality differences (16kHz vs 8kHz)

### Acceptance Criteria
- HFP 1.6 devices connect at 16kHz when supported
- Graceful fallback to 8kHz with automatic resampling
- Audio quality difference documented
- Tested with at least 2 different Bluetooth headsets

### Files
- `crates/lexicon-core/src/audio/capture.rs` - Audio capture
- `crates/lexicon-core/src/audio/mod.rs` - AudioSource trait

### Platform Notes
- **macOS**: Use `AVAudioSession.setCategory` with `.allowBluetooth` and check `currentRoute.outputs`
- **Windows**: Use `Windows.Media.Audio.AudioGraph` with Bluetooth endpoint
- **Linux**: Use PulseAudio `pa_context_set_default_source` or BlueZ D-Bus API

---

## Work Plan 7: Security Audit Preparation (WP-ROADBLOCK-12)

**Priority:** LOW | **Effort:** Medium | **Phase:** Release

### Objective
Prepare codebase for security audit per Release exit criterion.

### Tasks

- [ ] Create `SECURITY.md`:
  - [ ] Security contact information
  - [ ] Vulnerability disclosure policy
  - [ ] Supported versions
- [ ] Create security audit checklist:
  - [ ] Model signature verification coverage
  - [ ] FFI boundary safety (no panics across FFI)
  - [ ] Input validation (audio data, API inputs)
  - [ ] SQL injection prevention (parameterized queries)
  - [ ] Path traversal prevention
  - [ ] Memory safety (unsafe code audit)
- [ ] Audit all `unsafe` code blocks
- [ ] Document threat model
- [ ] Verify FFI boundary tests exist and pass

### Acceptance Criteria
- SECURITY.md exists with complete disclosure policy
- All unsafe code blocks documented and justified
- FFI boundary tests verify no panics escape
- Input validation documented for all external interfaces

### Files
- `SECURITY.md` (to create)
- `crates/lexicon-ffi/src/lib.rs` - FFI boundary review
- `crates/lexicon-core/src/models/registry.rs` - Signature verification

---

## Execution Order

### Week 1: MVP Closure
1. **Work Plan 1**: Validate 10-minute transcription
2. **Work Plan 2**: Validate benchmarks, create BENCHMARKS.md

### Week 2-3: Beta Core
3. ~~**Work Plan 4**: Server rate limiting~~ ✅ **COMPLETE** (2026-01-20)
4. **Work Plan 5**: FFI download cancellation (unblocks mobile UX)

### Week 4-6: Beta Quality
5. **Work Plan 3**: TTS phoneme tokenization (highest effort, critical for quality)

### Week 7+: Beta Completion & Release Prep
6. **Work Plan 6**: Bluetooth HFP 1.6 (if resources available)
7. **Work Plan 7**: Security audit preparation

---

## Dependencies

```
Work Plan 1 (MVP Validation)
    └── No dependencies

Work Plan 2 (Benchmarks)
    └── Depends on: benchmark files exist ✅

Work Plan 3 (TTS Phonemes)
    └── Depends on: G2P research complete

Work Plan 4 (Rate Limiting) ✅ COMPLETE
    └── Custom token bucket implementation (no external deps)

Work Plan 5 (Download Cancel)
    └── Depends on: tokio cancellation pattern

Work Plan 6 (Bluetooth HFP)
    └── Depends on: platform API research

Work Plan 7 (Security)
    └── Depends on: Beta feature complete
```

---

*This document should be updated as work progresses. Check off tasks and update status as items complete.*
