# Research Report: Desktop Bluetooth HFP 1.6 Codec Negotiation

**ID:** R-BLE-02
**Status:** Complete (Research Phase)
**Category:** Audio / Bluetooth
**Created:** 2026-01-23
**Last Updated:** 2026-01-23
**Related:** R-BLE-01 (Mobile Bluetooth), WP-ROADBLOCK-06
**Output:** Implementation guide for desktop HFP 1.6 negotiation

---

## Executive Summary

### Platform Support Matrix

| Platform | HFP Version | 16kHz Support | Codec Control | Implementation Complexity |
|----------|-------------|---------------|---------------|--------------------------|
| **macOS** | HFP 1.7 | ✅ Automatic | Limited | LOW |
| **Windows** | HFP 1.6+ | ✅ Manual | Full | MEDIUM |
| **Linux** | HFP 1.7 | ✅ Configurable | Full | HIGH |

### Key Findings

1. **macOS handles HFP negotiation automatically** — No explicit codec selection needed; system negotiates best available
2. **Windows requires explicit profile selection** — Use `Windows.Devices.Bluetooth` APIs to query and select codec
3. **Linux offers most control via BlueZ** — Can force specific codec via `bluetoothctl` or D-Bus APIs
4. **16kHz (mSBC) is the key codec** — HFP 1.6+ supports mSBC codec at 16kHz; older HFP uses CVSD at 8kHz

### Recommendation: CONDITIONAL IMPLEMENTATION

| Platform | Recommendation |
|----------|----------------|
| **macOS** | No code needed — automatic negotiation works well |
| **Windows** | Implement detection only — show warning if 8kHz |
| **Linux** | Implement full control — BlueZ allows codec selection |

---

## Problem Statement

WP-ROADBLOCK-06 requires implementing HFP 1.6 codec negotiation to ensure Bluetooth headsets connect at 16kHz when supported. Whisper requires 16kHz audio input; 8kHz (HFP 1.5) degrades STT accuracy.

The existing R-BLE-01 research covers mobile platforms. This report addresses **desktop platforms** (macOS, Windows, Linux).

---

## Platform Analysis

### macOS (CoreBluetooth + AVFoundation)

**HFP Support:** macOS supports HFP 1.7 and automatically negotiates the best available codec.

**Codec Negotiation:** Handled by the system. When a Bluetooth headset connects:
1. System queries device capabilities
2. Negotiates HFP version and codec
3. Selects mSBC (16kHz) if both sides support it
4. Falls back to CVSD (8kHz) for legacy devices

**API Access:**
```swift
import AVFoundation
import CoreBluetooth

// Check current audio route
let session = AVAudioSession.sharedInstance()
let currentRoute = session.currentRoute

for output in currentRoute.outputs {
    if output.portType == .bluetoothHFP {
        // HFP is active - check sample rate
        let sampleRate = session.sampleRate // Will be 8000 or 16000
        if sampleRate < 16000 {
            print("Warning: HFP connected at \(sampleRate)Hz - quality may be degraded")
        }
    }
}
```

**Limitation:** macOS does not expose APIs to force a specific codec. The system handles negotiation automatically.

**Recommendation:** Detect connection quality and warn user if 8kHz. No active negotiation possible.

---

### Windows (Windows.Devices.Bluetooth)

**HFP Support:** Windows 10/11 support HFP 1.6+ with mSBC codec.

**Codec Negotiation:** Windows allows querying device capabilities but codec selection depends on driver implementation.

**API Access (WinRT):**
```rust
// Using windows-rs crate
use windows::Devices::Bluetooth::BluetoothDevice;
use windows::Devices::Bluetooth::Rfcomm::RfcommDeviceService;

// Query connected Bluetooth devices
async fn check_hfp_capability(device_id: &str) -> Result<bool, Error> {
    let device = BluetoothDevice::FromIdAsync(device_id)?.await?;

    // Check if HFP service is available
    let services = device.GetRfcommServicesForIdAsync(
        RfcommServiceId::FromUuid(HFP_SERVICE_UUID)?
    )?.await?;

    // HFP 1.6+ indicates mSBC support
    // Check SDP records for supported features
    Ok(services.Services()?.Size()? > 0)
}
```

