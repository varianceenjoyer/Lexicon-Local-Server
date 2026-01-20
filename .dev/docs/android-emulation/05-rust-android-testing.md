# Rust/Android Testing on Cuttlefish

> **Work Plan:** WP-CUTTLEFISH-05
>
> **Status:** TODO
>
> **Purpose:** Document testing Rust code on Android via Cuttlefish.

---

## Prerequisites

- Complete [03-running-cuttlefish.md](03-running-cuttlefish.md)
- Rust toolchain installed
- Cuttlefish instance running

---

## Build Setup

### Android NDK Installation

```bash
# Option 1: Using sdkmanager (recommended)
sdkmanager "ndk;26.1.10909125"

# Option 2: Direct download
# Visit: https://developer.android.com/ndk/downloads
# Extract to ~/Android/Sdk/ndk/

# Set environment variable
export ANDROID_NDK_HOME=~/Android/Sdk/ndk/26.1.10909125
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
```

### cargo-ndk Configuration

```bash
# Install cargo-ndk
cargo install cargo-ndk

# Verify installation
cargo ndk --version
```

### Cross-Compilation Targets

```bash
# Add Android targets
rustup target add aarch64-linux-android    # ARM64 (most devices)
rustup target add armv7-linux-androideabi  # ARM32 (older devices)
rustup target add x86_64-linux-android     # x86_64 emulator
rustup target add i686-linux-android       # x86 emulator

# Verify targets
rustup target list --installed | grep android
```

### Cargo Configuration

Create or update `.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
linker = "aarch64-linux-android26-clang"

[target.armv7-linux-androideabi]
linker = "armv7a-linux-androideabi26-clang"

[target.x86_64-linux-android]
linker = "x86_64-linux-android26-clang"

[target.i686-linux-android]
linker = "i686-linux-android26-clang"
```

---

## Building for Android

### Building with cargo-ndk

```bash
# Build for ARM64 (production devices, Cuttlefish ARM)
cargo ndk -t arm64-v8a build --release

# Build for x86_64 (Cuttlefish x86)
cargo ndk -t x86_64 build --release

# Build multiple targets
cargo ndk -t arm64-v8a -t x86_64 build --release

# Build specific package
cargo ndk -t arm64-v8a build --release -p lexicon-ffi
```

### Output Locations

```bash
# After build, find libraries at:
target/aarch64-linux-android/release/liblexicon_ffi.so
target/x86_64-linux-android/release/liblexicon_ffi.so

# Verify library
file target/aarch64-linux-android/release/liblexicon_ffi.so
# Output: ELF 64-bit LSB shared object, ARM aarch64...
```

---

## Deployment

### Pushing Libraries to Device

```bash
# Ensure Cuttlefish is running and ADB connected
adb connect localhost:6520

# Create test directory
adb shell mkdir -p /data/local/tmp/lexicon

# Push shared library
adb push target/aarch64-linux-android/release/liblexicon_ffi.so \
    /data/local/tmp/lexicon/

# Push test binaries (if any)
adb push target/aarch64-linux-android/release/test_runner \
    /data/local/tmp/lexicon/

# Verify
adb shell ls -la /data/local/tmp/lexicon/
```

### Test Harness Setup

Create a simple test runner:

```bash
#!/bin/bash
# scripts/deploy_android.sh

TARGET_DIR="/data/local/tmp/lexicon"
BUILD_TARGET="${1:-aarch64-linux-android}"

# Build
cargo ndk -t arm64-v8a build --release -p lexicon-ffi

# Deploy
adb shell "rm -rf $TARGET_DIR && mkdir -p $TARGET_DIR"
adb push target/$BUILD_TARGET/release/liblexicon_ffi.so $TARGET_DIR/

echo "Deployed to $TARGET_DIR"
```

### Library Path Configuration

```bash
# On device, set library path
adb shell "export LD_LIBRARY_PATH=/data/local/tmp/lexicon:\$LD_LIBRARY_PATH"

# Or for specific command:
adb shell "LD_LIBRARY_PATH=/data/local/tmp/lexicon ./my_test"
```

---

## Test Execution

### cargo-dinghy Integration

