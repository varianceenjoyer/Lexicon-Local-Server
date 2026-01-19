# Research Plan: Cross-Platform Build & Distribution

**ID:** 005
**Status:** Complete
**Impact:** MEDIUM
**Created:** 2026-01-19
**Last Updated:** 2026-01-19

---

## Overview

This research addresses the build toolchain, mobile app wrapping, app store distribution, and update mechanisms required to deploy on 5 platforms: Windows, macOS, Linux, iOS, and Android.

---

## Research Questions

### RQ-1: Mobile App Wrapper Strategy

**Problem Statement:**
Go produces native binaries. What wraps them for iOS (Swift/ObjC) and Android (Kotlin/Java)? Options include gomobile, manual FFI, or embedding as subprocess.

**Wrapper Approaches:**

| Approach | iOS Support | Android Support | Complexity |
|----------|-------------|-----------------|------------|
| gomobile bind | Creates .xcframework | Creates .aar | Low |
| Manual FFI | Swift/ObjC C-interop | JNI/JNA | High |
| Embedded binary | Subprocess | Subprocess | Medium |
| Flutter/Go | Via plugin | Via plugin | Medium |
| WebView wrapper | HTTP to local Go server | HTTP to local Go server | Low |

**Areas of Inquiry:**
- [ ] Does gomobile support CGO-dependent packages?
- [ ] What is the binary size overhead per approach?
- [ ] How do native UI elements interact with Go code?
- [ ] Is a WebView-based approach viable for this use case?
- [ ] What is the debugging experience for each approach?

**Success Criteria:**
- Decision: Wrapper strategy per platform
- Documented: Build process for each approach
- Identified: Limitations of chosen approach

---

### RQ-2: App Store Distribution Requirements

**Problem Statement:**
iOS requires signed IPAs with specific entitlements. Android requires signed APKs/AABs. What are the requirements and restrictions?

**iOS Requirements:**
- [ ] Apple Developer Program membership
- [ ] Provisioning profiles and certificates
- [ ] Required entitlements (microphone, background audio)
- [ ] App Store Review guidelines compliance
- [ ] Binary size limits
- [ ] Privacy manifest requirements (new)

**Android Requirements:**
- [ ] Google Play Developer account
- [ ] APK/AAB signing configuration
- [ ] Required permissions (microphone, foreground service)
- [ ] Play Store policies compliance
- [ ] Target API level requirements
- [ ] 64-bit requirement

**Areas of Inquiry:**
- [ ] Are there restrictions on native code execution?
- [ ] How do app review processes handle ML model downloads?
- [ ] What privacy declarations are needed for audio processing?
- [ ] Is sideloading (non-store distribution) acceptable for any use cases?

**Success Criteria:**
- Documented: Complete distribution requirements per store
- Identified: Policy compliance concerns
- Checklist: Pre-submission requirements

---

### RQ-3: CI/CD Pipeline for 5 Platforms

**Problem Statement:**
Building, testing, and distributing for Windows, macOS, Linux, iOS, and Android requires a complex CI/CD pipeline. What tooling and infrastructure is needed?

**Build Matrix:**

| Platform | Host Requirements | Toolchain | Artifacts |
|----------|-------------------|-----------|-----------|
| Windows | Any (cross-compile) | MinGW/MSVC | .exe, .dll |
| macOS | macOS host | Xcode | .app, .framework |
| Linux | Linux/any | GCC | Binary, .deb, .rpm |
| iOS | macOS host | Xcode, gomobile | .ipa, .xcframework |
| Android | Any | NDK, gomobile | .apk, .aab, .aar |

**Areas of Inquiry:**
- [ ] Which CI platforms support macOS runners? (iOS builds require macOS)
- [ ] What is the build time per platform?
- [ ] How are CGO cross-compilation dependencies managed?
- [ ] Can builds be parallelized?
- [ ] What testing can be automated per platform?

**CI Platform Options:**
- [ ] GitHub Actions (macOS runners available)
- [ ] GitLab CI
- [ ] CircleCI
- [ ] Self-hosted runners

**Success Criteria:**
- Documented: CI/CD architecture
- Estimated: Pipeline costs (macOS runner minutes)
- Defined: Build and release workflow

---

### RQ-4: Update Mechanism

**Problem Statement:**
How do model updates and server updates reach deployed mobile instances? Mobile apps typically go through app store review for updates.

