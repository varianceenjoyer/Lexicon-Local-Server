#!/bin/bash
# build-android.sh
# Cross-compile Lexicon for Android targets
#
# Usage: ./build-android.sh [target] [--release] [--stt]
#   target: arm64 (default), x86_64, arm32, x86
#   --release: Build in release mode
#   --stt: Include STT (speech-to-text) feature
#   --no-default-features: Build without any features

set -e

# Configuration
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-/home/dev/android-ndk-r26d}"
export ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-26}"  # Android 8.0+

# Map friendly names to Rust targets
declare -A TARGETS=(
    ["arm64"]="aarch64-linux-android"
    ["x86_64"]="x86_64-linux-android"
    ["arm32"]="armv7-linux-androideabi"
    ["x86"]="i686-linux-android"
)

# Parse arguments
TARGET_NAME="${1:-arm64}"
BUILD_MODE="debug"
RELEASE_FLAG=""
FEATURES=""
NO_DEFAULT_FEATURES="--no-default-features"

shift || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_MODE="release"
            RELEASE_FLAG="--release"
            ;;
        --stt)
            FEATURES="stt"
            ;;
        --no-default-features)
            NO_DEFAULT_FEATURES="--no-default-features"
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
    shift
done

RUST_TARGET="${TARGETS[$TARGET_NAME]}"
if [ -z "$RUST_TARGET" ]; then
    echo "Unknown target: $TARGET_NAME"
    echo "Available targets: ${!TARGETS[*]}"
    exit 1
fi

echo "=== Lexicon Android Build ==="
echo "Target: $TARGET_NAME ($RUST_TARGET)"
echo "Mode: $BUILD_MODE"
echo "Features: ${FEATURES:-none}"
echo "NDK: $ANDROID_NDK_HOME"
echo "API Level: $ANDROID_API_LEVEL"
echo ""

# Verify NDK
if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "ERROR: Android NDK not found at $ANDROID_NDK_HOME"
    echo "Set ANDROID_NDK_HOME or install NDK to default location"
    exit 1
fi

# Map target to Android ABI
ANDROID_ABI=$(case $TARGET_NAME in
    arm64) echo "arm64-v8a" ;;
    x86_64) echo "x86_64" ;;
    arm32) echo "armeabi-v7a" ;;
    x86) echo "x86" ;;
esac)

# Clear conflicting env vars
unset CFLAGS
unset CXXFLAGS
unset CC
unset CXX

# Set proper Android toolchain for cmake
# These are picked up by whisper-rs-sys build.rs (patched to include ANDROID_*)
export CMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake"
export ANDROID_ABI="$ANDROID_ABI"
export ANDROID_PLATFORM="android-$ANDROID_API_LEVEL"

# Skip bindgen regeneration (use pre-generated bindings)
export WHISPER_DONT_GENERATE_BINDINGS=1

# whisper.cpp build options
export WHISPER_BUILD_TESTS="OFF"
export WHISPER_BUILD_EXAMPLES="OFF"

echo "CMake toolchain: $CMAKE_TOOLCHAIN_FILE"
echo "Android ABI: $ANDROID_ABI"
echo "Android Platform: $ANDROID_PLATFORM"
echo ""

# Check if target is installed
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
    echo "Installing Rust target: $RUST_TARGET"
    rustup target add "$RUST_TARGET"
fi

# Build command
BUILD_CMD="cargo ndk -t $ANDROID_ABI build -p lexicon-ffi $NO_DEFAULT_FEATURES"
if [ -n "$FEATURES" ]; then
    BUILD_CMD="$BUILD_CMD --features $FEATURES"
fi
if [ -n "$RELEASE_FLAG" ]; then
    BUILD_CMD="$BUILD_CMD $RELEASE_FLAG"
fi

echo "Building..."
echo "Command: $BUILD_CMD"
echo ""

eval $BUILD_CMD

# Output location
OUTPUT_DIR="target/$RUST_TARGET/$BUILD_MODE"
echo ""
echo "=== Build Complete ==="
echo "Output: $OUTPUT_DIR/liblexicon_ffi.so"

if [ -f "$OUTPUT_DIR/liblexicon_ffi.so" ]; then
    ls -lh "$OUTPUT_DIR/liblexicon_ffi.so"
    # Show file type if 'file' command is available
    if command -v file &> /dev/null; then
        file "$OUTPUT_DIR/liblexicon_ffi.so"
    fi

    # Note about libc++_shared.so for STT builds
    if [ "$FEATURES" = "stt" ]; then
        echo ""
        echo "NOTE: STT build requires libc++_shared.so to be bundled with the library."
        echo "Copy from: \$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/<arch>/libc++_shared.so"
    fi
else
    echo "WARNING: Library not found at expected location"
    echo "Check build output for errors"
fi