**Audio Endpoint Configuration:**
```rust
use windows::Win32::Media::Audio::{
    IMMDeviceEnumerator, IMMDevice, IAudioClient,
    WAVEFORMATEX, AUDCLNT_SHAREMODE_SHARED,
};

// Get audio format for Bluetooth device
fn get_bt_audio_format(device: &IMMDevice) -> Result<WAVEFORMATEX, Error> {
    let audio_client: IAudioClient = device.Activate()?;
    let mut format: *mut WAVEFORMATEX = std::ptr::null_mut();
    audio_client.GetMixFormat(&mut format)?;

    // format.nSamplesPerSec will be 8000 or 16000 for HFP
    Ok(unsafe { *format })
}
```

**Limitation:** While Windows exposes device capabilities, actual codec selection is handled by the Bluetooth driver. Most modern Windows Bluetooth stacks prefer mSBC when available.

**Recommendation:** Query device sample rate at connection; warn if 8kHz; no active codec forcing.

---

### Linux (BlueZ + PulseAudio/PipeWire)

**HFP Support:** BlueZ supports HFP 1.7 with full codec negotiation control.

**Codec Negotiation:** Linux provides the most control via:
1. BlueZ configuration files
2. D-Bus APIs
3. PulseAudio/PipeWire profile selection

**BlueZ Configuration:**
```ini
# /etc/bluetooth/main.conf
[General]
# Enable mSBC codec (HFP 1.6+)
Enable=Source,Sink,Media,Socket
FastConnectable=true

# Force high-quality codec
[HFP]
# 0 = CVSD only, 1 = mSBC preferred
WideBAnd=true
```

**D-Bus API for Codec Control:**
```rust
use dbus::blocking::Connection;

fn set_hfp_codec(device_path: &str, use_msbc: bool) -> Result<(), Error> {
    let conn = Connection::new_system()?;

    // Access BlueZ MediaTransport interface
    let proxy = conn.with_proxy(
        "org.bluez",
        device_path,
        std::time::Duration::from_secs(5)
    );

    // Set codec preference
    // Configuration varies by BlueZ version
    proxy.method_call(
        "org.bluez.MediaTransport1",
        "SelectConfiguration",
        (if use_msbc { "mSBC" } else { "CVSD" },)
    )?;

    Ok(())
}
```

**PulseAudio Profile Selection:**
```bash
# List Bluetooth device profiles
pactl list cards | grep -A 20 "bluez"

# Switch to HFP profile (enables microphone)
pactl set-card-profile bluez_card.XX_XX_XX_XX_XX_XX headset_head_unit

# Check current sample rate
pactl list sources | grep -A 5 "bluez"
```

**PipeWire Configuration:**
```lua
-- ~/.config/pipewire/pipewire.conf.d/bluetooth.conf
context.properties = {
    bluez5.enable-msbc = true
    bluez5.enable-sbc-xq = true
    bluez5.hfphsp-backend = "native"
}
```

**Recommendation:** Implement full codec control on Linux via D-Bus/BlueZ APIs.

---

## Implementation Strategy

### Phase 1: Detection (All Platforms)

Implement sample rate detection for all platforms:

```rust
pub struct BluetoothAudioInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub is_hfp: bool,
    pub supports_wideband: bool,
}

pub fn detect_bluetooth_audio() -> Option<BluetoothAudioInfo> {
    #[cfg(target_os = "macos")]
    return detect_macos_bluetooth();

    #[cfg(target_os = "windows")]
    return detect_windows_bluetooth();

    #[cfg(target_os = "linux")]
    return detect_linux_bluetooth();

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return None;
}
```

### Phase 2: User Warning

Display warning when 8kHz connection detected:

```rust
pub enum BluetoothQualityWarning {
    None,
    NarrowbandDetected {
        device_name: String,
        recommendation: String,
    },
}

pub fn check_bluetooth_quality(info: &BluetoothAudioInfo) -> BluetoothQualityWarning {
    if info.sample_rate < 16000 {
        BluetoothQualityWarning::NarrowbandDetected {
            device_name: info.device_name.clone(),
            recommendation: "Speech recognition quality may be reduced. \
                Consider using a wired headset or upgrading to HFP 1.6+ device.".into(),
        }
    } else {
        BluetoothQualityWarning::None
    }
}
```

### Phase 3: Active Control (Linux Only)

On Linux, attempt to force mSBC codec:

```rust
#[cfg(target_os = "linux")]
pub fn request_wideband_codec(device_path: &str) -> Result<bool, Error> {
    // Try BlueZ D-Bus API
    if let Ok(()) = set_bluez_msbc_preference(device_path, true) {
        return Ok(true);
    }

    // Try PipeWire/PulseAudio
    if let Ok(()) = set_pipewire_msbc_preference(true) {
        return Ok(true);
    }

    // Codec control not available
    Ok(false)
}
```

