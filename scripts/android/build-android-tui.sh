#!/bin/bash
# Lexicon TUI - Android Cross-Compilation Script (Linux/macOS)
#
# Cross-compiles the lexicon CLI binary for Android (aarch64-linux-android).
# The resulting binary can run in Termux on Android devices.
#
# Usage:
#   ./scripts/android/build-android-tui.sh              # Debug build for ARM64
#   ./scripts/android/build-android-tui.sh --release    # Release build
#   ./scripts/android/build-android-tui.sh --push       # Build and push to device via adb
#   ./scripts/android/build-android-tui.sh arm64        # Specify target (arm64, x86_64, arm32, x86)
#
# Prerequisites:
#   - Rust with target: rustup target add aarch64-linux-android
#   - Android NDK r26+ (set ANDROID_NDK_HOME)
#   - CMake (for whisper-rs)
#   - LLVM/Clang (for bindgen)
#   - ADB (optional, for --push)

set -e

# Configuration
export ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-26}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Target mapping
declare -A TARGETS=(
    ["arm64"]="aarch64-linux-android"
    ["x86_64"]="x86_64-linux-android"
    ["arm32"]="armv7-linux-androideabi"
    ["x86"]="i686-linux-android"
)

declare -A ABIS=(
    ["arm64"]="arm64-v8a"
    ["x86_64"]="x86_64"
    ["arm32"]="armeabi-v7a"
    ["x86"]="x86"
)

# Parse arguments
TARGET_NAME="arm64"
BUILD_MODE="debug"
RELEASE_FLAG=""
DO_PUSH=false
FEATURES="stt,storage,audio"

show_help() {
    cat << 'EOF'
Lexicon TUI - Android Cross-Compilation Script

USAGE:
    ./build-android-tui.sh [TARGET] [OPTIONS]

TARGETS:
    arm64   (default) ARM64 - most modern Android devices
    x86_64            x86_64 - emulators and some Chromebooks
    arm32             ARM32 - older devices
    x86               x86 - older emulators

OPTIONS:
    --release       Build in release mode (optimized, stripped)
    --push          Push binary to connected Android device via adb
    --features STR  Comma-separated features (default: stt,storage,audio)
    --help          Show this help message

EXAMPLES:
    # Debug build for ARM64
    ./build-android-tui.sh

    # Release build
    ./build-android-tui.sh --release

    # Build for x86_64 emulator
    ./build-android-tui.sh x86_64 --release

    # Build and push to device
    ./build-android-tui.sh --release --push

PREREQUISITES:
    1. Install Android NDK r26+ and set ANDROID_NDK_HOME
    2. Install Rust target: rustup target add aarch64-linux-android
    3. Install CMake and LLVM (for native dependencies)

DEVICE SETUP (Termux):
    # After pushing the binary:
    cp /data/local/tmp/lexicon ~/
    chmod +x ~/lexicon
    ./lexicon --help
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        arm64|x86_64|arm32|x86)
            TARGET_NAME="$1"
            shift
            ;;
        --release|-r)
            BUILD_MODE="release"
            RELEASE_FLAG="--release"
            shift
            ;;
        --push|-p)
            DO_PUSH=true
            shift
            ;;
        --features)
            FEATURES="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

RUST_TARGET="${TARGETS[$TARGET_NAME]}"
ANDROID_ABI="${ABIS[$TARGET_NAME]}"

if [ -z "$RUST_TARGET" ]; then
    echo "ERROR: Unknown target: $TARGET_NAME"
    echo "Available targets: ${!TARGETS[*]}"
    exit 1
fi

echo ""
echo "=== Lexicon TUI - Android Cross-Compilation ==="
echo ""

# Find Android NDK
find_ndk() {
    # Check environment variable
    if [ -n "$ANDROID_NDK_HOME" ] && [ -d "$ANDROID_NDK_HOME" ]; then
        echo "$ANDROID_NDK_HOME"
        return
    fi

    # Common paths on Linux/macOS
    local search_paths=(
        "$HOME/Android/Sdk/ndk"
        "$HOME/Library/Android/sdk/ndk"
        "/opt/android-ndk"
        "/usr/local/android-ndk"
    )

    for base in "${search_paths[@]}"; do
        if [ -d "$base" ]; then
            local latest=$(ls -1 "$base" 2>/dev/null | sort -V | tail -1)
            if [ -n "$latest" ]; then
                echo "$base/$latest"
                return
            fi
        fi
    done
}

NDK_PATH=$(find_ndk)

if [ -z "$NDK_PATH" ]; then
    echo "ERROR: Android NDK not found!"
    echo ""
    echo "Please install the Android NDK and set ANDROID_NDK_HOME:"
    echo ""
    echo "  export ANDROID_NDK_HOME=/path/to/android-ndk-r26d"
    echo ""
    echo "Or install via Android Studio SDK Manager."
    exit 1
fi

