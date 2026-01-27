#!/bin/bash
# deploy-cuttlefish.sh
# Build and deploy Lexicon to Cuttlefish emulator for testing
#
# Usage:
#   ./deploy-cuttlefish.sh              # Build and push all dependencies
#   ./deploy-cuttlefish.sh --run        # Build, push, and run TUI test
#   ./deploy-cuttlefish.sh --push-only  # Skip build, just push existing artifacts
#
# Prerequisites:
#   - Cuttlefish running and accessible via adb
#   - Android NDK installed
#   - Models downloaded (ggml-base.bin or ggml-tiny.bin)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Parse arguments
DO_BUILD=true
DO_RUN=false
MODEL="ggml-base.bin"

while [[ $# -gt 0 ]]; do
    case $1 in
        --run|-r)
            DO_RUN=true
            shift
            ;;
        --push-only|-p)
            DO_BUILD=false
            shift
            ;;
        --tiny)
            MODEL="ggml-tiny.bin"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --run, -r       Run TUI test after deployment"
            echo "  --push-only, -p Skip build, just push existing artifacts"
            echo "  --tiny          Use whisper-tiny model instead of base"
            echo "  --help, -h      Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=== Lexicon Cuttlefish Deployment ==="
echo ""

# Check adb connectivity
echo "Checking Cuttlefish connection..."
if ! adb devices | grep -q "device$"; then
    echo "ERROR: No device connected via adb"
    echo ""
    echo "Make sure Cuttlefish is running:"
    echo "  cd ~/cuttlefish && HOME=\$PWD ./bin/launch_cvd --daemon"
    echo ""
    echo "Then connect:"
    echo "  cd ~/cuttlefish && HOME=\$PWD ./bin/adb connect localhost:6520"
    exit 1
fi
echo "Device connected"
echo ""

# Build if requested
if $DO_BUILD; then
    echo "=== Building for x86_64 (Cuttlefish) ==="
    echo ""
    bash "$SCRIPT_DIR/build-android-tui.sh" x86_64 --release
    echo ""
fi

# Verify artifacts exist
BINARY="$PROJECT_DIR/target/x86_64-linux-android/release/lexicon"
LIBCPP="$PROJECT_DIR/dist/android/x86_64/libc++_shared.so"
MODEL_FILE="$PROJECT_DIR/models/$MODEL"
AUDIO_FILE="$PROJECT_DIR/tests/fixtures/audio/woodchuck.wav"

echo "=== Checking Artifacts ==="

check_file() {
    if [ -f "$1" ]; then
        echo "  ✓ $1"
        return 0
    else
        echo "  ✗ $1 (MISSING)"
        return 1
    fi
}

MISSING=false
check_file "$BINARY" || MISSING=true
check_file "$LIBCPP" || MISSING=true
check_file "$MODEL_FILE" || MISSING=true
check_file "$AUDIO_FILE" || MISSING=true

if $MISSING; then
    echo ""
    echo "ERROR: Missing required files"
    echo ""
    echo "To fix:"
    echo "  - Binary: Run build-android-tui.sh x86_64 --release"
    echo "  - libc++: Run build-android.sh x86_64 --release --stt (creates dist/)"
    echo "  - Model: curl -L -o models/$MODEL https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$MODEL"
    exit 1
fi
echo ""

# Push to device
echo "=== Pushing to Cuttlefish ==="
echo ""

echo "Pushing lexicon binary..."
adb push "$BINARY" /data/local/tmp/lexicon
adb shell chmod +x /data/local/tmp/lexicon

echo "Pushing libc++_shared.so..."
adb push "$LIBCPP" /data/local/tmp/

echo "Pushing model ($MODEL)..."
adb push "$MODEL_FILE" /data/local/tmp/

echo "Pushing test audio..."
adb push "$AUDIO_FILE" /data/local/tmp/

echo ""
echo "=== Deployment Complete ==="
echo ""

# Show what was deployed
adb shell "ls -lh /data/local/tmp/lexicon /data/local/tmp/libc++_shared.so /data/local/tmp/*.bin /data/local/tmp/*.wav 2>/dev/null" || true

echo ""

# Run if requested
if $DO_RUN; then
    echo "=== Running TUI Test ==="
    echo ""
    echo "Press Ctrl+C to exit"
    echo ""

    # Run with proper environment and --no-mouse to avoid output bug
    adb shell "cd /data/local/tmp && \
        mkdir -p /data/local/tmp/lexicon-data && \
        LD_LIBRARY_PATH=/data/local/tmp \
        LEXICON_DATA_DIR=/data/local/tmp/lexicon-data \
        /data/local/tmp/lexicon tui \
        --no-mouse \
        --model /data/local/tmp/$MODEL \
        --simulated-audio /data/local/tmp/woodchuck.wav"
else
    echo "To run manually on device:"
    echo ""
    echo "  adb shell"
    echo "  mkdir -p /data/local/tmp/lexicon-data"
    echo "  LD_LIBRARY_PATH=/data/local/tmp \\"
    echo "  LEXICON_DATA_DIR=/data/local/tmp/lexicon-data \\"
    echo "    /data/local/tmp/lexicon tui \\"
    echo "    --no-mouse \\"
    echo "    --model /data/local/tmp/$MODEL \\"
    echo "    --simulated-audio /data/local/tmp/woodchuck.wav"
fi
