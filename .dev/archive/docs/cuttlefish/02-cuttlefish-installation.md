# Cuttlefish Installation

> **Work Plan:** WP-CUTTLEFISH-02
>
> **Status:** TODO
>
> **Purpose:** Document Cuttlefish binary and Android image installation.

---

## Prerequisites

Complete [01-environment-setup.md](01-environment-setup.md) before proceeding.

---

## Downloading Cuttlefish

### From Android CI

Cuttlefish packages are available from Android's continuous integration system.

```bash
# Create installation directory
mkdir -p ~/cuttlefish
cd ~/cuttlefish

# Download latest Cuttlefish host package
# Visit: https://ci.android.com/builds/branches/aosp-main/grid
# Select: aosp_cf_x86_64_phone-trunk_staging-userdebug
# Download: cvd-host_package.tar.gz

# Or use direct link (update build ID as needed):
# wget https://ci.android.com/builds/submitted/<BUILD_ID>/aosp_cf_x86_64_phone-trunk_staging-userdebug/latest/raw/cvd-host_package.tar.gz
```

### Version Selection Guide

| API Level | Android Version | Use Case |
|-----------|-----------------|----------|
| 30 | Android 11 | Thermal API testing (minimum) |
| 33 | Android 13 | Recommended for new features |
| 34 | Android 14 | Latest stable |
| 35 | Android 15 | Preview/testing |

For WP-ANDROID testing requirements, **API 30+** is required for `PowerManager.getThermalHeadroom()`.

---

## Installing Host Tools

### Extracting cvd-host-package

```bash
cd ~/cuttlefish

# Extract host tools
tar -xzvf cvd-host_package.tar.gz

# Verify extraction
ls -la
# Should see: bin/, etc/, lib64/, usr/
```

### Setting Up Environment Variables

**Critical:** The following environment variables must be set for Cuttlefish to work properly:

```bash
# Add to ~/.bashrc or ~/.profile
export CUTTLEFISH_HOME="$HOME/cuttlefish"
export ANDROID_HOST_OUT="$CUTTLEFISH_HOME"
export PATH="$CUTTLEFISH_HOME/bin:$PATH"

# Apply changes
source ~/.bashrc

# Verify
which launch_cvd
# Should output: /home/<user>/cuttlefish/bin/launch_cvd
```

**Important Notes:**
- `ANDROID_HOST_OUT` is required for the `cvd` command to locate binaries correctly
- Without `ANDROID_HOST_OUT`, you may see errors like: `exec of /bin/assemble_cvd failed (No such file or directory)`
- The `cvd` wrapper command is preferred over `launch_cvd` directly

### Verifying Installation

```bash
# Check launch_cvd is accessible
launch_cvd --help

# Check other tools
cvd --help
stop_cvd --help
```

---

## Android System Images

### Selecting API Level

For Lexicon Local Server testing, select images based on test requirements:

| Test Requirement | Minimum API | Recommended |
|------------------|-------------|-------------|
| Basic library loading | 26 | 30+ |
| Thermal headroom API | 29 | 30+ |
| XNNPACK acceleration | 26 | 30+ |
| Foreground service | 26 | 30+ |

### ARM64 vs x86_64

| Architecture | Pros | Cons |
|--------------|------|------|
| **x86_64** | Native speed on x86 hosts | Different arch than target devices |
| **ARM64** | Matches real devices | Requires QEMU translation (slower) |

**Recommendation:** Use **x86_64** for fast CI testing, **ARM64** for final verification.

### Downloading Images

```bash
cd ~/cuttlefish

# Download system image (from same CI build as host package)
# Download: aosp_cf_x86_64_phone-img-<BUILD_ID>.zip

# Extract images
unzip aosp_cf_x86_64_phone-img-*.zip -d images/

# Verify
ls images/
# Should see: boot.img, super.img, userdata.img, etc.
```

---

## Initial Configuration

### cuttlefish.config Reference

TODO: Document configuration file format and options.

```bash
# Example configuration (create if needed)
cat > ~/cuttlefish/cuttlefish.config << EOF
# Cuttlefish configuration
# See launch_cvd --help for all options
EOF
```

### Resource Allocation

```bash
# Launch with custom resources
launch_cvd \
    --cpus=4 \
    --memory_mb=4096 \
    --daemon
```

| Parameter | Default | Recommendation |
|-----------|---------|----------------|
| `--cpus` | 2 | 4 for testing |
| `--memory_mb` | 2048 | 4096 for STT/TTS |
| `--daemon` | false | true for CI |

### Headless Mode Setup

For CI environments, run without display:

```bash
# Headless mode (no display)
launch_cvd \
    --daemon \
    --start_webrtc=false \
    --gpu_mode=drm_virgl
```

---

## Directory Structure

After installation, the directory structure should be:

```
~/cuttlefish/
├── bin/
│   ├── launch_cvd
│   ├── stop_cvd
│   ├── cvd
│   └── ...
├── lib64/
│   └── ...
├── images/
│   ├── boot.img
│   ├── super.img
│   ├── userdata.img
│   └── ...
└── cuttlefish.config (optional)
```

---

## Verification

```bash
# Quick test: launch and verify boot
launch_cvd --daemon

# Wait for boot (check logs)
tail -f ~/cuttlefish_runtime/cuttlefish_runtime.*/kernel.log

# Connect via ADB
adb connect localhost:6520
adb devices
# Should show: localhost:6520 device

# Run simple command
adb shell getprop ro.build.version.release
# Should output Android version (e.g., "14")

# Stop when done
stop_cvd
```

---

## Next Steps

Once installation is verified, proceed to [03-running-cuttlefish.md](03-running-cuttlefish.md).
