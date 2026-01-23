# WP-BLE: Bluetooth HFP 1.6 Audio Support

> **Status**: Ready for Implementation | **Priority**: MEDIUM | **Phase**: Beta
>
> **Research**: [R-BLE-01](../design/research/R-BLE-01-bluetooth-audio.md), [R-BLE-02](../design/research/R-BLE-02-desktop-hfp-negotiation.md)
>
> **Created**: 2026-01-23 | **Last Updated**: 2026-01-23

---

## Overview

Implement Bluetooth HFP 1.6 codec negotiation for high-quality 16kHz audio capture from Bluetooth headsets. Currently, audio capture works via cpal but doesn't actively negotiate codec selection, potentially falling back to 8kHz narrowband audio.

**Impact**: 16kHz wideband audio significantly improves Whisper transcription accuracy compared to 8kHz narrowband.

---

## Scope

### Included
- HFP 1.6 codec preference negotiation (prefer 16kHz over 8kHz)
- Audio route change detection and stream rebuild
- Graceful fallback to 8kHz with automatic resampling
- Graceful fallback to device microphone on Bluetooth disconnection
- Platform-specific implementations:
  - macOS: AVAudioSession/CoreBluetooth
  - Windows: Windows.Media.Audio or MMDevice API
  - Linux: BlueZ/PulseAudio HFP profile control

### Excluded
- Bluetooth LE Audio (future technology, limited device support)
- A2DP (media profile, not for microphone input)
- iOS/Android (handled by WP-IOS, WP-ANDROID via FFI)
- Bluetooth pairing/discovery (OS handles this)

---

## Interface Contract

### Bluetooth Audio Manager

```rust
// crates/lexicon-core/src/audio/bluetooth.rs

/// Bluetooth audio connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothState {
    /// No Bluetooth audio device connected
    Disconnected,
    /// Connected but negotiating codec
    Connecting,
    /// Connected with narrowband (8kHz) - resampling needed
    ConnectedNarrowband,
    /// Connected with wideband (16kHz) - optimal quality
    ConnectedWideband,
    /// Connection failed
    Failed(String),
}

/// Bluetooth audio route information
#[derive(Debug, Clone)]
pub struct BluetoothAudioInfo {
    /// Device name
    pub device_name: String,
    /// Current sample rate (8000 or 16000)
    pub sample_rate: u32,
    /// Whether HFP 1.6+ is supported
    pub hfp_wideband: bool,
    /// Connection state
    pub state: BluetoothState,
}

/// Callback for Bluetooth state changes
pub trait BluetoothStateCallback: Send + Sync {
    /// Called when Bluetooth audio state changes
    fn on_state_change(&self, info: BluetoothAudioInfo);

    /// Called when audio route changes (BT connect/disconnect)
    fn on_route_change(&self, new_route: AudioRoute);
}

/// Audio route types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioRoute {
    /// Built-in device microphone
    BuiltIn,
    /// USB/external microphone
    External,
    /// Bluetooth headset
    Bluetooth(BluetoothAudioInfo),
}

/// Bluetooth audio manager
pub struct BluetoothAudioManager {
    state: Arc<RwLock<BluetoothState>>,
    callback: Option<Box<dyn BluetoothStateCallback>>,
}

impl BluetoothAudioManager {
    /// Create manager for current platform
    pub fn new() -> Result<Self, AudioError>;

    /// Set callback for state changes
    pub fn set_callback(&mut self, callback: impl BluetoothStateCallback + 'static);

    /// Get current Bluetooth audio info
    pub fn get_audio_info(&self) -> Option<BluetoothAudioInfo>;

    /// Request HFP wideband mode if available
    pub fn request_wideband(&self) -> Result<bool, AudioError>;

    /// Check if currently using Bluetooth audio
    pub fn is_bluetooth_active(&self) -> bool;

    /// Get current audio route
    pub fn current_route(&self) -> AudioRoute;
}
```

### Updated AudioSource

```rust
// crates/lexicon-core/src/audio/capture.rs

impl CpalAudioSource {
    /// Create audio source with Bluetooth support
    pub fn new_with_bluetooth() -> Result<Self, AudioError>;

    /// Handle route change (called by BluetoothAudioManager callback)
    pub fn handle_route_change(&mut self, new_route: AudioRoute) -> Result<(), AudioError>;

    /// Rebuild audio stream after device change
    fn rebuild_stream(&mut self) -> Result<(), AudioError>;
}
```

---

## Platform Implementation Details

### macOS (CoreAudio/AVFoundation)

```rust
#[cfg(target_os = "macos")]
mod macos_bluetooth {
    // Use AVAudioSession for route monitoring
    // AVAudioSessionRouteChangeNotification for callbacks
    // Check AVAudioSessionPortDescription for HFP support

    pub fn setup_route_monitoring() -> Result<(), AudioError> {
        // Register for AVAudioSessionRouteChangeNotification
        // Check AVAudioSessionPortBluetoothHFP
    }

    pub fn get_bluetooth_sample_rate() -> Option<u32> {
        // Query current route's sample rate
        // HFP 1.6+ will report 16000
    }
}
```

### Windows (MMDevice/Windows.Media.Audio)

