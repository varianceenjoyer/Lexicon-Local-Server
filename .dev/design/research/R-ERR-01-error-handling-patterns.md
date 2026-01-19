# Research Plan: Rust Error Handling Patterns

**ID:** R-ERR-01
**Status:** Complete
**Category:** Architecture
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Error handling guide

---

## Problem Statement

Audio/ML pipelines have complex failure modes spanning multiple subsystems:
- Audio capture failures (device disconnected, permissions revoked)
- Model loading failures (file not found, corrupt model, OOM)
- Inference failures (malformed input, internal errors)
- I/O failures (disk full, network timeout)

Without a coherent error handling strategy:
- Errors may be lost or poorly communicated
- Recovery becomes ad-hoc and inconsistent
- FFI boundaries add complexity (Rust errors to Swift/Kotlin)
- User experience suffers from cryptic error messages

**Engineering Challenge:** Design error handling patterns that are robust, informative, and work across FFI boundaries.

---

## Research Questions

### RQ1: What error categories exist in the pipeline? ✅ COMPLETE

**Dependency Error Types Cataloged:**

#### whisper-rs ([WhisperError](https://docs.rs/whisper-rs/latest/whisper_rs/enum.WhisperError.html))
| Variant | Description | Recoverable |
|---------|-------------|-------------|
| `ContextCreateFailed` | Failed to create context | No |
| `StateCreateFailed` | Creating state pointer failed | No |
| `SpectrogramFailed` | Failed to calculate spectrogram | No |
| `ModelEvalFailed` | Failed to evaluate model | No |
| `InvalidMelBands` | Invalid number of mel bands | No |
| `InvalidUtf8` | Invalid UTF-8 in string from Whisper | No |
| `NullByte` | Null byte in user-provided string | Yes (validate input) |
| `NullPointer` | Whisper returned null pointer | No |
| `NoSamples` | No samples provided | Yes (validate input) |
| `TokenizeFailed` | Failed to tokenize text | No |
| `GenericError` | Generic whisper error | Depends |

#### ort ([OrtError](https://docs.rs/onnxruntime/latest/onnxruntime/error/enum.OrtError.html))
| Variant | Description | Recoverable |
|---------|-------------|-------------|
| `Session(OrtApiError)` | Session creation failed | No |
| `Run(OrtApiError)` | Inference run failed | Maybe |
| `FileDoesNotExists` | Model file not found | Yes (download) |
| `NonMatchingDimensions` | Input shape mismatch | Yes (validate input) |
| `CreateTensor` | Tensor creation failed | No |
| `DownloadError` | Model download failed | Yes (retry) |

#### cpal ([StreamError](https://docs.rs/cpal/latest/cpal/enum.StreamError.html))
| Variant | Description | Recoverable |
|---------|-------------|-------------|
| `DeviceNotAvailable` | Device disconnected | Yes (reconnect) |
| `BackendSpecific` | Backend-specific error | Depends |
| `StreamInvalidated` | Stream must be rebuilt | Yes (rebuild) |
| `BufferUnderrun` | Buffer underrun/overrun | Yes (adjust buffer) |

**Other cpal errors:** `DevicesError`, `DeviceNameError`, `PauseStreamError`, `PlayStreamError`, `SupportedStreamConfigsError`

#### Platform-Specific Errors
| Platform | Error Type | Description |
|----------|------------|-------------|
| iOS | Permission denied | Microphone access not granted |
| iOS | Session interrupted | Phone call, Siri, etc. |
| Android | Permission denied | RECORD_AUDIO not granted |
| Android | Audio focus lost | Another app took audio focus |
| Both | Background terminated | OS killed background process |

### RQ2: What Rust error handling patterns apply? ✅ COMPLETE

**Key Finding: Use `thiserror` for libraries, `anyhow` for applications**

| Crate | Use When | Pattern |
|-------|----------|---------|
| **thiserror** | Callers need to match on error variants | Define enum with `#[derive(thiserror::Error)]` |
| **anyhow** | Callers just propagate/log errors | Use `anyhow::Result<T>` and `.context()` |
| **Both together** | Library exposes errors, app adds context | thiserror in lib, anyhow in app |

