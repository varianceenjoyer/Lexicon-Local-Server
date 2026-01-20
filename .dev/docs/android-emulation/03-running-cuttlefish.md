# Running and Managing Cuttlefish

> **Work Plan:** WP-CUTTLEFISH-03
>
> **Status:** TODO
>
> **Purpose:** Document day-to-day Cuttlefish operation.

---

## Prerequisites

Complete [02-cuttlefish-installation.md](02-cuttlefish-installation.md) before proceeding.

---

## Starting Cuttlefish

### Basic Launch

```bash
# Start with default settings
launch_cvd

# Start in daemon mode (background)
launch_cvd --daemon
```

### Headless Mode

For CI and automated testing, use headless mode:

```bash
launch_cvd \
    --daemon \
    --start_webrtc=false \
    --gpu_mode=guest_swiftshader
```

### Multiple Instances

```bash
# Launch first instance
launch_cvd --daemon --base_instance_num=1

# Launch second instance (different ports)
launch_cvd --daemon --base_instance_num=2

# Each instance uses different ADB port:
# Instance 1: localhost:6520
# Instance 2: localhost:6521
```

### Boot Verification

```bash
# Watch boot progress
tail -f ~/cuttlefish_runtime/cuttlefish_runtime.1/kernel.log

# Wait for boot complete (scripted)
adb wait-for-device
adb shell getprop sys.boot_completed
# Returns "1" when fully booted
```

---

## ADB Access

### Local Connection

```bash
# Connect to default instance
adb connect localhost:6520

# Verify connection
adb devices
# Output: localhost:6520    device

# Run commands
adb shell ls /data
```

### Remote Connection

For CI agents connecting remotely:

```bash
# On Cuttlefish host: ensure port is accessible
# May need firewall rules:
sudo ufw allow 6520/tcp

# On remote machine:
adb connect <cuttlefish-host-ip>:6520
```

### Multi-Device Setup

```bash
# List all connected devices
adb devices -l

# Target specific device
adb -s localhost:6520 shell whoami
adb -s localhost:6521 shell whoami

# Set default device
export ANDROID_SERIAL=localhost:6520
```

---

## Lifecycle Management

### Starting and Stopping

```bash
# Start
launch_cvd --daemon

# Graceful stop
stop_cvd

# Force stop (if graceful fails)
pkill -9 -f crosvm
```

### Log Collection

```bash
# Runtime logs location
ls ~/cuttlefish_runtime/cuttlefish_runtime.1/

# Key log files:
# - kernel.log       - Kernel boot messages
# - launcher.log     - Cuttlefish launcher output
# - logcat           - Android logcat output

# Collect logcat
adb logcat -d > device_logs.txt

# Collect kernel log
adb shell dmesg > kernel_dmesg.txt
```

### Crash Recovery

```bash
# If device becomes unresponsive:

# 1. Try graceful stop
stop_cvd

# 2. If that fails, force kill
pkill -9 -f crosvm
pkill -9 -f launch_cvd

# 3. Clean up runtime directory
rm -rf ~/cuttlefish_runtime/cuttlefish_runtime.1

# 4. Restart
launch_cvd --daemon
```

---

## Health Checks

### Boot Detection Scripts

```bash
#!/bin/bash
# wait_for_boot.sh - Wait for Cuttlefish to fully boot

TIMEOUT=300  # 5 minutes
INTERVAL=5

echo "Waiting for device boot..."

for ((i=0; i<TIMEOUT; i+=INTERVAL)); do
    if adb shell getprop sys.boot_completed 2>/dev/null | grep -q "1"; then
        echo "Device booted successfully!"
        exit 0
    fi
    sleep $INTERVAL
done

echo "ERROR: Boot timeout after ${TIMEOUT}s"
exit 1
```

### Resource Monitoring

```bash
# Check Cuttlefish process resource usage
ps aux | grep crosvm

# Check device memory
adb shell cat /proc/meminfo | head -5

# Check device CPU
adb shell top -n 1 -b | head -20
```

### Automated Recovery

TODO: Document automated recovery scripts for CI.

```bash
#!/bin/bash
# health_check.sh - Monitor and recover Cuttlefish

# Check if device is responding
if ! adb shell echo "ping" 2>/dev/null | grep -q "ping"; then
    echo "Device unresponsive, restarting..."
    stop_cvd
    sleep 5
    launch_cvd --daemon
    ./wait_for_boot.sh
fi
```

---

## Common Operations

### Installing APKs

```bash
# Install APK
adb install app-debug.apk

# Install with replacement
adb install -r app-debug.apk

# Install to specific user
adb install --user 0 app-debug.apk
```

### File Transfer

```bash
# Push file to device
adb push local_file.txt /sdcard/

# Pull file from device
adb pull /sdcard/remote_file.txt ./

# Push directory
adb push ./libs/ /data/local/tmp/
```

### Running Shell Commands

```bash
# Interactive shell
adb shell

# Single command
adb shell ls /data/local/tmp

# Run as root (if available)
adb root
adb shell whoami  # Should show "root"
```

---

## Instance Configuration

### Per-Instance Settings

Each instance can have different configurations:

```bash
# Instance 1: Standard testing
launch_cvd --daemon \
    --base_instance_num=1 \
    --cpus=2 \
    --memory_mb=4096

# Instance 2: Heavy workload testing
launch_cvd --daemon \
    --base_instance_num=2 \
    --cpus=4 \
    --memory_mb=8192
```

### Persistent Data

```bash
# Data persists in runtime directory
ls ~/cuttlefish_runtime/cuttlefish_runtime.1/

# To start fresh, remove and recreate:
rm -rf ~/cuttlefish_runtime/cuttlefish_runtime.1
launch_cvd --daemon
```

---

## Next Steps

- For CI integration, see [04-ci-integration.md](04-ci-integration.md)
- For Rust testing, see [05-rust-android-testing.md](05-rust-android-testing.md)
