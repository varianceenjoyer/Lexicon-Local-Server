#!/bin/bash
# build-android-stt.sh
# Cross-compile Lexicon WITH STT for Android using pre-built whisper.cpp
#
# Usage: ./build-android-stt.sh [target] [--release]
#   target: arm64 (default), x86_64

set -e

# Configuration
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-/home/dev/android-ndk-r26d}"
export ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-26}"
WHISPER_ROOT="${WHISPER_ROOT:-$HOME/whisper.cpp}"

# Map friendly names to Rust targets
declare -A TARGETS=(
    ["arm64"]="aarch64-linux-android"
    ["x86_64"]="x86_64-linux-android"
)

declare -A ABIS=(
    ["arm64"]="arm64-v8a"
    ["x86_64"]="x86_64"
)

# Parse arguments
TARGET_NAME="${1:-arm64}"
BUILD_MODE="debug"
EXTRA_ARGS=""

shift || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_MODE="release"
            EXTRA_ARGS="--release"
            ;;
        *)
            EXTRA_ARGS="$EXTRA_ARGS $1"
            ;;
    esac
    shift
done

RUST_TARGET="${TARGETS[$TARGET_NAME]}"
ANDROID_ABI="${ABIS[$TARGET_NAME]}"

if [ -z "$RUST_TARGET" ]; then
    echo "Unknown target: $TARGET_NAME"
    echo "Available targets: ${!TARGETS[*]}"
    exit 1
fi

echo "=== Lexicon Android Build (with STT) ==="
echo "Target: $TARGET_NAME ($RUST_TARGET)"
echo "Mode: $BUILD_MODE"
echo "NDK: $ANDROID_NDK_HOME"
echo "API Level: $ANDROID_API_LEVEL"
echo "Whisper root: $WHISPER_ROOT"
echo ""

# Verify NDK
if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "ERROR: Android NDK not found at $ANDROID_NDK_HOME"
    exit 1
fi

# Verify whisper.cpp pre-built libraries
WHISPER_BUILD="$WHISPER_ROOT/build-android-$TARGET_NAME"
if [ ! -f "$WHISPER_BUILD/src/libwhisper.a" ]; then
    echo "ERROR: Pre-built whisper.cpp not found at $WHISPER_BUILD"
    echo "Please build whisper.cpp for Android first:"
    echo "  cd $WHISPER_ROOT"
    echo "  mkdir -p build-android-$TARGET_NAME && cd build-android-$TARGET_NAME"
    echo "  cmake .. -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake \\"
    echo "    -DANDROID_ABI=$ANDROID_ABI -DANDROID_PLATFORM=android-$ANDROID_API_LEVEL \\"
    echo "    -DBUILD_SHARED_LIBS=OFF -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=OFF"
    echo "  make -j$(nproc)"
    exit 1
fi

echo "Found pre-built whisper.cpp at $WHISPER_BUILD"

# Check if target is installed
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
    echo "Installing Rust target: $RUST_TARGET"
    rustup target add "$RUST_TARGET"
fi

# Set up environment for whisper-rs-sys
# Skip bindgen (use pre-generated bindings)
export WHISPER_DONT_GENERATE_BINDINGS=1

# Point to pre-built whisper libraries
export WHISPER_LIB_DIR="$WHISPER_BUILD/src"
export GGML_LIB_DIR="$WHISPER_BUILD/ggml/src"

# Disable whisper-rs-sys CMake build by providing library paths
# (This requires patching whisper-rs-sys or using build overrides)

# For now, set CMAKE vars that cargo-ndk needs
export CMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake"
export ANDROID_ABI="$ANDROID_ABI"
export ANDROID_PLATFORM="android-$ANDROID_API_LEVEL"

# Build with cargo-ndk
echo "Building with STT feature..."
cargo ndk -t "$ANDROID_ABI" build -p lexicon-ffi --features stt $EXTRA_ARGS 2>&1 || {
    echo ""
    echo "Build failed. This is expected - whisper-rs-sys doesn't support pre-built libraries."
    echo "A patch to whisper-rs-sys or a custom build.rs is needed."
    exit 1
}

echo ""
echo "=== Build Complete ==="
