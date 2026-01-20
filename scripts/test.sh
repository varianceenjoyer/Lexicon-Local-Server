#!/bin/bash
# Lexicon Local Server - Test Script (Linux/macOS)
#
# Usage:
#   ./scripts/test.sh          # Run all tests
#   ./scripts/test.sh quick    # Run quick tests only (no E2E)
#   ./scripts/test.sh core     # Run lexicon-core tests only

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

test_all() {
    info "Running all tests..."
    cargo test --workspace --all-features
    info "All tests passed!"
}

test_quick() {
    info "Running quick tests (lexicon-core only)..."
    cargo test -p lexicon-core --all-features
    info "Quick tests passed!"
}

test_core() {
    info "Running lexicon-core tests..."
    cargo test -p lexicon-core --all-features
    info "Core tests passed!"
}

test_cli() {
    info "Running CLI tests..."
    cargo test -p lexicon-cli
    info "CLI tests passed!"
}

test_server() {
    info "Running server tests..."
    cargo test -p lexicon-server
    info "Server tests passed!"
}

test_ffi() {
    info "Running FFI tests..."
    cargo test -p lexicon-ffi
    info "FFI tests passed!"
}

case "${1:-all}" in
    all)
        test_all
        ;;
    quick)
        test_quick
        ;;
    core)
        test_core
        ;;
    cli)
        test_cli
        ;;
    server)
        test_server
        ;;
    ffi)
        test_ffi
        ;;
    *)
        echo "Usage: $0 [all|quick|core|cli|server|ffi]"
        exit 1
        ;;
esac

info "Done!"
