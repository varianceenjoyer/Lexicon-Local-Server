# Research Plan: Cross-Platform Build & Distribution

**ID:** 005
**Status:** Ongoing
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

### Mobile Wrapper Comparison
| Approach | Feasibility | Notes |
|----------|-------------|-------|
| _Pending_ | | |

### App Store Requirements
_Pending research_

### CI/CD Architecture
_Pending research_

### Update Strategy
_Pending research_

---

## Blockers Identified

_None yet_

---

## Recommendations

_Pending completion of research tasks_

---

## References

- gomobile: https://pkg.go.dev/golang.org/x/mobile/cmd/gomobile
- Apple App Store Guidelines: https://developer.apple.com/app-store/review/guidelines/
- Google Play Policies: https://play.google.com/about/developer-content-policy/
- GitHub Actions pricing: https://docs.github.com/en/billing/managing-billing-for-github-actions

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