**For Lexicon (a library):** Use `thiserror` because:
- Mobile apps need to handle errors specifically (show permission dialog, retry download, etc.)
- FFI requires structured error types for translation to Swift/Kotlin exceptions
- Error variants drive recovery actions

**Recommended Pattern:**
```rust
// In library: structured errors with thiserror
#[derive(Debug, thiserror::Error)]
pub enum LexiconError { ... }

// In application/wrapper: context with anyhow
fn transcribe_file(path: &str) -> anyhow::Result<String> {
    let audio = load_audio(path)
        .context("Failed to load audio file")?;
    ...
}
```

### RQ3: How do errors cross FFI boundaries? ✅ COMPLETE

#### UniFFI (Recommended for Swift/Kotlin)

UniFFI automatically translates Rust `Result<T, E>` to native exceptions:

```rust
// Rust (with UniFFI)
#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum LexiconError {
    #[error("Model not found: {path}")]
    ModelNotFound { path: String },
}

#[uniffi::export]
fn transcribe(audio: &[f32]) -> Result<String, LexiconError> { ... }
```

```swift
// Swift (generated)
do {
    let result = try transcribe(audio: samples)
} catch LexiconError.ModelNotFound(let path) {
    // Handle specific error
}
```

```kotlin
// Kotlin (generated)
try {
    val result = transcribe(audio)
} catch (e: LexiconError.ModelNotFound) {
    // Handle specific error
}
```

#### C ABI (Manual FFI)

**Rule:** Never let panics cross FFI boundaries. Use `catch_unwind`:

```rust
#[no_mangle]
pub extern "C" fn lexicon_transcribe(
    audio: *const f32,
    len: usize,
    out_error: *mut LexiconErrorCode,
) -> *mut c_char {
    let result = std::panic::catch_unwind(|| {
        // Actual implementation
    });

    match result {
        Ok(Ok(text)) => {
            *out_error = LexiconErrorCode::Ok;
            CString::new(text).unwrap().into_raw()
        }
        Ok(Err(e)) => {
            *out_error = e.into();  // Convert to error code
            std::ptr::null_mut()
        }
        Err(_panic) => {
            *out_error = LexiconErrorCode::InternalPanic;
            std::ptr::null_mut()
        }
    }
}
```

**Important:** `extern "C"` functions that panic will abort. Use `catch_unwind` for graceful handling.

### RQ4: How should errors be communicated to users? ✅ COMPLETE

#### Error Message Layers

| Layer | Audience | Content | Example |
|-------|----------|---------|---------|
| **Technical** | Developers/logs | Full details, stack traces | `WhisperError::SpectrogramFailed at audio.rs:142` |
| **Operational** | App developers | Actionable info | `Model file corrupted: sha256 mismatch` |
| **User-facing** | End users | Simple, actionable | `The speech model is damaged. Please re-download.` |

#### Logging with tracing

```rust
use tracing::{error, warn, info, instrument};

#[instrument(skip(audio), fields(audio_len = audio.len()))]
pub fn transcribe(audio: &[f32]) -> Result<String, LexiconError> {
    if audio.is_empty() {
        warn!("Empty audio provided");
        return Err(LexiconError::AudioTooShort { min_seconds: 0.1 });
    }

    match whisper_ctx.transcribe(audio) {
        Ok(text) => {
            info!(text_len = text.len(), "Transcription complete");
            Ok(text)
        }
        Err(e) => {
            error!(error = ?e, "Whisper transcription failed");
            Err(LexiconError::Inference(e.into()))
        }
    }
}
```

#### Log Levels Policy

| Level | Use For |
|-------|---------|
| `ERROR` | Unrecoverable failures, data loss |
| `WARN` | Recoverable issues, degraded operation |
| `INFO` | Significant state changes (session start/end) |
| `DEBUG` | Detailed operation flow |
| `TRACE` | Very verbose, per-sample data |

