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

Attempts to force single-threaded operation in ggml on Android to work around pthread
issues in the Cuttlefish x86_64 emulator.

**Issue:** `invalid pthread_t passed to pthread_join` crash during whisper initialization
**Cause:** Incompatibility between ggml/whisper.cpp threading and Cuttlefish's bionic libc
**Attempted Workaround:** Force `n_threads=1` in `ggml_threadpool_new_impl`

**Status:** ⚠️ DOES NOT RESOLVE THE ISSUE

The patch is applied correctly and the code is compiled into the binary, but the crash
still occurs. Investigation shows:
- The crash happens during `whisper_init_state`, not during inference
- The patched function (`ggml_threadpool_new_impl`) is not called during initialization
- The pthread issue may originate from a different code path (possibly C++ `std::thread`
  usage in whisper.cpp's mel spectrogram or graph building code)

This issue does NOT affect:
- Real ARM64 devices
- ARM64 emulators on ARM hosts (e.g., Apple Silicon)
- Native x86_64 Linux/macOS builds

To apply for Cuttlefish testing:
```bash
./scripts/android/build-android-patched.sh x86_64 --release
```

To restore original source:
```bash
./scripts/android/build-android-patched.sh --restore
```
