# Test Coverage Expansion Work Plans

> **Created**: 2026-01-20 | **Status**: Planning
>
> This document defines actionable work plans for expanding test coverage across the Lexicon Local Server codebase. Based on current coverage analysis (44.72% overall), these plans target the 80%+ coverage goal defined in design-plan.md.

---

## Executive Summary

**Current State:**
- Overall coverage: 44.72% (1449/3240 lines)
- 222+ unit tests across workspace
- Strong coverage in: storage (83%), TTS types (100%), diarization (92%)
- Weak coverage in: CLI (0%), FFI lib (0%), whisper STT (16%), TTS manager (3%)

**Target State:**
- Overall coverage: 80%+
- All security-critical paths: 100%
- All public APIs: 90%+

---

## Coverage Gap Analysis by Module

### Critical Gaps (0-20% coverage)

| Module | Current | Target | Gap | Priority |
|--------|---------|--------|-----|----------|
| `lexicon-cli/src/main.rs` | 0% (0/391) | 80% | HIGH | P1 |
| `lexicon-ffi/src/lib.rs` | 0% (0/198) | 75% | HIGH | P1 |
| `stt/whisper.rs` | 16% (15/95) | 80% | HIGH | P2 |
| `tts/mod.rs` | 3% (3/108) | 80% | HIGH | P2 |

### Moderate Gaps (20-60% coverage)

| Module | Current | Target | Gap | Priority |
|--------|---------|--------|-----|----------|
| `audio/capture.rs` | 17% (27/157) | 85% | MEDIUM | P2 |
| `models/registry.rs` | 27% (116/424) | 90% | MEDIUM | P2 |
| `tts/kokoro.rs` | 23% (23/100) | 80% | MEDIUM | P3 |
| `tts/platform.rs` | 15% (20/137) | 80% | MEDIUM | P3 |
| `server/routes.rs` | 50% (153/306) | 80% | MEDIUM | P3 |

### Adequate Coverage (60%+)

| Module | Current | Target | Status |
|--------|---------|--------|--------|
| `storage/mod.rs` | 83% (260/314) | 90% | Near target |
| `config.rs` | 69% (81/118) | 90% | Needs work |
| `error.rs` | 87% (40/46) | 95% | Near target |
| `tts/types.rs` | 100% (103/103) | 100% | Complete |
| `stt/diarization.rs` | 92% (135/147) | 90% | Complete |

---

## Work Plan 1: CLI Test Suite (WP-TEST-CLI)

**Priority:** P1 | **Effort:** Medium | **Coverage Target:** 0% → 80%

### Problem Statement

The CLI module has 0% test coverage (391 lines). This includes:
- Command parsing and validation
- Transcription workflow
- Model management commands
- Session management commands
- Export functionality

### Scope

Create comprehensive CLI tests using command-line execution:

```
tests/
└── cli_tests.rs (expand existing stub)
    ├── test_cli_help
    ├── test_cli_version
    ├── test_cli_transcribe_*
    ├── test_cli_models_*
    ├── test_cli_sessions_*
    └── test_cli_export_*
```

### Test Cases

#### Command Parsing Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_cli_help_displays` | `--help` shows usage | main.rs:179-220 |
| `test_cli_version_displays` | `--version` shows version | main.rs:221-230 |
| `test_cli_unknown_command` | Unknown command returns error | main.rs:231-250 |
| `test_cli_config_file_loading` | `--config` loads file | main.rs:250-290 |
| `test_cli_verbose_flag` | `-v` increases log level | main.rs:290-310 |

#### Transcribe Command Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_transcribe_missing_file` | Error on missing audio file | main.rs:320-350 |
| `test_transcribe_invalid_format` | Error on unsupported format | main.rs:350-380 |
| `test_transcribe_with_model` | `--model` selects model | main.rs:380-420 |
| `test_transcribe_with_language` | `--language` sets language | main.rs:420-450 |
| `test_transcribe_with_output` | `--output` writes to file | main.rs:450-500 |
| `test_transcribe_to_stdout` | Default outputs to stdout | main.rs:500-530 |

#### Models Command Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_models_list_empty` | List shows "no models" when empty | main.rs:600-630 |
| `test_models_list_with_models` | List shows available models | main.rs:630-670 |
| `test_models_info_valid` | Info shows model details | main.rs:700-750 |
| `test_models_info_invalid` | Error on unknown model | main.rs:750-780 |

#### Sessions Command Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_sessions_list` | List shows sessions | main.rs:780-820 |
| `test_sessions_delete` | Delete removes session | main.rs:820-860 |
| `test_sessions_delete_invalid` | Error on invalid session ID | main.rs:860-890 |

