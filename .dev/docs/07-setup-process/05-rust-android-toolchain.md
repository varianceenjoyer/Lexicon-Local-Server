# Rust Android Toolchain Setup

**Date:** 2026-01-24

## Status: PARTIAL

### Completed ✓

1. **Rust Android targets installed**
   ```bash
   $ rustup target list --installed | grep android
   aarch64-linux-android
   x86_64-linux-android
   ```

2. **cargo-ndk installed**
   ```bash
   $ cargo ndk --version
   cargo-ndk 4.1.2
   ```

### Installed This Session

1. **Android NDK r26d**
   ```bash
   $ cat /home/dev/android-ndk-r26d/source.properties
   Pkg.Revision = 26.3.11579264
   Pkg.ReleaseName = r26d
   ```

   Set environment variable:
   ```bash
   export ANDROID_NDK_HOME=/home/dev/android-ndk-r26d
   ```

## Installing Android NDK

### Option 1: Using sdkmanager (recommended if Android SDK installed)

```bash
# If Android SDK is installed
sdkmanager "ndk;26.1.10909125"
export ANDROID_NDK_HOME=~/Android/Sdk/ndk/26.1.10909125
```

### Option 2: Direct download

```bash
# Download NDK r26c (latest LTS)
cd /tmp
wget https://dl.google.com/android/repository/android-ndk-r26c-linux.zip

# Extract to home directory
unzip android-ndk-r26c-linux.zip -d ~/
mv ~/android-ndk-r26c ~/android-ndk

# Set environment variable (add to ~/.bashrc)
export ANDROID_NDK_HOME=~/android-ndk
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
```

### Option 3: Using apt (older version)

```bash
# Ubuntu package (may be older version)
sudo apt install google-android-ndk-installer
export ANDROID_NDK_HOME=/usr/lib/android-ndk
```

## Verify Installation

After installing NDK:

```bash
# Check NDK version
cat $ANDROID_NDK_HOME/source.properties | grep Pkg.Revision

# Verify clang is available
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/ | grep clang

# Test cargo-ndk build (from project directory)
cargo ndk -t x86_64 check
```

## Configuration

Once NDK is installed, cargo-ndk will auto-detect it from `ANDROID_NDK_HOME`.

For manual linker configuration, create `.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
linker = "aarch64-linux-android26-clang"

[target.x86_64-linux-android]
linker = "x86_64-linux-android26-clang"
```

## Build Challenges

### whisper-rs-sys CMake Cross-Compilation

The `whisper-rs-sys` crate requires cross-compiling whisper.cpp for Android. This is complex because:

1. whisper-rs-sys uses the `cmake` crate which invokes CMake
2. cargo-ndk sets up Rust cross-compilation but CMake picks up conflicting env vars
3. The result is CMake trying to compile with mixed ARM/x86 flags

Error observed:
```
error: unknown target CPU 'armv7-a'  (when building for x86_64)
```

The CMake command shows conflicting targets:
```
--target=armv7-none-linux-androideabi21 ... --target=x86_64-linux-android21
```

### Solutions

#### Option 1: Pre-build whisper.cpp for Android (COMPLETED)

whisper.cpp has been pre-built for Android. Libraries are at:

```
~/whisper.cpp/build-android-arm64/src/libwhisper.a  (11M)
~/whisper.cpp/build-android-x86_64/src/libwhisper.a (9.6M)
```

Supporting ggml libraries:
```
~/whisper.cpp/build-android-{arch}/ggml/src/libggml.a
~/whisper.cpp/build-android-{arch}/ggml/src/libggml-base.a
~/whisper.cpp/build-android-{arch}/ggml/src/libggml-cpu.a
```

To rebuild (if needed):
```bash
ANDROID_NDK_HOME=/home/dev/android-ndk-r26d

# ARM64
cmake -S ~/whisper.cpp -B ~/whisper.cpp/build-android-arm64 \
    -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake \
    -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-26 \
    -DBUILD_SHARED_LIBS=OFF -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=OFF
make -C ~/whisper.cpp/build-android-arm64 -j$(nproc)

# x86_64
cmake -S ~/whisper.cpp -B ~/whisper.cpp/build-android-x86_64 \
    -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake \
    -DANDROID_ABI=x86_64 -DANDROID_PLATFORM=android-26 \
    -DBUILD_SHARED_LIBS=OFF -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=OFF
make -C ~/whisper.cpp/build-android-x86_64 -j$(nproc)
```

Next step: Configure whisper-rs to use pre-built libraries instead of building from source.

#### Option 2: Use whisper-rs Android patches

Check if whisper-rs has Android-specific build instructions or patches.

#### Option 3: Build without STT initially

For initial Android testing (TTS, config, etc.):
```bash
ANDROID_NDK_HOME=/home/dev/android-ndk-r26d cargo ndk -t x86_64 build \
    -p lexicon-ffi --no-default-features --features tts
```

This bypasses whisper entirely for initial bring-up.

## Current Status

### Working
- ✅ lexicon-core builds for Android (without STT)
- ✅ lexicon-ffi builds for Android (without STT/TTS) - 140MB .so
- ✅ ARM64 and x86_64 targets both work

### Not Working
- ❌ whisper-rs-sys: CMake/cargo-ndk toolchain conflict (armv7 flags mixed with x86_64)
- ❌ ort-sys: No pre-built ONNX Runtime for Android x86_64

### Build Command
```bash
# Build for x86_64 (Cuttlefish emulator)
./scripts/android/build-android.sh x86_64

# Build for ARM64 (real devices)
./scripts/android/build-android.sh arm64

# Output: target/{target}/debug/liblexicon_ffi.so
```

## Next Steps

1. ✓ Android NDK installed
2. ⏳ Cuttlefish launch (blocked on sudo)
3. ⏳ Test deployment with `adb push`
4. ⏳ STT support: Fix whisper-rs-sys CMake or use pre-built libs
5. ⏳ TTS support: Build ONNX Runtime for Android or use ort-tract