---

## Codec Technical Details

### CVSD (Continuously Variable Slope Delta Modulation)

- **Sample Rate:** 8kHz
- **Bit Rate:** 64 kbps
- **HFP Version:** 1.0+
- **Quality:** Low (telephone quality)
- **STT Impact:** Significant degradation; high-frequency speech details lost

### mSBC (Modified Subband Codec)

- **Sample Rate:** 16kHz
- **Bit Rate:** 56 kbps
- **HFP Version:** 1.6+
- **Quality:** Good (wideband voice)
- **STT Impact:** Near-native quality for Whisper

### Codec Comparison for STT

| Codec | Sample Rate | Whisper Accuracy | Resampling Needed |
|-------|-------------|------------------|-------------------|
| CVSD | 8kHz | ~70-80% WER increase | Yes (upsample 2x) |
| mSBC | 16kHz | Baseline | No |
| LC3 (LE Audio) | 16-32kHz | Better than baseline | No |

---

## Testing Requirements

### Test Matrix

| Test | macOS | Windows | Linux |
|------|-------|---------|-------|
| Detect HFP connection | Required | Required | Required |
| Read sample rate | Required | Required | Required |
| Display quality warning | Required | Required | Required |
| Force mSBC codec | N/A | N/A | Required |
| Verify STT quality | Required | Required | Required |

### Test Devices

| Device | HFP Version | Expected Sample Rate |
|--------|-------------|---------------------|
| AirPods Pro 2 | 1.7 | 16kHz |
| Sony WH-1000XM5 | 1.7 | 16kHz |
| Generic BT Headset | 1.5 | 8kHz |
| Jabra Evolve2 | 1.7 | 16kHz |

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| User has HFP 1.5-only device | Degraded STT quality | Clear warning + resampling |
| BlueZ API changes | Linux code breaks | Version detection + fallback |
| Windows driver variations | Inconsistent behavior | Detection-only approach |
| Multiple BT devices connected | Wrong device selected | Let user select preferred input |

---

## Dependencies

- **cpal** - Audio device enumeration (already in use)
- **windows-rs** - Windows Bluetooth APIs (optional, for better detection)
- **dbus** - Linux BlueZ communication (optional, for codec control)
- **R-BLE-01** - Mobile Bluetooth research (related)
- **R-AUD-01** - Audio capture patterns (related)

---

## Completion Criteria

- [x] Document platform-specific HFP negotiation approaches
- [x] Identify APIs for sample rate detection per platform
- [x] Document codec control capabilities per platform
- [x] Provide implementation strategy with code samples
- [x] Define test matrix for validation

---

## Recommendations for WP-ROADBLOCK-06

Based on this research:

1. **Implement detection-only for macOS/Windows** — System handles negotiation automatically
2. **Implement full control for Linux** — BlueZ provides codec selection APIs
3. **Always detect and warn on 8kHz** — User should know about quality limitation
4. **Implement resampling fallback** — 8kHz → 16kHz upsampling for legacy devices
5. **Document audio quality differences** — Help users understand BT vs wired tradeoffs

### Estimated Implementation Effort

| Component | Effort | Priority |
|-----------|--------|----------|
| Sample rate detection (all platforms) | LOW | HIGH |
| Quality warning UI | LOW | HIGH |
| Linux BlueZ codec control | MEDIUM | MEDIUM |
| Resampling fallback | LOW | HIGH |
| Documentation | LOW | HIGH |

---

## References

- [BlueZ HFP Implementation](https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/profiles/audio)
- [Windows Bluetooth APIs](https://docs.microsoft.com/en-us/windows/uwp/devices-sensors/bluetooth)
- [macOS AVAudioSession](https://developer.apple.com/documentation/avfaudio/avaudiosession)
- [PipeWire Bluetooth](https://gitlab.freedesktop.org/pipewire/pipewire/-/wikis/Bluetooth)
- [HFP 1.7 Specification](https://www.bluetooth.com/specifications/specs/hands-free-profile-1-8/)
- [mSBC Codec Details](https://www.bluetooth.com/specifications/specs/hands-free-profile-1-8/)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Initial research report created |
| 2026-01-23 | Platform analysis complete (macOS, Windows, Linux) |
| 2026-01-23 | Implementation strategy documented |
