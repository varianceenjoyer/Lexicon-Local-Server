---
id: WP-AUDIO-SIM
type: work-plan
status: in-progress
phase: beta
created: 2026-01-27
updated: 2026-01-27
author: design-lead
consumers: [developer]
depends-on: []
blocks: []
priority: high
tags: [android, testing, audio, tui, simulation]
---

# WP-AUDIO-SIM: Simulated Audio Device for Android Testing

> **Phase**: Beta | **Priority**: HIGH | **Status**: In Progress (core implementation complete)

## Summary

Implement a simulated audio device that can load test audio files and stream them as if from a real microphone. This enables testing audio capture and transcription features on Android (and other headless environments) where real audio hardware is unavailable due to cpal/oboe requiring an Android app context.

---

## Scope

### Included
- Simulated audio source that streams audio from files
- Support for common audio formats (WAV, MP3, FLAC, OGG)
- Real-time streaming at natural playback speed (simulating live capture)
- Integration with TUI's existing audio/transcription pipeline
- Test audio fixtures for verification
- Command-line flag to enable simulation mode
- Loop mode for continuous testing

### Excluded
- Real audio capture on Android (requires native app integration)
- Audio output/playback
- Recording to file
- Network audio streaming
- Complex audio processing (EQ, effects, etc.)

---

## Interface Contract

```rust
// crates/lexicon-cli/src/tui/audio_sim.rs

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Arc;

/// Configuration for the simulated audio source.
#[derive(Debug, Clone)]
pub struct SimulatedAudioConfig {
    /// Path to the audio file to stream.
    pub file_path: PathBuf,
    /// Whether to loop the audio file continuously.
    pub loop_playback: bool,
    /// Playback speed multiplier (1.0 = real-time, 2.0 = 2x speed).
    pub speed: f32,
}

/// A simulated audio source that streams audio from a file.
pub struct SimulatedAudioSource {
    config: SimulatedAudioConfig,
    /// Current audio level (0.0 - 1.0), stored as u32 bits of f32.
    level: Arc<AtomicU32>,
    /// Whether playback is active.
    playing: Arc<AtomicBool>,
    /// Worker thread handle.
    worker_handle: Option<std::thread::JoinHandle<()>>,
}

impl SimulatedAudioSource {
    /// Creates a new simulated audio source.
    pub fn new(config: SimulatedAudioConfig) -> Result<Self, AudioSimError>;

    /// Starts streaming audio to the provided sender.
    /// Audio is sent as Vec<f32> chunks at 16kHz mono (Whisper format).
    pub fn start(&mut self, audio_tx: mpsc::UnboundedSender<Vec<f32>>) -> Result<(), AudioSimError>;

    /// Stops streaming.
    pub fn stop(&mut self);

    /// Returns the current audio level (0.0 - 1.0).
    pub fn get_level(&self) -> f32;

    /// Returns whether the source is currently playing.
    pub fn is_playing(&self) -> bool;

    /// Returns total duration of the audio file.
    pub fn duration(&self) -> std::time::Duration;

    /// Returns current playback position.
    pub fn position(&self) -> std::time::Duration;
}

#[derive(Debug, thiserror::Error)]
pub enum AudioSimError {
    #[error("Failed to load audio file: {0}")]
    LoadError(String),
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),
    #[error("Playback error: {0}")]
    PlaybackError(String),
}

// Integration with existing AudioDeviceManager
// crates/lexicon-cli/src/tui/audio.rs

/// Information about an audio input device (extended).
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub id: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
    /// True if this is a simulated device.
    pub is_simulated: bool,
}

// TUI config extension
// crates/lexicon-cli/src/tui/app.rs

pub struct TuiConfig {
    // ... existing fields ...

    /// Path to simulated audio file (enables simulation mode).
    pub simulated_audio: Option<PathBuf>,
    /// Loop simulated audio playback.
    pub simulated_loop: bool,
}
```

---

## Acceptance Criteria

- [ ] `SimulatedAudioSource` can load WAV files and stream at 16kHz mono
- [ ] `SimulatedAudioSource` can load MP3, FLAC, OGG via symphonia
- [ ] Audio streams at real-time speed (configurable)
- [ ] Audio level meter shows accurate RMS levels from simulated audio
- [ ] TUI transcription works end-to-end with simulated audio
- [ ] Loop mode repeats audio file continuously
- [ ] CLI flag `--simulated-audio <path>` enables simulation mode
- [ ] CLI flag `--simulated-loop` enables loop mode
- [ ] Works on Android target (no cpal dependency in simulation path)
- [ ] Test audio fixtures provided in `tests/fixtures/audio/`
- [ ] Unit tests verify audio loading and resampling
- [ ] Integration test verifies transcription with simulated audio

