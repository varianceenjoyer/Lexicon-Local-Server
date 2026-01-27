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

adb push "/home/dev/projects/Lexicon-Local-Server/target/x86_64-linux-android/release/lexicon" /data/local/tmp/lexicon
adb push dist/android/x86_64/libc++_shared.so /data/local/tmp/
adb push models/ggml-base.bin /data/local/tmp/
adb push tests/fixtures/audio/woodchuck.wav /data/local/tmp/
adb shell chmod +x /data/local/tmp/lexicon

#### RUN WITHOUT MOUSE SUPPORT

**Important:** Always use `--no-mouse` to avoid terminal corruption bug on Cuttlefish.

```bash
mkdir -p /data/local/tmp/lexicon-data
LD_LIBRARY_PATH=/data/local/tmp \
LEXICON_DATA_DIR=/data/local/tmp/lexicon-data \
  /data/local/tmp/lexicon tui \
  --no-mouse \
  --model /data/local/tmp/ggml-base.bin \
  --simulated-audio /data/local/tmp/woodchuck.wav
```

### Known Issues

1. **Mouse tracking bug**: Without `--no-mouse`, mouse escape sequences flood the terminal after exit
2. **pthread crash**: Whisper inference crashes with `invalid pthread_t passed to pthread_join` on x86_64 emulator - works on real ARM64 devices
3. **Performance**: Whisper is very slow on emulator (CPU-only, emulated x86_64)