---

## Known Patterns & Prior Art

| Pattern | Use Case | Crate |
|---------|----------|-------|
| `thiserror` | Library error types | thiserror |
| `anyhow` | Application error handling | anyhow |
| Error enums | Categorical errors | std |
| Error codes | C FFI | custom |
| Result types | UniFFI | uniffi |

---

## Proposed Error Architecture

### Error Hierarchy
```rust
/// Top-level error type for the library
#[derive(Debug, thiserror::Error)]
pub enum LexiconError {
    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    #[error("Model error: {0}")]
    Model(#[from] ModelError),

    #[error("Inference error: {0}")]
    Inference(#[from] InferenceError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("No audio input device available")]
    NoInputDevice,

    #[error("Microphone permission denied")]
    PermissionDenied,

    #[error("Audio device disconnected")]
    DeviceDisconnected,

    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    #[error("Audio stream error: {0}")]
    StreamError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Model file not found: {path}")]
    NotFound { path: String },

    #[error("Model file corrupted or invalid")]
    InvalidModel,

    #[error("Insufficient memory to load model (need {required_mb}MB, have {available_mb}MB)")]
    OutOfMemory { required_mb: u64, available_mb: u64 },

    #[error("Model version incompatible")]
    VersionMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("Audio too short for processing (minimum {min_seconds}s)")]
    AudioTooShort { min_seconds: f32 },

    #[error("Inference timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Internal inference error: {0}")]
    Internal(String),
}
```

### FFI Error Representation
```rust
/// C-compatible error code for FFI
#[repr(C)]
pub enum LexiconErrorCode {
    Ok = 0,
    AudioNoDevice = 100,
    AudioPermissionDenied = 101,
    AudioDisconnected = 102,
    ModelNotFound = 200,
    ModelInvalid = 201,
    ModelOom = 202,
    InferenceTooShort = 300,
    InferenceTimeout = 301,
    InferenceInternal = 302,
    IoError = 400,
    ConfigError = 500,
    Unknown = 999,
}

/// C-compatible result with error message
#[repr(C)]
pub struct LexiconResult {
    pub code: LexiconErrorCode,
    pub message: *const c_char,  // Caller must free
}
```

---

## Agent Tasks

### Task 1: Catalog error scenarios ✅
- [x] List all whisper-rs error types — 11 variants documented with recoverability
- [x] List all ort error types — 25+ variants, key ones documented
- [x] List all cpal error types — StreamError, DeviceNameError, etc.
- [x] Document platform-specific errors (iOS, Android) — Permission, interruption, background
- [x] Identify recoverable vs fatal errors — Matrix provided

### Task 2: Design error type hierarchy ✅
- [x] Define error categories — Audio, Model, Inference, I/O, Config
- [x] Define conversion traits — `#[from]` with thiserror
- [x] Define FFI representation — Error codes + UniFFI enums
- [x] Define logging strategy — tracing with structured fields

### Task 3: Implement error types 🔄 (Deferred to Implementation)
- [ ] Create `lexicon-error` crate or module
- [ ] Implement `thiserror` derives
- [ ] Implement `From` conversions
- [ ] Add `tracing` integration

### Task 4: Document FFI patterns ✅
- [x] Document C ABI error handling — catch_unwind + error codes
- [x] Document UniFFI error handling — Result<T, E> → exceptions
- [x] Provide Swift bridging examples — do/catch pattern
- [x] Provide Kotlin bridging examples — try/catch pattern

### Task 5: Create error handling guide ✅
- [x] Best practices for error propagation — Use `?` with thiserror in lib
- [x] When to use `?` vs explicit handling — Match when recovery needed
- [x] Panic policy (never in library code) — Always return Result
- [x] User message guidelines — Three-layer approach

---

## Success Criteria