---

## Dependencies

### Requires
- symphonia (already in workspace) - audio file decoding
- rubato (already in workspace) - sample rate conversion
- hound (already in workspace) - WAV file reading

### Blocks
- Android TUI testing capability

---

## Files

### New Files
- `crates/lexicon-cli/src/tui/audio_sim.rs` - Simulated audio source
- `tests/fixtures/audio/test_speech.wav` - Test fixture (short speech sample)
- `tests/fixtures/audio/test_silence.wav` - Test fixture (silence for edge cases)

### Modified Files
- `crates/lexicon-cli/src/tui/mod.rs` - Export audio_sim module
- `crates/lexicon-cli/src/tui/app.rs` - Add simulated audio config and integration
- `crates/lexicon-cli/src/tui/transcription.rs` - Support simulated audio source
- `crates/lexicon-cli/src/commands/tui.rs` - Add CLI flags

### Test Files
- `crates/lexicon-cli/src/tui/audio_sim.rs` (inline tests)
- `tests/e2e_tests.rs` - Add simulated audio integration test

---

## Architecture

### Audio Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         TUI Application                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐         ┌──────────────────────────────┐  │
│  │  CLI Arguments   │         │      TranscriptionManager     │  │
│  │                  │         │                               │  │
│  │ --simulated-audio├────────►│  ┌─────────────────────────┐ │  │
│  │ --simulated-loop │         │  │    Audio Source         │ │  │
│  └──────────────────┘         │  │  (SimulatedAudioSource  │ │  │
│                               │  │   or cpal::Stream)      │ │  │
│                               │  └───────────┬─────────────┘ │  │
│                               │              │               │  │
│                               │              ▼               │  │
│                               │  ┌─────────────────────────┐ │  │
│                               │  │   Audio Buffer (16kHz)  │ │  │
│                               │  └───────────┬─────────────┘ │  │
│                               │              │               │  │
│                               │              ▼               │  │
│                               │  ┌─────────────────────────┐ │  │
│                               │  │   WhisperTranscriber    │ │  │
│                               │  └───────────┬─────────────┘ │  │
│                               │              │               │  │
│                               └──────────────┼───────────────┘  │
│                                              │                  │
│                                              ▼                  │
│                               ┌──────────────────────────────┐  │
│                               │    TranscriptionBuffer       │  │
│                               │    (UI Display)              │  │
│                               └──────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Simulated Audio Source Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌──────────────┐
│ Audio File  │────►│   Symphonia  │────►│   Rubato    │────►│ Audio Chunks │
│ (any format)│     │   Decoder    │     │  Resampler  │     │ (16kHz mono) │
└─────────────┘     └──────────────┘     └─────────────┘     └──────┬───────┘
                                                                     │
                    ┌────────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Timing Controller                                │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Chunk Size: ~100ms of audio (1600 samples at 16kHz)            │   │
│  │  Sleep Duration: chunk_duration / speed_multiplier               │   │
│  │  This simulates real-time audio capture                          │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                    │
                    ▼
          ┌─────────────────┐
          │ mpsc::channel   │────► TranscriptionManager
          │ (audio_tx)      │
          └─────────────────┘
