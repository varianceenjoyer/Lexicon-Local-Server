# Test Coverage Expansion Work Plans

> **Created**: 2026-01-20 | **Phase**: MVP/Beta Transition
>
> This document defines work plans for expanding test coverage to meet targets defined in design-plan.md.

---

## Executive Summary

Current test coverage analysis reveals:
- **214 unit tests** across the workspace
- **27+ integration tests** for server endpoints
- **Critical gaps** in CLI E2E tests, FFI tests, and graceful shutdown testing
- **Missing tests** for long-running stability, TTS fallback, and platform-specific features

### Coverage Gap Summary

| Package | Target | Estimated Current | Gap |
|---------|--------|-------------------|-----|
| WP-CORE | 90% | ~85% | 5% |
| WP-AUDIO | 85% | ~80% | 5% |
| WP-DB | 90% | ~90% | 0% |
| WP-MODEL | 90% | ~90% | 0% |
| WP-STT | 80% | ~75% | 5% |
| WP-TTS | 80% | ~60% | 20% |
| WP-FFI | 75% | ~0% | 75% |
| WP-SERVER | 80% | ~75% | 5% |
| WP-CLI | 80% | ~0% | 80% |
| **Overall** | **80%** | **~70%** | **10%** |

---

## Work Plan TC-01: CLI End-to-End Tests

**Priority:** CRITICAL | **Effort:** Medium | **Phase:** MVP Blocker

### Problem
The CLI test file (`crates/lexicon-cli/tests/cli_tests.rs`) contains only structural validation tests that don't actually invoke the CLI binary. MVP exit criterion "Transcribe command produces valid SRT" cannot be validated.

**Current State:** 21 tests that validate formats and helper functions, but 0 tests that invoke `lexicon` binary.

### Tasks

- [ ] Add `assert_cmd` to lexicon-cli dev-dependencies
- [ ] Add `predicates` for output assertions
- [ ] Implement actual CLI invocation tests:
  - [ ] `test_cli_help` - Verify help output
  - [ ] `test_cli_version` - Verify version output
  - [ ] `test_cli_transcribe_file` - Transcribe test audio to SRT
  - [ ] `test_cli_transcribe_progress` - Verify progress output
  - [ ] `test_cli_transcribe_missing_file` - Error handling
  - [ ] `test_cli_transcribe_missing_model` - Error handling
  - [ ] `test_cli_models_list` - List available models
  - [ ] `test_cli_models_download` - Download model (mock server)
  - [ ] `test_cli_serve_starts` - Server starts on default port
  - [ ] `test_cli_export_srt` - Export session to SRT
  - [ ] `test_cli_exit_codes` - Verify correct exit codes
- [ ] Generate shell completion scripts in tests
- [ ] Add CI step for CLI integration tests

### Implementation

```rust
// crates/lexicon-cli/tests/cli_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    Command::cargo_bin("lexicon")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("transcribe"))
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("models"));
}

#[test]
fn test_cli_transcribe_file() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("tests/fixtures/audio/sine_440hz_2s.wav");

    Command::cargo_bin("lexicon")
        .unwrap()
        .args(["transcribe", fixtures.to_str().unwrap(), "--format", "srt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-->"));
}
```

### Acceptance Criteria
- All 11 CLI tests pass
- Help and version commands work
- Transcribe produces valid SRT output
- Exit codes follow Unix conventions (0=success, 1=error, 2=usage)

### Files
- `crates/lexicon-cli/Cargo.toml` - Add assert_cmd, predicates
- `crates/lexicon-cli/tests/cli_tests.rs` - Implement tests

---

## Work Plan TC-02: FFI Boundary Tests

**Priority:** HIGH | **Effort:** Medium | **Phase:** Beta Blocker

### Problem
No tests exist for the FFI layer. Critical safety requirements are untested:
- Panic safety (panics must not cross FFI boundary)
- Thread safety (concurrent FFI calls)
- Memory leaks (Arc/reference counting)
- Error mapping (Rust errors → platform exceptions)

### Tasks

