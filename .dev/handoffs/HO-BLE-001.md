# Handoff: WP-BLE Bluetooth HFP Implementation

> **ID**: HO-BLE-001 | **Work Package**: WP-BLE
>
> **Created**: 2026-01-23 | **Status**: 🟡 SCAFFOLDING COMPLETE - Awaiting Hardware Testing

---

## Summary

Implement Bluetooth HFP 1.6 codec negotiation for 16kHz wideband audio. Currently, Bluetooth audio may fall back to 8kHz narrowband, reducing transcription accuracy.

**Complexity**: MEDIUM-HIGH (platform-specific implementations required)

---

## Current Progress

### Completed (2026-01-23)
- [x] Core types: `BluetoothState`, `BluetoothAudioInfo`, `AudioRoute`
- [x] `BluetoothStateCallback` trait for state change notifications
- [x] `BluetoothAudioManager` struct with state management
- [x] 16 unit tests passing
- [x] Module exported from `audio/mod.rs`

### Remaining (Requires Hardware)
- [ ] **HUMAN ACTION**: Test with physical Bluetooth devices
- [ ] Platform-specific implementations (macOS, Windows, Linux)
- [ ] Integration with `CpalAudioSource` for route change handling
- [ ] 8kHz -> 16kHz resampling integration

---

## Quick Start

1. **Read the work package**: [.dev/work/WP-BLE.md](../work/WP-BLE.md)
2. **Read the research**: [R-BLE-01](../design/research/R-BLE-01-bluetooth-audio.md), [R-BLE-02](../design/research/R-BLE-02-desktop-hfp-negotiation.md)
3. **Start with route change detection** (cross-platform cpal callback)
4. **Then platform-specific HFP negotiation**

---

## Implementation Order

### Phase 1: Core Types (30 min)

Create `crates/lexicon-core/src/audio/bluetooth.rs`:

```rust
use std::sync::Arc;
use parking_lot::RwLock;

/// Bluetooth audio connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothState {
    Disconnected,
    Connecting,
    ConnectedNarrowband,  // 8kHz
    ConnectedWideband,    // 16kHz
    Failed(String),
}

/// Bluetooth audio route information
#[derive(Debug, Clone)]
pub struct BluetoothAudioInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub hfp_wideband: bool,
    pub state: BluetoothState,
}

/// Audio route types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioRoute {
    BuiltIn,
    External,
    Bluetooth(String),  // device name
}

/// Callback for route changes
pub trait RouteChangeCallback: Send + Sync {
    fn on_route_change(&self, new_route: AudioRoute);
}

pub struct BluetoothAudioManager {
    current_route: Arc<RwLock<AudioRoute>>,
    callback: Arc<RwLock<Option<Box<dyn RouteChangeCallback>>>>,
}

impl BluetoothAudioManager {
    pub fn new() -> Self {
        Self {
            current_route: Arc::new(RwLock::new(AudioRoute::BuiltIn)),
            callback: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_callback<F: RouteChangeCallback + 'static>(&self, callback: F) {
        *self.callback.write() = Some(Box::new(callback));
    }

    pub fn current_route(&self) -> AudioRoute {
        self.current_route.read().clone()
    }

    pub fn is_bluetooth_active(&self) -> bool {
        matches!(*self.current_route.read(), AudioRoute::Bluetooth(_))
    }

    // Platform-specific methods below
}
```

### Phase 2: Route Change Detection via cpal (1 hour)

Modify `crates/lexicon-core/src/audio/capture.rs`:

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

impl CpalAudioSource {
    /// Create with Bluetooth monitoring
    pub fn new_with_bluetooth_support() -> Result<Self, AudioError> {
        let host = cpal::default_host();

        // Get initial device
        let device = host.default_input_device()
            .ok_or(AudioError::NoDevice)?;

        let device_name = device.name().unwrap_or_default();

        // Detect if initial device is Bluetooth
        let is_bluetooth = device_name.contains("Bluetooth")
            || device_name.contains("AirPods")
            || device_name.contains("WH-")  // Sony
            || device_name.contains("Bose");

        // Build stream
        let config = Self::optimal_config(&device)?;
        let stream = Self::build_stream(&device, &config)?;

        Ok(Self {
            device,
            stream,
            is_bluetooth,
            // ...
        })
    }

    /// Called when audio route changes
    pub fn handle_route_change(&mut self, new_route: AudioRoute) -> Result<(), AudioError> {
        tracing::info!("Audio route changed to: {:?}", new_route);

        // Stop current stream
        self.stream.pause().ok();

        // Get new device
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or(AudioError::NoDevice)?;

        // Rebuild stream
        let config = Self::optimal_config(&device)?;
        let stream = Self::build_stream(&device, &config)?;

        self.device = device;
        self.stream = stream;
        self.is_bluetooth = matches!(new_route, AudioRoute::Bluetooth(_));

        // Resume
        self.stream.play().map_err(|e| AudioError::Stream(e.to_string()))?;

        Ok(())
    }

