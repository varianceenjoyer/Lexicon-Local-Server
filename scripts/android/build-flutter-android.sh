#!/bin/bash
# Build lexicon-flutter for Android targets
# Requires: Android NDK r26+ installed

set -e

# Configuration
ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$HOME/Android/Sdk/ndk/android-ndk-r26d}"
ANDROID_API="${ANDROID_API:-26}"
TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"

if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "Error: Android NDK not found at $ANDROID_NDK_HOME"
    echo "Set ANDROID_NDK_HOME environment variable"
    exit 1
fi

echo "Using Android NDK: $ANDROID_NDK_HOME"
echo "API Level: $ANDROID_API"

# Build function
build_target() {
    local TARGET=$1
    local CC_VAR=$2
    local CC_BIN=$3
    local FEATURES=${4:-"default"}

    echo ""
    echo "=== Building for $TARGET (features: $FEATURES) ==="

    export CC_${CC_VAR}="$TOOLCHAIN/${CC_BIN}${ANDROID_API}-clang"
    export CXX_${CC_VAR}="$TOOLCHAIN/${CC_BIN}${ANDROID_API}-clang++"
    export AR_${CC_VAR}="$TOOLCHAIN/llvm-ar"
    export ANDROID_NDK_HOME
    export CMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake"

    if [ "$FEATURES" = "default" ]; then
        cargo build -p lexicon-flutter --target "$TARGET" --release
    else
        cargo build -p lexicon-flutter --target "$TARGET" --release --no-default-features --features "$FEATURES"
    fi

    echo "Output: target/$TARGET/release/liblexicon_flutter.so"
}

# Default: build ARM64 with all features
TARGET="${1:-aarch64-linux-android}"

case "$TARGET" in
    aarch64-linux-android|arm64)
        build_target "aarch64-linux-android" "aarch64_linux_android" "aarch64-linux-android" "default"
        ;;
    x86_64-linux-android|x86_64)
        # x86_64 only supports STT (ONNX Runtime not available for this target)
        build_target "x86_64-linux-android" "x86_64_linux_android" "x86_64-linux-android" "stt"
        echo "Note: TTS disabled for x86_64 (ONNX Runtime not available)"
        ;;
    armv7-linux-androideabi|arm32)
        build_target "armv7-linux-androideabi" "armv7_linux_androideabi" "armv7a-linux-androideabi" "stt"
        ;;
    all)
        build_target "aarch64-linux-android" "aarch64_linux_android" "aarch64-linux-android" "default"
        build_target "x86_64-linux-android" "x86_64_linux_android" "x86_64-linux-android" "stt"
        ;;
    *)
        echo "Usage: $0 [target]"
        echo "Targets:"
        echo "  aarch64-linux-android (arm64) - ARM64 with STT+TTS [default]"
        echo "  x86_64-linux-android (x86_64) - x86_64 with STT only"
        echo "  armv7-linux-androideabi (arm32) - ARM32 with STT only"
        echo "  all - Build all targets"
        exit 1
        ;;
esac

echo ""
echo "Build complete!"
ls -lh target/*/release/liblexicon_flutter.so 2>/dev/null || true
