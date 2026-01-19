# Research Plan: Cross-Platform Build & Distribution

**ID:** 005
**Status:** Complete
**Impact:** MEDIUM
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Adapted for:** Rust implementation

---

## Overview

This research addresses the build toolchain, mobile app wrapping, app store distribution, and update mechanisms required to deploy Rust-based libraries on 5 platforms: Windows, macOS, Linux, iOS, and Android.

---

## Key Findings

### Rust Cross-Compilation

**Status: EXCELLENT**

Rust has first-class cross-compilation support via `rustup` target management:

```bash
# Add cross-compilation targets
rustup target add aarch64-apple-ios
rustup target add aarch64-linux-android
rustup target add x86_64-pc-windows-msvc
rustup target add aarch64-apple-darwin
```

| Target | Triple | Toolchain Requirements |
|--------|--------|------------------------|
| iOS (device) | `aarch64-apple-ios` | Xcode (macOS only) |
| iOS (simulator) | `aarch64-apple-ios-sim` | Xcode (macOS only) |
| Android (arm64) | `aarch64-linux-android` | Android NDK |
| Android (arm32) | `armv7-linux-androideabi` | Android NDK |
| Windows | `x86_64-pc-windows-msvc` | MSVC or MinGW |
| macOS (arm64) | `aarch64-apple-darwin` | Xcode |
| macOS (x86) | `x86_64-apple-darwin` | Xcode |
| Linux | `x86_64-unknown-linux-gnu` | GCC |

---

### Mobile Integration Strategy

#### iOS Integration

**Recommended: Static Library + Swift Wrapper**

1. **Build process:**
   ```bash
   cargo build --release --target aarch64-apple-ios
   ```
   - Produces `liblexicon.a` static library
   - Use `cbindgen` to generate C headers
   - Import into Xcode via bridging header or Swift Package

2. **UniFFI for Swift bindings (alternative):**
   - Mozilla's UniFFI generates Swift bindings from Rust
   - `uniffi-rs` crate handles type marshaling
   - More ergonomic than raw C FFI

3. **cargo-mobile2:**
   - Scaffolds iOS/Android projects with Rust library integration
   - Generates Xcode project with proper linking

#### Android Integration

**Recommended: JNI via jni crate + Android NDK**

1. **Build process:**
   ```bash
   cargo ndk -t arm64-v8a -o ./jniLibs build --release
   ```
   - Produces `.so` shared library
   - Use `jni` crate for JNI bindings
   - Or use UniFFI for Kotlin bindings

2. **Android 2025+ requirements:**
   - 16KB page size: add to `.cargo/config.toml`:
     ```toml
     [target.aarch64-linux-android]
     rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384"]
     ```
   - Target API 21+ for NDK 28 compatibility

---

### Rust Mobile Tooling

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo-ndk` | Android NDK builds | Simplifies NDK cross-compilation |
| `cargo-mobile2` | Mobile project scaffolding | Generates iOS/Android projects |
| `cbindgen` | C header generation | For FFI interfaces |
| `uniffi-rs` | Swift/Kotlin binding generation | Higher-level than raw FFI |
| `cross` | Docker-based cross-compilation | For CI/CD pipelines |

---

### App Store Requirements

#### iOS App Store

**Requirements:**
- Apple Developer Program: Required ($99/year)
- Provisioning profiles and certificates: Required
- Microphone entitlement: `NSMicrophoneUsageDescription`
- Background audio: "Audio, AirPlay, and Picture in Picture" capability
- Privacy manifest: Required for third-party AI usage

**ML Model Downloads In-App:** ✅ Permitted
- Only data files (not executable code) may be downloaded
- All code must be contained within the app bundle
- Rust code compiles to native binary (bundled)
- Model files (GGML/ONNX) are data, not executable

#### Google Play Store

**Requirements:**
- Google Play Developer account: $25 one-time
- APK/AAB signing: Required
- Microphone permission: `android.permission.RECORD_AUDIO`
- Foreground service: `android.permission.FOREGROUND_SERVICE_MICROPHONE` (Android 14+)
- 64-bit: Required (arm64-v8a)
- Target API: Must target recent API level

---

### CI/CD Architecture

**GitHub Actions Recommended:**

| Platform | Runner | Build Command |
|----------|--------|---------------|
| Windows | `windows-latest` | `cargo build --release --target x86_64-pc-windows-msvc` |
| macOS (arm64) | `macos-14` | `cargo build --release --target aarch64-apple-darwin` |
| macOS (x86) | `macos-14` | `cargo build --release --target x86_64-apple-darwin` |
| Linux | `ubuntu-latest` | `cargo build --release` |
| iOS | `macos-14` | `cargo build --release --target aarch64-apple-ios` |
| Android | `ubuntu-latest` | `cargo ndk -t arm64-v8a build --release` |

**Build Matrix Example:**
```yaml
jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: macos-14
            target: aarch64-apple-darwin
          - os: macos-14
            target: aarch64-apple-ios
