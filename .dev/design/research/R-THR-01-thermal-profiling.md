# Research Plan: Thermal Behavior Profiling

**ID:** R-THR-01
**Status:** Ongoing
**Category:** Performance
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Throttling thresholds documentation

---

## Problem Statement

Sustained ML inference on mobile devices generates significant heat. When devices overheat:
- CPUs/GPUs throttle performance (reduced clock speeds)
- Transcription latency increases dramatically
- Battery drain accelerates
- User experience degrades
- Device may forcibly terminate the app

**Engineering Challenge:** Profile thermal behavior of sustained Whisper inference to determine safe operating limits and implement appropriate throttling strategies.

---

## Research Questions

### RQ1: What thermal profiles exist for different model sizes?

**Metrics to capture:**
- [ ] Device temperature over time (surface, SoC if available)
- [ ] CPU frequency over time (throttling detection)
- [ ] Inference latency over time (performance degradation)
- [ ] Battery consumption rate

**Test matrix:**
| Model | Duration | Device Class | Expected Outcome |
|-------|----------|--------------|------------------|
| tiny.en | 30 min | Low-end phone | Baseline |
| base.en | 30 min | Low-end phone | Potential throttling |
| tiny.en | 30 min | High-end phone | Minimal impact |
| base.en | 30 min | High-end phone | Moderate heat |

### RQ2: When does thermal throttling begin?

**Areas to investigate:**
- [ ] Time to first throttling event
- [ ] Throttling severity (% performance reduction)
- [ ] Recovery time after throttling
- [ ] OS thermal state notifications (iOS/Android APIs)

### RQ3: How do cooling periods affect sustained performance?

**Areas to investigate:**
- [ ] Optimal inference/pause duty cycle
- [ ] VAD-driven natural cooling (pauses during silence)
- [ ] Active cooling detection (fans, cold environment)
- [ ] Battery temperature correlation

### RQ4: What mitigation strategies are effective?

**Areas to investigate:**
- [ ] Reducing inference frequency (skip frames)
- [ ] Model switching (base → tiny when hot)
- [ ] Processing quality reduction
- [ ] User notification strategies

---

## Known Thermal Behaviors

| Platform | Thermal Management | Developer Access |
|----------|-------------------|------------------|
| iOS | Aggressive throttling | Limited (ProcessInfo.thermalState) |
| Android | Varies by OEM | PowerManager thermal APIs (API 29+) |
| macOS | Fan control + throttling | IOKit thermal sensors |
| Windows | Varies by device | WMI thermal sensors |

---

## Technical Setup

### iOS Thermal Monitoring
```swift
// Swift wrapper needed for Rust FFI
import Foundation

let thermalState = ProcessInfo.processInfo.thermalState
// .nominal, .fair, .serious, .critical

NotificationCenter.default.addObserver(
    forName: ProcessInfo.thermalStateDidChangeNotification,
    object: nil,
    queue: .main
) { _ in
    // Handle thermal state change
}
```

### Android Thermal Monitoring
```kotlin
// Kotlin wrapper needed for Rust FFI
val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
val thermalStatus = powerManager.currentThermalStatus
// THERMAL_STATUS_NONE through THERMAL_STATUS_SHUTDOWN
```

### Rust Profiling Code
```rust
use std::time::{Duration, Instant};

struct ThermalProfile {
    start_time: Instant,
    samples: Vec<ThermalSample>,
}

struct ThermalSample {
    timestamp: Duration,
    inference_latency_ms: u64,
    // Platform-specific thermal readings via FFI
}

fn profile_sustained_inference(
    ctx: &WhisperContext,
    duration: Duration,
) -> ThermalProfile {
    // Run inference in a loop, sampling performance metrics
}
```

---

## Agent Tasks

### Task 1: Research platform thermal APIs
- [ ] Document iOS ProcessInfo.thermalState behavior
- [ ] Document Android PowerManager thermal APIs
- [ ] Identify Rust FFI requirements for each platform
- [ ] Find any existing Rust thermal monitoring crates

### Task 2: Design profiling methodology
- [ ] Define standard test audio (30-minute continuous speech)
- [ ] Define measurement intervals (e.g., every 10 seconds)
- [ ] Define thermal reading method per platform
- [ ] Define success/failure criteria

### Task 3: Implement profiling harness
- [ ] Create test app for iOS
- [ ] Create test app for Android
- [ ] Implement inference loop with metrics collection
- [ ] Implement data export (CSV/JSON)

### Task 4: Run thermal profiles
- [ ] Profile tiny.en on 3 device tiers
- [ ] Profile base.en on 3 device tiers
- [ ] Run extended (1-hour) test on one device
- [ ] Document ambient temperature conditions

### Task 5: Analyze results and recommend mitigations
- [ ] Identify throttling thresholds
- [ ] Recommend model selection by device tier
- [ ] Recommend duty cycle if needed
- [ ] Document monitoring/alerting approach

---

## Test Devices (Recommended)

| Tier | iOS | Android | Notes |
|------|-----|---------|-------|
| Low-end | iPhone SE (2nd gen) | Pixel 4a | 2-3GB RAM, older SoC |
| Mid-range | iPhone 12 | Pixel 6 | 4GB RAM |
| High-end | iPhone 14 Pro | Pixel 8 Pro | 6GB+ RAM, efficient SoC |

---

## Success Criteria

- [ ] Thermal profiles documented for tiny.en and base.en
- [ ] Throttling onset time identified per device tier
- [ ] Performance degradation quantified (% latency increase)
- [ ] Mitigation strategy recommended
- [ ] Platform thermal API integration documented

---

## Mitigation Strategies to Evaluate

1. **Adaptive model selection:**
   - Start with base.en
   - Switch to tiny.en when thermal state degrades
   - Switch back when cooled

2. **VAD-driven duty cycling:**
   - Only process during speech
   - Natural cooling during silence
   - Already planned for accuracy; also helps thermal

3. **Quality/latency tradeoff:**
   - Reduce beam search iterations when hot
   - Accept slightly lower accuracy for thermal headroom

4. **User communication:**
   - "Device warming up, quality may vary"
   - Suggest breaks for long transcriptions

---

## Dependencies

- R-WRS-01 (whisper-rs benchmarks inform baseline)
- R-AUD-01 (audio capture also generates heat)

---

## References

- iOS Thermal State: https://developer.apple.com/documentation/foundation/processinfo/thermalstate
- Android Thermal API: https://developer.android.com/reference/android/os/PowerManager#getCurrentThermalStatus()
- WWDC Thermal Management: https://developer.apple.com/videos/play/wwdc2020/10063/
- Android Performance: https://developer.android.com/topic/performance/power

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
