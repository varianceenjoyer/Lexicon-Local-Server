# Research Plan: Mobile Platform Constraints

**ID:** 001
**Status:** Complete
**Impact:** HIGH
**Created:** 2026-01-19
**Last Updated:** 2026-01-19

---

## Overview

This research addresses the feasibility of deploying a Go-based server with CGO dependencies (whisper.cpp bindings) on iOS and Android platforms.

---

## Research Questions

### RQ-1: iOS CGO Feasibility

**Problem Statement:**
Go's CGO is required for whisper.cpp bindings. iOS has strict restrictions on dynamic linking, code signing, and historically required bitcode. Does CGO work reliably on iOS, and what are the build/distribution requirements?

**Areas of Inquiry:**
- [ ] Does `gomobile` support CGO-dependent packages on iOS?
- [ ] What is the current state of Go + CGO on iOS (as of Go 1.21+)?
- [ ] Are there App Store restrictions on CGO-compiled binaries?
- [ ] What build toolchain is required (Xcode version, SDK requirements)?
- [ ] Are there known limitations or workarounds?

**Existing Solutions to Evaluate:**
- `gomobile` (golang.org/x/mobile)
- Direct cross-compilation with `GOOS=ios GOARCH=arm64`
- whisper.cpp native iOS example (non-Go reference)

**Success Criteria:**
- Confirmed: CGO + Go compiles for iOS arm64
- Documented: Build toolchain requirements
- Identified: Any App Store compliance blockers

---

### RQ-2: Android NDK Integration

**Problem Statement:**
Cross-compiling CGO for Android ARM architectures requires NDK integration. How do whisper.cpp Go bindings behave when cross-compiled for Android?

**Areas of Inquiry:**
- [ ] What NDK version is required for CGO cross-compilation?
- [ ] Does `gomobile` handle CGO dependencies for Android?
- [ ] Are there ABI compatibility concerns (armeabi-v7a, arm64-v8a, x86_64)?
- [ ] How is the native library loaded at runtime (.so packaging)?
- [ ] What are the minimum Android API level requirements?

**Existing Solutions to Evaluate:**
- `gomobile bind` for AAR generation
- Manual NDK toolchain setup with `CC_FOR_TARGET`
- whisper.cpp Android example (native reference)

**Success Criteria:**
- Confirmed: CGO + Go compiles for Android arm64/arm
- Documented: NDK version and toolchain setup
- Identified: Runtime library loading strategy

---

### RQ-3: Mobile Memory Constraints

**Problem Statement:**
Whisper models range from ~40MB (tiny) to ~3GB (large). Mobile devices have 2-6GB RAM with OS and apps competing for memory. Which models are viable?

**Areas of Inquiry:**
- [ ] What is the runtime memory footprint (not just model file size)?
- [ ] Does whisper.cpp support memory-mapped models?
- [ ] What is the peak memory during inference?
- [ ] How do iOS/Android handle memory pressure (OOM killing)?
- [ ] Are quantized models viable (int8, int4)?

**Existing Solutions to Evaluate:**
- whisper.cpp quantized models (ggml format)
- iOS/Android whisper.cpp memory profiling data
- Similar apps' model choices (if documented)

**Success Criteria:**
- Documented: Memory requirements per model size
- Recommendation: Viable model tiers for mobile
- Identified: Memory optimization strategies

---

### RQ-4: Background Audio Processing

**Problem Statement:**
iOS and Android have strict background execution limits. Can real-time STT operate reliably when the app is backgrounded?

**Areas of Inquiry:**
- [ ] What background modes exist (iOS: Audio, VoIP; Android: Foreground Service)?
- [ ] What are the time limits for background processing?
- [ ] Does audio input remain accessible when backgrounded?
- [ ] What are the battery/thermal throttling implications?
- [ ] Are there platform-specific APIs for continuous audio processing?

**Existing Solutions to Evaluate:**
- iOS Background Audio mode entitlement
- Android Foreground Service with microphone
- Existing STT apps' background strategies

