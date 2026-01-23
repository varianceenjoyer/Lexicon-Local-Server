---
id: WP-TUI-WIN-RECORDING
type: work-plan
status: ready
phase: beta
created: 2026-01-23
updated: 2026-01-23
author: design-lead
consumers: [developer]
depends-on: [WP-TUI]
blocks: []
priority: high
tags: [tui, windows, bug, recording, stt]
---

# WP-TUI-WIN-RECORDING: Windows Ctrl+T Recording Bug Investigation

> **Phase**: Beta | **Priority**: HIGH

## Summary

When testing the TUI on Windows, pressing Ctrl+T to start recording immediately stops the recording without producing any transcription output. The audio level meter indicates that audio input is being received, but the recording toggle appears to fail silently. This prevents the TUI from being usable for transcription on Windows.

---

## Problem Description

### Observed Behavior
1. Launch TUI with `lexicon tui --model <path>`
2. Audio level meter shows activity (indicating audio input works)
3. Press Ctrl+T to start recording
4. Recording indicator briefly appears then immediately stops
5. No transcription output is produced
6. No visible error message

### Expected Behavior
1. Press Ctrl+T
2. Recording indicator shows "● REC"
3. Transcription text appears as user speaks
4. Recording continues until Ctrl+T is pressed again

---

## Investigation Areas

### 1. Transcription Manager Startup

The `TranscriptionManager::start()` method in [transcription.rs:138-169](crates/lexicon-cli/src/tui/transcription.rs) does several things that could fail on Windows:

```rust
pub fn start(&mut self, config: TranscriptionManagerConfig) -> Result<()> {
    // Create channel for audio samples
    let (audio_tx, audio_rx) = mpsc::unbounded_channel();
    self.audio_tx = Some(audio_tx);

    // Spawn worker thread
    self.worker_handle = Some(thread::spawn(move || {
        transcription_worker(/* ... */);
    }));

    // Start audio capture
    self.start_audio_capture(config.device_name)?;  // Could fail here
    Ok(())
}
```

**Possible issues:**
- Audio capture fails silently on Windows
- Worker thread panics during model loading
- Model path resolution differs on Windows

### 2. Model Loading in Worker Thread

The worker thread loads the Whisper model synchronously:

```rust
fn transcription_worker(/* ... */) {
    // Load model
    let mut transcriber = match WhisperTranscriber::new() {
        Ok(t) => t,
        Err(e) => {
            let _ = ui_tx.send(TranscriptionMessage::ModelError(/* ... */));
            return;  // Worker exits immediately on error
        }
    };

    if let Err(e) = transcriber.load_model(&model_path) {
        let _ = ui_tx.send(TranscriptionMessage::ModelError(/* ... */));
        return;  // Worker exits immediately on error
    }
    // ...
}
```

**Possible issues:**
- Windows path handling (backslashes vs forward slashes)
- Model file not found or inaccessible
- whisper.cpp DLL loading fails on Windows
- BLAS/OpenBLAS dependency issues

### 3. Audio Capture on Windows

The `start_audio_capture` method uses cpal:

```rust
fn start_audio_capture(&mut self, device_name: Option<String>) -> Result<()> {
    let host = cpal::default_host();  // WASAPI on Windows

    let device = if let Some(name) = device_name {
        // Find device by name - could fail if name format differs
    } else {
        host.default_input_device()  // Could return None
    };
    // ...
}
```

**Possible issues:**
- WASAPI exclusive mode conflicts
- Sample format mismatch (expecting f32, getting i16)
- Device name encoding issues

### 4. Toggle Recording Logic

The `toggle_recording` method in [app.rs:762-779](crates/lexicon-cli/src/tui/app.rs):

```rust
fn toggle_recording(&mut self) {
    self.recording = !self.recording;

    if self.recording {
        if let Some(ref trans) = self.transcription {
            trans.start_recording();  // Sets atomic bool
            self.set_status("Recording started");
        } else {
            self.set_status("Recording (no transcription model)");
        }
    } else {
        // Stop recording
        if let Some(ref trans) = self.transcription {
            trans.stop_recording();
        }
        self.set_status("Recording stopped");
    }
}
```

**Possible issues:**
- `self.transcription` is `None` (manager never initialized)
- Recording atomic flag set but worker thread already exited

---

## Diagnostic Steps

### Step 1: Add Logging
Add tracing/logging throughout the startup sequence to identify where failure occurs:

```rust
// In TuiApp::with_config
tracing::info!("Creating transcription manager...");
tracing::info!("Model path: {:?}", model_path);
tracing::info!("Model exists: {}", model_path.exists());

// In TranscriptionManager::start
tracing::info!("Starting transcription manager...");
tracing::info!("Spawning worker thread...");
tracing::info!("Starting audio capture...");

// In transcription_worker
tracing::info!("Worker thread started");
tracing::info!("Creating WhisperTranscriber...");
tracing::info!("Loading model: {:?}", model_path);
```

### Step 2: Run with RUST_LOG
```bash
set RUST_LOG=lexicon_cli=debug
lexicon tui --model models/ggml-base.en.bin
```

### Step 3: Check Model Loading Independently
Test the model loading outside the TUI:
```bash
lexicon transcribe --model models/ggml-base.en.bin test.wav
```

### Step 4: Check Audio Capture
Verify audio devices work:
```bash
lexicon tui  # Without --model, just test audio level meter
```

---

## Scope

### Included
- Investigate and identify root cause of Windows recording failure
- Add diagnostic logging to track startup sequence
- Fix the identified issue
- Verify fix on Windows

### Excluded
- Linux/macOS testing (already working per user report)
- Performance optimization
- Feature additions

---

## Acceptance Criteria

- [ ] Root cause identified and documented
- [ ] Fix implemented for Windows-specific issue
- [ ] Ctrl+T successfully starts recording on Windows
- [ ] Transcription text appears when speaking
- [ ] Ctrl+T successfully stops recording on Windows
- [ ] No regression on other platforms

---

## Files to Investigate/Modify

| File | Investigation |
|------|---------------|
| [crates/lexicon-cli/src/tui/app.rs](crates/lexicon-cli/src/tui/app.rs) | Transcription manager initialization, toggle logic |
| [crates/lexicon-cli/src/tui/transcription.rs](crates/lexicon-cli/src/tui/transcription.rs) | Worker thread, audio capture, model loading |
| [crates/lexicon-core/src/stt/whisper.rs](crates/lexicon-core/src/stt/whisper.rs) | WhisperTranscriber implementation |

---

## Potential Fixes

Depending on root cause:

1. **Model path issue**: Ensure proper Windows path handling with `PathBuf`
2. **Worker thread panic**: Add `catch_unwind` around worker, report errors properly
3. **Audio capture failure**: Improve error handling, try different sample formats
4. **DLL loading**: Ensure whisper.cpp dependencies are bundled/available on Windows
5. **Async timing**: Add delay or synchronization for worker thread startup

---

## Test Plan

### Manual Testing (Windows)

| Test | Procedure | Pass Criteria |
|------|-----------|---------------|
| Basic recording | Launch TUI, press Ctrl+T, speak, press Ctrl+T | Transcription appears |
| Model loading | Watch logs during startup | "Model loaded: X" message appears |
| Error display | Launch with invalid model path | Clear error message displayed |
| Level meter during record | Press Ctrl+T, speak | Level meter shows activity |

---

## Dependencies

- WP-TUI (complete) - Base TUI implementation
- Windows build environment for testing

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work plan created based on Windows build testing |
