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

#### RUN WITHOUT MOUSE SUPPORT

**Important:** Always use `--no-mouse` to avoid terminal corruption bug on Cuttlefish.

```bash
adb shell "export HOME=/data/local/tmp && \
  export LEXICON_DATA_DIR=/data/local/tmp/lexicon-data && \
  mkdir -p /data/local/tmp/lexicon-data && \
  LD_LIBRARY_PATH=/data/local/tmp \
  /data/local/tmp/lexicon tui \
    --no-mouse \
    --model base \
    --simulated-audio /data/local/tmp/woodchuck.wav"
```

#### TRANSCRIBE TEST (bypasses TUI)

```bash
adb shell "export HOME=/data/local/tmp && \
  export LEXICON_DATA_DIR=/data/local/tmp/lexicon-data && \
  mkdir -p /data/local/tmp/lexicon-data && \
  LD_LIBRARY_PATH=/data/local/tmp \
  /data/local/tmp/lexicon transcribe /data/local/tmp/woodchuck.wav --model base"
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

## ARM64 Emulation (QEMU on x86_64)

This section covers testing ARM64 Android builds using QEMU TCG emulation on an x86_64 host.

> **Note:** For Android development and testing, use Android Studio instead. This QEMU approach is for advanced use cases only.

> **Documentation:** See [.dev/docs/android-emulation/09-arm64-qemu-emulation.md](../docs/android-emulation/09-arm64-qemu-emulation.md) for full setup instructions.

### Start ARM64 Emulator

```bash
qemu-system-aarch64 \
  -machine virt,gic-version=2 \
  -cpu max \
  -accel tcg,thread=multi \
  -m 4096 \
  -smp 4 \
  -bios ~/cuttlefish/etc/bootloader_aarch64/bootloader.qemu \
  -nographic \
  -drive file=~/cuttlefish-arm64/super.img,format=raw,if=virtio \
  -drive file=~/cuttlefish-arm64/boot.img,format=raw,if=virtio \
  -netdev user,id=net0,hostfwd=tcp::5555-:5555 \
  -device virtio-net-pci,netdev=net0 \
  &
```

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

```bash
# Connect to device
adb connect localhost:5555

# Push ARM64 binary
adb push target/aarch64-linux-android/release/lexicon /data/local/tmp/lexicon

# Push ARM64 libc++ (if built)
adb push dist/android/arm64/libc++_shared.so /data/local/tmp/

# Push model
adb push models/ggml-base.bin /data/local/tmp/

# Push test audio
adb push tests/fixtures/audio/woodchuck.wav /data/local/tmp/

# Make executable
adb shell chmod +x /data/local/tmp/lexicon
```

### Setup Models Directory (ARM64)

```bash
adb shell "mkdir -p /data/local/tmp/models && cp /data/local/tmp/ggml-base.bin /data/local/tmp/models/"
```

### Run TUI Test (ARM64)

```bash
adb shell "export HOME=/data/local/tmp && \
  export LEXICON_DATA_DIR=/data/local/tmp/lexicon-data && \
  mkdir -p /data/local/tmp/lexicon-data && \
  LD_LIBRARY_PATH=/data/local/tmp \
  /data/local/tmp/lexicon tui \
    --no-mouse \
    --model base \
    --simulated-audio /data/local/tmp/woodchuck.wav"
```

### Transcribe Test (ARM64)

```bash
adb shell "export HOME=/data/local/tmp && \
  export LEXICON_DATA_DIR=/data/local/tmp/lexicon-data && \
  mkdir -p /data/local/tmp/lexicon-data && \
  LD_LIBRARY_PATH=/data/local/tmp \
  /data/local/tmp/lexicon transcribe /data/local/tmp/woodchuck.wav --model base"
```

### Stop ARM64 Emulator

```bash
# Find PID
ps aux | grep qemu-system-aarch64

# Kill by PID
kill -9 <PID>

# Or kill all QEMU
pkill -f qemu-system-aarch64
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