#### Exit Code Tests
| Test | Description | Expected |
|------|-------------|----------|
| `test_exit_code_success` | Successful command returns 0 | exit(0) |
| `test_exit_code_error` | Failed command returns 1 | exit(1) |
| `test_exit_code_usage` | Bad usage returns 2 | exit(2) |

### Implementation Strategy

1. **Mock Infrastructure**: Create `MockTranscriber`, `MockDatabase` for isolated testing
2. **Process Execution**: Use `std::process::Command` to run CLI binary
3. **Temp Directories**: Use `tempfile` crate for test isolation
4. **Fixture Audio**: Use `tests/fixtures/audio/` files

### Completion Criteria

- [ ] 30+ CLI test cases pass
- [ ] Coverage reaches 80% for `main.rs`
- [ ] All exit codes tested
- [ ] Error messages validated

---

## Work Plan 2: FFI Binding Tests (WP-TEST-FFI)

**Priority:** P1 | **Effort:** High | **Coverage Target:** 0% → 75%

### Problem Statement

The FFI module has 0% coverage on the library implementation (198 lines). This includes:
- Context creation/destruction
- Model loading/unloading
- Transcription operations
- TTS synthesis
- Callback handling

### Scope

Test FFI bindings at the Rust layer (not Swift/Kotlin):

```
crates/lexicon-ffi/
├── src/
│   └── lib.rs (test inline or in tests/)
└── tests/
    ├── context_tests.rs
    ├── model_tests.rs
    ├── transcription_tests.rs
    └── synthesis_tests.rs
```

### Test Cases

#### Context Management
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_create_context_default` | Creates context with defaults | lib.rs:65-100 |
| `test_create_context_custom` | Creates context with config | lib.rs:100-150 |
| `test_context_get_config` | Returns config reference | lib.rs:150-180 |
| `test_context_shutdown` | Clean shutdown | lib.rs:180-200 |

#### Model Operations
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_list_available_models` | Lists models from registry | lib.rs:235-280 |
| `test_get_model_info` | Returns model metadata | lib.rs:280-320 |
| `test_download_model_progress` | Progress callback fires | lib.rs:320-400 |
| `test_download_model_cancel` | Cancellation works | lib.rs:696-702 |
| `test_load_model_success` | Model loads correctly | lib.rs:400-450 |
| `test_load_model_not_found` | Error on missing model | lib.rs:450-480 |
| `test_unload_model` | Model unloads cleanly | lib.rs:480-510 |

#### Transcription Operations
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_transcribe_audio_success` | Transcribes audio buffer | lib.rs:510-560 |
| `test_transcribe_empty_audio` | Handles empty input | lib.rs:560-580 |
| `test_transcribe_callback` | Segment callbacks fire | lib.rs:580-620 |
| `test_transcribe_with_config` | Config options applied | lib.rs:620-660 |

#### Synthesis Operations
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_synthesize_text` | Synthesizes English text | lib.rs:660-690 |
| `test_synthesize_streaming` | Streaming chunks returned | lib.rs:690-720 |
| `test_synthesize_empty` | Handles empty text | lib.rs:720-740 |

#### Error Boundary Tests
| Test | Description | Expected |
|------|-------------|----------|
| `test_error_no_panic` | Errors don't panic | LexiconFfiError returned |
| `test_error_mapping` | Errors map correctly | Correct error variant |
| `test_thread_safety` | Concurrent calls safe | No data races |

### Implementation Strategy

1. **Unit Tests**: Test each FFI function in isolation
2. **Integration Tests**: Test common workflows
3. **Mock Models**: Use mock transcriber/synthesizer where possible
4. **Real Models**: Some tests require actual model files (mark as `#[ignore]`)

### Completion Criteria

- [ ] 25+ FFI test cases pass
- [ ] Coverage reaches 75% for `lib.rs`
- [ ] No panics across FFI boundary
- [ ] Thread safety verified

---

## Work Plan 3: Whisper STT Tests (WP-TEST-STT)

**Priority:** P2 | **Effort:** High | **Coverage Target:** 16% → 80%

### Problem Statement

The whisper integration has 16% coverage (15/95 lines). Most code paths require an actual model file, making testing challenging without fixtures.

### Scope

```
crates/lexicon-core/src/stt/
├── whisper.rs (tests at bottom or in tests.rs)
└── tests.rs (if separate)
```

### Test Cases