    fn optimal_config(device: &cpal::Device) -> Result<cpal::StreamConfig, AudioError> {
        let supported = device.supported_input_configs()
            .map_err(|e| AudioError::Config(e.to_string()))?;

        // Prefer 16kHz for Whisper, or highest available
        let mut best = None;
        for config in supported {
            if config.max_sample_rate().0 >= 16000 {
                best = Some(config.with_sample_rate(cpal::SampleRate(16000)));
                break;
            }
            best = Some(config.with_max_sample_rate());
        }

        best.ok_or(AudioError::NoConfig)
    }
}
```

### Phase 3: macOS HFP Detection (1-2 hours)

Create `crates/lexicon-core/src/audio/bluetooth/macos.rs`:

```rust
#[cfg(target_os = "macos")]
mod macos {
    use objc::{class, msg_send, sel, sel_impl, runtime::Object};
    use std::ffi::CStr;

    /// Check if current audio route is Bluetooth HFP with wideband support
    pub fn get_bluetooth_audio_info() -> Option<super::BluetoothAudioInfo> {
        unsafe {
            // Get AVAudioSession shared instance
            let session: *mut Object = msg_send![class!(AVAudioSession), sharedInstance];

            // Get current route
            let route: *mut Object = msg_send![session, currentRoute];
            let inputs: *mut Object = msg_send![route, inputs];

            let count: usize = msg_send![inputs, count];
            if count == 0 {
                return None;
            }

            // Get first input port
            let port: *mut Object = msg_send![inputs, objectAtIndex: 0usize];
            let port_type: *mut Object = msg_send![port, portType];

            // Check if HFP
            let hfp_type: *mut Object = msg_send![class!(AVAudioSessionPortBluetoothHFP), class];
            let is_hfp: bool = msg_send![port_type, isEqualToString: hfp_type];

            if !is_hfp {
                return None;
            }

            // Get device name
            let name: *mut Object = msg_send![port, portName];
            let name_str: *const i8 = msg_send![name, UTF8String];
            let device_name = CStr::from_ptr(name_str).to_string_lossy().into_owned();

            // Get sample rate (16000 = wideband, 8000 = narrowband)
            let sample_rate: f64 = msg_send![session, sampleRate];

            Some(super::BluetoothAudioInfo {
                device_name,
                sample_rate: sample_rate as u32,
                hfp_wideband: sample_rate >= 16000.0,
                state: if sample_rate >= 16000.0 {
                    super::BluetoothState::ConnectedWideband
                } else {
                    super::BluetoothState::ConnectedNarrowband
                },
            })
        }
    }

    /// Register for route change notifications
    pub fn register_route_change_callback<F: Fn(super::AudioRoute) + Send + 'static>(
        callback: F,
    ) {
        // Use NSNotificationCenter to observe AVAudioSessionRouteChangeNotification
        // This is complex - see R-BLE-02 for full implementation
        todo!("Implement notification observer")
    }
}
```

### Phase 4: Windows HFP Detection (1-2 hours)

Create `crates/lexicon-core/src/audio/bluetooth/windows.rs`:

```rust
#[cfg(target_os = "windows")]
mod windows {
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;
    use windows::core::*;

    pub fn get_bluetooth_audio_info() -> Option<super::BluetoothAudioInfo> {
        unsafe {
            // Initialize COM
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

            // Get device enumerator
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;

            // Get default input device
            let device: IMMDevice = enumerator
                .GetDefaultAudioEndpoint(eCapture, eConsole)
                .ok()?;

            // Get device properties
            let store: IPropertyStore = device.OpenPropertyStore(STGM_READ).ok()?;

            // Get friendly name
            let name_prop = store.GetValue(&PKEY_Device_FriendlyName).ok()?;
            let name = name_prop.to_string();

            // Check if Bluetooth
            if !name.to_lowercase().contains("bluetooth") {
                return None;
            }

            // Get audio client to check sample rate
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).ok()?;

            let mut format: *mut WAVEFORMATEX = std::ptr::null_mut();
            client.GetMixFormat(&mut format).ok()?;

            let sample_rate = (*format).nSamplesPerSec;

            Some(super::BluetoothAudioInfo {
                device_name: name,
                sample_rate,
                hfp_wideband: sample_rate >= 16000,
                state: if sample_rate >= 16000 {
                    super::BluetoothState::ConnectedWideband
                } else {
                    super::BluetoothState::ConnectedNarrowband
                },
            })
        }
    }
}
```

### Phase 5: Linux BlueZ/PulseAudio (1-2 hours)

Create `crates/lexicon-core/src/audio/bluetooth/linux.rs`:

```rust
#[cfg(target_os = "linux")]
mod linux {
    use std::process::Command;

