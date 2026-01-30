#!/bin/bash
# Build lexicon-flutter for Android targets
# Requires: Android NDK r29+ installed, cargo-ndk

set -e

# Memory management for WSL2/Linux
# Rust compilation is memory-intensive; limit parallel jobs to prevent OOM
# Adjust CARGO_BUILD_JOBS based on available RAM (each job can use 1-2GB+)
# WSL2 users: memory may not release after build - run 'wsl --shutdown' from PowerShell
# or set memory limits in %USERPROFILE%\.wslconfig
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

# Configuration
# Note: When building in WSL, use a Linux NDK (not the Windows one mounted via /mnt/)
# Install Linux NDK: wget https://dl.google.com/android/repository/android-ndk-r29-linux.zip
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$HOME/android-ndk-r29}"
export ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-26}"

# CMake's Android module looks for these env vars
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
export ANDROID_NDK="$ANDROID_NDK_HOME"

# Clear conflicting env vars
unset CFLAGS
unset CXXFLAGS
unset CC
unset CXX

# Skip bindgen regeneration (use pre-generated bindings)
export WHISPER_DONT_GENERATE_BINDINGS=1
export WHISPER_BUILD_TESTS="OFF"
export WHISPER_BUILD_EXAMPLES="OFF"

if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "Error: Android NDK not found at $ANDROID_NDK_HOME"
    echo "Set ANDROID_NDK_HOME environment variable"
    echo ""
    echo "For WSL, download the Linux NDK (not Windows):"
    echo "  wget https://dl.google.com/android/repository/android-ndk-r29-linux.zip"
    echo "  unzip android-ndk-r29-linux.zip -d ~/"
    exit 1
fi

# Check for cargo-ndk
if ! command -v cargo-ndk &> /dev/null; then
    echo "Error: cargo-ndk not found"
    echo "Install with: cargo install cargo-ndk"
    exit 1
fi

echo "Using Android NDK: $ANDROID_NDK_HOME"
echo "API Level: $ANDROID_API_LEVEL"

# Map friendly names to Android ABIs
declare -A ABIS=(
    ["aarch64-linux-android"]="arm64-v8a"
    ["arm64"]="arm64-v8a"
    ["x86_64-linux-android"]="x86_64"
    ["x86_64"]="x86_64"
    ["armv7-linux-androideabi"]="armeabi-v7a"
    ["arm32"]="armeabi-v7a"
)

# ONNX Runtime for TTS
ONNX_ANDROID_DIR="${ONNXRUNTIME_ANDROID_DIR:-$HOME/.onnxruntime-android}"

# Source ONNX Runtime env if available
if [ -f "$ONNX_ANDROID_DIR/env.sh" ]; then
    source "$ONNX_ANDROID_DIR/env.sh"
fi

# Build function using cargo-ndk
build_target() {
    local ANDROID_ABI=$1

    echo ""
    echo "=== Building for $ANDROID_ABI ==="

    # Set up ONNX Runtime paths for TTS using setup function if available
    if command -v setup_ort_for_android &> /dev/null; then
        # Map ABI to target name for setup function
        local ORT_TARGET=$(case $ANDROID_ABI in
            arm64-v8a) echo "arm64" ;;
            x86_64) echo "x86_64" ;;
            armeabi-v7a) echo "arm32" ;;
            x86) echo "x86" ;;
        esac)
        setup_ort_for_android "$ORT_TARGET"
        export ORT_PREFER_DYNAMIC_LINK=1
    elif [ -d "$ONNX_ANDROID_DIR/jni/$ANDROID_ABI" ]; then
        # Fallback: set up ONNX Runtime paths directly
        export ORT_LIB_LOCATION="$ONNX_ANDROID_DIR/jni/$ANDROID_ABI"
        export ORT_INCLUDE_DIR="$ONNX_ANDROID_DIR/headers"
        export ORT_PREFER_DYNAMIC_LINK=1
        echo "ONNX Runtime: $ORT_LIB_LOCATION"
    else
        echo "Warning: ONNX Runtime not found for $ANDROID_ABI"
        echo "Run: scripts/android/setup-onnx-android.sh"
        echo "Building without TTS support..."
    fi

    cargo ndk -t "$ANDROID_ABI" build -p lexicon-flutter --release

    echo "Build for $ANDROID_ABI complete"
}

# Default: build ARM64 with all features
TARGET="${1:-arm64}"

case "$TARGET" in
    aarch64-linux-android|arm64)
        build_target "arm64-v8a"
        ;;
    x86_64-linux-android|x86_64)
        build_target "x86_64"
        ;;
    armv7-linux-androideabi|arm32)
        build_target "armeabi-v7a"
        ;;
    all)
        build_target "arm64-v8a"
        build_target "x86_64"
        ;;
    *)
        echo "Usage: $0 [target]"
        echo "Targets:"
        echo "  aarch64-linux-android (arm64) - ARM64 with STT+TTS [default]"
        echo "  x86_64-linux-android (x86_64) - x86_64 with STT+TTS"
        echo "  armv7-linux-androideabi (arm32) - ARM32 with STT+TTS"
        echo "  all - Build all targets"
        exit 1
        ;;
esac

echo ""
echo "Build complete!"
ls -lh target/*/release/liblexicon_flutter.so 2>/dev/null || true
