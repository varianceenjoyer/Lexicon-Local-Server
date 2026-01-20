# Next Steps Work Plans

> **Created**: 2026-01-20 | **Phase**: MVP → Beta Transition
>
> Actionable work plans based on unchecked items in design-plan.md

---

## Summary

| Priority | Work Plan | Phase | Effort | Status |
|----------|-----------|-------|--------|--------|
| 1 | MVP Validation | MVP | Low | 🟡 In Progress |
| 2 | Benchmark Validation | Beta | Low | ✅ Complete |
| 3 | TTS Phoneme Tokenization | Beta | High | ✅ Complete |
| 4 | Server Rate Limiting | Beta | Medium | ✅ Complete |
| 5 | FFI Download Cancellation | Beta | Medium | ✅ Complete |
| 6 | Bluetooth HFP 1.6 | Beta | High | ⬜ Not Started |
| 7 | Security Audit Prep | Release | Medium | ✅ Complete |
| 8 | CLI E2E Tests | MVP | Medium | ✅ Complete |
| 9 | FFI Boundary Safety Tests | Beta | Medium | ✅ Complete |
| 10 | Server Graceful Shutdown Tests | Beta | Low | ✅ Complete |
| 11 | TTS Fallback Integration Tests | Beta | Medium | ✅ Complete |
| 12 | Long-Running STT Stability Tests | Beta | Medium | ✅ Complete |

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

## Work Plan 3: TTS Phoneme Tokenization (WP-ROADBLOCK-03) ✅ COMPLETE

**Priority:** HIGH | **Effort:** High | **Phase:** Beta Quality

**Completed:** 2026-01-20

### Implementation Summary
Integrated `misaki-rs` (MIT licensed) for proper grapheme-to-phoneme conversion:
- No GPL dependencies (espeak-ng not required)
- Supports both US and British English pronunciation
- G2P performance verified at <50ms for typical sentences
- Added phonemes.rs module with Kokoro vocabulary mapping and basic fallback tokenizer

### Tasks (All Complete)

- [x] Research Kokoro model's expected input format (IPA phonemes via misaki G2P)
- [x] Evaluate G2P options:
  - [x] ~~Option A: `espeakng-sys` crate~~ (GPL, not chosen)
  - [x] ~~Option B: CMU Pronouncing Dictionary~~ (Arpabet vs IPA mismatch)
  - [x] **Option C: `misaki-rs` - native Rust port of hexgrad/misaki** (chosen)
- [x] Implement chosen G2P solution
- [x] Add phoneme tokenization unit tests (19 tests)
- [x] Benchmark phoneme conversion latency (<50ms verified)
- [x] Fallback to platform TTS works if phoneme conversion fails

### Acceptance Criteria (All Met)
- Kokoro produces natural-sounding speech for common English sentences
- Phoneme tokenization is tested with known inputs
- Performance impact documented (target: <50ms for typical sentence)
- Fallback to platform TTS works if phoneme conversion fails

### Files Modified
- `crates/lexicon-core/src/tts/kokoro.rs` - G2P integration with misaki-rs
- `crates/lexicon-core/src/tts/mod.rs` - Added phonemes module export
- `crates/lexicon-core/src/tts/phonemes.rs` - New Kokoro vocabulary mapping
- `crates/lexicon-core/Cargo.toml` - Added misaki-rs dependency

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

## Work Plan 5: FFI Download Cancellation (WP-ROADBLOCK-04) ✅ COMPLETE

**Priority:** MEDIUM | **Effort:** Medium | **Phase:** Beta (Mobile UX)

**Completed:** 2026-01-20

### Implementation Summary
Implemented download cancellation in the FFI layer using `tokio_util::CancellationToken`:
- Added `cancel_tokens` HashMap to `ModelDownloader` to track active downloads
- Added `on_cancelled` callback to `DownloadCallback` trait
- Implemented `cancel_download()` method that returns `true` if cancellation succeeded
- Added `is_downloading()` method to check if a download is active
- Integrated with `download_model_cancellable()` in ModelRegistry

### Tasks (All Complete)

- [x] Add cancellation token type (e.g., `tokio_util::sync::CancellationToken`)
- [x] Modify `ModelRegistry::download_model()` to accept cancellation token (uses download_model_cancellable)
- [x] Store active download handles in `ModelDownloader` struct
- [x] Implement `cancel_download()` to:
  - [x] Find active download by model_id
  - [x] Trigger cancellation token
  - [x] Clean up partial download file
- [x] Update progress callback to check cancellation state
- [x] Handle cancellation gracefully (return appropriate error type)
- [x] Test re-download after cancellation works correctly

### Acceptance Criteria (All Met)
- `cancel_download()` aborts an active download
- Partial files are cleaned up on cancellation (via registry)
- Cancellation fires callback with `on_cancelled` method
- Re-downloading after cancellation works correctly

### Files Modified
- `crates/lexicon-ffi/src/lib.rs` - Implemented cancel_download and related infrastructure
- `crates/lexicon-ffi/Cargo.toml` - Added tokio-util dependency
- `crates/lexicon-ffi/tests/ffi_tests.rs` - Added 8 cancellation tests

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
4. ~~**Work Plan 5**: FFI download cancellation~~ ✅ **COMPLETE** (2026-01-20)

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

Work Plan 5 (Download Cancel) ✅ COMPLETE
    └── Uses tokio_util::CancellationToken with download_model_cancellable

Work Plan 6 (Bluetooth HFP)
    └── Depends on: platform API research

Work Plan 7 (Security)
    └── Depends on: Beta feature complete
```

---

*This document should be updated as work progresses. Check off tasks and update status as items complete.*
