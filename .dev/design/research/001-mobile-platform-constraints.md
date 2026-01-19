# Research Plan: Mobile Platform Constraints

**ID:** 001
**Status:** Ongoing
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
_Pending research_

### Android NDK
_Pending research_

### Memory Analysis
_Pending research_

### Background Processing
_Pending research_

---

## Blockers Identified

_None yet_

---

## Recommendations

_Pending completion of research tasks_

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
