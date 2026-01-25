# First Launch Attempt

**Date:** 2026-01-24
**Updated:** 2026-01-25

## Status: RESOLVED

**Last checked:** 2026-01-25 01:19 UTC

### Previous Issue: Missing /etc/cvd_config Directory

This was resolved by creating a symlink:
```bash
sudo ln -s /home/dev/cuttlefish/etc/cvd_config /etc/cvd_config
```

### Previous Issue: Binary Path Resolution

When running `launch_cvd` or `cvd`, the tools couldn't find binaries like `assemble_cvd`:
```
exec of /bin/assemble_cvd failed (No such file or directory)
```

**Root Cause:** The `ANDROID_HOST_OUT` environment variable was not set.

**Solution:** Set required environment variables:
```bash
export CUTTLEFISH_HOME="$HOME/cuttlefish"
export ANDROID_HOST_OUT="$CUTTLEFISH_HOME"
export PATH="$CUTTLEFISH_HOME/bin:$PATH"
```

## Successful Launch

With environment variables set, Cuttlefish starts successfully:

```bash
cd ~/cuttlefish
export ANDROID_HOST_OUT=$PWD
export PATH="$PWD/bin:$PATH"

cvd start --daemon
# Output:
# GPU auto mode: did not detect prerequisites for accelerated rendering support, enabling --gpu_mode=guest_swiftshader.
# Kernel log: /home/dev/cuttlefish/instances/cvd-1/kernel.log
# Logcat output: /home/dev/cuttlefish/instances/cvd-1/logs/logcat

cvd status
# Output: run_cvd is active.
```

## Configuration Notes

- GPU mode: `guest_swiftshader` (software rendering, no GPU acceleration detected)
- Instance directory: `~/cuttlefish/instances/cvd-1/`
- ADB port: `localhost:6520`

## Known Issue: crosvm Device Crashes (Kernel 6.17 Incompatibility)

**Status:** BLOCKED

Multiple crosvm virtual devices crash with SIGSEGV (signal 11) shortly after VM startup:
```
[ERROR:external/crosvm/src/linux/mod.rs:1878] child pcivirtio-console died: signo 17, status 11, code 2
[ERROR:external/crosvm/src/linux/mod.rs:1878] child xhci controller died: signo 17, status 11, code 2
```

**Environment:**
- Host kernel: 6.17.0-8-generic
- crosvm binary: from hipexscape/cuttlefish_releases (20250619)

**Symptoms:**
- `cvd status` shows "run_cvd is active"
- Port 6520 is listening
- ADB cannot connect (device list is empty)
- Launcher log shows device crashes

**Root cause:** The prebuilt crosvm binary was compiled against an older kernel and has seccomp/syscall incompatibilities with kernel 6.17+.

**Resolution options:**
1. **Build Cuttlefish from source** - Compile against current kernel headers
   ```bash
   cd /home/dev/projects/Lexicon-Local-Server/scripts/cuttlefish
   sudo ./setup-cuttlefish-debian.sh
   ```
2. **Use Docker-based Cuttlefish** - Runs with its own kernel
3. **Try official AOSP builds** - May have newer crosvm versions
4. **Alternative emulator** - Use Android Emulator (QEMU-based) instead

## Images Installed

Android images installed at `~/cuttlefish/`:

```
-rw-r--r-- 1 dev dev  64M boot.img
-rw-r--r-- 1 dev dev 8.0M init_boot.img
-rw-r--r-- 1 dev dev 7.0G super.img
-rw-r--r-- 1 dev dev 6.0G userdata.img
-rw-r--r-- 1 dev dev  12K vbmeta.img
-rw-r--r-- 1 dev dev 4.0K vbmeta_system.img
-rw-r--r-- 1 dev dev  64M vendor_boot.img
```

Source: `hipexscape/cuttlefish_releases` release 20250619 (Android 13 with GApps)

## Next Steps

1. Verify ADB connectivity: `adb connect localhost:6520`
2. Test basic device operations
3. Proceed to Rust Android toolchain setup (05-rust-android-toolchain.md)
