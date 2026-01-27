#!/bin/bash
# wait-for-arm64.sh
# Poll for ARM64 Android device availability via ADB
#
# Usage: ./wait-for-arm64.sh [--timeout SECONDS]
#
# Polls ADB every 30 seconds until the device is ready or timeout expires.

set -e

TIMEOUT="${1:-1800}"  # Default 30 minutes
INTERVAL=30
ADB_PORT="${ADB_PORT:-5555}"
ADB_HOST="${ADB_HOST:-localhost}"

ELAPSED=0

echo "=== Waiting for ARM64 Android Device ==="
echo "Host: $ADB_HOST:$ADB_PORT"
echo "Timeout: ${TIMEOUT}s"
echo "Polling interval: ${INTERVAL}s"
echo ""
echo "Started at: $(date)"
echo ""

while [ $ELAPSED -lt $TIMEOUT ]; do
    # Try to connect
    adb connect "$ADB_HOST:$ADB_PORT" >/dev/null 2>&1 || true

    # Check if device is online
    if adb -s "$ADB_HOST:$ADB_PORT" shell getprop ro.build.version.release 2>/dev/null | grep -qE "^[0-9]+"; then
        ANDROID_VERSION=$(adb -s "$ADB_HOST:$ADB_PORT" shell getprop ro.build.version.release 2>/dev/null | tr -d '\r')
        ARCH=$(adb -s "$ADB_HOST:$ADB_PORT" shell getprop ro.product.cpu.abi 2>/dev/null | tr -d '\r')

        echo ""
        echo "=== Device Ready ==="
        echo "Android version: $ANDROID_VERSION"
        echo "Architecture: $ARCH"
        echo "Boot time: ${ELAPSED}s"
        echo "Ready at: $(date)"
        echo ""
        echo "Connect with: adb -s $ADB_HOST:$ADB_PORT shell"
        exit 0
    fi

    # Show progress
    printf "\r[%ds / %ds] Waiting for boot..." $ELAPSED $TIMEOUT

    sleep $INTERVAL
    ELAPSED=$((ELAPSED + INTERVAL))
done

echo ""
echo "ERROR: Timeout after ${TIMEOUT}s"
echo "Device did not become available."
echo ""
echo "Troubleshooting:"
echo "  1. Check if QEMU is running: ps aux | grep qemu-system-aarch64"
echo "  2. Check QEMU logs: tail -f ~/cuttlefish-arm64-runtime/qemu.log"
echo "  3. ARM64 emulation is slow - try increasing timeout"
exit 1