**Update Components:**

| Component | Update Frequency | Distribution Method |
|-----------|------------------|---------------------|
| App binary | Infrequent | App store |
| Whisper models | Rare | In-app download |
| TTS voices | Rare | In-app download |
| Translation models | Rare | In-app download |
| Configuration | Frequent | In-app/remote config |

**Areas of Inquiry:**
- [ ] Can models be downloaded separately from app updates?
- [ ] What is the total download size budget for users?
- [ ] How are model versions managed?
- [ ] Is delta/incremental update viable for large models?
- [ ] What happens if models are unavailable at download time?

**Mobile-Specific Considerations:**
- [ ] iOS: Background download capabilities
- [ ] Android: Download Manager API
- [ ] Storage permissions requirements
- [ ] Network usage on cellular (user consent)

**Success Criteria:**
- Defined: Update strategy per component type
- Documented: Model delivery infrastructure
- Recommendation: Version compatibility approach

---

## Discovery Tasks

### Phase 1: Mobile Wrapper Evaluation
- [ ] **T1.1:** Prototype gomobile bind with minimal CGO package
- [ ] **T1.2:** Evaluate gomobile output for iOS (size, structure)
- [ ] **T1.3:** Evaluate gomobile output for Android (size, structure)
- [ ] **T1.4:** Test WebView + local HTTP server approach

### Phase 2: Distribution Requirements
- [ ] **T2.1:** Review Apple App Store guidelines for relevant restrictions
- [ ] **T2.2:** Review Google Play policies for relevant restrictions
- [ ] **T2.3:** Document required permissions and entitlements
- [ ] **T2.4:** Research privacy manifest requirements

### Phase 3: CI/CD Setup
- [ ] **T3.1:** Create GitHub Actions workflow for Windows/Linux builds
- [ ] **T3.2:** Create GitHub Actions workflow for macOS/iOS builds
- [ ] **T3.3:** Create GitHub Actions workflow for Android builds
- [ ] **T3.4:** Document cross-compilation dependency management

### Phase 4: Update Infrastructure
- [ ] **T4.1:** Design model download and caching system
- [ ] **T4.2:** Research model hosting options (CDN, storage)
- [ ] **T4.3:** Prototype in-app model update flow
- [ ] **T4.4:** Define version compatibility strategy

---

## Findings

### Mobile Wrapper Strategy

**Key Finding:** sherpa-onnx changes the mobile wrapper calculus significantly - it provides native bindings that eliminate CGO cross-compilation complexity.

| Approach | iOS | Android | Feasibility | Notes |
|----------|-----|---------|-------------|-------|
| **sherpa-onnx native** | ✅ Build from source | ✅ Pre-built libs | **Recommended** | Eliminates gomobile/CGO complexity |
| **Flutter + sherpa_onnx** | ✅ Package available | ✅ Package available | Good | Cross-platform UI with native audio |
| gomobile bind | ⚠️ CGO issues | ✅ Works | Risky | CGO-dependent packages problematic on iOS |
| WebView + local server | ✅ | ✅ | Viable | Simpler but less native feel |

**sherpa-onnx Build Requirements:**

| Platform | Build Host | Toolchain | Min Version | Pre-built? |
|----------|-----------|-----------|-------------|------------|
| **iOS** | macOS only | Xcode 14.2+ | iOS 13.0+ | ❌ Build from source |
| **Android** | Linux/macOS | NDK (22.1+ recommended) | API varies | ✅ Available on releases |
| **Windows** | Windows/Linux | MinGW/MSVC | N/A | ✅ Available |
| **macOS** | macOS | Xcode | 10.9+ | ✅ Available |
| **Linux** | Linux | GCC | N/A | ✅ Available |

**Library Sizes (sherpa-onnx Android arm64-v8a):**
- `libonnxruntime.so`: ~15MB
- `libsherpa-onnx-jni.so`: ~3.7MB
- **Total runtime**: ~19MB (excluding models)
- Models are additional (varies by quality)

---

### App Store Requirements

#### iOS App Store

**Requirements Documented:**
- [x] Apple Developer Program: Required ($99/year)
- [x] Provisioning profiles and certificates: Required (manual setup in CI)
- [x] Microphone entitlement: `NSMicrophoneUsageDescription` required
- [x] Background audio: Enable "Audio, AirPlay, and Picture in Picture" capability
- [x] Privacy manifest: Required for third-party AI usage (November 2025 guideline 5.1.2)

