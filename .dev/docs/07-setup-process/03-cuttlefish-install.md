# Cuttlefish Installation

**Date:** 2026-01-24
**Updated:** 2026-01-25

## Current Status

### Host Package: INSTALLED
```bash
$ ls ~/cuttlefish/bin/launch_cvd
/home/dev/cuttlefish/bin/launch_cvd
```

Downloaded from: `hipexscape/cuttlefish_releases` GitHub release (20250619)

### Android System Images: INSTALLED

```bash
$ ls ~/cuttlefish/*.img
boot.img  init_boot.img  super.img  userdata.img  vbmeta.img  vbmeta_system.img  vendor_boot.img
```

Source: `hipexscape/cuttlefish_releases` (Android 13 with GApps)

### System Configuration: CONFIGURED

- `/etc/cvd_config` symlink exists pointing to `~/cuttlefish/etc/cvd_config`
- User is in `cvdnetwork` and `kvm` groups

## Download Android System Images

### Manual Download Steps

1. Navigate to: https://ci.android.com/builds/branches/aosp-android-latest-release/grid

2. In the target selector, find: `aosp_cf_x86_64_phone` or `aosp_cf_x86_64_only_phone`

3. Click "userdebug" under the latest green build

4. In the details panel, download:
   - `aosp_cf_x86_64_phone-img-<BUILD_ID>.zip` (system images, ~2GB)

5. Extract to ~/cuttlefish:
   ```bash
   cd ~/cuttlefish
   unzip ~/Downloads/aosp_cf_x86_64_phone-img-*.zip
   ```

### Alternative: Using fetch_cvd

If `fetch_cvd` is available in the host package:
```bash
cd ~/cuttlefish
./bin/fetch_cvd -default_build=aosp-android-latest-release/aosp_cf_x86_64_phone-userdebug
```

## Environment Variables (Critical)

The following environment variables **must** be set for Cuttlefish to work:

```bash
export CUTTLEFISH_HOME="$HOME/cuttlefish"
export ANDROID_HOST_OUT="$CUTTLEFISH_HOME"
export PATH="$CUTTLEFISH_HOME/bin:$PATH"
```

**Why `ANDROID_HOST_OUT` is required:**
- Without it, `launch_cvd` and `cvd` look for binaries in `/bin/` instead of `~/cuttlefish/bin/`
- You will see errors like: `exec of /bin/assemble_cvd failed (No such file or directory)`

Add these to `~/.bashrc` for persistence.

## Running Cuttlefish

### Using cvd (Recommended)

```bash
cd ~/cuttlefish
export ANDROID_HOST_OUT=$PWD
export PATH="$PWD/bin:$PATH"

# Start device
cvd start --daemon

# Check status
cvd status
# Output: run_cvd is active.

# Stop device
cvd stop
```

### Logs Location

After starting, logs are available at:
- Kernel log: `~/cuttlefish/instances/cvd-1/kernel.log`
- Logcat: `~/cuttlefish/instances/cvd-1/logs/logcat`
- Launcher log: `~/cuttlefish/instances/cvd-1/logs/launcher.log`

## Verification

```bash
cd ~/cuttlefish
export ANDROID_HOST_OUT=$PWD
export PATH="$PWD/bin:$PATH"

# Start
cvd start --daemon

# Check status
cvd status

# Connect ADB
adb connect localhost:6520
adb devices
```

## Next Steps

1. ~~Download Android system images~~ DONE
2. ~~Test launch~~ DONE - `cvd start --daemon` works
3. Verify ADB connectivity
4. Document results in 04-first-launch.md