- [ ] Create `crates/lexicon-ffi/tests/` directory
- [ ] Implement safety tests:
  - [ ] `test_panic_does_not_cross_ffi` - Verify panic is caught
  - [ ] `test_concurrent_ffi_calls` - Thread safety
  - [ ] `test_error_mapping` - Rust error → FFI error
  - [ ] `test_context_lifecycle` - Create/destroy without leak
  - [ ] `test_session_lifecycle` - Start/stop without leak
  - [ ] `test_callback_invocation` - Progress callbacks work
  - [ ] `test_null_pointer_handling` - Invalid inputs don't crash
- [ ] Add leak detection test (100 cycle create/destroy)
- [ ] Document FFI safety guarantees

### Implementation

```rust
// crates/lexicon-ffi/tests/safety_tests.rs
use std::panic;
use lexicon_ffi::*;

#[test]
fn test_panic_does_not_cross_ffi() {
    // This should return an error, not panic
    let result = panic::catch_unwind(|| {
        // Call FFI function that might panic internally
        let ctx = create_context_with_invalid_config();
        // Should not reach here if panic escapes
    });

    // Panic should be caught by FFI layer, not propagate
    assert!(result.is_ok(), "Panic escaped FFI boundary!");
}

#[test]
fn test_concurrent_ffi_calls() {
    use std::thread;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                // Each thread creates/destroys context
                let config = default_test_config();
                let ctx = LexiconContext::new(config).unwrap();
                drop(ctx);
            })
        })
        .collect();

    for h in handles {
        h.join().expect("Thread panicked");
    }
}

#[test]
fn test_memory_leak_100_cycles() {
    for _ in 0..100 {
        let config = default_test_config();
        let ctx = LexiconContext::new(config).unwrap();
        // Context should be fully cleaned up on drop
    }
    // If we get here without OOM, no major leaks
}
```

### Acceptance Criteria
- Panics do not cross FFI boundary (caught and converted to errors)
- Concurrent FFI calls are thread-safe
- 100-cycle test shows no memory growth
- All FFI errors map to appropriate error codes

### Files
- `crates/lexicon-ffi/tests/safety_tests.rs` - New file
- `crates/lexicon-ffi/Cargo.toml` - Add test dependencies

---

## Work Plan TC-03: Server Graceful Shutdown Tests

**Priority:** MEDIUM | **Effort:** Low | **Phase:** Beta

### Problem
WP-SERVER test requirement `graceful_shutdown` is missing. Data preservation on server shutdown is untested.

### Tasks

- [ ] Add graceful shutdown test to `integration_tests.rs`:
  - [ ] Start server
  - [ ] Create session with data
  - [ ] Send SIGTERM/shutdown signal
  - [ ] Verify session data persisted
  - [ ] Restart server
  - [ ] Verify data retrievable
- [ ] Test WebSocket connection cleanup on shutdown
- [ ] Verify no data corruption during shutdown

### Implementation

```rust
// crates/lexicon-server/tests/integration_tests.rs

#[tokio::test]
async fn test_graceful_shutdown_preserves_data() {
    // Start server
    let server = TestServer::new().await;

    // Create session with segments
    let session_id = server.create_session().await;
    server.add_segment(session_id, "Test segment").await;

    // Initiate graceful shutdown
    server.shutdown().await;

    // Start new server instance with same data dir
    let server2 = TestServer::new_with_data_dir(server.data_dir()).await;

    // Verify data persisted
    let segments = server2.get_segments(session_id).await;
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "Test segment");
}

#[tokio::test]
async fn test_shutdown_closes_websocket_connections() {
    let server = TestServer::new().await;
    let ws_client = server.connect_websocket("/api/v1/stt/stream").await;

    // Start shutdown
    let shutdown_handle = server.begin_shutdown();

    // WebSocket should receive close frame
    let close_frame = ws_client.recv_close().await;
    assert!(close_frame.is_ok());

    shutdown_handle.await;
}
```

### Acceptance Criteria
- Session data persists across graceful shutdown
- WebSocket connections receive close frames
- No data corruption on shutdown
- Server restarts cleanly after shutdown

### Files
- `crates/lexicon-server/tests/integration_tests.rs` - Add shutdown tests

---

## Work Plan TC-04: TTS Fallback Integration Tests

**Priority:** HIGH | **Effort:** Medium | **Phase:** Beta

### Problem
TTS fallback tests are incomplete. The test requirements from design-plan.md are partially covered:
- `fallback_non_english_auto` - Partially tested
- `fallback_model_failure` - Not fully tested
- `platform_tts_*` - Blocked by WP-ROADBLOCK-13 (audio capture)

