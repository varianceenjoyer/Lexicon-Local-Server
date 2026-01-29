# Android Development Environment Setup - Summary

**Date:** 2026-01-24
**Status:** Mostly Complete - 1 sudo command required

## What Was Accomplished

### 1. Cuttlefish Virtual Device Environment
- ✅ KVM virtualization verified and working
- ✅ User added to kvm and libvirt groups
- ✅ cvd-host_package.tar.gz extracted to `~/cuttlefish`
- ✅ Android 13 images downloaded (with GApps, ~13GB total)
- ⏳ **BLOCKED:** Needs `/etc/cvd_config` symlink (requires sudo)

### 2. Rust Android Toolchain
- ✅ Rust Android targets installed (`aarch64-linux-android`, `x86_64-linux-android`)
- ✅ cargo-ndk v4.1.2 installed
- ✅ Android NDK r26d installed at `~/android-ndk-r26d`
- ✅ `.cargo/config.toml` created with Android target configuration
- ✅ Platform detection bug fixed for Android x86_64 emulators

### 3. whisper.cpp Native Libraries
- ✅ whisper.cpp cloned to `~/whisper.cpp`
- ✅ Built for Android ARM64: `~/whisper.cpp/build-android-arm64/src/libwhisper.a` (11MB)
- ✅ Built for Android x86_64: `~/whisper.cpp/build-android-x86_64/src/libwhisper.a` (9.6MB)
- ✅ Supporting ggml libraries built for both architectures

### 4. Test Builds
- ✅ lexicon-core builds for Android x86_64 (without STT feature)
- ✅ lexicon-ffi builds for Android x86_64 and ARM64 (140MB .so, without STT/TTS)
- ⏳ Full lexicon-ffi with STT: needs whisper-rs-sys CMake fix
- ⏳ Full lexicon-ffi with TTS: ort-sys needs Android pre-built binaries

## Files Created/Modified

| File | Purpose |
|------|---------|
| `.cargo/config.toml` | Cargo cross-compilation configuration |
| `scripts/cuttlefish/setup-cuttlefish-debian.sh` | Full Debian package installer |
| `scripts/cuttlefish/download-images.sh` | Image download helper |
| `scripts/cuttlefish/launch-cuttlefish.sh` | Launch wrapper script |
| `scripts/cuttlefish/test-launch.sh` | Quick test script |
| `scripts/android/build-android.sh` | Android build helper |
| `crates/lexicon-core/src/platform.rs` | Added Android x86/x86_64 support |
| `.dev/docs/07-setup-process/*.md` | Setup documentation |

## Single Blocker - Requires sudo

```bash
sudo ln -s /home/dev/cuttlefish/etc/cvd_config /etc/cvd_config
```

This creates a symlink so Cuttlefish can find its configuration files.

## After the Symlink - Test Commands

### Launch Cuttlefish
```bash
cd ~/cuttlefish
HOME=$PWD ./bin/launch_cvd --daemon --cpus=4 --memory_mb=4096
```

### Wait for boot and connect
```bash
cd ~/cuttlefish
# Wait ~60 seconds for boot
HOME=$PWD ./bin/adb connect localhost:6520
HOME=$PWD ./bin/adb devices
HOME=$PWD ./bin/adb shell getprop ro.build.version.release
```

### Build for Android
```bash
# Set NDK path
export ANDROID_NDK_HOME=/home/dev/android-ndk-r26d

# Build lexicon-core (without STT)
cargo ndk -t x86_64 build -p lexicon-core --no-default-features --features storage

# For full build with STT, whisper-rs linking needs configuration
```

## Environment Variables (add to ~/.bashrc)

```bash
export ANDROID_NDK_HOME=/home/dev/android-ndk-r26d
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
```

## Remaining Work After Unblocking

1. **Cuttlefish testing** - Once symlink is created, verify emulator boots
2. **Device deployment** - Push built .so to emulator for testing:
   ```bash
   # After Cuttlefish is running:
   cd ~/cuttlefish
   HOME=$PWD ./bin/adb push target/x86_64-linux-android/debug/liblexicon_ffi.so /data/local/tmp/
   ```

## Future Work (STT/TTS Support)

1. **whisper-rs-sys CMake fix** - The cmake crate conflicts with cargo-ndk's toolchain setup.
   Options:
   - Fork whisper-rs-sys to support pre-built libraries
   - Patch build.rs to detect and use pre-built whisper.cpp
   - Pre-built libraries available: `~/whisper.cpp/build-android-*/src/libwhisper.a`

2. **ort-sys Android binaries** - ONNX Runtime doesn't provide pre-built Android x86_64.
   Options:
   - Build ONNX Runtime from source for Android
   - Use ort-tract backend instead (pure Rust, no native deps)
   - Wait for official Android support from ort crate

3. **CI integration** - Set up automated Android builds and testing
