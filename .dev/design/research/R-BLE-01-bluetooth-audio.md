# Research Plan: Bluetooth Audio Latency and Disconnection Handling

**ID:** R-BLE-01
**Status:** Ongoing
**Category:** Edge Cases
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Compatibility matrix documentation

---

## Problem Statement

Bluetooth audio introduces unique challenges for real-time speech processing:
- Codec-dependent latency (40-200ms+ additional delay)
- Connection instability (dropouts, reconnections)
- Quality degradation (bandwidth-limited codecs)
- Platform-specific Bluetooth stack behaviors
- Multiple profile support (A2DP, HFP, HSP)

**Engineering Challenge:** Document Bluetooth audio behavior across platforms and implement robust handling for connection changes and latency variations.

---

## Research Questions

### RQ1: What latency does Bluetooth add to audio capture?

**Metrics to measure:**
- [ ] Latency by codec (SBC, AAC, aptX, LDAC)
- [ ] Latency by Bluetooth version (4.x, 5.x)
- [ ] Latency by audio profile (A2DP, HFP)
- [ ] Round-trip latency (mic input to speaker output)

**Known baseline:**
- SBC: ~170-270ms latency
- aptX: ~70-130ms latency
- AAC: ~80-150ms latency
- HFP (voice): ~50-150ms latency

### RQ2: How do Bluetooth disconnections manifest?

**Areas to investigate:**
- [ ] Audio stream behavior on disconnect
- [ ] Automatic reconnection behavior
- [ ] Partial disconnection (one direction fails)
- [ ] Range-related dropouts

### RQ3: How do platforms handle audio route changes?

**Areas to investigate:**
- [ ] iOS: AVAudioSession route change notifications
- [ ] Android: AudioManager callback patterns
- [ ] Wired → Bluetooth transitions
- [ ] Bluetooth → speaker fallback

### RQ4: What quality tradeoffs exist for voice vs music profiles?

**Areas to investigate:**
- [ ] HFP (8kHz, mono) vs A2DP (48kHz, stereo)
- [ ] Microphone availability per profile
- [ ] Automatic profile switching behavior
- [ ] Manual profile control APIs

---

## Known Bluetooth Behaviors

| Platform | Bluetooth Stack | Notes |
|----------|----------------|-------|
| iOS | CoreBluetooth + AVFoundation | Limited codec control |
| Android | Android Bluetooth stack | Codec negotiation visible |
| macOS | CoreBluetooth | Similar to iOS |
| Windows | Windows Bluetooth stack | Variable by driver |
| Linux | BlueZ | Most configurable |

### Bluetooth Audio Profiles

| Profile | Use Case | Quality | Microphone |
|---------|----------|---------|------------|
| A2DP | Music playback | High (up to 48kHz stereo) | No |
| HFP | Phone calls | Low (8kHz mono) | Yes |
| HSP | Headset | Low (8kHz mono) | Yes |

**Key insight:** For STT, we need microphone input, which typically forces HFP/HSP with degraded quality. Some devices support A2DP input but it's not universal.

---

## Technical Setup

### iOS Audio Route Handling
```swift
import AVFoundation

class AudioRouteObserver {
    init() {
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleRouteChange),
            name: AVAudioSession.routeChangeNotification,
            object: nil
        )
    }

    @objc func handleRouteChange(notification: Notification) {
        guard let reason = notification.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
              let routeChangeReason = AVAudioSession.RouteChangeReason(rawValue: reason) else {
            return
        }

        switch routeChangeReason {
        case .newDeviceAvailable:
            // Bluetooth connected
        case .oldDeviceUnavailable:
            // Bluetooth disconnected
        case .categoryChange:
            // Category changed (may affect Bluetooth profile)
        default:
            break
        }
    }
}
```

### Android Audio Route Handling
```kotlin
class BluetoothAudioReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            BluetoothDevice.ACTION_ACL_CONNECTED -> {
                // Bluetooth connected
            }
            BluetoothDevice.ACTION_ACL_DISCONNECTED -> {
                // Bluetooth disconnected
            }
            AudioManager.ACTION_SCO_AUDIO_STATE_UPDATED -> {
                val state = intent.getIntExtra(AudioManager.EXTRA_SCO_AUDIO_STATE, -1)
                // SCO state changed (voice profile)
            }
        }
    }
}
```

---

## Agent Tasks

### Task 1: Research Bluetooth audio latency
- [ ] Survey existing latency measurements by codec
- [ ] Identify latency measurement methodology
- [ ] Document typical AirPods/Galaxy Buds latency
- [ ] Identify lowest-latency Bluetooth options

### Task 2: Document platform Bluetooth APIs
- [ ] iOS AVAudioSession Bluetooth handling
- [ ] Android AudioManager Bluetooth handling
- [ ] Document route change notification patterns
- [ ] Document profile switching behavior

### Task 3: Create compatibility matrix
- [ ] Test popular Bluetooth devices (AirPods, Galaxy Buds, Sony)
- [ ] Test wired headsets for baseline
- [ ] Document latency per device/platform combo
- [ ] Document any incompatibilities

### Task 4: Design disconnection handling
- [ ] Define reconnection strategy
- [ ] Define fallback behavior (to built-in mic)
- [ ] Define user notification approach
- [ ] Define audio buffer handling during dropout

### Task 5: Implement Bluetooth handling module
- [ ] Route change detection
- [ ] Automatic fallback logic
- [ ] Latency compensation (if measurable)
- [ ] User preference persistence

---

## Test Devices

| Device | Codec Support | Profile Support | Notes |
|--------|---------------|-----------------|-------|
| AirPods Pro | AAC | A2DP, HFP | iOS optimized |
| AirPods Pro | SBC | A2DP, HFP | Android |
| Galaxy Buds Pro | SBC, AAC, Scalable | A2DP, HFP | Samsung optimized |
| Sony WH-1000XM5 | SBC, AAC, LDAC | A2DP, HFP | High-quality |
| Generic BT headset | SBC | HFP | Baseline |

---

## Success Criteria

- [ ] Latency documented for 5+ Bluetooth devices
- [ ] Route change handling implemented for iOS and Android
- [ ] Disconnection recovery strategy documented
- [ ] Compatibility matrix published
- [ ] Fallback behavior tested and documented

---

## Latency Compensation Strategy

If Bluetooth latency is measurable and consistent:

1. **Timestamp adjustment:** Offset audio timestamps by measured latency
2. **Buffer pre-fill:** Add latency-equivalent buffer at stream start
3. **User calibration:** Allow manual latency adjustment in settings

If latency is variable or unmeasurable:

1. **Accept latency:** Don't compensate; total pipeline latency increases
2. **Recommend wired:** Suggest wired connection for lowest latency
3. **Display warning:** "Bluetooth may add delay to transcription"

---

## Dependencies

- R-AUD-01 (cpal audio capture, route handling may overlap)

---

## References

- Bluetooth Latency Explained: https://www.soundguys.com/bluetooth-latency-explained-24272/
- iOS AVAudioSession: https://developer.apple.com/documentation/avfaudio/avaudiosession
- Android Bluetooth Audio: https://developer.android.com/guide/topics/connectivity/bluetooth/profiles
- aptX Latency: https://www.qualcomm.com/products/features/aptx
- Bluetooth Audio Codecs: https://www.androidauthority.com/bluetooth-codecs-997074/

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
