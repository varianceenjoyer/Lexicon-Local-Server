# Research Plan: ort Crate Mobile Builds

**ID:** R-ORT-01
**Status:** Ongoing
**Category:** Feasibility
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Build guide + gotchas documentation

---

## Problem Statement

The `ort` crate (Rust bindings for ONNX Runtime) is required for Piper TTS. While ort works well on desktop platforms, mobile deployment introduces challenges:
- iOS requires static linking and has no dynamic library support
- Android requires NDK cross-compilation
- ONNX Runtime mobile builds are a different artifact than desktop builds
- Execution providers (CoreML, NNAPI) require platform-specific setup

**Engineering Challenge:** Verify ort crate builds successfully for iOS and Android targets and document any configuration requirements.

---

## Research Questions

### RQ1: Does ort compile for iOS targets?

**Areas to investigate:**
- [ ] Build with `cargo build --target aarch64-apple-ios`
- [ ] Build with `cargo build --target aarch64-apple-ios-sim`
- [ ] Static library linking requirements
- [ ] CoreML execution provider availability
- [ ] Metal execution provider availability

**Potential blockers:**
- ort may require pre-built ONNX Runtime libraries
- iOS may need custom build configuration
- Execution providers may need separate setup

### RQ2: Does ort compile for Android targets?

**Areas to investigate:**
- [ ] Build with `cargo ndk -t arm64-v8a build`
- [ ] NDK version requirements
- [ ] Shared library output configuration
- [ ] NNAPI execution provider availability
- [ ] 16KB page size compatibility

### RQ3: What is the binary size impact?

**Areas to investigate:**
- [ ] ort contribution to final binary size
- [ ] Size difference with/without execution providers
- [ ] Stripping and optimization options
- [ ] Comparison to desktop builds

### RQ4: How do execution providers affect mobile builds?

**Areas to investigate:**
- [ ] CoreML setup for iOS
- [ ] NNAPI setup for Android
- [ ] Fallback to CPU when accelerators unavailable
- [ ] Performance comparison: CPU vs accelerated

---

## Known Issues & Prior Art

| Issue | Platform | Notes |
|-------|----------|-------|
| ort mobile builds | Both | Documented as experimental in some versions |
| ONNX Runtime Mobile | Both | Separate build with reduced op set |
| Static linking | iOS | Required; no dylib support |
| NNAPI deprecation | Android | Some devices deprecating NNAPI |

---

## Technical Setup

### Build Environment

**iOS:**
```bash
# Targets
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim

# Build
cargo build --target aarch64-apple-ios --release
```

**Android:**
```bash
# Install cargo-ndk
cargo install cargo-ndk

# Targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi

# Build
cargo ndk -t arm64-v8a -o ./jniLibs build --release
```

### Test Cargo.toml
```toml
[package]
name = "ort-mobile-test"
version = "0.1.0"
edition = "2021"

[dependencies]
ort = { version = "2", features = ["load-dynamic"] }

[target.'cfg(target_os = "ios")'.dependencies]
ort = { version = "2", features = ["coreml"] }

[target.'cfg(target_os = "android")'.dependencies]
ort = { version = "2" }
```

---

## Agent Tasks

### Task 1: Set up cross-compilation environment
- [ ] Install Xcode command line tools
- [ ] Install Android NDK
- [ ] Add Rust cross-compilation targets
- [ ] Create minimal test project

### Task 2: Attempt iOS builds
- [ ] Build for aarch64-apple-ios
- [ ] Build for aarch64-apple-ios-sim (simulator)
- [ ] Document any errors encountered
- [ ] Test CoreML feature flag
- [ ] Measure binary size

### Task 3: Attempt Android builds
- [ ] Build with cargo-ndk for arm64-v8a
- [ ] Build for armv7-linux-androideabi (legacy)
- [ ] Document any errors encountered
- [ ] Test with NDK 25, 26, 27
- [ ] Verify 16KB page size compliance

### Task 4: Test minimal ONNX inference
- [ ] Load a small ONNX model (Piper phonemizer or similar)
- [ ] Run inference on iOS simulator
- [ ] Run inference on Android emulator
- [ ] Verify output correctness

### Task 5: Document findings
- [ ] Step-by-step build guide
- [ ] Known issues and workarounds
- [ ] Feature flag recommendations
- [ ] Binary size report
- [ ] Go/no-go recommendation

---

## Test Cases

### Minimal ort Usage
```rust
use ort::{Environment, Session, Value};

fn test_ort_load() -> Result<(), Box<dyn std::error::Error>> {
    let environment = Environment::builder()
        .with_name("test")
        .build()?;

    let session = Session::builder(&environment)?
        .with_model_from_file("test_model.onnx")?;

    println!("Model loaded successfully");
    println!("Input count: {}", session.inputs.len());
    println!("Output count: {}", session.outputs.len());

    Ok(())
}
```

---

## Success Criteria

- [ ] ort compiles for aarch64-apple-ios without errors
- [ ] ort compiles for aarch64-linux-android without errors
- [ ] Minimal ONNX model loads and runs inference
- [ ] Binary size documented and acceptable (<30MB for ort contribution)
- [ ] Build guide documented with all required steps
- [ ] Known issues catalogued with workarounds

---

## Fallback Options

If ort mobile builds prove infeasible:

1. **Tract**: Pure Rust ONNX runtime (no native dependencies)
   - Pros: Easier cross-compilation
   - Cons: Potentially slower, less op coverage

2. **onnxruntime-mobile**: Use C API directly
   - Pros: Official mobile support
   - Cons: Manual FFI, more complex integration

3. **Pre-built libraries**: Use pre-compiled ONNX Runtime
   - Pros: Known working builds
   - Cons: Dependency management complexity

---

## Dependencies

- R-WRS-01 (similar cross-compilation challenges)
- R-AUD-01 (cpal also needs mobile builds)

---

## References

- ort crate: https://crates.io/crates/ort
- ort documentation: https://ort.pyke.io/
- ONNX Runtime Mobile: https://onnxruntime.ai/docs/build/custom.html
- ONNX Runtime iOS: https://onnxruntime.ai/docs/execution-providers/CoreML-ExecutionProvider.html
- ONNX Runtime Android: https://onnxruntime.ai/docs/execution-providers/NNAPI-ExecutionProvider.html
- tract: https://github.com/sonos/tract

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