```

**iOS code signing in CI:** Requires manual setup:
- Base64-encode certificate and provisioning profile
- Import into temporary keychain per workflow run

---

### Library Sizes (Estimated)

| Component | Size | Notes |
|-----------|------|-------|
| Rust core library | ~5-10MB | Depends on features |
| whisper.cpp (via whisper-rs) | ~5-15MB | With model loading code |
| ONNX Runtime (via ort) | ~15-30MB | For TTS |
| **Total runtime** | ~25-55MB | Excluding models |

Models are additional:
- tiny.en q5_1: ~15MB
- base q5_1: ~26MB
- Piper voice: 20-100MB

---

### Update Strategy

| Component | Size | Update Method | Notes |
|-----------|------|---------------|-------|
| App binary | ~30-60MB | App store | Requires review cycle |
| Rust library | Bundled | With app | Versioned with app |
| STT models | 15MB-3GB | In-app download | On-demand per quality tier |
| TTS voices | 20-100MB | In-app download | Per-language selection |
| Configuration | <1KB | Remote config | Real-time updates |

**Model Download Strategy:**
1. App ships without models (minimize app store binary size)
2. First launch: user selects quality tier and languages
3. Progressive download in background
4. Models persist in app storage
5. Graceful fallback if download fails

---

## Recommendations

### R1: Use cargo-mobile2 for Mobile Scaffolding ⭐

```bash
cargo install cargo-mobile2
cargo mobile init
cargo apple build --release
cargo android build --release
```

Benefits:
- Generates proper Xcode/Android Studio projects
- Handles linking and signing configuration
- Integrates with native UI frameworks

### R2: UniFFI for High-Level Bindings

Instead of manual FFI:
```rust
// Define interface in UDL
uniffi::include_scaffolding!("lexicon");

// Generates Swift and Kotlin bindings automatically
```

### R3: Tiered Model Strategy

| Tier | Models | Total Size | Use Case |
|------|--------|------------|----------|
| **Lite** | tiny.en q5 + Piper (1 voice) | ~50MB | Quick start, English |
| **Standard** | base q5 + Piper (5 voices) | ~150MB | Multilingual basic |
| **Quality** | small + Piper (10 voices) | ~600MB | Best mobile quality |

### R4: CI/CD Architecture

```
GitHub Actions Workflow:
├── desktop-builds (parallel)
│   ├── windows (windows-latest)
│   ├── linux (ubuntu-latest)
│   └── macos (macos-14, universal binary)
├── mobile-builds (parallel)
│   ├── android (ubuntu-latest + NDK)
│   └── ios (macos-14 + Xcode)
└── release
    └── Upload artifacts to GitHub Releases
```

### R5: Use cross for Linux Variants

For musl, ARM Linux, etc.:
```bash
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
```

---

## References

- cargo-mobile2: https://github.com/niceda/cargo-mobile2
- cargo-ndk: https://github.com/niceda/cargo-ndk
- cbindgen: https://github.com/mozilla/cbindgen
- UniFFI: https://github.com/mozilla/uniffi-rs
- cross: https://github.com/cross-rs/cross
- jni crate: https://crates.io/crates/jni
- Rust Platform Support: https://doc.rust-lang.org/nightly/rustc/platform-support.html
- Apple App Store Guidelines: https://developer.apple.com/app-store/review/guidelines/
- Android NDK: https://developer.android.com/ndk

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial Go-based research |
| 2026-01-19 | Adapted for Rust implementation; replaced gomobile with cargo-mobile2, added Rust cross-compilation details |
