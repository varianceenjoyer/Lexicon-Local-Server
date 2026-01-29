#!/bin/bash
# Lexicon-Local-Server - Build Script (Linux/macOS)
#
# Usage:
#   ./scripts/build.sh          # Debug build
#   ./scripts/build.sh release  # Release build
#   ./scripts/build.sh clean    # Clean build artifacts

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

check_dependencies() {
    info "Checking dependencies..."

    # Check Rust
    if ! command -v cargo &> /dev/null; then
        error "Rust/Cargo not found. Install from https://rustup.rs"
    fi

    # Check Rust version
    RUST_VERSION=$(rustc --version | grep -oE '[0-9]+\.[0-9]+' | head -1)
    REQUIRED_VERSION="1.85"
    if [ "$(printf '%s\n' "$REQUIRED_VERSION" "$RUST_VERSION" | sort -V | head -n1)" != "$REQUIRED_VERSION" ]; then
        error "Rust $REQUIRED_VERSION+ required, found $RUST_VERSION"
    fi

    # Check cmake
    if ! command -v cmake &> /dev/null; then
        warn "cmake not found - whisper-rs-sys may fail to build"
    fi

    # Check for libclang
    if [ -z "$LIBCLANG_PATH" ]; then
        if [ -d "/usr/lib/llvm-14/lib" ]; then
            export LIBCLANG_PATH="/usr/lib/llvm-14/lib"
        elif [ -d "/usr/lib/llvm-15/lib" ]; then
            export LIBCLANG_PATH="/usr/lib/llvm-15/lib"
        elif command -v brew &> /dev/null; then
            LLVM_PREFIX=$(brew --prefix llvm 2>/dev/null || true)
            if [ -n "$LLVM_PREFIX" ]; then
                export LIBCLANG_PATH="$LLVM_PREFIX/lib"
            fi
        fi
    fi

    info "Dependencies OK"
}

build_debug() {
    info "Building debug configuration..."
    cargo build --workspace --all-features
    info "Debug build complete: target/debug/"
}

build_release() {
    info "Building release configuration..."
    cargo build --workspace --all-features --release
    info "Release build complete: target/release/"
}

clean() {
    info "Cleaning build artifacts..."
    cargo clean
    info "Clean complete"
}

case "${1:-debug}" in
    debug)
        check_dependencies
        build_debug
        ;;
    release)
        check_dependencies
        build_release
        ;;
    clean)
        clean
        ;;
    *)
        echo "Usage: $0 [debug|release|clean]"
        exit 1
        ;;
esac

info "Done!"