```rust
#[cfg(target_os = "windows")]
mod windows_bluetooth {
    // Use IMMDeviceEnumerator for device monitoring
    // IMMNotificationClient for connection callbacks
    // Check device properties for HFP support

    pub fn get_bluetooth_device() -> Option<BluetoothAudioInfo> {
        // Enumerate audio endpoints
        // Filter for Bluetooth HFP devices
        // Query sample rate capabilities
    }
}
```

### Linux (PulseAudio/BlueZ)

```rust
#[cfg(target_os = "linux")]
mod linux_bluetooth {
    // Use PulseAudio for audio routing
    // BlueZ D-Bus API for HFP codec negotiation
    // pactl set-card-profile for profile switching

    pub fn set_hfp_wideband(device: &str) -> Result<bool, AudioError> {
        // Connect to PulseAudio
        // Set card profile to headset_head_unit or a]dp_sink
        // Negotiate mSBC codec for 16kHz
    }
}
```

---

## Acceptance Criteria

### HFP Negotiation
- [ ] Detects HFP 1.6+ capable devices on macOS
- [ ] Detects HFP 1.6+ capable devices on Windows
- [ ] Detects HFP 1.6+ capable devices on Linux
- [ ] Prefers 16kHz sample rate when available
- [ ] Reports current sample rate accurately

### Route Change Handling
- [ ] Detects Bluetooth device connection
- [ ] Detects Bluetooth device disconnection
- [ ] Rebuilds audio stream on route change without crash
- [ ] Falls back to device microphone on BT disconnect
- [ ] No audio stuttering during transition (R-AUD-01 requirement)

### Fallback Behavior
- [ ] 8kHz audio is upsampled to 16kHz for Whisper
- [ ] Upsampling quality doesn't significantly degrade transcription
- [ ] Clear indication in logs when using narrowband fallback
- [ ] User notification option for suboptimal audio quality

### Testing Requirements
- [ ] Tested with at least 2 different Bluetooth headsets
- [ ] Tested connection/disconnection scenarios
- [ ] Tested audio quality (WER comparison 8kHz vs 16kHz)
- [ ] No crashes on rapid connect/disconnect

---

## Dependencies

### Requires
- `cpal` (already in use) - Audio capture foundation
- Platform-specific audio APIs (compile-time conditional)
- `rubato` (already in use) - Resampling for 8kHz fallback

### Blocks
- Beta exit criteria (Bluetooth HFP 1.6 verified at 16kHz)

---

## Files

### New Files
- `crates/lexicon-core/src/audio/bluetooth.rs` - Bluetooth manager
- `crates/lexicon-core/src/audio/bluetooth/macos.rs` - macOS implementation
- `crates/lexicon-core/src/audio/bluetooth/windows.rs` - Windows implementation
- `crates/lexicon-core/src/audio/bluetooth/linux.rs` - Linux implementation

### Modified Files
- `crates/lexicon-core/src/audio/mod.rs` - Export bluetooth module
- `crates/lexicon-core/src/audio/capture.rs` - Add route change handling
- `crates/lexicon-core/Cargo.toml` - Platform-specific dependencies

### Test Files
- `crates/lexicon-core/src/audio/bluetooth/tests.rs` - Unit tests
- Manual test plan for Bluetooth devices

---

## Test Plan

### Unit Tests

| Test | Input | Expected Output |
|------|-------|-----------------|
| `parse_sample_rate_16k` | HFP 1.6 device info | `sample_rate: 16000` |
| `parse_sample_rate_8k` | HFP 1.5 device info | `sample_rate: 8000` |
| `route_change_to_bt` | Connect event | `AudioRoute::Bluetooth` |
| `route_change_to_builtin` | Disconnect event | `AudioRoute::BuiltIn` |
| `resample_8k_to_16k` | 8kHz samples | 16kHz samples, same duration |

### Integration Tests

| Test | Scenario | Pass Criteria |
|------|----------|---------------|
| `bt_connect_wideband` | Connect HFP 1.6 headset | 16kHz stream established |
| `bt_connect_narrowband` | Connect HFP 1.5 headset | 8kHz + resampling active |
| `bt_disconnect_fallback` | Disconnect during capture | Seamless switch to built-in |
| `bt_rapid_toggle` | Connect/disconnect 5x | No crashes, stream recovers |

### Manual Test Matrix

| Device | Platform | Expected Behavior |
|--------|----------|-------------------|
| AirPods Pro | macOS | 16kHz wideband |
| Sony WH-1000XM4 | Windows | 16kHz wideband |
| Generic BT 4.0 | Linux | 8kHz + resample |
| Any headset | All | Disconnect fallback works |

---

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Device doesn't support HFP 1.6 | MEDIUM | MEDIUM | Graceful 8kHz fallback |
| Platform API differences | MEDIUM | HIGH | Platform-specific modules |
| Audio stuttering on switch | HIGH | MEDIUM | Buffer during transition |
| BlueZ configuration varies | MEDIUM | HIGH | Document PulseAudio setup |

---

## Research References

- **R-BLE-01**: Bluetooth audio latency and compatibility matrix
- **R-BLE-02**: Desktop HFP negotiation implementation guide
- **R-AUD-01**: cpal audio capture patterns (route change handling)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work package created from Beta requirements |

---

*Ready for implementation. Platform APIs researched in R-BLE-02.*