#### Model Loading (requires model file)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_whisper_load_tiny_model` | Loads tiny.en model | whisper.rs:49-76 |
| `test_whisper_load_invalid_path` | Error on bad path | whisper.rs:76-90 |
| `test_whisper_unload` | Clean unload | whisper.rs:90-100 |

#### Transcription (requires model file)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_transcribe_5s_english` | Transcribes short audio | whisper.rs:100-150 |
| `test_transcribe_with_timestamps` | Timestamps are accurate | whisper.rs:150-180 |
| `test_transcribe_silence` | Handles silence input | whisper.rs:180-200 |
| `test_transcribe_short_audio` | Handles <1s audio | whisper.rs:200-220 |

#### Translation (requires model file)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_translate_to_english` | Non-English → English | whisper.rs:220-250 |
| `test_translate_config` | Translation config applied | whisper.rs:250-270 |

#### Configuration Tests (no model required)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_transcription_config_defaults` | Default values correct | Already covered in e2e_tests.rs |
| `test_transcription_config_builder` | Builder works | Already covered |
| `test_segment_creation` | Segment struct works | Already covered |

### Implementation Strategy

1. **Mock Layer**: Create `MockWhisperContext` for unit tests
2. **Integration Tests**: Mark model-dependent tests as `#[ignore]` by default
3. **CI Fixture**: Add tiny.en model to CI cache for integration tests
4. **Test Audio**: Use `tests/fixtures/audio/speech_like_5s.wav`

### Completion Criteria

- [ ] 15+ STT test cases defined
- [ ] Mock tests pass without model
- [ ] Integration tests pass with model (CI)
- [ ] Coverage reaches 80%

---

## Work Plan 4: TTS Manager Tests (WP-TEST-TTS)

**Priority:** P2 | **Effort:** Medium | **Coverage Target:** 3% → 80%

### Problem Statement

The TTS manager (`tts/mod.rs`) has only 3% coverage (3/108 lines). The actual synthesis implementations (`kokoro.rs`, `platform.rs`) also have low coverage.

### Scope

```
crates/lexicon-core/src/tts/
├── mod.rs (manager tests)
├── kokoro.rs (ONNX tests - requires model)
└── platform.rs (tts-rs tests)
```

### Test Cases

#### TTS Manager Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_tts_manager_creation` | Creates manager | mod.rs:120-150 |
| `test_tts_manager_synthesize` | Synthesizes text | mod.rs:150-200 |
| `test_tts_manager_streaming` | Streaming synthesis | mod.rs:200-280 |
| `test_tts_manager_fallback` | Falls back to platform | mod.rs:280-350 |
| `test_tts_manager_voice_selection` | Voice selection works | mod.rs:350-400 |

#### Kokoro Tests (requires model)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_kokoro_load_model` | Loads ONNX model | kokoro.rs:40-80 |
| `test_kokoro_synthesize` | Synthesizes text | kokoro.rs:80-150 |
| `test_kokoro_tokenize` | Tokenization works | kokoro.rs:60-90 |

#### Platform TTS Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_platform_tts_create` | Creates platform synth | platform.rs:35-80 |
| `test_platform_tts_list_voices` | Lists system voices | platform.rs:80-120 |
| `test_platform_tts_synthesize` | Synthesizes (note: currently returns silence) | platform.rs:140-180 |

### Implementation Strategy

1. **Mock Synthesizer**: Create mock for manager tests
2. **Platform Tests**: Test tts-rs integration on all platforms
3. **Streaming Tests**: Verify streaming channel behavior
4. **Fallback Tests**: Test automatic fallback logic

### Completion Criteria

- [ ] 20+ TTS test cases pass
- [ ] Manager coverage reaches 80%
- [ ] Fallback logic fully tested
- [ ] Streaming verified

---

## Work Plan 5: Audio Capture Tests (WP-TEST-AUDIO)

**Priority:** P2 | **Effort:** Medium | **Coverage Target:** 17% → 85%

### Problem Statement

Audio capture (`capture.rs`) has 17% coverage (27/157 lines). Most code interacts with hardware, requiring mocking or integration testing.

### Scope

```
crates/lexicon-core/src/audio/
├── capture.rs (audio source tests)
├── mod.rs (pipeline tests)
└── tests/ (if separate)
```

### Test Cases

#### Audio Source Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_cpal_list_devices` | Lists audio devices | capture.rs:60-90 |
| `test_cpal_default_device` | Gets default device | capture.rs:90-120 |
| `test_audio_config_creation` | Creates audio config | capture.rs:120-150 |
| `test_channel_conversion_mono` | Stereo to mono | capture.rs:116-140 |
| `test_channel_conversion_passthrough` | Mono passthrough | capture.rs:140-160 |