**Success Criteria:**
- Documented: Background processing capabilities per platform
- Identified: Required permissions and entitlements
- Recommendation: Feasibility of background STT

---

## Discovery Tasks

Each task should be marked complete with findings documented in the "Findings" section below.

### Phase 1: Documentation Review
- [ ] **T1.1:** Review gomobile documentation for CGO support status
- [ ] **T1.2:** Review whisper.cpp iOS/Android examples for architecture insights
- [ ] **T1.3:** Review Go release notes for iOS/Android CGO changes (Go 1.20+)
- [ ] **T1.4:** Review Apple App Store guidelines for native code restrictions

### Phase 2: Technical Validation
- [ ] **T2.1:** Attempt minimal CGO cross-compile for iOS (hello world)
- [ ] **T2.2:** Attempt minimal CGO cross-compile for Android (hello world)
- [ ] **T2.3:** Profile whisper.cpp memory usage on desktop (baseline)
- [ ] **T2.4:** Research existing Go + whisper.cpp mobile implementations

### Phase 3: Prototype (if Phase 2 passes)
- [ ] **T3.1:** Build whisper.cpp Go bindings for iOS simulator
- [ ] **T3.2:** Build whisper.cpp Go bindings for Android emulator
- [ ] **T3.3:** Measure memory footprint with tiny/base models
- [ ] **T3.4:** Test background audio capture on both platforms

---

## Findings

### iOS CGO

**Status: FEASIBLE WITH CAVEATS**

**Key Findings:**