echo "NDK: $NDK_PATH"
echo "Target: $TARGET_NAME ($RUST_TARGET)"
echo "Mode: $BUILD_MODE"
echo "Features: $FEATURES"
echo "API Level: $ANDROID_API_LEVEL"
echo ""

# Determine host platform
HOST_TAG=""
case "$(uname -s)" in
    Linux*)  HOST_TAG="linux-x86_64" ;;
    Darwin*) HOST_TAG="darwin-x86_64" ;;
    *)       echo "Unsupported host platform"; exit 1 ;;
esac

TOOLCHAIN_BASE="$NDK_PATH/toolchains/llvm/prebuilt/$HOST_TAG/bin"

if [ ! -d "$TOOLCHAIN_BASE" ]; then
    echo "ERROR: NDK toolchain not found at: $TOOLCHAIN_BASE"
    exit 1
fi

# Map target to clang prefix
case "$RUST_TARGET" in
    aarch64-linux-android)    CLANG_PREFIX="aarch64-linux-android" ;;
    x86_64-linux-android)     CLANG_PREFIX="x86_64-linux-android" ;;
    armv7-linux-androideabi)  CLANG_PREFIX="armv7a-linux-androideabi" ;;
    i686-linux-android)       CLANG_PREFIX="i686-linux-android" ;;
esac

CLANG="$TOOLCHAIN_BASE/${CLANG_PREFIX}${ANDROID_API_LEVEL}-clang"
CLANGXX="$TOOLCHAIN_BASE/${CLANG_PREFIX}${ANDROID_API_LEVEL}-clang++"
AR="$TOOLCHAIN_BASE/llvm-ar"

if [ ! -f "$CLANG" ]; then
    echo "ERROR: Clang not found: $CLANG"
    echo "Ensure NDK r26+ is installed and API level $ANDROID_API_LEVEL is available"
    exit 1
fi

# Check if Rust target is installed
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
    echo "Installing Rust target: $RUST_TARGET"
    rustup target add "$RUST_TARGET"
fi

# Set environment for cross-compilation
export CC="$CLANG"
export CXX="$CLANGXX"
export AR="$AR"

# Cargo linker configuration
LINKER_VAR="CARGO_TARGET_${RUST_TARGET^^}_LINKER"
LINKER_VAR="${LINKER_VAR//-/_}"
export "$LINKER_VAR"="$CLANG"

# CMake toolchain for whisper-rs
export CMAKE_TOOLCHAIN_FILE="$NDK_PATH/build/cmake/android.toolchain.cmake"
export ANDROID_ABI="$ANDROID_ABI"
export ANDROID_PLATFORM="android-$ANDROID_API_LEVEL"

# Skip bindgen regeneration
export WHISPER_DONT_GENERATE_BINDINGS=1
export WHISPER_BUILD_TESTS=OFF
export WHISPER_BUILD_EXAMPLES=OFF

echo "CC: $CLANG"
echo "CMake Toolchain: $CMAKE_TOOLCHAIN_FILE"
echo ""

# Build
echo "=== Building ==="
echo ""

cd "$PROJECT_DIR"

BUILD_CMD="cargo build -p lexicon-cli --target $RUST_TARGET"
if [ -n "$FEATURES" ]; then
    BUILD_CMD="$BUILD_CMD --no-default-features --features $FEATURES"
fi
if [ -n "$RELEASE_FLAG" ]; then
    BUILD_CMD="$BUILD_CMD $RELEASE_FLAG"
fi

echo "$ $BUILD_CMD"
echo ""

eval $BUILD_CMD

# Output location
BINARY_PATH="$PROJECT_DIR/target/$RUST_TARGET/$BUILD_MODE/lexicon"

if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found at: $BINARY_PATH"
    exit 1
fi

echo ""
echo "=== Build Complete ==="
echo ""
echo "Binary: $BINARY_PATH"
ls -lh "$BINARY_PATH"

# Show file type
if command -v file &> /dev/null; then
    echo ""
    file "$BINARY_PATH"
fi

# Push to device if requested
if $DO_PUSH; then
    echo ""
    echo "=== Pushing to Device ==="

    if ! command -v adb &> /dev/null; then
        echo "WARNING: adb not found in PATH"
        echo "Push manually: adb push \"$BINARY_PATH\" /data/local/tmp/lexicon"
    else
        # Check for connected device
        if ! adb devices | grep -q "device$"; then
            echo "WARNING: No Android device connected"
            echo "Connect a device with USB debugging and try again"
        else
            REMOTE_PATH="/data/local/tmp/lexicon"
            echo "Pushing to $REMOTE_PATH..."
            adb push "$BINARY_PATH" "$REMOTE_PATH"
            adb shell chmod +x "$REMOTE_PATH"
            echo ""
            echo "Binary pushed! In Termux:"
            echo "  cp /data/local/tmp/lexicon ~/"
            echo "  chmod +x ~/lexicon"
            echo "  ./lexicon --help"
        fi
    fi
fi

echo ""
echo "To push manually:"
echo "  adb push \"$BINARY_PATH\" /data/local/tmp/lexicon"
echo ""