#### Ring Buffer Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_ring_buffer_write` | Writes samples | capture.rs:200-230 |
| `test_ring_buffer_read` | Reads chunks | capture.rs:230-260 |
| `test_ring_buffer_overflow` | Handles overflow | capture.rs:260-290 |

#### Device Event Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_device_disconnect_handling` | Handles disconnect | capture.rs:300-340 |
| `test_route_change_handling` | Handles route change | capture.rs:340-370 |

### Implementation Strategy

1. **Mock Audio Backend**: Create mock cpal device for testing
2. **Ring Buffer Unit Tests**: Test buffer in isolation
3. **Integration Tests**: Test with actual audio devices (marked `#[ignore]`)

### Completion Criteria

- [ ] 15+ audio test cases pass
- [ ] Ring buffer fully tested
- [ ] Device handling tested
- [ ] Coverage reaches 85%

---

## Work Plan 6: Model Registry Tests (WP-TEST-REGISTRY)

**Priority:** P2 | **Effort:** Medium | **Coverage Target:** 27% → 90%

### Problem Statement

Model registry has 27% coverage (116/424 lines). Security-critical paths (signature verification, hash checking) need 100% coverage.

### Scope

```
crates/lexicon-core/src/models/
├── registry.rs (registry tests)
├── manifest.rs (manifest tests)
└── tests.rs (existing tests)
```

### Test Cases

#### Security Tests (100% required)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_manifest_signature_valid` | Accepts valid signature | registry.rs:150-180 |
| `test_manifest_signature_invalid` | Rejects invalid signature | registry.rs:180-210 |
| `test_manifest_signature_tampered` | Rejects tampered content | registry.rs:210-240 |
| `test_download_hash_verification` | Verifies SHA-256 | registry.rs:300-350 |
| `test_download_hash_mismatch` | Rejects bad hash | registry.rs:350-400 |
| `test_load_time_verification` | Re-verifies on load | registry.rs:400-450 |

#### Registry Operations
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_registry_new` | Creates registry | registry.rs:70-100 |
| `test_registry_list_models` | Lists available models | registry.rs:100-130 |
| `test_registry_get_model_path` | Returns model path | registry.rs:130-160 |
| `test_registry_download_progress` | Progress callback | registry.rs:450-500 |
| `test_registry_cache_hit` | Skips download if cached | registry.rs:500-550 |

#### Device Profile Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_device_profile_detect` | Detects profile | Already covered |
| `test_device_profile_tier` | Recommends tier | Already covered |
| `test_thermal_headroom_switch` | Thermal triggers switch | registry.rs:800-850 |
| `test_thermal_recovery` | Upgrades after cooling | registry.rs:850-900 |

### Implementation Strategy

1. **Test Fixtures**: Use `tests/fixtures/manifests/` for signature tests
2. **Mock Network**: Mock HTTP client for download tests
3. **Security Focus**: Ensure all verification paths tested

### Completion Criteria

- [ ] 20+ registry test cases pass
- [ ] Security tests: 100% coverage
- [ ] Thermal switching tested
- [ ] Overall coverage reaches 90%

---

## Work Plan 7: Server Route Tests (WP-TEST-SERVER)

**Priority:** P3 | **Effort:** Medium | **Coverage Target:** 50% → 80%

### Problem Statement

Server routes have 50% coverage (153/306 lines). WebSocket endpoints and some REST endpoints lack tests.

### Scope

```
crates/lexicon-server/
├── src/routes.rs (route handlers)
└── tests/
    ├── integration_tests.rs (existing)
    └── websocket_tests.rs (existing)
```

### Test Cases

#### REST API Tests (expand existing)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_models_list_response` | JSON format correct | routes.rs:200-250 |
| `test_models_download_endpoint` | Download initiates | routes.rs:250-300 |
| `test_sessions_list_pagination` | Pagination works | routes.rs:460-500 |
| `test_sessions_export_formats` | Multiple formats | routes.rs:520-580 |
| `test_transcribe_batch` | Batch transcription | routes.rs:580-650 |

#### WebSocket Tests (expand existing)
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_ws_stt_connection` | Connection established | routes.rs:700-750 |
| `test_ws_stt_audio_stream` | Audio streaming works | routes.rs:750-850 |
| `test_ws_stt_error_handling` | Errors handled | routes.rs:850-900 |
| `test_ws_tts_connection` | TTS connection works | routes.rs:900-950 |
| `test_ws_tts_streaming` | Audio chunks returned | routes.rs:950-1000 |

#### Error Handling Tests
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_error_response_format` | Error JSON format | routes.rs:100-150 |
| `test_404_response` | Not found handling | routes.rs:150-180 |
| `test_400_bad_request` | Bad request handling | routes.rs:180-200 |

