# WP-IOS: iOS Platform Integration

> **Status**: Blocked (Release Phase) | **Priority**: HIGH | **Phase**: Release
>
> **Research**: [R-THR-01](../design/research/R-THR-01-thermal-profiling.md), [R-ORT-01](../design/research/R-ORT-01-ort-mobile-builds.md)
>
> **Created**: 2026-01-23 | **Last Updated**: 2026-01-23

---

## Overview

Create iOS static library and Swift package for embedding Lexicon in iOS apps. This enables on-device speech-to-text, translation, and TTS without cloud dependencies.

**Target**: iOS 13+ devices (iPhone 8 and newer)

---

## Scope

### Included
- Static library build for `aarch64-apple-ios` (device)
- Static library build for `aarch64-apple-ios-sim` (simulator)
- Universal XCFramework creation
- Swift package wrapper (LexiconSwift)
- UniFFI Swift binding generation
- Background audio configuration
- Microphone permission handling
- CoreML/ANE acceleration
- Thermal monitoring and model switching
- Example iOS app

### Excluded
- App Store publication (user responsibility)
- iOS-specific UI components
- Siri/Shortcuts integration
- Widget support

---

## Interface Contract

### Swift Package API

```swift
// Sources/LexiconSwift/LexiconContext.swift

import Foundation

/// Main entry point for Lexicon functionality
public class LexiconContext {
    /// Initialize with configuration
    public init(config: LexiconConfig) throws

    /// Load a transcription model
    public func loadModel(modelId: String) async throws

    /// Start a transcription session
    public func startSession() throws -> TranscriptionSession

    /// Get available models for this device tier
    public func availableModels() -> [ModelInfo]

    /// Get current thermal state
    public var thermalState: ThermalState { get }

    /// Callback for thermal state changes
    public var onThermalStateChange: ((ThermalState) -> Void)?
}

/// Transcription session
public class TranscriptionSession {
    /// Process audio samples (16kHz mono float)
    public func processAudio(_ samples: [Float]) throws -> [Segment]

    /// Finish session and get final results
    public func finish() throws -> [Segment]

    /// Export to SRT format
    public func exportSRT() -> String
}

/// Configuration
public struct LexiconConfig {
    /// Model storage directory
    public var modelDir: URL

    /// Database storage directory
    public var dataDir: URL

    /// Enable speaker diarization
    public var diarization: Bool

    /// Maximum speakers for diarization
    public var maxSpeakers: Int

    /// Preferred Whisper model
    public var preferredModel: String?
}

/// Thermal state from iOS ProcessInfo
public enum ThermalState: Int {
    case nominal = 0
    case fair = 1
    case serious = 2
    case critical = 3
}

/// Transcription segment
public struct Segment {
    public let startMs: Int64
    public let endMs: Int64
    public let text: String
    public let confidence: Float?
    public let speakerId: UInt32?
}
```

### Native Swift Helpers

```swift
// Sources/LexiconSwift/Platform/ThermalMonitor.swift

import Foundation

/// Monitor iOS thermal state and notify Rust core
public class ThermalMonitor {
    private var observer: NSObjectProtocol?

    public init() {
        observer = NotificationCenter.default.addObserver(
            forName: ProcessInfo.thermalStateDidChangeNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handleThermalChange()
        }
    }

    public var currentState: ThermalState {
        ThermalState(rawValue: ProcessInfo.processInfo.thermalState.rawValue) ?? .nominal
    }

    public var thermalHeadroom: Float {
        // iOS doesn't expose headroom directly, estimate from state
        switch currentState {
        case .nominal: return 1.0
        case .fair: return 0.7
        case .serious: return 0.4
        case .critical: return 0.1
        }
    }

    private func handleThermalChange() {
        // Notify Rust core to potentially switch models
    }
}
```

### System Profiler (Swift)

```swift
// Sources/LexiconSwift/Platform/SystemProfiler.swift

import Foundation
import Metal

/// System capability detection for iOS
public struct SystemProfiler {
    public static func detect() -> SystemProfile {
        let device = MTLCreateSystemDefaultDevice()

        return SystemProfile(
            ramGB: Float(ProcessInfo.processInfo.physicalMemory) / 1_073_741_824,
            cpuCores: ProcessInfo.processInfo.processorCount,
            gpuName: device?.name,
            gpuMemoryGB: device.map { Float($0.recommendedMaxWorkingSetSize) / 1_073_741_824 },
            platform: "ios",
            iosVersion: ProcessInfo.processInfo.operatingSystemVersionString
        )
    }

    public static func recommendedModel() -> String {
        let profile = detect()

        // iOS devices typically use tiny or base models
        if profile.ramGB < 3.0 {
            return "whisper-tiny.en"
        } else if profile.ramGB < 4.0 {
            return "whisper-base.en"
        } else {
            return "whisper-base.en" // Limit for thermal reasons
        }
    }
}
```

---

## Acceptance Criteria