    pub fn get_bluetooth_audio_info() -> Option<super::BluetoothAudioInfo> {
        // Use pactl to check current source
        let output = Command::new("pactl")
            .args(["info"])
            .output()
            .ok()?;

        let info = String::from_utf8_lossy(&output.stdout);

        // Parse default source
        let source_line = info.lines()
            .find(|l| l.starts_with("Default Source:"))?;

        let source_name = source_line.split(':').nth(1)?.trim();

        // Check if Bluetooth
        if !source_name.contains("bluez") && !source_name.contains("bluetooth") {
            return None;
        }

        // Get source details
        let output = Command::new("pactl")
            .args(["list", "sources"])
            .output()
            .ok()?;

        let sources = String::from_utf8_lossy(&output.stdout);

        // Parse sample rate from source info
        // Look for "Sample Specification:" line
        let mut sample_rate = 8000u32;
        let mut in_target_source = false;

        for line in sources.lines() {
            if line.contains(source_name) {
                in_target_source = true;
            }
            if in_target_source && line.contains("Sample Specification:") {
                // Format: "s16le 2ch 16000Hz"
                if let Some(hz_part) = line.split_whitespace().find(|s| s.ends_with("Hz")) {
                    if let Ok(rate) = hz_part.trim_end_matches("Hz").parse() {
                        sample_rate = rate;
                    }
                }
                break;
            }
        }

        Some(super::BluetoothAudioInfo {
            device_name: source_name.to_string(),
            sample_rate,
            hfp_wideband: sample_rate >= 16000,
            state: if sample_rate >= 16000 {
                super::BluetoothState::ConnectedWideband
            } else {
                super::BluetoothState::ConnectedNarrowband
            },
        })
    }

    /// Attempt to enable HFP wideband (mSBC codec)
    pub fn request_wideband() -> Result<bool, super::AudioError> {
        // This requires changing BlueZ profile
        // See R-BLE-02 for D-Bus implementation
        Ok(false)
    }
}
```

### Phase 6: 8kHz Upsampling Fallback (30 min)

Ensure 8kHz audio is properly resampled in `capture.rs`:

```rust
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

impl CpalAudioSource {
    fn create_resampler(from_rate: u32, to_rate: u32) -> impl Resampler<f32> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        SincFixedIn::<f32>::new(
            to_rate as f64 / from_rate as f64,
            2.0,  // max relative ratio
            params,
            1024, // chunk size
            1,    // channels
        ).expect("Failed to create resampler")
    }

    pub fn read_chunk(&mut self) -> Result<AudioChunk, AudioError> {
        let raw_samples = self.read_raw()?;

        // Resample if needed (e.g., 8kHz to 16kHz)
        let samples = if self.input_sample_rate != 16000 {
            let mut resampler = Self::create_resampler(self.input_sample_rate, 16000);
            let input = vec![raw_samples];
            let output = resampler.process(&input, None)
                .map_err(|e| AudioError::Resample(e.to_string()))?;
            output.into_iter().next().unwrap_or_default()
        } else {
            raw_samples
        };

        Ok(AudioChunk {
            samples,
            has_speech: self.vad.process(&samples),
            timestamp_ms: self.timestamp_ms(),
        })
    }
}
```

---

## Testing

### Manual Testing Required

| Scenario | Platform | Expected |
|----------|----------|----------|
| Connect AirPods Pro | macOS | 16kHz detected |
| Connect older BT headset | macOS | 8kHz + resample |
| Disconnect mid-capture | All | Falls back to built-in |
| Rapid connect/disconnect | All | No crash |

### Unit Tests

```bash
cargo test -p lexicon-core bluetooth
cargo test -p lexicon-core resample
```

---

## Acceptance Checklist

- [ ] `BluetoothAudioInfo` reports correct sample rate
- [ ] Route change detection works on all platforms
- [ ] Stream rebuilds on route change without crash
- [ ] 8kHz audio properly resampled to 16kHz
- [ ] Falls back to built-in mic on BT disconnect
- [ ] Logs indicate wideband vs narrowband status
- [ ] Tested with at least 2 BT headsets

---

## Notes

- macOS requires linking against AVFoundation framework
- Windows requires COM initialization
- Linux may need PulseAudio running (not pure ALSA)
- Keep platform code in separate modules with `#[cfg]`

---

*This is a MEDIUM priority task. Can be done in parallel with other Beta work.*