[cargo-dinghy](https://github.com/cecton/cargo-dinghy) enables running Rust tests directly on Android:

```bash
# Install dinghy
cargo install cargo-dinghy

# Configure Android device
# Create ~/.dinghy.toml:
cat > ~/.dinghy.toml << EOF
[platforms.android]
rustc_triple = "aarch64-linux-android"

[devices]
cuttlefish = { platform = "android", id = "localhost:6520" }
EOF

# Run tests on device
cargo dinghy -d cuttlefish test -p lexicon-core

# Run specific test
cargo dinghy -d cuttlefish test -p lexicon-core -- test_audio_capture
```

### Manual ADB Testing

For tests that don't use cargo-dinghy:

```bash
# Push test binary
adb push target/aarch64-linux-android/release/lexicon_tests /data/local/tmp/

# Make executable
adb shell chmod +x /data/local/tmp/lexicon_tests

# Run tests
adb shell "cd /data/local/tmp && LD_LIBRARY_PATH=. ./lexicon_tests"

# Capture output
adb shell "cd /data/local/tmp && LD_LIBRARY_PATH=. ./lexicon_tests" > test_output.txt 2>&1
```

### Kotlin/JNI Tests

For testing FFI bindings with Kotlin:

```bash
# Build AAR
./gradlew :lexicon-android:assembleDebug

# Build test APK
./gradlew :lexicon-android:assembleAndroidTest

# Install and run
adb install -r lexicon-android/build/outputs/apk/debug/lexicon-android-debug.apk
adb install -r lexicon-android/build/outputs/apk/androidTest/debug/lexicon-android-debug-androidTest.apk

# Run instrumented tests
adb shell am instrument -w com.lexicon.android.test/androidx.test.runner.AndroidJUnitRunner
```

---

## Debugging

### Remote Debugging Setup

```bash
# Start lldb-server on device
adb push $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/lib64/clang/*/lib/linux/aarch64/lldb-server /data/local/tmp/
adb shell chmod +x /data/local/tmp/lldb-server
adb shell /data/local/tmp/lldb-server platform --listen "*:1234" --server &

# Forward port
adb forward tcp:1234 tcp:1234

# Connect with lldb
lldb
(lldb) platform select remote-android
(lldb) platform connect connect://localhost:1234
(lldb) target create target/aarch64-linux-android/release/my_binary
```

### Log Collection

```bash
# Filter logcat for Rust/native logs
adb logcat -s RustStdout:* lexicon:*

# Capture to file
adb logcat -d > android_logs.txt

# Clear and start fresh
adb logcat -c
# ... run tests ...
adb logcat -d > test_logs.txt
```

### Crash Analysis

```bash
# Check for tombstones (native crash dumps)
adb shell ls /data/tombstones/

# Pull crash dump
adb pull /data/tombstones/tombstone_00

# Symbolicate with ndk-stack
$ANDROID_NDK_HOME/ndk-stack -sym target/aarch64-linux-android/release/ \
    -dump tombstone_00
```

---

## Integration with WP-FFI and WP-ANDROID

### FFI Test Matrix

| Test | Cuttlefish Command |
|------|-------------------|
| `uniffi_generates_kotlin` | `./gradlew :lexicon-android:compileDebugKotlin` |
| `kotlin_creates_context` | Run Kotlin unit tests on device |
| `memory_leak_kotlin` | LeakCanary integration test |
| `thread_safety` | Concurrent FFI calls from Kotlin |

### WP-ANDROID Test Coverage

```bash
#!/bin/bash
# scripts/run_wp_android_tests.sh

echo "=== WP-ANDROID Test Suite ==="

# library_builds_arm64
echo "Testing: library_builds_arm64"
cargo ndk -t arm64-v8a build --release -p lexicon-ffi || exit 1

# library_builds_arm32
echo "Testing: library_builds_arm32"
cargo ndk -t armeabi-v7a build --release -p lexicon-ffi || exit 1

# kotlin_api_compiles
echo "Testing: kotlin_api_compiles"
./gradlew :lexicon-android:assembleRelease || exit 1

# Deploy and test on device
echo "Testing: runtime tests on Cuttlefish"
./scripts/deploy_android.sh
adb shell "cd /data/local/tmp/lexicon && LD_LIBRARY_PATH=. ./run_tests"

echo "=== All WP-ANDROID tests passed ==="
```

---

## Performance Considerations

### Cuttlefish vs Real Device

| Aspect | Cuttlefish | Real Device |
|--------|------------|-------------|
| CPU | Host CPU (fast) | ARM (varies) |
| Memory | Configurable | Fixed |
| GPU | Software/virgl | Hardware |
| Thermal | Simulated | Real |
| XNNPACK | CPU only | May use NPU |

### Recommendations

1. **Use Cuttlefish for:** Functional tests, API verification, FFI boundary tests
2. **Use Real Devices for:** Performance benchmarks, thermal testing, GPU acceleration

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| Library not found | Check `LD_LIBRARY_PATH` |
| Wrong architecture | Verify target matches device (aarch64 vs x86_64) |
| Permission denied | `chmod +x` on binary |
| Linker errors | Check NDK version matches build config |

### Debugging Build Issues

```bash
# Verbose build output
cargo ndk -t arm64-v8a build --release -p lexicon-ffi -vv

# Check linker being used
CARGO_LOG=cargo::core::compiler::fingerprint=info cargo ndk -t arm64-v8a build

# Verify NDK setup
echo $ANDROID_NDK_HOME
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/ | grep clang
```

---

## Next Steps

- For troubleshooting, see [06-troubleshooting.md](06-troubleshooting.md)
- For CI integration, see [04-ci-integration.md](04-ci-integration.md)
