#!/bin/bash
# test-launch.sh
# Quick test script to launch Cuttlefish and verify it works
# Run after: sudo ln -s /home/dev/cuttlefish/etc/cvd_config /etc/cvd_config

set -e

CUTTLEFISH_HOME="${CUTTLEFISH_HOME:-$HOME/cuttlefish}"

echo "=== Cuttlefish Launch Test ==="
echo ""

# Pre-flight checks
echo "Checking prerequisites..."

if [ ! -d "/etc/cvd_config" ]; then
    echo "ERROR: /etc/cvd_config not found"
    echo ""
    echo "Run: sudo ln -s $CUTTLEFISH_HOME/etc/cvd_config /etc/cvd_config"
    exit 1
fi
echo "  ✓ /etc/cvd_config exists"

if [ ! -c "/dev/kvm" ]; then
    echo "  ✗ /dev/kvm not found"
    exit 1
fi
echo "  ✓ /dev/kvm available"

if [ ! -f "$CUTTLEFISH_HOME/super.img" ]; then
    echo "  ✗ Android images not found in $CUTTLEFISH_HOME"
    exit 1
fi
echo "  ✓ Android images present"

echo ""
echo "Stopping any existing instance..."
cd "$CUTTLEFISH_HOME"
HOME="$CUTTLEFISH_HOME" ./bin/stop_cvd 2>/dev/null || true

echo ""
echo "Launching Cuttlefish (headless, 4 CPUs, 4GB RAM)..."
cd "$CUTTLEFISH_HOME"
HOME="$CUTTLEFISH_HOME" ./bin/launch_cvd \
    --daemon \
    --cpus=4 \
    --memory_mb=4096 \
    --start_webrtc=false

echo ""
echo "Waiting for boot (up to 120 seconds)..."
for i in {1..24}; do
    sleep 5
    if HOME="$CUTTLEFISH_HOME" ./bin/adb shell getprop sys.boot_completed 2>/dev/null | grep -q "1"; then
        echo ""
        echo "=== SUCCESS: Device booted! ==="
        echo ""
        HOME="$CUTTLEFISH_HOME" ./bin/adb devices
        echo ""
        echo "Android version:"
        HOME="$CUTTLEFISH_HOME" ./bin/adb shell getprop ro.build.version.release
        echo ""
        echo "To connect: cd $CUTTLEFISH_HOME && HOME=\$PWD ./bin/adb shell"
        echo "To stop:    cd $CUTTLEFISH_HOME && HOME=\$PWD ./bin/stop_cvd"
        exit 0
    fi
    echo "  ... still booting ($((i*5))s)"
done

echo ""
echo "ERROR: Boot timeout after 120 seconds"
echo "Check logs: $CUTTLEFISH_HOME/cuttlefish_runtime/cuttlefish_runtime.1/kernel.log"
exit 1
