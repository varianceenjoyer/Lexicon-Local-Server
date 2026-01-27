# Testing Commands

## Block-based Transcription

cd ~/projects/Lexicon-Local-Server/target/debug && ./lexicon transcribe ~/projects/Lexicon-Local-Server/tests/fixtures/audio/woodchuck.wav

## Android Emulator

docker-compose -f ~/projects/Lexicon-Local-Server/scripts/cuttlefish/docker/docker-compose.yml exec cuttlefish docker-entrypoint.sh adb shell

### REBUILD

bash scripts/android/build-android-tui.sh x86_64 --release

### PUSH BUILD + deps

adb push "/home/dev/projects/Lexicon-Local-Server/target/x86_64-linux-android/release/lexicon" data/local/tmp/lexicon
adb push dist/android/x86_64/libc++_shared.so /data/local/tmp/
adb push models/ggml-base.bin /data/local/tmp/
adb push tests/fixtures/audio/woodchuck.wav /data/local/tmp/
adb shell chmod +x /data/local/tmp/lexicon

### RUN WITHOUT MOUSE SUPPORT

mkdir -p /data/local/tmp/lexicon-data
LD_LIBRARY_PATH=/data/local/tmp \
LEXICON_DATA_DIR=/data/local/tmp/lexicon-data \
  /data/local/tmp/lexicon tui \
  --no-mouse \
  --model /data/local/tmp/ggml-base.bin \
  --simulated-audio /data/local/tmp/woodchuck.wav