Note: WP-ROADBLOCK-13 (Platform TTS Audio Capture) was marked complete, so we can now test these.

### Tasks

- [ ] Add TTS fallback integration tests:
  - [ ] `test_fallback_non_english_triggers_platform_tts` - French text triggers fallback
  - [ ] `test_fallback_on_kokoro_load_failure` - Missing model triggers fallback
  - [ ] `test_fallback_user_preference` - User can force platform TTS
  - [ ] `test_platform_tts_produces_audio` - Verify non-zero audio output
- [ ] Add platform-specific TTS tests (conditional compilation):
  - [ ] macOS: AVSpeechSynthesizer test
  - [ ] Windows: SAPI test
  - [ ] Linux: espeak test
- [ ] Test fallback chain: Kokoro → Platform → Error

### Implementation

```rust
// crates/lexicon-core/src/tts/tests.rs

#[test]
fn test_fallback_non_english_triggers_platform_tts() {
    let config = TtsConfig {
        preferred_engine: TtsEngine::Kokoro,
        fallback_enabled: true,
    };

    let synthesizer = TtsSynthesizer::new(config).unwrap();

    // French text should trigger platform fallback
    let result = synthesizer.synthesize("Bonjour le monde");
    assert!(result.is_ok());

    // Verify we used platform TTS (check engine used)
    let audio = result.unwrap();
    assert!(!audio.samples.is_empty());
}

#[test]
fn test_fallback_on_kokoro_load_failure() {
    let config = TtsConfig {
        preferred_engine: TtsEngine::Kokoro,
        fallback_enabled: true,
    };

    // Create with missing Kokoro model
    let synthesizer = TtsSynthesizer::new_without_kokoro(config).unwrap();

    // Should fall back to platform TTS
    let result = synthesizer.synthesize("Hello world");
    assert!(result.is_ok());
}

#[test]
fn test_platform_tts_produces_nonzero_audio() {
    let synthesizer = PlatformSynthesizer::new().unwrap();
    let audio = synthesizer.synthesize("Hello world").unwrap();

    // Verify audio is not silent
    let max_amplitude = audio.samples.iter()
        .map(|s| s.abs())
        .fold(0.0f32, |a, b| a.max(b));

    assert!(max_amplitude > 0.01, "Platform TTS produced silent audio");
}
```

### Acceptance Criteria
- Non-English text triggers platform TTS fallback
- Missing Kokoro model triggers platform fallback
- User preference for platform TTS works
- Platform TTS produces non-zero audio on all platforms

### Files
- `crates/lexicon-core/src/tts/mod.rs` - Add integration tests
- `crates/lexicon-core/src/tts/tests.rs` - New test module

---

## Work Plan TC-05: Long-Running Stability Tests

**Priority:** MEDIUM | **Effort:** Medium | **Phase:** Beta

### Problem
WP-STT test requirement `stability_30min` is missing. Long-running transcription stability is untested.

### Tasks

- [ ] Create stability test infrastructure
- [ ] Implement 30-minute transcription test:
  - [ ] Generate or obtain 30-minute test audio
  - [ ] Transcribe entire file
  - [ ] Monitor memory usage throughout
  - [ ] Verify no memory growth pattern
  - [ ] Verify transcription quality consistent
- [ ] Add as optional CI test (runs on schedule, not per-PR)
- [ ] Document memory behavior expectations

### Implementation

```rust
// tests/stability_tests.rs

#[test]
#[ignore] // Run with: cargo test --ignored -- stability
fn test_stt_30_minute_stability() {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let mut memory_samples = Vec::new();

    // Generate 30 minutes of test audio (in chunks)
    let chunk_duration = Duration::from_secs(30);
    let total_duration = Duration::from_secs(30 * 60);

    let transcriber = WhisperTranscriber::new_with_model("tiny").unwrap();

    while start.elapsed() < total_duration {
        // Generate audio chunk
        let audio = generate_test_audio(chunk_duration);

        // Transcribe
        let result = transcriber.transcribe(&audio);
        assert!(result.is_ok());

        // Sample memory
        let mem = get_process_memory();
        memory_samples.push(mem);
    }

    // Verify no significant memory growth
    let initial_avg = memory_samples[..10].iter().sum::<usize>() / 10;
    let final_avg = memory_samples[memory_samples.len()-10..].iter().sum::<usize>() / 10;

    let growth = (final_avg as f64 - initial_avg as f64) / initial_avg as f64;
    assert!(growth < 0.1, "Memory grew by {:.1}% over 30 minutes", growth * 100.0);
}
```

