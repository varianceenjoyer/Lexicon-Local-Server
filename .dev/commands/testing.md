# Testing Commands

## Block-based Transcription

cd ~/projects/Lexicon-Local-Server/target/debug && ./lexicon transcribe ~/projects/Lexicon-Local-Server/tests/fixtures/audio/woodchuck.wav

## Android Emulator

docker-compose -f ~/projects/Lexicon-Local-Server/scripts/cuttlefish/docker/docker-compose.yml exec cuttlefish docker-entrypoint.sh adb shell
