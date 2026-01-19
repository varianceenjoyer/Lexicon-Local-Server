# Research Plan: cpal Audio Capture on Mobile Platforms

**ID:** R-AUD-01
**Status:** Ongoing
**Category:** Feasibility
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Integration patterns documentation

---

## Problem Statement

The `cpal` crate is the standard Rust library for cross-platform audio I/O. While it has broad platform support, mobile audio has unique challenges:
- iOS requires AVAudioSession configuration for microphone access
- Android requires AudioRecord/Oboe integration
- Both platforms have background audio restrictions
- Bluetooth audio adds latency and connection complexity

**Engineering Challenge:** Determine if cpal provides sufficient mobile audio support or if platform-native APIs must be used directly.

---

## Research Questions

### RQ1: Does cpal support iOS audio capture?

**Areas to investigate:**
- [ ] Microphone input support on iOS
- [ ] AVAudioSession category configuration
- [ ] Background audio mode compatibility
- [ ] Sample rate and buffer size control
- [ ] Route change handling (headphones, Bluetooth)

**Known constraints:**
- iOS requires specific Info.plist entitlements
- AVAudioSession must be configured before audio starts
- Route changes can interrupt audio sessions

### RQ2: Does cpal support Android audio capture?

**Areas to investigate:**
- [ ] Microphone input support on Android
- [ ] AudioRecord vs Oboe backend
- [ ] Foreground service requirements
- [ ] Permission handling integration
- [ ] Sample rate negotiation

**Known constraints:**
- Android 14+ requires FOREGROUND_SERVICE_MICROPHONE
- Background recording requires visible notification
- Audio focus management for multi-app scenarios

### RQ3: What sample rates and formats are available?

**Areas to investigate:**
- [ ] Supported sample rates (16kHz for Whisper, 22.05kHz for Piper)
- [ ] Bit depth options (16-bit PCM for Whisper)
- [ ] Channel configurations (mono for STT)
- [ ] Buffer size control for latency

### RQ4: How does cpal handle audio interruptions?

**Areas to investigate:**
- [ ] Phone call interruption handling
- [ ] Other app audio takeover
- [ ] Bluetooth disconnection
- [ ] Session resumption patterns

---

## Known Solutions & Prior Art

| Platform | Native API | cpal Backend | Notes |
|----------|-----------|--------------|-------|
| iOS | AVAudioEngine | coreaudio-rs | May need AVAudioSession wrapper |
| Android | AudioRecord/Oboe | oboe-rs | Oboe recommended for low latency |
| macOS | Core Audio | coreaudio-rs | Well supported |
| Windows | WASAPI | cpal native | Well supported |
| Linux | ALSA/PulseAudio | cpal native | Well supported |

---

## Technical Setup

### Test Cargo.toml
```toml
[package]
name = "cpal-mobile-test"
version = "0.1.0"
edition = "2021"

[dependencies]
cpal = "0.15"

[target.'cfg(target_os = "ios")'.dependencies]
# May need additional iOS audio bindings

[target.'cfg(target_os = "android")'.dependencies]
# May need oboe-rs directly
```

### iOS Requirements
```xml
<!-- Info.plist -->
<key>NSMicrophoneUsageDescription</key>
<string>Used for speech-to-text transcription</string>
<key>UIBackgroundModes</key>
<array>
    <string>audio</string>
</array>
```

### Android Requirements
```xml
<!-- AndroidManifest.xml -->
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MICROPHONE" />
```

---

## Agent Tasks

### Task 1: Audit cpal mobile support
- [ ] Review cpal GitHub issues for iOS/Android reports
- [ ] Check cpal supported hosts documentation
- [ ] Identify any mobile-specific feature flags
- [ ] Check last mobile-related commits

### Task 2: Test iOS audio capture
- [ ] Create minimal iOS project with cpal
- [ ] Test microphone enumeration
- [ ] Test audio capture stream
- [ ] Test AVAudioSession configuration requirements
- [ ] Document any Swift/ObjC bridging needed

### Task 3: Test Android audio capture
- [ ] Create minimal Android project with cpal
- [ ] Test microphone enumeration
- [ ] Test audio capture stream
- [ ] Test with foreground service
- [ ] Document JNI requirements

### Task 4: Evaluate platform-native alternatives
- [ ] Assess direct AVAudioEngine usage (iOS)
- [ ] Assess direct Oboe usage (Android)
- [ ] Compare API ergonomics
- [ ] Compare performance characteristics

### Task 5: Document integration patterns
- [ ] Recommended cpal configuration for STT
- [ ] Platform-specific setup requirements
- [ ] Error handling for common scenarios
- [ ] Sample code for audio capture loop

---

## Test Cases

### Basic Audio Capture
```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn test_audio_capture() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device available")?;

    println!("Using device: {}", device.name()?);

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            println!("Received {} samples", data.len());
        },
        move |err| {
            eprintln!("Stream error: {}", err);
        },
        None,
    )?;

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}
```

---

## Success Criteria

- [ ] cpal successfully captures audio on iOS simulator
- [ ] cpal successfully captures audio on Android emulator
- [ ] 16kHz mono capture confirmed working
- [ ] Background audio mode documented
- [ ] Route change handling documented
- [ ] Clear recommendation: use cpal vs platform-native

---

## Fallback Options

If cpal mobile support is insufficient:

1. **Platform-native + FFI:**
   - iOS: AVAudioEngine with Swift wrapper
   - Android: Oboe with JNI bindings
   - Pros: Full platform feature access
   - Cons: More code, platform-specific

2. **oboe-rs directly for Android:**
   - Pros: Low-latency audio designed for Android
   - Cons: Android-only, separate from iOS solution

3. **Hybrid approach:**
   - cpal for desktop
   - Platform-native for mobile
   - Shared audio processing pipeline

---

## Dependencies

- R-ORT-01 (similar mobile build challenges)
- R-BLE-01 (Bluetooth audio routing)

---

## References

- cpal: https://crates.io/crates/cpal
- cpal hosts: https://docs.rs/cpal/latest/cpal/platform/index.html
- AVAudioSession: https://developer.apple.com/documentation/avfaudio/avaudiosession
- Android AudioRecord: https://developer.android.com/reference/android/media/AudioRecord
- Oboe: https://github.com/google/oboe
- oboe-rs: https://crates.io/crates/oboe

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
