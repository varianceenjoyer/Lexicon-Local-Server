# Patches

This directory contains patches for upstream dependencies.

## Production Patches

### whisper-rs-sys-android.patch

Required patch for Android cross-compilation. Allows passing `ANDROID_*` environment
variables through to CMake.

**Status:** Required for Android builds
**Applied:** Manually to cargo registry source before building

## Testing-Only Patches

### ggml-android-singlethread.patch

**⚠️ TESTING ONLY - NOT FOR RELEASE**

Forces single-threaded operation in ggml on Android to work around pthread issues
in the Cuttlefish x86_64 emulator. This significantly degrades performance.

**Issue:** `invalid pthread_t passed to pthread_join` crash during whisper inference
**Cause:** ggml threadpool incompatibility with Cuttlefish's bionic libc emulation
**Workaround:** Force `n_threads=1` on Android

This issue does NOT affect:
- Real ARM64 devices
- ARM64 emulators on ARM hosts
- Native x86_64 Linux/macOS builds

To apply for Cuttlefish testing:
```bash
./scripts/android/build-android-patched.sh x86_64 --release
```
