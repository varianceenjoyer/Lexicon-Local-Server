#!/bin/bash
# launch-cuttlefish.sh
# Launches Cuttlefish using the tarball-based installation
#
# This script handles the HOME directory workaround needed when using
# the cvd-host_package.tar.gz extraction method (vs Debian packages).
#
# Usage: ./launch-cuttlefish.sh [OPTIONS]
# Options are passed through to launch_cvd

set -e

CUTTLEFISH_HOME="${CUTTLEFISH_HOME:-$HOME/cuttlefish}"
LAUNCH_CVD="$CUTTLEFISH_HOME/bin/launch_cvd"

# Default options for CI/headless operation
DEFAULT_OPTS="--daemon"

echo "=== Cuttlefish Launcher ==="
echo "Cuttlefish home: $CUTTLEFISH_HOME"
echo ""

# Pre-flight checks
if [ ! -x "$LAUNCH_CVD" ]; then
    echo "ERROR: launch_cvd not found at $LAUNCH_CVD"
    echo "Make sure cvd-host_package.tar.gz has been extracted."
    exit 1
fi

# Check for system images
if ! ls "$CUTTLEFISH_HOME"/*.img 2>/dev/null | head -1 > /dev/null; then
    echo "ERROR: No Android system images found in $CUTTLEFISH_HOME"
    echo ""
    echo "Please download images first. Run:"
    echo "  ./download-images.sh"
    echo ""
    echo "Or manually download from:"
    echo "  https://ci.android.com/builds/branches/aosp-android-latest-release/grid"
    exit 1
fi

# Check KVM
if [ ! -c /dev/kvm ]; then
    echo "ERROR: /dev/kvm not found. KVM is required."
    echo "Check that:"
    echo "  1. Virtualization is enabled in BIOS"
    echo "  2. KVM modules are loaded: lsmod | grep kvm"
    exit 1
fi

if [ ! -r /dev/kvm ] || [ ! -w /dev/kvm ]; then
    echo "ERROR: Cannot access /dev/kvm"
    echo "Make sure your user is in the 'kvm' group:"
    echo "  sudo usermod -aG kvm \$USER"
    echo "  # Then log out and back in"
    exit 1
fi

echo "Pre-flight checks passed."
echo ""

# Stop any existing instance
echo "Stopping any existing Cuttlefish instance..."
cd "$CUTTLEFISH_HOME"
HOME="$CUTTLEFISH_HOME" ./bin/stop_cvd 2>/dev/null || true

# Clean up runtime if requested
if [ "$1" = "--clean" ]; then
    echo "Cleaning runtime directory..."
    rm -rf "$CUTTLEFISH_HOME/cuttlefish_runtime"
    shift
fi

# Launch with HOME set to cuttlefish directory
# This avoids needing cvdnetwork group and /etc/cvd_config
echo "Launching Cuttlefish..."
echo "Command: HOME=$CUTTLEFISH_HOME $LAUNCH_CVD $DEFAULT_OPTS $*"
echo ""

cd "$CUTTLEFISH_HOME"
HOME="$CUTTLEFISH_HOME" "$LAUNCH_CVD" $DEFAULT_OPTS "$@"

echo ""
echo "=== Cuttlefish Started ==="
echo ""
echo "Logs: $CUTTLEFISH_HOME/cuttlefish_runtime/cuttlefish_runtime.1/"
echo ""
echo "Wait for boot, then connect:"
echo "  cd $CUTTLEFISH_HOME"
echo "  HOME=\$PWD ./bin/adb connect localhost:6520"
echo "  HOME=\$PWD ./bin/adb devices"
echo ""
echo "WebRTC interface (if enabled): https://localhost:8443"
echo ""
echo "To stop:"
echo "  cd $CUTTLEFISH_HOME"
echo "  HOME=\$PWD ./bin/stop_cvd"