**ML Model Downloads In-App:**

Per [Apple Developer Documentation](https://developer.apple.com/documentation/coreml/downloading-and-compiling-a-model-on-the-user-s-device), downloading ML models after app installation is **permitted** with conditions:

1. ✅ Only data files (not executable code) may be downloaded
2. ✅ All code must be contained within the app bundle
3. ✅ Model files used strictly as data, never executed as code
4. ✅ Privacy disclosures must be clear and accurate
5. ✅ Third-party AI usage must be documented in privacy policy

**Apple DTS has confirmed** this approach does not violate guidelines.

#### Google Play Store

**Requirements Documented:**
- [x] Google Play Developer account: Required ($25 one-time)
- [x] APK/AAB signing: Required (keystore management)
- [x] Microphone permission: `android.permission.RECORD_AUDIO`
- [x] Foreground service: `android.permission.FOREGROUND_SERVICE_MICROPHONE` (Android 14+)
- [x] Target API: Must target recent API (rolling requirement)
- [x] 64-bit: Required (arm64-v8a supported by sherpa-onnx)

**Foreground Service Requirements (Android 14+):**

Per [Android Developers documentation](https://developer.android.com/develop/background-work/services/fgs/declare):

1. Declare service with `foregroundServiceType="microphone"` in manifest
2. Request `FOREGROUND_SERVICE_MICROPHONE` permission
3. System verifies `RECORD_AUDIO` permission when service created
4. **Cannot start** microphone foreground service from background
5. Must show persistent notification while service running

```xml
<service
    android:name=".AudioProcessingService"
    android:foregroundServiceType="microphone" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MICROPHONE" />
<uses-permission android:name="android.permission.RECORD_AUDIO" />
```

---

### CI/CD Architecture

**GitHub Actions Recommended** - supports all 5 platforms with native runners.

#### Runner Requirements

| Platform | Runner | Notes |
|----------|--------|-------|
| Windows | `windows-latest` | Cross-compile possible |
| macOS | `macos-14` or `macos-15` | Required for iOS builds |
| Linux | `ubuntu-latest` | Standard |
| iOS | `macos-14` + Xcode | **macOS required** |
| Android | `ubuntu-latest` + NDK | Can use Linux |

**Key Findings:**

1. **Apple Silicon (M1) runners** reduce iOS build times by up to 80% vs Intel ([GitHub Blog](https://github.blog/news-insights/product-news/introducing-the-new-apple-silicon-powered-m1-macos-larger-runner-for-github-actions/))
   - 6-core CPU, 8-core GPU, 14GB RAM
   - Use `macos-14-xlarge` label

2. **macOS 26 image** available in public preview (September 2025)
   - arm64 runners only (no Intel support)
   - Required for latest Xcode/iOS SDK

3. **iOS code signing in CI** requires manual setup:
   - Base64-encode certificate and provisioning profile
   - Import into temporary keychain per workflow run
   - **Automatic signing not supported** in GitHub Actions

4. **Known Issue:** Xcode 26 simulator performance issues reported in community

#### Estimated Build Matrix

| Platform | Build Time | Runner Cost | Notes |
|----------|-----------|-------------|-------|
| Windows | ~5-10 min | Low | Standard runner |
| Linux | ~5-10 min | Low | Standard runner |
| macOS | ~10-15 min | Medium | Xcode compilation |
| iOS | ~15-25 min | Medium-High | Code signing + archive |
| Android | ~10-15 min | Low | NDK compilation |

**Pre-built Library Strategy:**

For sherpa-onnx, consider:
- Download pre-built libraries for Windows, Linux, macOS, Android from [releases](https://github.com/k2-fsa/sherpa-onnx/releases)
- Only build iOS from source (no pre-built available)
- Cache library downloads in CI to reduce build times

---

### Update Strategy

#### Component Update Approach

| Component | Size | Update Method | Notes |
|-----------|------|---------------|-------|
| App binary | ~20-30MB | App store | Requires review cycle |
| sherpa-onnx libs | ~19MB | Bundled with app | Versioned with app |
| STT models | 26MB-3GB | In-app download | On-demand per quality tier |
| TTS voices | 20-100MB | In-app download | Per-language selection |
| Configuration | <1KB | Remote config / in-app | Real-time updates |

#### Model Download Strategy

**Recommended: On-Demand Download with Tiers**

1. **Minimal bundle**: App ships without models (~30MB)
2. **First launch**: User selects quality tier and languages
3. **Progressive download**: Models download in background
4. **Caching**: Models persist in app storage
5. **Fallback**: Graceful degradation if download fails

**iOS Background Downloads:**
- Use `URLSession` with background configuration
- Downloads continue when app backgrounded
- System manages battery/network constraints

**Android Downloads:**
- Use `DownloadManager` API
- Handles network interruptions, resume
- Storage permissions may be required

#### Model Hosting Options

| Option | Pros | Cons |
|--------|------|------|
| **GitHub Releases** | Free, version-aligned | Limited bandwidth |
| **CDN (CloudFlare, etc.)** | Fast, global | Cost scales with usage |
| **Cloud Storage (S3, GCS)** | Reliable, scalable | Requires infrastructure |
| **Hugging Face Hub** | Free, ML-focused | May have limits |

**Recommendation:** Use sherpa-onnx's existing model hosting on GitHub releases for initial deployment; migrate to CDN if bandwidth becomes an issue.

---

## Blockers Identified

**B1: iOS Build Requires macOS**
- No cross-compilation option for iOS
- CI/CD must include macOS runners (cost implications)
- Requires Apple Developer Program membership

**B2: Code Signing Complexity**
- iOS requires certificate management in CI
- Team/organization certificates need secure storage
- Renewal cycles must be tracked

**B3: Model Download Size**
- High-quality models (small+) may deter mobile users
- Need clear UX for download size/quality tradeoffs
- Cellular data usage concerns

---

## Recommendations

### R1: Use sherpa-onnx Native Bindings (STRONG)

Avoid gomobile/CGO complexity by using sherpa-onnx's native platform support:
- **iOS**: Build sherpa-onnx from source, integrate via Swift/ObjC
- **Android**: Use pre-built JNI libraries from releases
- **Desktop**: Use pre-built Go bindings

### R2: Flutter for Cross-Platform UI (CONSIDER)

The [sherpa_onnx Flutter package](https://pub.dev/packages/sherpa_onnx) provides:
- Single codebase for iOS/Android UI
- Native audio processing via sherpa-onnx
- Reduces platform-specific UI code

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
│   ├── windows (ubuntu runner, cross-compile)
│   ├── linux (ubuntu runner)
│   └── macos (macos-14 runner)
├── mobile-builds (parallel)
│   ├── android (ubuntu runner + NDK)
│   └── ios (macos-14 runner + Xcode)
└── release
    └── Upload artifacts to GitHub Releases
```

Use matrix strategy for ABIs:
- Android: arm64-v8a (primary), armeabi-v7a (legacy)
- iOS: arm64 (device), x86_64 (simulator)

---

## References

- [x] sherpa-onnx GitHub: https://github.com/k2-fsa/sherpa-onnx
- [x] sherpa-onnx iOS build docs: https://k2-fsa.github.io/sherpa/onnx/ios/build-sherpa-onnx-swift.html
- [x] sherpa-onnx Android build docs: https://k2-fsa.github.io/sherpa/onnx/android/build-sherpa-onnx.html
- [x] sherpa-onnx Go package: https://pkg.go.dev/github.com/k2-fsa/sherpa-onnx
- [x] sherpa_onnx Flutter: https://pub.dev/packages/sherpa_onnx
- [x] Apple Core ML model downloads: https://developer.apple.com/documentation/coreml/downloading-and-compiling-a-model-on-the-user-s-device
- [x] Apple App Store Guidelines: https://developer.apple.com/app-store/review/guidelines/
- [x] Android Foreground Services: https://developer.android.com/develop/background-work/services/fgs/declare
- [x] Android Microphone Service Type: https://developer.android.com/develop/background-work/services/fgs/service-types
- [x] GitHub Actions M1 runners: https://github.blog/news-insights/product-news/introducing-the-new-apple-silicon-powered-m1-macos-larger-runner-for-github-actions/
- [x] GitHub Actions macOS 26: https://github.blog/changelog/2025-09-11-actions-macos-26-image-now-in-public-preview/
- [x] OpenForge App Store AI Guidelines: https://openforge.io/app-store-review-guidelines-2025-essential-ai-app-rules/

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Research complete: sherpa-onnx distribution strategy, app store requirements, CI/CD architecture documented |
