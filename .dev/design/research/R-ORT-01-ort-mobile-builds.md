# Research Report: ort Crate Mobile Builds

**ID:** R-ORT-01
**Status:** ✅ Complete (Research Phase)
**Category:** Feasibility
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Build guide + gotchas documentation

---

## Executive Summary

| Platform | Feasibility | Recommended Approach | Key Challenge |
|----------|-------------|---------------------|---------------|
| **iOS** | ✅ Feasible | Static linking with `ORT_LIB_LOCATION` | Requires pre-built ONNX Runtime libraries |
| **Android** | ✅ Feasible | cargo-ndk with `system` strategy | NNAPI deprecated in Android 15 |
| **Fallback** | ✅ Available | tract crate (pure Rust) | 85% ONNX op coverage |

### Key Findings

1. **ort supports mobile via `system` linking strategy** - Pre-built ONNX Runtime libraries required
2. **iOS requires static linking** - Use `ORT_LIB_LOCATION` + `ORT_LIB_PROFILE=Release`
3. **Android NNAPI is deprecated** - Use XNNPACK or QNN for hardware acceleration instead
4. **Binary size can be reduced** - Custom minimal builds ~7.5MB full → model-specific smaller
5. **tract is a viable pure-Rust fallback** - MIT/Apache-2.0 licensed, no native deps

### Recommendation: CONDITIONAL GO ✅

ort mobile builds are feasible but require:
1. Pre-building ONNX Runtime for each target platform
2. CI/CD pipeline for cross-platform library management
3. Testing on actual devices (not just emulators)

**Alternative:** Consider tract for simpler deployment if ONNX op coverage is sufficient.

---

## Problem Statement

The `ort` crate (Rust bindings for ONNX Runtime) is required for TTS inference. While ort works well on desktop platforms, mobile deployment introduces challenges:

> **Note:** Piper TTS was originally planned but has been replaced by **Kokoro TTS** (Apache 2.0) to avoid GPL licensing issues with espeak-ng. See [R-TTS-01](R-TTS-01-alternative-tts-engines.md) for details. Kokoro also uses ONNX, so this research remains applicable.
- iOS requires static linking and has no dynamic library support
- Android requires NDK cross-compilation
- ONNX Runtime mobile builds are a different artifact than desktop builds
- Execution providers (CoreML, NNAPI) require platform-specific setup

**Engineering Challenge:** Verify ort crate builds successfully for iOS and Android targets and document any configuration requirements.

---

## Research Questions

### RQ1: Does ort compile for iOS targets? ✅ RESEARCHED

**Findings:**
- [x] iOS requires **static linking** - dynamic libraries not supported
- [x] Use `system` strategy with `ORT_LIB_LOCATION` pointing to pre-built libraries
- [x] Set `ORT_LIB_PROFILE=Release` for iOS builds
- [x] CoreML execution provider available via `--use_coreml` build flag
- [x] Pre-built ONNX Runtime with CoreML EP available in CocoaPod `onnxruntime-c`

**Build Requirements:**
```bash
# Environment variables for iOS
ORT_LIB_LOCATION=~/onnxruntime/build/iOS/Release
ORT_LIB_PROFILE=Release
cargo build --target aarch64-apple-ios --release
```

**CoreML Requirements:**
- iOS 13+ for NeuralNetwork format
- iOS 15+ for MLProgram format (recommended)
- Apple Neural Engine recommended for optimal performance

### RQ2: Does ort compile for Android targets? ✅ RESEARCHED

**Findings:**
- [x] Use `cargo-ndk` (v4.1.2) for Android builds
- [x] NDK auto-detected from Android Studio default location
- [x] Set `ANDROID_NDK_HOME` to override NDK path
- [x] ⚠️ **NNAPI deprecated in Android 15** - use XNNPACK or QNN instead
- [x] Use `--link-builtins` and `--link-libcxx-shared` flags if linking issues

**Build Requirements:**
```bash
# Install targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build with cargo-ndk
cargo ndk -t arm64-v8a -o ./jniLibs build --release
```

**Android Execution Provider Options:**
| Provider | Status | Notes |
|----------|--------|-------|
| NNAPI | ⚠️ Deprecated | Android 15+ deprecates underlying API |
| XNNPACK | ✅ Recommended | CPU-based, cross-platform |
| QNN | ✅ Available | Qualcomm Snapdragon devices only |

### RQ3: What is the binary size impact? ✅ RESEARCHED

**Findings:**
- [x] Full ONNX Runtime build: ~7.5 MB
- [x] Minimal/custom build: Significantly smaller (model-specific)
- [x] Use `MinSizeRel` configuration for smallest binary
- [x] Use `--minimal_build` flag to strip unused operators

