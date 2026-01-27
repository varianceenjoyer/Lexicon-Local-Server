#!/bin/bash
# build-android-patched.sh
# Build with single-thread patch for Cuttlefish testing
#
# !! TESTING ONLY - NOT FOR RELEASE !!
# This patches ggml to force single-threaded operation on Android
# to work around pthread issues in Cuttlefish x86_64 emulator.
#
# Usage: ./build-android-patched.sh [--release] [--restore]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Find whisper-rs-sys source
WHISPER_SYS_DIR=$(find ~/.cargo/registry/src -name "whisper-rs-sys-*" -type d 2>/dev/null | head -1)
GGML_CPU_FILE="$WHISPER_SYS_DIR/whisper.cpp/ggml/src/ggml-cpu/ggml-cpu.c"

if [ -z "$WHISPER_SYS_DIR" ] || [ ! -f "$GGML_CPU_FILE" ]; then
    echo "ERROR: whisper-rs-sys not found. Run 'cargo fetch' first."
    exit 1
fi

# Handle --restore flag
if [ "$1" = "--restore" ]; then
    if [ -f "$GGML_CPU_FILE.backup" ]; then
        cp "$GGML_CPU_FILE.backup" "$GGML_CPU_FILE"
        rm "$GGML_CPU_FILE.backup"
        echo "Restored ggml-cpu.c to original state"
    else
        echo "No backup found - file is already clean"
    fi
    exit 0
fi

echo "========================================"
echo "!! PATCHED BUILD - TESTING ONLY !!"
echo "!! NOT FOR RELEASE !!"
echo "========================================"
echo ""

# Parse arguments
BUILD_MODE="debug"
RELEASE_FLAG=""
if [ "$1" = "--release" ] || [ "$2" = "--release" ]; then
    BUILD_MODE="release"
    RELEASE_FLAG="--release"
fi

# Check if already patched
if grep -q "TESTING ONLY.*single-threaded" "$GGML_CPU_FILE"; then
    echo "Source already patched"
else
    echo "Applying single-thread patch to: $GGML_CPU_FILE"

    # Create backup
    cp "$GGML_CPU_FILE" "$GGML_CPU_FILE.backup"

    # Apply patch: insert single-thread check after function signature
    sed -i '/^static struct ggml_threadpool \* ggml_threadpool_new_impl(/,/struct ggml_threadpool \* threadpool =/{
        /struct ggml_threadpool \* threadpool =/i\
\
    // TESTING ONLY: Force single-threaded on Android (Cuttlefish workaround)\
#if defined(__ANDROID__)\
    if (tpp->n_threads > 1) {\
        fprintf(stderr, "GGML Android: forcing n_threads from %d to 1\\n", tpp->n_threads);\
        tpp->n_threads = 1;\
    }\
#endif\

    }' "$GGML_CPU_FILE"

    echo "Patch applied"
fi

# Clean build cache to force recompile
rm -rf "$PROJECT_ROOT/target/x86_64-linux-android/$BUILD_MODE/.fingerprint/whisper-rs-sys-"*
rm -rf "$PROJECT_ROOT/target/x86_64-linux-android/$BUILD_MODE/build/whisper-rs-sys-"*
rm -rf "$PROJECT_ROOT/target/x86_64-linux-android/$BUILD_MODE/deps/"*whisper*

# Build using the standard script
echo ""
echo "Building with patched source..."
bash "$SCRIPT_DIR/build-android-tui.sh" x86_64 $RELEASE_FLAG

echo ""
echo "========================================"
echo "!! PATCHED BUILD COMPLETE !!"
echo "!! Remember: This is for TESTING ONLY !!"
echo "!! Run '$0 --restore' to remove patch !!"
echo "========================================"
