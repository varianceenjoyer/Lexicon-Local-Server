# Human Intervention Required

> Last Updated: 2026-01-25 by design-lead (TUI verification complete)
>
> This file lists all tasks requiring human intervention. Complete these to unblock Beta release.

---

## ~~Priority 1: Security (WP-HASH-VERIFY)~~ ✅ COMPLETE

**Task**: ~~Acquire SHA-256 hashes for model files~~ **DONE (2026-01-25)**

All model hashes have been acquired and verified. The following hashes are now in `crates/lexicon-core/src/models/hashes.rs`:

| Model | Status |
|-------|--------|
| whisper-tiny | ✅ Verified |
| whisper-tiny-q5_1 | ✅ Verified |
| whisper-base | ✅ Verified |
| whisper-base-q5_1 | ✅ Verified |
| whisper-small | ✅ Verified |
| whisper-small-q5_1 | ✅ Verified |
| whisper-medium | ✅ Verified |
| whisper-large-v3 | ✅ Verified |
| whisper-large-v3-turbo | ✅ Verified |
| kokoro-tts | ✅ Verified |

**Remaining**: Integration of `ModelHashRegistry` into download flow (see [WP-HASH-VERIFY](work/WP-HASH-VERIFY.md) Phase 3)

**Related**: [HO-HASH-001](handoffs/HO-HASH-001.md)

---

## ~~Priority 2: TUI Hardware Test (WP-TUI)~~ ✅ COMPLETE

**Task**: ~~Test diagnostic TUI with actual audio hardware and Whisper model~~ **DONE (2026-01-25)**

TUI functionality verified working to MVP spec on **Windows** and **bash** builds.

### Verified:
   - [x] Audio device selector shows available devices
   - [x] Level meter responds to audio input
   - [x] Recording toggle (Ctrl+T) starts/stops capture
   - [x] Transcription appears in text area
   - [x] Save (Ctrl+S) stores to database
   - [x] Saved list (Ctrl+L) shows stored transcriptions

**Related**: [HO-TUI-001](handoffs/HO-TUI-001.md)

---

## ~~Priority 3: TTS Quality Evaluation (WP-TTS-QUALITY)~~ ✅ COMPLETE

**Task**: ~~Listen to generated TTS audio and rate quality~~ **DONE (2026-01-25)**

### Evaluation Result

| Metric | Result | Original Criterion |
|--------|--------|-------------------|
| Average MOS | **3.0** | >= 3.5 |

**Assessment**: Sufficient for understanding general text, but clearly synthetic.

**Decision**: Ship with current quality as known limitation. TTS improvement deferred to Release phase.

**Documentation**: [docs/TTS-QUALITY.md](../docs/TTS-QUALITY.md)

**Related**: [HO-TTS-QUALITY-001](handoffs/HO-TTS-QUALITY-001.md), [WP-TTS-QUALITY](work/WP-TTS-QUALITY.md)

---

## Priority 4: Bluetooth Hardware Test (WP-BLE)

**Task**: Test Bluetooth audio with physical headsets

### Requirements

- Bluetooth headset with HFP profile (AirPods, Sony WH-*, Bose, etc.)
- macOS, Windows, or Linux system with Bluetooth

### Steps

1. Connect Bluetooth headset to computer

2. Run diagnostic:
   ```bash
   lexicon tui
   # Select Bluetooth device in device dropdown
   ```

3. Verify:
   - [ ] Bluetooth device appears in device list
   - [ ] Audio capture works from Bluetooth mic
   - [ ] Sample rate detected (check logs for 8kHz vs 16kHz)
   - [ ] Transcription accuracy is acceptable

4. Test with 2+ different Bluetooth headsets if available

5. Document results including:
   - Device model
   - Detected sample rate
   - Transcription quality notes

**Related**: [HO-BLE-001](handoffs/HO-BLE-001.md)

---

## Native Development Required

### iOS Development (Release Phase)

- macOS with Xcode required
- Apple Developer account for TestFlight
- iOS device for on-device testing

### Android Development (Release Phase - Major Progress)

**Completed:**
- Rust Android targets (aarch64, x86_64) installed
- cargo-ndk 4.1.2 installed
- Android NDK r26d at `~/android-ndk-r26d`
- whisper.cpp pre-built for Android ARM64/x86_64
- **STT working** via patched whisper-rs-sys
- **TTS working** via ONNX Runtime AAR from Maven Central
- **Cuttlefish emulator tested** (API 33) - FFI + Synthesizer working
- Android project structure at `android/lexicon-android/`
- Build scripts: `scripts/android/build-android.sh --tts`

**Remaining work:**
- Physical device testing (thermal behavior, real-world performance)
- AAR package creation with Gradle
- Example Android app

See `.dev/docs/07-setup-process/` and [WP-ANDROID](work/WP-ANDROID.md) for details.

---

## Hardware Dependencies

| Hardware | Purpose | Work Package |
|----------|---------|--------------|
| Audio microphone | TUI testing | WP-TUI |
| Bluetooth headset | HFP validation | WP-BLE |
| iOS device | Mobile testing | WP-IOS |
| Android device | Mobile testing | WP-ANDROID |

---

*After completing any task, update the corresponding handoff document status and mark checkboxes.*
