#!/bin/bash
# setup-onnx-android.sh
# Download and extract ONNX Runtime Android libraries for TTS builds
#
# Usage: ./setup-onnx-android.sh
#
# This script downloads the official ONNX Runtime Android AAR and extracts
# the native libraries needed for building lexicon-ffi with TTS support.

set -e

ONNX_VERSION="${ONNX_VERSION:-1.23.2}"
ONNX_DIR="${ONNX_DIR:-$HOME/.onnxruntime-android}"
MAVEN_URL="https://repo1.maven.org/maven2/com/microsoft/onnxruntime/onnxruntime-android"

echo "=== ONNX Runtime Android Setup ==="
echo "Version: $ONNX_VERSION"
echo "Target: $ONNX_DIR"
echo ""

# Create directory
mkdir -p "$ONNX_DIR"
cd "$ONNX_DIR"

# Download AAR if not present
AAR_FILE="onnxruntime-android-$ONNX_VERSION.aar"
if [ ! -f "$AAR_FILE" ]; then
    echo "Downloading ONNX Runtime AAR..."
    curl -L -o "$AAR_FILE" "$MAVEN_URL/$ONNX_VERSION/$AAR_FILE"
else
    echo "AAR already downloaded: $AAR_FILE"
fi

# Extract if not already done
if [ ! -d "jni" ]; then
    echo "Extracting AAR..."
    unzip -q "$AAR_FILE"
else
    echo "Already extracted"
fi

# Verify libraries
echo ""
echo "=== Available Libraries ==="
for arch in arm64-v8a armeabi-v7a x86 x86_64; do
    if [ -f "jni/$arch/libonnxruntime.so" ]; then
        size=$(ls -lh "jni/$arch/libonnxruntime.so" | awk '{print $5}')
        echo "  $arch: $size"
    fi
done

echo ""
echo "=== Headers ==="
ls headers/*.h | wc -l | xargs -I{} echo "  {} header files"

# Create environment setup script
cat > env.sh << 'ENVEOF'
# Source this file before building lexicon-ffi with TTS for Android
# Usage: source ~/.onnxruntime-android/env.sh

export ONNXRUNTIME_ANDROID_DIR="$HOME/.onnxruntime-android"

# Set ORT_LIB_LOCATION based on target architecture
# Call this function with the target arch before building
setup_ort_for_android() {
    local arch=$1

    case $arch in
        arm64|aarch64-linux-android)
            export ORT_LIB_LOCATION="$ONNXRUNTIME_ANDROID_DIR/jni/arm64-v8a"
            ;;
        arm32|armv7-linux-androideabi)
            export ORT_LIB_LOCATION="$ONNXRUNTIME_ANDROID_DIR/jni/armeabi-v7a"
            ;;
        x86_64|x86_64-linux-android)
            export ORT_LIB_LOCATION="$ONNXRUNTIME_ANDROID_DIR/jni/x86_64"
            ;;
        x86|i686-linux-android)
            export ORT_LIB_LOCATION="$ONNXRUNTIME_ANDROID_DIR/jni/x86"
            ;;
        *)
            echo "Unknown architecture: $arch"
            return 1
            ;;
    esac

    export ORT_INCLUDE_DIR="$ONNXRUNTIME_ANDROID_DIR/headers"
    echo "ORT_LIB_LOCATION=$ORT_LIB_LOCATION"
}
ENVEOF

echo ""
echo "=== Setup Complete ==="
echo ""
echo "To build lexicon-ffi with TTS for Android:"
echo ""
echo "  source $ONNX_DIR/env.sh"
echo "  setup_ort_for_android arm64  # or x86_64"
echo "  bash scripts/android/build-android.sh arm64 --release --tts"
echo ""