```

---

## Test Plan

### Unit Tests

| Test | Input | Expected Output |
|------|-------|-----------------|
| `test_load_wav` | Valid WAV file | Successfully loaded, correct duration |
| `test_load_mp3` | Valid MP3 file | Successfully loaded, correct duration |
| `test_load_invalid` | Non-audio file | `AudioSimError::LoadError` |
| `test_resample_48k_to_16k` | 48kHz audio | 16kHz output, correct sample count |
| `test_level_calculation` | Known audio | Accurate RMS level |
| `test_loop_mode` | Short audio, loop=true | Repeats after EOF |
| `test_speed_multiplier` | Audio, speed=2.0 | Completes in half time |

### Integration Tests

| Test | Scenario | Pass Criteria |
|------|----------|---------------|
| `test_simulated_transcription` | Load test_speech.wav, run transcription | Produces non-empty text |
| `test_tui_simulated_mode` | Start TUI with --simulated-audio | TUI shows "Simulated" device |
| `test_level_meter_simulated` | Play audio through simulator | Level meter responds |

### Manual Tests

| Test | Procedure | Pass Criteria |
|------|-----------|---------------|
| Android TUI launch | `./lexicon tui --simulated-audio test.wav` on Cuttlefish | TUI opens, no crash |
| Transcription on Android | Start recording with simulated audio | Text appears in buffer |
| Loop playback | Enable loop mode, wait for audio to repeat | Audio loops seamlessly |

---

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Timing drift in simulation | Medium | Medium | Use precise sleep with drift correction |
| Large audio files exhaust memory | Low | Low | Stream audio, don't load entirely into RAM |
| Resampling artifacts | Low | Low | Use high-quality rubato resampler |
| Thread synchronization issues | Medium | Low | Use atomic operations and channels |

---

## Implementation Notes

### Task Breakdown

| Task | Priority | Effort | Notes |
|------|----------|--------|-------|
| 1. Create `audio_sim.rs` module skeleton | High | S | Basic structure |
| 2. Implement audio file loading (WAV) | High | M | Use hound for simplicity |
| 3. Implement audio file loading (symphonia) | High | M | MP3, FLAC, OGG support |
| 4. Implement resampling to 16kHz | High | M | Use rubato |
| 5. Implement real-time streaming | High | M | Timing controller |
| 6. Implement level meter support | Medium | S | RMS calculation |
| 7. Add CLI flags | High | S | --simulated-audio, --simulated-loop |
| 8. Integrate with TranscriptionManager | High | M | Conditional audio source |
| 9. Create test audio fixtures | Medium | S | Short speech samples |
| 10. Write unit tests | Medium | M | Cover all major paths |
| 11. Write integration test | Medium | M | End-to-end verification |
| 12. Test on Android emulator | High | M | Verify on Cuttlefish |

### Key Implementation Details

1. **No cpal dependency in simulation path**: The `SimulatedAudioSource` must not import or use cpal at all, ensuring it works on Android.

2. **Chunk-based streaming**: Send audio in ~100ms chunks (1600 samples at 16kHz) to match the existing transcription pipeline's expectations.

3. **Timing accuracy**: Use `std::thread::sleep` with drift compensation to maintain accurate real-time playback speed.

4. **Graceful EOF handling**: When audio ends without loop mode, send an empty chunk to signal completion rather than dropping the channel.

5. **Test fixtures**: Use short (5-10 second) audio clips with clear speech for faster tests. Include both speech and silence samples.

---

## Architecture References

- WP-TUI - TUI architecture and audio integration points
- Existing `transcription.rs` - Current audio pipeline implementation

---

## Implementation Progress

### ✅ Completed (2026-01-27)

1. **Created `audio_sim.rs` module** (`crates/lexicon-cli/src/tui/audio_sim.rs`)
   - `SimulatedAudioConfig` struct with file_path, loop_playback, speed
   - `SimulatedAudioSource` struct with streaming, level monitoring, position tracking
   - `AudioSimError` enum for error handling
   - Audio loading via symphonia (WAV, MP3, FLAC, OGG support)
   - Resampling to 16kHz mono via rubato
   - Real-time streaming with drift-corrected pacing
   - RMS level calculation for meter display
   - Unit tests for RMS calculation, config, and WAV loading

2. **Added CLI flags** (`crates/lexicon-cli/src/main.rs`)
   - `--simulated-audio <path>` - Path to audio file for simulation mode
   - `--simulated-loop` - Enable loop playback
   - Available on both `tui` and `diag` commands

3. **Updated TuiConfig** (`crates/lexicon-cli/src/tui/app.rs`)
   - Added `simulated_audio: Option<PathBuf>`
   - Added `simulated_loop: bool`

4. **Integrated with TranscriptionManager** (`crates/lexicon-cli/src/tui/transcription.rs`)
   - Added `simulated_source: Option<SimulatedAudioSource>` field
   - Updated `start()` to use simulated audio when configured
   - `get_level()` returns simulated source level when active
   - `shutdown()` stops simulated source
   - Android platform support via simulated audio

5. **Added dependencies** (`crates/lexicon-cli/Cargo.toml`)
   - `symphonia` for audio file decoding
   - `rubato` for sample rate conversion

### 🔄 Remaining (Testing)

- [x] Test audio fixtures - using existing `tests/fixtures/audio/` files
- [x] Add `test_load_fixture_wav()` - verifies woodchuck.wav loads
- [x] Add `test_load_fixture_mp3()` - verifies dolly.mp3 loads
- [x] Add `test_streaming_sends_audio_chunks()` - verifies audio streaming works
- [x] E2E tests for simulated audio - fixture validation, CLI flag verification
- [ ] E2E transcription test with simulated audio (requires Whisper model)
- [ ] Manual test on Android emulator (Cuttlefish)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-27 | Initial work plan created |
| 2026-01-27 | Core implementation complete - audio_sim module, CLI flags, TUI integration |
| 2026-01-27 | Fixture tests added - WAV and MP3 loading verified |
| 2026-01-27 | Streaming test added - audio channel transmission verified |
| 2026-01-27 | E2E tests added - fixture validation, CLI flag verification |

