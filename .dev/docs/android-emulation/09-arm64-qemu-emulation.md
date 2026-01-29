# ARM64 Android Emulation on x86_64

> **Work Plan:** WP-CUTTLEFISH-09
>
> **Status:** VERIFIED
>
> **Purpose:** Run ARM64 Android on x86_64 host using QEMU TCG emulation.
>
> **Last Verified:** 2026-01-27 on Ubuntu 25.10 (kernel 6.17.0-8-generic)

---

## Overview

This document covers running ARM64 (aarch64) Android Cuttlefish images on an x86_64 host using QEMU's TCG (Tiny Code Generator) software emulation. This is useful for testing ARM64-specific behavior when native ARM64 hardware is not available.

**Performance Warning:** TCG emulation translates every ARM64 instruction in software. Expect boot times of 5-30 minutes and significantly slower runtime performance compared to native execution or x86_64 Cuttlefish.

---

## Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Host RAM | 8GB | 16GB+ |
| Host CPU | 4 cores | 8+ cores |
| Disk Space | 10GB | 20GB |
| OS | Ubuntu 22.04+ | Ubuntu 24.04+ |

### Required Packages

```bash
# Install QEMU ARM64 emulator
sudo apt-get update
sudo apt-get install -y qemu-system-arm qemu-efi-aarch64

# Verify installation
qemu-system-aarch64 --version
# Expected: QEMU emulator version 9.0+ or 10.0+
```

---

## Setup

### 1. Download ARM64 Cuttlefish Images

```bash
# Create directory
mkdir -p ~/cuttlefish-arm64
cd ~/cuttlefish-arm64

# Download from Android CI (replace BUILD_ID with current build)
# Visit: https://ci.android.com/builds/branches/aosp-android-latest-release/grid
# Target: aosp_cf_arm64_only_phone-userdebug
```

**Required files:**
- `super.img` - System partition
- `boot.img` - Boot image
- `vendor_boot.img` - Vendor boot image

### 2. Download x86_64 Host Tools

The host tools (bootloader, utilities) must be x86_64 even when running ARM64 guests:

```bash
mkdir -p ~/cuttlefish
cd ~/cuttlefish

# Download from GitHub mirror (hipexscape/cuttlefish_releases)
curl -L -o cvd-host_package.tar.gz \
    "https://github.com/hipexscape/cuttlefish_releases/releases/download/20250619/cvd-host_package.tar.gz"

tar -xzf cvd-host_package.tar.gz

# Verify bootloader exists
ls etc/bootloader_aarch64/bootloader.qemu
```

### 3. Create Runtime Directory

```bash
mkdir -p ~/cuttlefish-arm64-runtime

# Create userdata image (4GB)
qemu-img create -f qcow2 ~/cuttlefish-arm64-runtime/userdata.qcow2 4G
```

---

## Running ARM64 Android

### Quick Start

```bash
# Run directly with QEMU:
qemu-system-aarch64 \
  -machine virt,gic-version=2 \
  -cpu max \
  -accel tcg,thread=multi \
  -m 4096 \
  -smp 4 \
  -bios ~/cuttlefish/etc/bootloader_aarch64/bootloader.qemu \
  -nographic \
  -drive file=~/cuttlefish-arm64/super.img,format=raw,if=virtio \
  -drive file=~/cuttlefish-arm64/boot.img,format=raw,if=virtio \
  -netdev user,id=net0,hostfwd=tcp::5555-:5555 \
  -device virtio-net-pci,netdev=net0 \
  &
```

### Wait for Boot

```bash
# Poll until device is ready (can take 5-30 minutes)
adb connect localhost:5555
adb devices
```

---

## QEMU Options Reference

### Essential Options

| Option | Description |
|--------|-------------|
| `-machine virt,gic-version=2` | ARM virtual machine with GICv2 interrupt controller |
| `-cpu max` | Emulate all available CPU features |
| `-accel tcg,thread=multi` | Multi-threaded software emulation |
| `-m 4096` | 4GB RAM |
| `-smp 4` | 4 virtual CPUs |
| `-bios <path>` | ARM64 bootloader |
| `-nographic` | No display, serial console only |

### Threading Options

| Option | Effect |
|--------|--------|
| `-accel tcg,thread=single` | Single-threaded TCG (slower, more compatible) |
| `-accel tcg,thread=multi` | Multi-threaded TCG (faster, default) |
| `-smp N` | N virtual CPUs, each gets its own host thread |

