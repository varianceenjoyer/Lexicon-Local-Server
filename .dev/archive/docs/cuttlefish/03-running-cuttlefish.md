# Running and Managing Cuttlefish

> **Work Plan:** WP-CUTTLEFISH-03
>
> **Status:** VERIFIED
>
> **Purpose:** Document day-to-day Cuttlefish operation.
>
> **Last Verified:** 2026-01-25 on Ubuntu 25.10 (kernel 6.17.0-8-generic)

---

## Prerequisites

Complete [02-cuttlefish-installation.md](02-cuttlefish-installation.md) before proceeding.

---

## Verified Quick Start (Ubuntu 25.10 / Kernel 6.17+)

This was verified working on 2026-01-25:

```bash
# 1. Clean any stale state
pkill -9 -f "launch_cvd|run_cvd|crosvm|modem_simulator" 2>/dev/null
rm -rf /home/dev/cuttlefish/instances

# 2. Launch with sandbox disabled (required for kernel 6.17+)
cd /home/dev/cuttlefish && ./bin/launch_cvd --enable_sandbox=false --daemon

# 3. Wait for boot (~30 seconds)
sleep 30

# 4. Verify device is connected
/home/dev/cuttlefish/bin/adb devices
# Expected: 0.0.0.0:6520    device

# 5. Verify Android is running
/home/dev/cuttlefish/bin/adb -s 0.0.0.0:6520 shell getprop ro.build.version.release
# Expected: 13

# 6. Access WebRTC UI (optional)
# Open https://localhost:8443 in browser (accept self-signed cert)
```

**Verified Configuration:**
| Component | Version |
|-----------|---------|
| OS | Ubuntu 25.10 (Questing Quokka) |
| Kernel | 6.17.0-8-generic |
| CPU | AMD Ryzen 5 3600 (SVM) |
| Android | 13 |
| Device | Cuttlefish x86_64 phone |

---

## Environment Setup (Required)

Before running any Cuttlefish commands, ensure these environment variables are set:

```bash
export CUTTLEFISH_HOME="$HOME/cuttlefish"
export ANDROID_HOST_OUT="$CUTTLEFISH_HOME"
export PATH="$CUTTLEFISH_HOME/bin:$PATH"
```

Add these to `~/.bashrc` for persistence. **Without `ANDROID_HOST_OUT`**, you will see errors like:
```
exec of /bin/assemble_cvd failed (No such file or directory)
```

---

## Starting Cuttlefish

### Important: Kernel 6.17+ Compatibility

On Ubuntu 25.10 with kernel 6.17+, Cuttlefish requires disabling the seccomp sandbox due to compatibility issues with crosvm. Without this flag, you will see `pcivirtio-console` crashes with SIGSEGV.

**Required flag for kernel 6.17+:**
```bash
cd /home/dev/cuttlefish && ./bin/launch_cvd --enable_sandbox=false
```

This was verified working on 2026-01-25. See [06-troubleshooting.md](06-troubleshooting.md) for details.

---

### Using cvd (Recommended)

The `cvd` wrapper is the preferred method for managing Cuttlefish:

```bash
# Start with default settings
cvd start

# Start in daemon mode (background)
cvd start --daemon

# Check status
cvd status

# Stop the device
cvd stop
```

### Using launch_cvd (Alternative)

```bash
# Start with default settings (kernel < 6.17)
launch_cvd

# Start in daemon mode (background)
launch_cvd --daemon

# Start with sandbox disabled (REQUIRED for kernel 6.17+)
cd /home/dev/cuttlefish && ./bin/launch_cvd --enable_sandbox=false --daemon
```

### Headless Mode

For CI and automated testing, use headless mode:

```bash
# For kernel < 6.17
launch_cvd \
    --daemon \
    --start_webrtc=false \
    --gpu_mode=guest_swiftshader

# For kernel 6.17+ (includes sandbox workaround)
cd /home/dev/cuttlefish && ./bin/launch_cvd \
    --daemon \
    --enable_sandbox=false \
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
# Use the Cuttlefish-provided ADB
/home/dev/cuttlefish/bin/adb devices
# Output: 0.0.0.0:6520    device

# Connect explicitly (usually auto-connected)
/home/dev/cuttlefish/bin/adb connect 0.0.0.0:6520

# Run commands
/home/dev/cuttlefish/bin/adb -s 0.0.0.0:6520 shell ls /data

# Verify Android version
/home/dev/cuttlefish/bin/adb -s 0.0.0.0:6520 shell getprop ro.build.version.release
# Output: 13
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