### Build
- [ ] Static library compiles for `aarch64-apple-ios`
- [ ] Static library compiles for `aarch64-apple-ios-sim`
- [ ] XCFramework bundles both architectures
- [ ] Swift package compiles in Xcode 15+
- [ ] No compiler warnings in release build

### FFI Integration
- [ ] UniFFI generates Swift bindings
- [ ] All public APIs accessible from Swift
- [ ] Errors map to Swift exceptions
- [ ] Async operations work correctly
- [ ] No memory leaks (Instruments)

### Functionality
- [ ] Load Whisper model on device
- [ ] Transcribe audio file successfully
- [ ] Real-time transcription from microphone
- [ ] TTS synthesis works
- [ ] Database persists sessions

### Platform Integration
- [ ] Microphone permission request works
- [ ] Background audio continues transcription
- [ ] CoreML/ANE acceleration active
- [ ] Thermal state monitoring works
- [ ] Model switches at serious thermal state

### Performance
- [ ] Memory usage <1GB peak
- [ ] Transcription latency <800ms (base model)
- [ ] App launch time <2s with model loaded
- [ ] No UI freezes during transcription

### Example App
- [ ] Builds and runs on device
- [ ] Demonstrates all major features
- [ ] Includes usage documentation

---

## Dependencies

### Requires
- WP-FFI (UniFFI bindings)
- Xcode 15+ with iOS SDK
- macOS development machine
- iOS device for testing (simulator has limitations)

### Blocks
- TestFlight distribution
- Production iOS apps

---

## Files

### New Files
- `ios/Lexicon.xcframework/` - Universal framework
- `ios/LexiconSwift/Package.swift` - Swift package manifest
- `ios/LexiconSwift/Sources/LexiconSwift/` - Swift wrapper code
- `ios/LexiconExample/` - Example iOS app
- `scripts/build-ios.sh` - Build script

### Modified Files
- `crates/lexicon-ffi/Cargo.toml` - iOS target configuration
- `.github/workflows/release.yml` - iOS build job

---

## Build Process

### Prerequisites

```bash
# Install iOS targets
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

# Install cargo-lipo for universal binaries
cargo install cargo-lipo
```

### Build Commands

```bash
# Build for device
cargo build --release --target aarch64-apple-ios -p lexicon-ffi

# Build for simulator
cargo build --release --target aarch64-apple-ios-sim -p lexicon-ffi

# Create XCFramework
xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/liblexicon_ffi.a \
    -headers include/ \
    -library target/aarch64-apple-ios-sim/release/liblexicon_ffi.a \
    -headers include/ \
    -output Lexicon.xcframework
```

### UniFFI Swift Generation

```bash
# Generate Swift bindings
cargo run --bin uniffi-bindgen generate \
    --library target/release/liblexicon_ffi.dylib \
    --language swift \
    --out-dir ios/LexiconSwift/Sources/LexiconSwift/Generated
```

---

## Test Plan

### Unit Tests (Swift)

| Test | Input | Expected Output |
|------|-------|-----------------|
| `testContextCreation` | Valid config | Context instance |
| `testModelLoading` | Model ID | No error |
| `testTranscription` | Audio samples | Segments array |
| `testThermalState` | Device state | Correct enum value |
| `testSystemProfile` | Device info | Valid profile |

### Integration Tests

| Test | Scenario | Pass Criteria |
|------|----------|---------------|
| `testFileTranscription` | Transcribe WAV file | Valid SRT output |
| `testMicrophoneCapture` | Record and transcribe | Real-time segments |
| `testBackgroundAudio` | App backgrounded | Transcription continues |
| `testThermalSwitch` | Simulate thermal | Model downgrade |

### Device Testing Matrix

| Device | iOS Version | Model | Expected Performance |
|--------|-------------|-------|----------------------|
| iPhone 12 | iOS 16+ | base | <800ms latency |
| iPhone 11 | iOS 15+ | tiny | <500ms latency |
| iPhone SE 2 | iOS 14+ | tiny | <600ms latency |
| iPad Air 4 | iOS 15+ | base | <600ms latency |

---

## Implementation Notes

### Background Audio

Add to Info.plist:
```xml
<key>UIBackgroundModes</key>
<array>
    <string>audio</string>
</array>
<key>NSMicrophoneUsageDescription</key>
<string>Lexicon needs microphone access for speech transcription.</string>
```

### ANE Acceleration

whisper.cpp with CoreML support enables ANE (Apple Neural Engine):
- 85% power reduction vs CPU
- 10x faster inference
- Requires CoreML model format or automatic conversion

### Memory Management

iOS has strict memory limits:
- Keep model loaded, unload on memory warning
- Process audio in chunks (5-10 seconds)
- Release intermediate buffers promptly

---

## Research References

- **R-ORT-01**: ort crate mobile builds
- **R-THR-01**: Thermal profiling and management
- **R-AUD-01**: cpal audio capture on iOS

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work package created for Release phase |

---

*Blocked until Release phase. Requires macOS with Xcode for development.*