### Storage Options

```bash
# Read-only system image
-drive file=super.img,format=raw,if=virtio,readonly=on

# Read-write with QCOW2 overlay
-drive file=userdata.qcow2,format=qcow2,if=virtio
```

### Networking Options

```bash
# User-mode networking with port forwarding
-netdev user,id=net0,hostfwd=tcp::5555-:5555
-device virtio-net-pci,netdev=net0

# TAP networking (requires root/capabilities)
-netdev tap,id=net0,ifname=tap0,script=no
-device virtio-net-pci,netdev=net0
```

---

## ADB Connection

### Connect to Device

```bash
# Connect via forwarded port
adb connect localhost:5555

# Verify connection
adb devices
# Expected: localhost:5555    device

# Check Android version
adb shell getprop ro.build.version.release

# Check architecture
adb shell getprop ro.product.cpu.abi
# Expected: arm64-v8a
```

### Troubleshooting ADB

```bash
# If connection fails, restart ADB server
adb kill-server
adb start-server
adb connect localhost:5555

# Check if port is open
nc -zv localhost 5555
```

---

## Performance Tuning

### CPU Options

```bash
# Use pointer authentication fast mode (reduces overhead)
-cpu max,pauth-impdef=on

# Enable all optimizations
-cpu max,pauth-impdef=on,sve=off
```

### Memory Options

```bash
# Increase RAM for better performance
-m 8192

# Enable huge pages (if host supports)
-mem-path /dev/hugepages
```

### Expected Performance

| Operation | x86_64 Native | ARM64 TCG |
|-----------|---------------|-----------|
| Boot time | 30-60s | 5-30 min |
| App launch | 1-5s | 30-120s |
| Whisper inference | ~10s | 5-15 min |

---

## Cuttlefish vs Direct QEMU

### Why Not Use Cuttlefish Directly?

Cuttlefish's `launch_cvd` with `--vm_manager=qemu_cli` has compatibility issues:

1. **QEMU 9.1+ removed `scsi=off`** - Cuttlefish passes deprecated virtio-blk options
2. **TAP network requires permissions** - Complex network setup
3. **Version mismatch** - Host tools may not match system QEMU

### Direct QEMU Advantages

- Works with any QEMU version
- Simpler networking (user-mode)
- Easier to customize and debug
- No dependency on Cuttlefish host tools for VM execution

---

## Stopping the Emulator

```bash
# Find QEMU process
ps aux | grep qemu-system-aarch64

# Graceful shutdown (if Android is responsive)
adb shell reboot -p

# Force kill
pkill -f qemu-system-aarch64

# Or by PID
kill -9 <PID>
```

---

## Files and Directories

| Path | Purpose |
|------|---------|
| `~/cuttlefish-arm64/` | ARM64 Android images |
| `~/cuttlefish/` | x86_64 host tools (bootloader) |
| `~/cuttlefish-arm64-runtime/` | Runtime state (userdata, logs) |

> **Note:** Helper scripts previously in `scripts/cuttlefish/` have been removed. Use Android Studio for Android development.

---

## Known Issues

### 1. Slow Boot Time

**Symptom:** Device takes 10+ minutes to boot.

**Cause:** TCG translates every instruction in software.

**Mitigation:** Use `-accel tcg,thread=multi` and `-smp 4` or higher.

### 2. Graphics Not Working

**Symptom:** No display output, WebRTC fails.

**Cause:** SwiftShader/graphics stack issues with QEMU.

**Workaround:** Use `-nographic` and access via ADB/serial only. Use `scrcpy` for display if needed.

### 3. QEMU Version Compatibility

**Symptom:** `Property 'virtio-blk-pci-non-transitional.scsi' not found`

**Cause:** QEMU 9.1+ removed deprecated `scsi` property.

**Solution:** Use direct QEMU launch (not Cuttlefish launch_cvd).

---

## Next Steps

- For deploying and testing Lexicon, see [testing.md](../../commands/testing.md)
- For troubleshooting, see [06-troubleshooting.md](06-troubleshooting.md)

> **Note:** Cuttlefish documentation has been archived. See [../../archive/docs/cuttlefish/](../../archive/docs/cuttlefish/_index.md) for historical reference.
