# Testing Commands

## Block-based Transcription

cd ~/projects/Lexicon-Local-Server/target/debug && ./lexicon transcribe ~/projects/Lexicon-Local-Server/tests/fixtures/audio/woodchuck.wav

## Android Testing (Android Studio)

Use Android Studio for Android development and testing.

### Manual Steps

#### REBUILD

bash scripts/android/build-android-tui.sh x86_64 --release

#### PUSH BUILD + deps

```bash
adb push target/x86_64-linux-android/release/lexicon /data/local/tmp/lexicon
adb push dist/android/x86_64/libc++_shared.so /data/local/tmp/
adb push models/ggml-base.bin /data/local/tmp/
adb push tests/fixtures/audio/woodchuck.wav /data/local/tmp/
adb shell chmod +x /data/local/tmp/lexicon
```

#### SETUP MODELS DIRECTORY

For bundled model detection to work, models should be in a `models/` directory next to the binary:

```bash
adb shell "mkdir -p /data/local/tmp/models && cp /data/local/tmp/ggml-base.bin /data/local/tmp/models/"
```

### Environment Variables

These must be set inside the adb shell:

| Variable | Purpose | Example Value |
|----------|---------|---------------|
| `HOME` | User home directory (for database path) | `/data/local/tmp` |
| `LEXICON_DATA_DIR` | Data directory override | `/data/local/tmp/lexicon-data` |
| `LD_LIBRARY_PATH` | Library search path (for libc++_shared.so) | `/data/local/tmp` |

### Known Issues

1. **Mouse tracking bug**: Without `--no-mouse`, mouse escape sequences flood the terminal after exit
2. **Performance**: Whisper is slow on emulator (CPU-only)

---


### Wait for Boot

```bash
adb connect localhost:5555
adb devices
```

**Expected boot time:** 5-30 minutes (TCG software emulation is slow)

### Build for ARM64

```bash
# Build ARM64 release binary
bash scripts/android/build-android-tui.sh arm64 --release

# Verify target
file target/aarch64-linux-android/release/lexicon
# Expected: ELF 64-bit LSB pie executable, ARM aarch64
```

### Push ARM64 Build + Dependencies


```

### ARM64 Build Quick Reference

| Item | Value |
|------|-------|
| **Binary path** | `target/aarch64-linux-android/release/lexicon` |
| **libc++ path** | `dist/android/arm64/libc++_shared.so` |
| **Build command** | `build-android-tui.sh arm64 --release` |

### ARM64 QEMU Known Issues

1. **Very slow boot**: TCG emulation is 10-100x slower than native. Be patient.
2. **High CPU usage**: 100% CPU is normal during emulation - all ARM instructions are translated in software.
3. **Graphics unavailable**: Use `-nographic` and access via ADB only.
