# Testing Guide

This document describes the testing strategy for Lexicon Local Server.

## Test Structure

```
tests/
├── fixtures/           # Shared test fixtures
│   ├── audio/          # Audio test files
│   ├── models/         # Model files (CI cache)
│   └── manifests/      # Manifest test files
├── e2e_tests.rs        # End-to-end integration tests
└── README.md           # This file

crates/
├── lexicon-core/
│   └── src/
│       ├── storage/tests.rs    # Storage layer unit tests
│       └── models/tests.rs     # Model registry unit tests
├── lexicon-server/
│   └── tests/
│       └── integration_tests.rs # Server API tests
└── lexicon-cli/
    └── tests/
        └── cli_tests.rs        # CLI unit tests
```

## Running Tests

### Run All Tests

```bash
cargo test --workspace
```

### Run Tests for Specific Crate

```bash
# Core library tests
cargo test -p lexicon-core

# Server tests
cargo test -p lexicon-server

# CLI tests
cargo test -p lexicon-cli
```

### Run Tests with Features

```bash
# Run all tests with all features
cargo test --workspace --all-features

# Run only storage tests
cargo test -p lexicon-core --features storage

# Run only STT tests
cargo test -p lexicon-core --features stt
```

### Run Specific Test

```bash
# Run tests matching a pattern
cargo test test_session

# Run tests in a specific module
cargo test storage::tests
```

### Run with Output

```bash
# Show test output
cargo test -- --nocapture

# Show test names as they run
cargo test -- --test-threads=1
```

## Test Categories

### Unit Tests

Located in `src/**/*.rs` files alongside the code they test.

- **Storage tests** (`storage/tests.rs`): Database CRUD, FTS5 search, SRT export
- **Model tests** (`models/tests.rs`): Manifest parsing, tier selection, thermal state

### Integration Tests

Located in `tests/` directories.

- **Server tests**: REST API endpoints, WebSocket handlers, error responses
- **CLI tests**: Argument parsing, output formatting, exit codes
- **E2E tests**: Full workflow tests across components

## Test Fixtures

### Audio Files

Test audio files should be:
- Small enough for repository (<10MB)
- Various formats (WAV, PCM)
- Different durations (5s, 30s, 30min)
- Multiple languages
- Multi-speaker for diarization tests

### Model Files

Model files are cached in CI and not committed:
- `tiny-q5_1.bin` - Smallest model for fast tests
- Downloaded on demand in CI

### Manifest Files

Test manifests for signature verification:
- `valid_signed.json` - Valid manifest with signature
- `invalid_signature.json` - Tampered signature
- `tampered_content.json` - Modified content

## Test Coverage

Target: **>80% code coverage**

Run with coverage:

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html
```

## Writing Tests

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        let result = my_function(42);
        assert_eq!(result, 84);
    }
}
```

### Async Test Example

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_operation().await;
    assert!(result.is_ok());
}
```

### Test with Temp Directory

```rust
use tempfile::TempDir;

#[test]
fn test_with_temp_dir() {
    let temp_dir = TempDir::new().unwrap();
    let db = Database::open(&temp_dir.path().join("test.db")).unwrap();
    // ... test code
    // temp_dir is automatically cleaned up
}
```

## CI Integration

Tests run automatically on:
- Pull requests
- Pushes to main branch

CI workflow:
1. Build all crates
2. Run unit tests
3. Run integration tests
4. Generate coverage report
5. Publish results

## Troubleshooting

### Tests Hang

If tests hang, they may be waiting for audio devices:
```bash
# Run without audio feature
cargo test --no-default-features --features storage,stt,tts
```

### Flaky Tests

If tests fail intermittently:
- Check for race conditions in async code
- Ensure test isolation (use temp directories)
- Add timeouts to async operations

### Memory Issues

Large model tests may require more memory:
```bash
# Increase stack size
RUST_MIN_STACK=8388608 cargo test
```