### Acceptance Criteria
- 30-minute transcription completes without crash
- Memory growth < 10% over duration
- Transcription quality consistent (no degradation)
- Test runs in CI on schedule (nightly)

### Files
- `tests/stability_tests.rs` - New file
- `.github/workflows/nightly.yml` - Add stability test job

---

## Work Plan TC-06: Coverage Reporting Infrastructure

**Priority:** LOW | **Effort:** Low | **Phase:** Beta

### Problem
Coverage targets are defined in design-plan.md but actual coverage is not measured or tracked. CI generates coverage but there's no enforcement.

### Tasks

- [ ] Add coverage threshold enforcement to CI
- [ ] Create coverage badge for README
- [ ] Document local coverage generation
- [ ] Set minimum coverage requirements per crate
- [ ] Add PR coverage diff reporting

### Implementation

```yaml
# .github/workflows/ci.yml addition

coverage:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: llvm-tools-preview

    - name: Install cargo-llvm-cov
      uses: taiki-e/install-action@cargo-llvm-cov

    - name: Generate coverage
      run: cargo llvm-cov --workspace --all-features --codecov --output-path codecov.json

    - name: Check coverage threshold
      run: |
        TOTAL=$(jq '.totals.lines.percent' codecov.json)
        echo "Total coverage: $TOTAL%"
        if (( $(echo "$TOTAL < 80" | bc -l) )); then
          echo "Coverage below 80% threshold!"
          exit 1
        fi

    - name: Upload to Codecov
      uses: codecov/codecov-action@v4
```

### Acceptance Criteria
- Coverage measured per-crate and workspace total
- PR comments show coverage diff
- Coverage threshold enforced (80% workspace)
- Local coverage generation documented

### Files
- `.github/workflows/ci.yml` - Add coverage enforcement
- `README.md` - Add coverage badge
- `CONTRIBUTING.md` - Document coverage requirements

---

## Priority Summary

| ID | Work Plan | Priority | Effort | Phase |
|----|-----------|----------|--------|-------|
| TC-01 | CLI E2E Tests | CRITICAL | Medium | MVP |
| TC-02 | FFI Boundary Tests | HIGH | Medium | Beta |
| TC-03 | Graceful Shutdown Tests | MEDIUM | Low | Beta |
| TC-04 | TTS Fallback Tests | HIGH | Medium | Beta |
| TC-05 | Long-Running Stability | MEDIUM | Medium | Beta |
| TC-06 | Coverage Infrastructure | LOW | Low | Beta |

---

## Execution Order

### Week 1: MVP Test Completion
1. **TC-01**: CLI E2E Tests (unblocks MVP exit criterion)

### Week 2: Beta Safety
2. **TC-02**: FFI Boundary Tests (critical for mobile platforms)
3. **TC-03**: Graceful Shutdown Tests (data safety)

### Week 3: Beta Quality
4. **TC-04**: TTS Fallback Tests (feature completeness)
5. **TC-05**: Long-Running Stability (production readiness)

### Week 4: Quality Infrastructure
6. **TC-06**: Coverage Reporting (quality gates)

---

## Dependencies

```
TC-01 (CLI E2E)
    └── Depends on: assert_cmd crate
    └── Depends on: test audio fixtures ✅

TC-02 (FFI Safety)
    └── Depends on: lexicon-ffi compiled
    └── Blocks: WP-IOS, WP-ANDROID tests

TC-03 (Graceful Shutdown)
    └── Depends on: TestServer helper
    └── Blocks: Production deployment confidence

TC-04 (TTS Fallback)
    └── Depends on: WP-ROADBLOCK-13 complete ✅
    └── Blocks: TTS feature validation

TC-05 (Stability)
    └── Depends on: 30-min test audio
    └── Blocks: Production deployment confidence

TC-06 (Coverage)
    └── Depends on: All other test work complete
    └── Blocks: Quality gates
```

---

*Update this document as work progresses. Check off tasks and update status as items complete.*