- [x] Comprehensive error type hierarchy defined — LexiconError with Audio/Model/Inference/IO/Config
- [x] All third-party errors wrapped appropriately — whisper-rs, ort, cpal cataloged with From traits
- [x] FFI error handling documented with examples — UniFFI and C ABI patterns
- [x] Error messages are user-friendly — Three-layer approach (technical/operational/user)
- [x] Logging integration documented — tracing with #[instrument] and structured fields
- [x] Recovery strategies documented per error type — Error Recovery Matrix provided

---

## Error Recovery Matrix

| Error | Recoverable | Recovery Strategy |
|-------|-------------|-------------------|
| No input device | Maybe | Prompt user to connect device |
| Permission denied | Yes | Prompt user to grant permission |
| Device disconnected | Yes | Reconnect or prompt user |
| Model not found | Yes | Trigger download |
| OOM | Maybe | Suggest smaller model |
| Inference timeout | Yes | Retry with shorter audio |
| Disk full | Maybe | Prompt to free space |

---

## Dependencies

- None (foundational research)

---

## References

- thiserror: https://crates.io/crates/thiserror
- anyhow: https://crates.io/crates/anyhow
- Rust Error Handling: https://doc.rust-lang.org/book/ch09-00-error-handling.html
- UniFFI Errors: https://mozilla.github.io/uniffi-rs/udl/errors.html
- tracing: https://crates.io/crates/tracing

---

## Summary & Recommendations

### Error Handling Architecture Summary

| Layer | Crate | Purpose |
|-------|-------|---------|
| **Library Core** | `thiserror` | Structured error enums for caller matching |
| **App/CLI** | `anyhow` | Context chaining for error reporting |
| **FFI (Swift/Kotlin)** | `uniffi` | Automatic exception translation |
| **FFI (C)** | Manual | `catch_unwind` + error codes |
| **Logging** | `tracing` | Structured, async-aware logging |

### Key Recommendations

1. **Use `thiserror` for all library errors** — Enables callers to match on specific variants for recovery
2. **Never panic in library code** — Always return `Result<T, E>`
3. **Use UniFFI for mobile bindings** — Automatic Swift/Kotlin exception handling
4. **Wrap `catch_unwind` around all C FFI exports** — Prevent panics from crossing FFI boundary
5. **Adopt `tracing` with `#[instrument]`** — Structured logs with async context
6. **Three-layer error messages** — Technical (logs), Operational (devs), User-facing (UI)

### Implementation Checklist

| Step | Description | Priority |
|------|-------------|----------|
| 1 | Define `LexiconError` enum with thiserror | High |
| 2 | Implement `From<WhisperError>` for wrapping | High |
| 3 | Implement `From<OrtError>` for wrapping | High |
| 4 | Implement `From<cpal::*Error>` for wrapping | High |
| 5 | Add `#[derive(uniffi::Error)]` for FFI | Medium |
| 6 | Define `LexiconErrorCode` for C ABI | Medium |
| 7 | Integrate tracing with error logging | Medium |
| 8 | Define user-facing message mapping | Low |

### Dependencies to Add

```toml
[dependencies]
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
anyhow = "1"  # For examples/tests

# For FFI
uniffi = "0.29"
```

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | **Research Complete**: Dependency errors cataloged, thiserror/anyhow patterns documented, UniFFI/C ABI FFI patterns documented, tracing integration documented |

---

## Research Sources

- [WhisperError docs](https://docs.rs/whisper-rs/latest/whisper_rs/enum.WhisperError.html)
- [OrtError docs](https://docs.rs/onnxruntime/latest/onnxruntime/error/enum.OrtError.html)
- [cpal StreamError docs](https://docs.rs/cpal/latest/cpal/enum.StreamError.html)
- [thiserror vs anyhow guide](https://momori.dev/posts/rust-error-handling-thiserror-anyhow/)
- [UniFFI Errors](https://mozilla.github.io/uniffi-rs/latest/udl/errors.html)
- [Rust catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- [tracing crate](https://docs.rs/tracing)
- [Structured logging with tracing](https://oneuptime.com/blog/post/2026-01-07-rust-tracing-structured-logs/view)