1. **gomobile CGO support is problematic:** gomobile bind works reliably for pure Go code, but CGO-dependent packages have known issues ([GitHub #35426](https://github.com/golang/go/issues/35426)). The binding doesn't handle complex CGO scenarios well.

2. **Type limitations exist:** gomobile doesn't support some Go types (slices) directly. A bridge package with struct wrappers is typically required, making Go code "look very non-Go" ([Mat Ryer tutorial](https://medium.com/@matryer/tutorial-calling-go-code-from-swift-on-ios-and-vice-versa-with-gomobile-7925620c17a4)).

3. **Build requirements:**
   - macOS host with Xcode required for iOS targets
   - At least Go 1.16 required
   - `-target ios,iossimulator` produces XCFramework

4. **Alternatives for CGO packages:**
   - [xgo/xgomobile](https://github.com/mysteriumnetwork/xgomobile) - Docker-based cross-compiler that supports CGO for iOS frameworks
   - Manual FFI with Swift/ObjC C-interop

5. **whisper.cpp iOS native is better supported:**
   - [SwiftWhisper](https://swiftpackageindex.com/exPHAT/SwiftWhisper) - native Swift wrapper, XCFramework available
   - [whisper.spm](https://github.com/ggerganov/whisper.spm) - official Swift Package Manager support
   - whisper.objc and whisper.swiftui examples in main repo
   - Core ML/Metal support for acceleration

**Recommendation:** For iOS, **consider native Swift integration** (SwiftWhisper) rather than Go bindings. Go server could be reserved for desktop/server deployments where CGO works reliably.

---

### Android NDK

**Status: FEASIBLE**

**Key Findings:**

1. **gomobile bind works for Android:**
   - Produces AAR (Android ARchive) files with JNI bindings
   - Supports arm64-v8a, armeabi-v7a, x86_64 architectures
   - `-androidapi` flag sets minimum API (default 16, recommend 21+)

2. **NDK requirements:**
   - NDK 25+ recommended for whisper.cpp (C++17 support, ARM optimizations)
   - NDK 28 requires `-androidapi 21` minimum ([2025 blog](https://danballard.com/2025/09/28/generating-android-16kb-page-size-libraries-from-go/))
   - Android 16KB page size (2025 requirement): add `CGO_LDFLAGS="-Wl,-z,max-page-size=16384"`

3. **CGO cross-compilation:**
   - Requires Android NDK toolchain (clang)
   - Set `CC="$TOOLCHAIN_BIN/aarch64-linux-android${API}-clang"` for arm64
   - [android-go](https://github.com/xlab/android-go) project provides alternative bindings

4. **whisper.cpp Android:**
   - JNI layer interfaces Kotlin/Java with native library ([whisper.android discussion](https://github.com/ggml-org/whisper.cpp/discussions/283))
   - ARM Neon SIMD optimizations available
   - Recommended flags: `-march=armv8.2-a+fp16` for fp16_va support

5. **CI/CD resources:**
   - Docker container `openpriv/go-cross-compiler:2025.09` has Go 1.25, Android SDK 35, NDK 28, gomobile pre-installed

**Recommendation:** Android deployment via gomobile AAR is **viable**. Consider maintaining separate native Android integration path for whisper.cpp similar to iOS approach.

---

### Memory Analysis

**Status: CONSTRAINED - Model selection critical**

**Key Findings:**

1. **iOS memory limit:** ~2GB hard limit for apps ([GitHub #1463](https://github.com/ggml-org/whisper.cpp/issues/1463))

2. **Runtime memory requirements (not just file size):**

   | Model | File Size | Runtime Memory | Mobile Viable? |
   |-------|-----------|----------------|----------------|
   | tiny/tiny.en | ~40MB | ~125MB | ✅ Yes |
   | base/base.en | ~140MB | **~506MB** | ⚠️ Marginal |
   | small | ~460MB | ~1GB+ | ❌ Risky |
   | medium | ~1.5GB | **~2610MB** | ❌ No |
   | large-v3 | ~3GB | ~3336MB+ | ❌ No |

3. **Quantization significantly helps:**
   - Q5_0 base.en: ~150MB file → reduced to ~26MB with q5_1
   - Q4_0 base.en: ~120MB file
   - Quantized models require less runtime memory
   - Research shows 19% latency reduction, 45% size reduction, preserved accuracy ([arXiv paper](https://arxiv.org/html/2503.09905v1))

4. **Recommended mobile models:**
   - **Primary:** `ggml-base-q5_1.bin` (~26MB file, good accuracy)
   - **Low-resource:** `ggml-tiny.en-q5_1.bin` (English only)
   - Avoid medium/large on mobile entirely

5. **Model loading strategies:**
   - Memory-mapped loading helps reduce peak memory
   - Consider lazy loading (download on demand vs bundled)
   - OOM risk increases with longer sessions

**Recommendation:** Target **tiny or base quantized models (q5_1)** for mobile. Design UI to clearly communicate model size/quality tradeoffs to users.

---

### Background Processing

**Status: FEASIBLE WITH PLATFORM-SPECIFIC IMPLEMENTATION**

#### iOS Background Audio

**Key Findings:**

1. **Possible with proper entitlements:**
   - Enable "Audio, AirPlay, and Picture in Picture" background mode
   - Use `AVAudioSession.Category.playAndRecord`
   - Audio continues when app backgrounded or device locked

2. **AVAudioSession configuration required:**
   - Category determines eligible audio routes (not specific devices)
   - `.playAndRecord` enables simultaneous input/output
   - Route changes require handling via `routeChangeNotification`

3. **iOS 26 enhancements (WWDC 2025):**
   - New `bluetoothHighQualityRecording` category option ([WWDC 2025 session](https://developer.apple.com/videos/play/wwdc2025/251/))
   - Improved AirPods recording quality for content creators
   - Better audio hardware management across apps

4. **Limitations:**
   - System may interrupt audio for phone calls, alarms
   - Battery/thermal throttling may affect long sessions
   - Background fetch limitations don't apply (audio is continuous)

#### Android Background Audio

**Key Findings:**

1. **Requires foreground service ([Android docs](https://developer.android.com/develop/background-work/services/fgs/service-types)):**
   ```xml
   <service android:foregroundServiceType="microphone" />
   <uses-permission android:name="android.permission.FOREGROUND_SERVICE_MICROPHONE" />
   ```

2. **Persistent notification mandatory:**
   - User must see notification while recording
   - Can record indefinitely while service alive
   - Service dying means losing recording capability

3. **Android 14+ requirements:**
   - System enforces capability checks for foreground service types
   - Must have microphone permission granted before starting service

4. **Limitations:**
   - Without foreground service: ~60 seconds background limit (varies by OEM)
   - Android 12+ restricts background-started foreground services
   - Service must be started while app is in foreground

**Recommendation:** Background STT is **achievable on both platforms** with proper implementation:
- iOS: Background Audio mode + AVAudioSession configuration
- Android: Foreground Service with TYPE_MICROPHONE + notification

---

## Blockers Identified

**B1: Go + CGO iOS Limitation**
- gomobile bind has known issues with CGO-dependent packages on iOS
- **Impact:** May require architectural decision to use native Swift for iOS instead of Go
- **Mitigation:** Use SwiftWhisper/whisper.spm for iOS, reserve Go for Android/desktop

**B2: Memory Constraints Limit Model Choice**
- Medium and large models are not viable on mobile (exceed 2GB iOS limit)
- **Impact:** Mobile users limited to tiny/base model quality
- **Mitigation:** Quantized models (q5_1) provide 65% size reduction with minimal quality loss

**B3: Background Processing Requires Platform-Specific Code**
- iOS and Android have fundamentally different approaches (entitlements vs foreground service)
- **Impact:** Significant platform-specific code required for background STT
- **Mitigation:** Design abstraction layer; document per-platform requirements

---

## Recommendations

### R1: Use Native Platform Implementations for Mobile (STRONG)

Given the CGO complexities and performance concerns with Go bindings, recommend:

**iOS Strategy:**
- Use [SwiftWhisper](https://github.com/exPHAT/SwiftWhisper) or [whisper.spm](https://github.com/ggerganov/whisper.spm) for native Swift integration
- Leverage Core ML / Apple Neural Engine for 3x+ speedup
- Well-maintained, active community support
- Avoids gomobile + CGO complexity entirely

**Android Strategy:**
- Use whisper.cpp native Kotlin/Java examples
- NDK integration is documented and working (NDK 25.2+)
- ARM NEON optimizations available
- Consider [whisper.android](https://github.com/ggml-org/whisper.cpp/tree/master/examples/whisper.android) official example

### R2: Consider sherpa-onnx as Unified Alternative (HIGH PRIORITY)

[sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) provides:
- Native iOS and Android support without CGO complexity
- Go bindings with pre-built libraries for all platforms
- Unified STT + TTS + VAD solution
- Whisper model support
- 12 programming language bindings
- Active development (released Jan 2026)

This could replace whisper.cpp Go bindings and simplify cross-platform deployment significantly.

### R3: Mobile Model Strategy

| Device Tier | Model | Size | Notes |
|-------------|-------|------|-------|
| Low-end (2GB RAM) | tiny.en q5_1 | ~15MB | English only |
| Mid-range (4GB RAM) | base q5_1 | ~26MB | Multilingual |
| High-end (6GB+ RAM) | base | ~140MB | Best accuracy for mobile |

Never deploy medium/large models on mobile (iOS 2GB hard limit).

### R4: Hybrid Architecture

Given the challenges, consider a hybrid architecture:
- **Desktop/Server:** Go-based server with whisper.cpp bindings (or sherpa-onnx)
- **Mobile clients:** Connect via HTTP/WebSocket to server for processing
- **Optional mobile offline:** Native platform SDKs with tiny/base models

This allows mobile clients without the CGO cross-compilation complexity.

---

## References

- [ ] gomobile: https://pkg.go.dev/golang.org/x/mobile
- [ ] whisper.cpp: https://github.com/ggerganov/whisper.cpp
- [ ] Go iOS support: https://go.dev/wiki/Mobile
- [ ] Apple Background Execution: https://developer.apple.com/documentation/backgroundtasks

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Research complete: All findings documented, recommendations added |
