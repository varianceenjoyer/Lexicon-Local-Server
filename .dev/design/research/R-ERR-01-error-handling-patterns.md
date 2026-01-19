# Research Plan: Rust Error Handling Patterns

**ID:** R-ERR-01
**Status:** Ongoing
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

### RQ1: What error categories exist in the pipeline?

**Areas to investigate:**
- [ ] Audio subsystem errors (cpal, platform APIs)
- [ ] Model loading errors (whisper-rs, ort)
- [ ] Inference errors (whisper-rs, ort)
- [ ] I/O errors (file system, network)
- [ ] Resource errors (OOM, disk full)
- [ ] Platform errors (permissions, background limits)

### RQ2: What Rust error handling patterns apply?

**Areas to investigate:**
- [ ] `thiserror` for custom error types
- [ ] `anyhow` for error context and chaining
- [ ] `Result<T, E>` propagation patterns
- [ ] Error conversion traits (`From`, `Into`)
- [ ] Logging and error reporting (`tracing`)

### RQ3: How do errors cross FFI boundaries?

**Areas to investigate:**
- [ ] C ABI error representation (error codes vs structs)
- [ ] UniFFI error handling
- [ ] Swift error bridging
- [ ] Kotlin/JNI exception handling
- [ ] Panic safety across FFI

### RQ4: How should errors be communicated to users?

**Areas to investigate:**
- [ ] User-facing vs developer-facing errors
- [ ] Error message localization
- [ ] Recovery suggestions
- [ ] Logging vs display separation

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

### Task 1: Catalog error scenarios
- [ ] List all whisper-rs error types
- [ ] List all ort error types
- [ ] List all cpal error types
- [ ] Document platform-specific errors (iOS, Android)
- [ ] Identify recoverable vs fatal errors

### Task 2: Design error type hierarchy
- [ ] Define error categories
- [ ] Define conversion traits
- [ ] Define FFI representation
- [ ] Define logging strategy

### Task 3: Implement error types
- [ ] Create `lexicon-error` crate or module
- [ ] Implement `thiserror` derives
- [ ] Implement `From` conversions
- [ ] Add `tracing` integration

### Task 4: Document FFI patterns
- [ ] Document C ABI error handling
- [ ] Document UniFFI error handling
- [ ] Provide Swift bridging examples
- [ ] Provide Kotlin bridging examples

### Task 5: Create error handling guide
- [ ] Best practices for error propagation
- [ ] When to use `?` vs explicit handling
- [ ] Panic policy (never in library code)
- [ ] User message guidelines

---

## Success Criteria

- [ ] Comprehensive error type hierarchy defined
- [ ] All third-party errors wrapped appropriately
- [ ] FFI error handling documented with examples
- [ ] Error messages are user-friendly
- [ ] Logging integration documented
- [ ] Recovery strategies documented per error type

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

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
