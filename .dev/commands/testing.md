# Testing Commands

## Block-based Transcription

cd ~/projects/Lexicon-Local-Server/target/debug && ./lexicon transcribe ~/projects/Lexicon-Local-Server/tests/fixtures/audio/woodchuck.wav

## Android Emulator (Cuttlefish)

### Connect to Shell

docker-compose -f ~/projects/Lexicon-Local-Server/scripts/cuttlefish/docker/docker-compose.yml exec cuttlefish docker-entrypoint.sh adb shell

### Deploy Script (Recommended)

```bash
# Build and push all dependencies
./scripts/android/deploy-cuttlefish.sh

# Build, push, and run TUI test
./scripts/android/deploy-cuttlefish.sh --run

# Push only (skip rebuild)
./scripts/android/deploy-cuttlefish.sh --push-only

# Use tiny model (faster on emulator)
./scripts/android/deploy-cuttlefish.sh --tiny --run
```

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
2. **pthread crash (x86_64 only)**: Whisper initialization crashes with `invalid pthread_t passed to pthread_join` on Cuttlefish x86_64 emulator. This is a compatibility issue between ggml/whisper.cpp threading and Cuttlefish's bionic libc emulation. **This does NOT affect real ARM64 devices or ARM64 emulators on ARM hosts.**
3. **Performance**: Whisper is very slow on emulator (CPU-only, emulated x86_64)

### Threading Patch (TESTING ONLY)

A single-thread patch is available for Cuttlefish testing:

```bash
# Build with single-thread patch
./scripts/android/build-android-patched.sh --release

# Restore original source after testing
./scripts/android/build-android-patched.sh --restore
```

**Note:** This patch has not been verified to fix the pthread crash. The issue appears to occur during whisper initialization before the patched code path is reached. Further investigation needed.