### Implementation Strategy

1. **Test Client**: Use `axum_test` or similar for HTTP tests
2. **WebSocket Client**: Use `tokio-tungstenite` for WS tests
3. **Mock Services**: Mock transcriber/synthesizer for isolation

### Completion Criteria

- [ ] 25+ server test cases pass
- [ ] WebSocket streaming tested end-to-end
- [ ] Error responses validated
- [ ] Coverage reaches 80%

---

## Work Plan 8: Config and Platform Tests (WP-TEST-CONFIG)

**Priority:** P3 | **Effort:** Low | **Coverage Target:** 69% → 90%

### Problem Statement

Config module has 69% coverage (81/118 lines). Some edge cases in config loading and platform detection are untested.

### Test Cases

#### Config Loading
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_config_from_file` | Loads from TOML | config.rs:250-280 |
| `test_config_from_env` | Environment override | config.rs:280-310 |
| `test_config_validation` | Validates paths | config.rs:310-340 |
| `test_config_defaults` | Default values | Already covered |

#### Platform Detection
| Test | Description | Lines Covered |
|------|-------------|---------------|
| `test_platform_detect_linux` | Linux detection | platform.rs:120-160 |
| `test_platform_capabilities` | Platform capabilities | platform.rs:160-200 |
| `test_memory_detection` | Memory size detection | platform.rs:200-250 |

### Completion Criteria

- [ ] 10+ config test cases pass
- [ ] All config paths tested
- [ ] Platform detection complete
- [ ] Coverage reaches 90%

---

## Implementation Prioritization

### Phase 1: Critical Infrastructure (Week 1)
1. **WP-TEST-CLI** - Enable CLI testing
2. **WP-TEST-FFI** - Enable mobile testing

### Phase 2: Core Engines (Week 2)
3. **WP-TEST-STT** - Whisper integration
4. **WP-TEST-TTS** - TTS manager
5. **WP-TEST-AUDIO** - Audio capture

### Phase 3: Security & Integration (Week 3)
6. **WP-TEST-REGISTRY** - Model security
7. **WP-TEST-SERVER** - Server routes
8. **WP-TEST-CONFIG** - Configuration

---

## Test Infrastructure Requirements

### Mock Implementations Needed

```rust
// Mock transcriber for FFI/CLI tests
pub struct MockTranscriber {
    pub segments: Vec<Segment>,
    pub delay_ms: u64,
}

// Mock synthesizer for TTS tests
pub struct MockSynthesizer {
    pub audio: AudioBuffer,
}

// Mock audio source for capture tests
pub struct MockAudioSource {
    pub samples: Vec<f32>,
    pub chunk_size: usize,
}

// Mock HTTP client for registry tests
pub struct MockHttpClient {
    pub responses: HashMap<String, Vec<u8>>,
}
```

### Test Fixture Requirements

| Fixture | Location | Status |
|---------|----------|--------|
| `5_second_english.wav` | tests/fixtures/audio/ | Needed |
| `speech_like_5s.wav` | tests/fixtures/audio/ | Exists |
| `silence_10s.wav` | tests/fixtures/audio/ | Exists |
| `tiny.en.bin` | tests/fixtures/models/ | Needed for CI |
| `valid_signed.json` | tests/fixtures/manifests/ | Exists |
| `invalid_signature.json` | tests/fixtures/manifests/ | Exists |

### CI Integration

```yaml
# Add to .github/workflows/ci.yml
test-coverage:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install cargo-tarpaulin
      run: cargo install cargo-tarpaulin
    - name: Run coverage
      run: cargo tarpaulin --workspace --out Lcov
    - name: Upload coverage
      uses: codecov/codecov-action@v4
```

---

## Success Metrics

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| Overall coverage | 44.72% | 80%+ | cargo-tarpaulin |
| Security coverage | ~70% | 100% | Manual audit |
| Test count | 222 | 400+ | cargo test |
| Test execution time | ~5s | <30s | CI metrics |

---

## References

- [design-plan.md](design-plan.md) - Test Plan section (lines 850-1560)
- [architecture.md](architecture.md) - System architecture
- Coverage report: `cargo tarpaulin --workspace --out Html`

---

*This document serves as the roadmap for test coverage expansion. Update as tests are implemented.*
