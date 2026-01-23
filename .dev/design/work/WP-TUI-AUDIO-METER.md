---
id: WP-TUI-AUDIO-METER
type: work-plan
status: complete
phase: beta
created: 2026-01-23
updated: 2026-01-23
author: design-lead
consumers: [developer]
depends-on: [WP-TUI]
blocks: []
priority: medium
tags: [tui, audio, level-meter, ux]
---

# WP-TUI-AUDIO-METER: Audio Level Meter Sensitivity Adjustment

> **Phase**: Beta | **Priority**: MEDIUM

## Summary

The TUI audio level meter displays signal levels that are significantly lower than the actual input levels. A microphone signal at approximately -5 to -7 dB (showing yellow on audio interface meters) displays as a tiny green dot, as if it were -30 to -50 dB. This makes the meter unusable for diagnosing audio input levels.

---

## Problem Analysis

### Current Implementation

The level meter in [audio.rs](crates/lexicon-cli/src/tui/audio.rs) and [transcription.rs](crates/lexicon-cli/src/tui/transcription.rs) calculates RMS (Root Mean Square) of raw audio samples:

```rust
// Lines 162-186 in audio.rs
let mono_sum: f32 = data
    .chunks(channels)
    .map(|chunk| {
        let sample = chunk.iter().sum::<f32>() / channels as f32;
        sample * sample
    })
    .sum();

let sample_count = data.len() / channels;
let rms = (mono_sum / sample_count as f32).sqrt();

// Clamp and store (0.0 - 1.0)
let clamped = smoothed.clamp(0.0, 1.0);
```

### Root Cause

1. **Linear RMS values are too small**: Speech audio typically has RMS values in the range 0.01-0.1 for normal speaking levels. A "hot" signal at -5 dB still only produces an RMS of ~0.5 at peak moments, but average levels are much lower.

2. **No dB-to-linear scaling**: The meter displays raw RMS directly as a 0.0-1.0 value without any gain or dB conversion. This means:
   - -20 dB RMS ≈ 0.1 linear → displays as 10% of meter
   - -40 dB RMS ≈ 0.01 linear → displays as 1% of meter

3. **Meter expects normalized levels**: The [level_meter.rs](crates/lexicon-cli/src/tui/widgets/level_meter.rs) widget correctly renders 0.0-1.0 values but assumes the input has already been scaled appropriately.

---

## Scope

### Included
- Add gain/scaling factor to amplify displayed level
- Consider logarithmic (dB) scaling for more natural response
- Make sensitivity adjustable (either constant or user-configurable)
- Update both `audio.rs` and `transcription.rs` level calculations
- Ensure meter responds visibly to typical microphone input levels

### Excluded
- Hardware calibration or absolute dB measurement
- Peak hold indicator (future enhancement)
- VU-style ballistics (future enhancement)

---

## Proposed Solution

### Option A: Simple Gain Multiplier (Recommended)
Apply a gain multiplier to make normal speech levels display prominently:

```rust
// Apply gain to make meter more visible
// Typical speech RMS is 0.02-0.1, we want that to show as 0.4-0.8
const METER_GAIN: f32 = 8.0;

let rms = (mono_sum / sample_count as f32).sqrt();
let scaled = (rms * METER_GAIN).clamp(0.0, 1.0);
```

### Option B: Logarithmic dB Scaling
Convert to dB scale for more natural audio meter response:

```rust
// Convert RMS to dB, map -60dB to 0dB range to 0.0-1.0
let rms = (mono_sum / sample_count as f32).sqrt();
let db = 20.0 * rms.max(1e-10).log10();  // Avoid log(0)
let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
// -60dB → 0.0, 0dB → 1.0
```

### Recommendation

**Option A** for simplicity. The gain can be tuned based on testing:
- Start with `METER_GAIN = 8.0`
- Adjust so typical speech at arm's length shows ~40-60% on the meter
- Loud speech / close mic shows 70-90%
- Clipping shows 100% (red zone)

---

## Interface Contract

```rust
// No new public API - internal constant change

// In audio.rs and transcription.rs
/// Gain multiplier for level meter display.
/// Scales RMS values so typical speech levels display prominently.
const METER_GAIN: f32 = 8.0;

fn calculate_display_level(rms: f32) -> f32 {
    (rms * METER_GAIN).clamp(0.0, 1.0)
}
```

---

## Acceptance Criteria

- [x] Normal conversation at arm's length from microphone shows 30-60% on meter
- [x] Loud speech / close to microphone shows 60-90% on meter
- [x] Very loud sounds / clipping shows meter in red zone (>90%)
- [x] Complete silence shows empty/minimal meter activity
- [x] Level meter updates in real-time (10+ fps) as before
- [x] Changes applied consistently in both `audio.rs` and `transcription.rs`

---

## Files to Modify

| File | Changes |
|------|---------|
| [crates/lexicon-cli/src/tui/audio.rs](crates/lexicon-cli/src/tui/audio.rs) | Add METER_GAIN constant, apply to RMS calculation |
| [crates/lexicon-cli/src/tui/transcription.rs](crates/lexicon-cli/src/tui/transcription.rs) | Apply same METER_GAIN scaling |

---

## Test Plan

### Manual Testing

| Test | Procedure | Pass Criteria |
|------|-----------|---------------|
| Normal speech | Speak at arm's length, normal volume | Meter shows 30-60% |
| Loud speech | Speak loudly, close to mic | Meter shows 60-90%, reaches yellow |
| Silence | No input | Meter shows minimal/no activity |
| Audio interface validation | Compare TUI meter to audio interface meter | Relative levels match |

---

## Dependencies

- WP-TUI (complete) - Base TUI implementation

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work plan created based on Windows build testing |
| 2026-01-23 | Implementation complete: METER_GAIN=8.0 applied to both audio.rs and transcription.rs |
