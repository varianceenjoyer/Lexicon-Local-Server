#!/bin/bash
# build-android.sh
# Cross-compile Lexicon for Android targets
#
# Usage: ./build-android.sh [target] [--release] [--stt] [--tts]
#   target: arm64 (default), x86_64, arm32, x86
#   --release: Build in release mode
#   --stt: Include STT (speech-to-text) feature
#   --tts: Include TTS (text-to-speech) feature (requires ONNX Runtime setup)
#   --no-default-features: Build without any features

set -e

# Configuration
# Note: When building in WSL, use a Linux NDK (not the Windows one mounted via /mnt/)
# Install Linux NDK: wget https://dl.google.com/android/repository/android-ndk-r29-linux.zip
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$HOME/android-ndk-r29}"
export ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-26}"  # Android 8.0+

# CMake's Android module looks for these env vars
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
export ANDROID_NDK="$ANDROID_NDK_HOME"

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
            if [ -z "$FEATURES" ]; then
                FEATURES="stt"
            else
                FEATURES="$FEATURES,stt"
            fi
            ;;
        --tts)
            if [ -z "$FEATURES" ]; then
                FEATURES="tts"
            else
                FEATURES="$FEATURES,tts"
            fi
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

# ONNX Runtime for TTS (if building with --tts)
ONNX_ANDROID_DIR="${ONNXRUNTIME_ANDROID_DIR:-$HOME/.onnxruntime-android}"
if [[ "$FEATURES" == *"tts"* ]]; then
    # Map target to ONNX Runtime directory
    ORT_ARCH_DIR=$(case $TARGET_NAME in
        arm64) echo "arm64-v8a" ;;
        x86_64) echo "x86_64" ;;
        arm32) echo "armeabi-v7a" ;;
        x86) echo "x86" ;;
    esac)

    if [ -d "$ONNX_ANDROID_DIR/jni/$ORT_ARCH_DIR" ]; then
        export ORT_LIB_LOCATION="$ONNX_ANDROID_DIR/jni/$ORT_ARCH_DIR"
        export ORT_INCLUDE_DIR="$ONNX_ANDROID_DIR/headers"
        # ort-sys requires dynamic linking for Android pre-built binaries
        export ORT_PREFER_DYNAMIC_LINK=1
        echo "ONNX Runtime: $ORT_LIB_LOCATION"
    else
        echo "ERROR: ONNX Runtime Android not found at $ONNX_ANDROID_DIR"
        echo "Run: scripts/android/setup-onnx-android.sh"
        exit 1
    fi
fi

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

    # Note about dependencies for STT/TTS builds
    if [[ "$FEATURES" == *"stt"* ]] || [[ "$FEATURES" == *"tts"* ]]; then
        echo ""
        echo "NOTE: This build requires bundling additional libraries:"
        echo "  libc++_shared.so - from NDK: \$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/<arch>/"
    fi
    if [[ "$FEATURES" == *"tts"* ]]; then
        echo "  libonnxruntime.so - from: $ONNX_ANDROID_DIR/jni/$ORT_ARCH_DIR/"
    fi
else
    echo "WARNING: Library not found at expected location"
    echo "Check build output for errors"
fi