**Size Reduction Strategies:**
1. **Minimal build**: Only include operators used by your model(s)
2. **ORT format**: Convert ONNX → ORT format (smaller, optimized)
3. **Type reduction**: `--enable_type_reduction` + `--enable_reduced_operator_type_support`
4. **Strip symbols**: Automatic with `MinSizeRel`

**Build Command for Minimal Size:**
```bash
--config=MinSizeRel --minimal_build --disable_ml_ops --disable_exceptions \
--include_ops_by_config <config_file> --skip_tests
```

### RQ4: How do execution providers affect mobile builds? ✅ RESEARCHED

**Findings:**
- [x] EPs must be enabled at ONNX Runtime build time AND in ort cargo features
- [x] Missing EP silently falls back to CPU (no error)
- [x] CoreML requires Apple Neural Engine for best performance

**iOS - CoreML:**
```toml
# Cargo.toml
[target.'cfg(target_os = "ios")'.dependencies]
ort = { version = "2", features = ["coreml"] }
```
- Requires ONNX Runtime built with `--use_coreml`
- Pre-built available in `onnxruntime-c` CocoaPod

**Android - XNNPACK (Recommended over deprecated NNAPI):**
```toml
# Cargo.toml
[target.'cfg(target_os = "android")'.dependencies]
ort = { version = "2", features = ["xnnpack"] }
```

**Fallback Behavior:**
- ort gracefully falls back to CPU if requested EP unavailable
- No runtime errors; performance degradation only

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

### Option 1: tract (Recommended Fallback) ✅

**tract** is a pure-Rust neural network inference toolkit.

| Aspect | Details |
|--------|---------|
| **License** | MIT OR Apache-2.0 (permissive, no GPL concerns) |
| **ONNX Coverage** | ~85% of ONNX backend tests pass |
| **Mobile Support** | Excellent - runs on Raspberry Pi Zero, ARM Cortex-M |
| **Dependencies** | Pure Rust, no native libraries |
| **Rust Version** | ≥1.75.0 |

**Pros:**
- No cross-compilation complexity (pure Rust)
- No pre-built library management
- Streaming support via "pulsing" technology
- Proven in production (Snips wake word detection)

**Cons:**
- ~85% ONNX op coverage (vs ~100% for ONNX Runtime)
- No hardware acceleration (CPU only)
- Some operators incomplete (e.g., Resize)

**Usage:**
```toml
[dependencies]
tract-onnx = "0.21"  # Check for latest version
```

```rust
use tract_onnx::prelude::*;

let model = tract_onnx::onnx()
    .model_for_path("model.onnx")?
    .into_optimized()?
    .into_runnable()?;
```

### Option 2: onnxruntime-mobile (C API)

- **Pros:** Official Microsoft mobile support, full op coverage
- **Cons:** Manual FFI bindings, more complex integration
- **When to use:** If ort crate has blocking issues

### Option 3: Pre-built ONNX Runtime Libraries

- **Pros:** Known working builds, official support
- **Cons:** Dependency management, CI/CD complexity
- **Sources:**
  - iOS: `onnxruntime-c` CocoaPod
  - Android: Maven `com.microsoft.onnxruntime:onnxruntime-mobile`

### Recommendation

1. **First choice:** ort with `system` strategy + pre-built libraries
2. **Fallback:** tract if ONNX ops are sufficient for Piper models
3. **Last resort:** Direct C API FFI bindings

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
| 2026-01-19 | Research complete (SilverCrane agent) |
| 2026-01-19 | Documented ort linking strategies for iOS/Android |
| 2026-01-19 | Identified NNAPI deprecation in Android 15 |
| 2026-01-19 | Added tract as recommended pure-Rust fallback |
| 2026-01-19 | Documented binary size reduction strategies |

## Research Sources

- [ort crate documentation](https://ort.pyke.io/)
- [ort linking guide](https://ort.pyke.io/setup/linking)
- [ONNX Runtime Mobile](https://onnxruntime.ai/docs/tutorials/mobile/)
- [ONNX Runtime CoreML EP](https://onnxruntime.ai/docs/execution-providers/CoreML-ExecutionProvider.html)
- [ONNX Runtime NNAPI EP](https://onnxruntime.ai/docs/execution-providers/NNAPI-ExecutionProvider.html)
- [ONNX Runtime Custom Build](https://onnxruntime.ai/docs/build/custom.html)
- [cargo-ndk](https://crates.io/crates/cargo-ndk)
- [tract-onnx](https://lib.rs/crates/tract-onnx)
- [ort-builder (static lib builder)](https://github.com/olilarkin/ort-builder)
