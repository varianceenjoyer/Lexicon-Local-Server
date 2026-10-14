# Security Policy

This document describes the security measures and threat model for Lexicon Local Server.

## Project Status

This project is published as-is and is not maintained. There are no supported versions, and security issues will not be triaged or fixed. Use it at your own risk; see [LICENSE](LICENSE).

If you find a vulnerability, you're welcome to open an issue so other users are aware of it, but no response or fix should be expected.

---

## Security Measures

### Code Safety

#### Memory Safety
- **Rust Language**: Core library written in Rust, providing memory safety guarantees
- **Unsafe Code Policy**: `unsafe` code is denied at the workspace level (`#![deny(unsafe_code)]`)
- **FFI Exception**: Only the `lexicon-ffi` crate allows unsafe code for UniFFI bindings
- **No Manual Memory Management**: All memory managed through Rust's ownership system

#### Input Validation
- **Audio Data**: Sample rates validated, buffer sizes bounded
- **API Inputs**: JSON schema validation on REST endpoints
- **File Paths**: Path traversal prevention in model loading
- **SQL Queries**: All database queries use parameterized statements (rusqlite)

### Model Security

#### Signature Verification
- Model manifests are cryptographically signed (minisign)
- SHA-256 checksums verified during download
- Signature verification before any model parsing
- Tampered models are rejected

#### Download Security
- HTTPS-only model downloads (rustls TLS)
- Streaming verification during download
- Partial downloads cleaned up on verification failure

### Network Security

#### Server Security
- Rate limiting prevents abuse (per-IP, per-endpoint)
- CORS configuration (permissive by default, configurable)
- Health check endpoint for monitoring
- Graceful shutdown preserves data integrity

#### WebSocket Security
- No authentication by default (local use case)
- Optional API key authentication available
- Connection limits enforced

### Data Security

#### Local Storage
- SQLite database with WAL mode
- No sensitive data stored in plaintext
- Session data stored locally only
- No telemetry or data collection

#### Privacy
- All processing happens locally
- No network calls except for model downloads
- Audio data never leaves the device
- Transcriptions stored only in local database

---

## Security Audit Checklist

This checklist is used for security reviews and audits.

### Code Review

- [x] All Rust code compiles with `#![deny(unsafe_code)]` (except FFI)
- [x] FFI boundary uses UniFFI (auto-generated safe bindings)
- [x] No raw pointer manipulation in application code
- [x] No `unwrap()` on user-controlled input
- [x] Error handling uses `Result` types throughout

### Input Validation

- [x] Audio sample rate validated (8kHz-48kHz accepted)
- [x] Audio buffer size bounded (max 10 minutes per request)
- [x] JSON request bodies validated
- [x] File paths sanitized (no `..` traversal)
- [x] Model IDs validated against allowlist

### Database Security

- [x] Parameterized queries used exclusively
- [x] No string interpolation in SQL
- [x] WAL mode prevents corruption
- [x] Database file permissions restricted

### Cryptographic Security

- [x] Model signatures use minisign (Ed25519)
- [x] File hashes use SHA-256
- [x] TLS 1.2+ for HTTPS (rustls)
- [x] No custom cryptography implementations

### FFI Security

- [x] UniFFI generates safe bindings
- [x] Panic boundaries at FFI layer
- [x] No raw pointer exposure to foreign code
- [x] Memory ownership clearly defined

### Network Security

- [x] Rate limiting implemented
- [x] Request size limits enforced
- [x] Timeout handling prevents DoS
- [x] No sensitive data in error messages

---

## Threat Model

### Assets

1. **User Audio Data**: Microphone input, recorded audio files
2. **Transcription Data**: Text output from speech recognition
3. **Model Files**: Downloaded ML models
4. **Session Data**: Stored transcription sessions

### Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    Trusted (Local Device)                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Audio Input │  │   Lexicon   │  │   Local Storage     │  │
│  │ (Microphone)│──│   Library   │──│ (SQLite, Models)    │  │
│  └─────────────┘  └──────┬──────┘  └─────────────────────┘  │
│                          │                                   │
└──────────────────────────┼───────────────────────────────────┘
                           │ HTTPS (model downloads only)
┌──────────────────────────┼───────────────────────────────────┐
│                    Untrusted (Network)                       │
│                          │                                   │
│              ┌───────────▼───────────┐                       │
│              │   Model Repository    │                       │
│              │ (Signed, Verified)    │                       │
│              └───────────────────────┘                       │
└──────────────────────────────────────────────────────────────┘
```

### Threat Categories

| Threat | Mitigation | Status |
|--------|------------|--------|
| Malicious model injection | Signature verification | Implemented |
| Audio data exfiltration | Local-only processing | By design |
| SQL injection | Parameterized queries | Implemented |
| Path traversal | Path sanitization | Implemented |
| DoS via large requests | Rate limiting, size limits | Implemented |
| Memory corruption | Rust memory safety | By design |
| FFI boundary exploits | UniFFI safe bindings | Implemented |

### Out of Scope

The following are considered out of scope for this library:

- Physical device security
- Operating system vulnerabilities
- Malware on the host system
- Social engineering attacks
- Side-channel attacks on ML inference

---

## Security-Related Configuration

### Rate Limiting (Server)

```rust
use lexicon_server::{ServerConfig, RateLimitConfig};

// Strict rate limiting for production
let config = ServerConfig::default()
    .with_rate_limit(RateLimitConfig::strict());

// Custom limits
let config = ServerConfig::default()
    .with_rate_limit(
        RateLimitConfig::default()
            .with_default_rpm(60)      // 60 requests/minute general
            .with_heavy_rpm(10)        // 10 requests/minute for transcribe
            .with_websocket_rpm(5)     // 5 WebSocket connections/minute
    );
```

### Model Verification

Model verification is automatic and cannot be disabled. All downloaded models must:

1. Have a valid manifest signature
2. Pass SHA-256 checksum verification
3. Come from the configured model repository

---

*Last updated: October 2026*
