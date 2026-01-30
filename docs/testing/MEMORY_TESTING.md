# Memory Testing Guide

This document describes how to test Lexicon Local Server for memory leaks across all supported platforms.

## Overview

Memory leak testing is critical for:
- Long-running server deployments
- Mobile apps with constrained memory
- Embedded systems with limited resources

### Test Coverage

| Component | Test Method | Cycles |
|-----------|-------------|--------|
| LexiconContext | Create/destroy | 100 |
| Database | Open/query/close | 100 |
| Audio pipeline | Process/release | 100 |
| TTS buffers | Allocate/free | 100 |
| Combined | All components | 100 |

---

## Desktop Platforms

### Linux (Valgrind)

Valgrind provides comprehensive memory leak detection:

```bash
# Build release test binary
cargo build --release --test memory_stress_test

# Find the test binary
TEST_BIN=$(find target/release/deps -name 'memory_stress_test-*' -type f -executable | head -1)

# Run with valgrind
valgrind --leak-check=full \
         --show-leak-kinds=all \
         --track-origins=yes \
         --verbose \
         --log-file=valgrind-report.txt \
         $TEST_BIN --test-threads=1

# Check for leaks
grep -E "(definitely|indirectly|possibly) lost" valgrind-report.txt
```

**Expected output:**
```
definitely lost: 0 bytes in 0 blocks
indirectly lost: 0 bytes in 0 blocks
possibly lost: 0 bytes in 0 blocks
```

### Linux (Heaptrack)

Heaptrack provides detailed allocation tracking with a GUI:

```bash
# Install heaptrack
sudo apt install heaptrack heaptrack-gui

# Build and run
cargo build --release --test memory_stress_test
TEST_BIN=$(find target/release/deps -name 'memory_stress_test-*' -type f -executable | head -1)

heaptrack $TEST_BIN --test-threads=1

# Analyze results
heaptrack_gui heaptrack.memory_stress_test.*.gz
```

**What to look for:**
- Memory consumption graph should plateau, not continuously increase
- "Leaked" memory should be minimal (< 1KB typically)
- Peak memory usage should be reasonable for the workload

### macOS (Instruments)

Use Xcode Instruments for memory analysis:

```bash
# Build release binary
cargo build --release --test memory_stress_test

# Find test binary
TEST_BIN=$(find target/release/deps -name 'memory_stress_test-*' -type f ! -name '*.d' | head -1)

# Run with Instruments (requires Xcode)
xcrun xctrace record --template "Leaks" --launch -- $TEST_BIN --test-threads=1
```

Or use the Instruments GUI:
1. Open Instruments (Xcode → Open Developer Tool → Instruments)
2. Select "Leaks" template
3. Choose the test binary as target
4. Run and analyze

### Windows (Visual Studio)

Use Visual Studio's diagnostic tools:

```powershell
# Build release test
cargo build --release --test memory_stress_test

# Run with CRT debug heap
$env:_CRTDBG_MAP_ALLOC = 1
.\target\release\deps\memory_stress_test-*.exe --test-threads=1
```

For detailed analysis, use Visual Studio:
1. Open Developer Command Prompt
2. Run: `devenv /debugexe target\release\deps\memory_stress_test-*.exe`
3. Enable native memory diagnostics
4. Run under debugger

---

## Mobile Platforms

### iOS (Instruments)

#### Prerequisites
- Xcode with Instruments
- iOS device or simulator
- Lexicon iOS test app

#### Using Instruments

1. **Open Instruments**
   ```bash
   open -a Instruments
   ```

2. **Select "Leaks" template**

3. **Choose target:**
   - For simulator: Select running app
   - For device: Connect device, select app

4. **Run test scenario:**
   ```swift
   // In your iOS test code
   for _ in 0..<100 {
       let context = try! LexiconContext(config: config)
       let transcriber = try! Transcriber()
       try! transcriber.loadModel(modelPath: modelPath)
       let result = try! transcriber.transcribe(audioData: testAudio, config: txConfig)
       transcriber.unloadModel()
       // context is released
   }
   ```

5. **Analyze results:**
   - Look for leaks in the Leaks instrument
   - Check Allocations instrument for memory growth
   - Ensure no persistent growth in "Live Bytes"

#### Automated XCTest

```swift
import XCTest
@testable import Lexicon

class MemoryLeakTests: XCTestCase {

    func testContextLifecycleNoLeak() {
        // Use XCTest's built-in memory tracking
        let options = XCTMeasureOptions()
        options.iterationCount = 100

        measure(metrics: [XCTMemoryMetric()], options: options) {
            autoreleasepool {
                let config = LexiconFfiConfig(
                    dataDir: NSTemporaryDirectory(),
                    modelDir: nil,
                    logLevel: "warn"
                )
                let context = try! LexiconContext(config: config)
                _ = context.getPlatformInfo()
            }
        }
    }
}
```

### Android (LeakCanary)

#### Setup LeakCanary

Add to `build.gradle`:
```groovy
dependencies {
    debugImplementation 'com.squareup.leakcanary:leakcanary-android:2.12'
}
```

#### Integration with Lexicon FFI

```kotlin
// In your Android test code
class MemoryLeakTest {

    @Test
    fun testContextLifecycleNoLeak() {
        repeat(100) { cycle ->
            val context = LexiconContext(
                LexiconFfiConfig(
                    dataDir = context.filesDir.absolutePath,
                    modelDir = null,
                    logLevel = "warn"
                )
            )

            val platformInfo = context.getPlatformInfo()
            assertNotNull(platformInfo)

            // Context is garbage collected
        }

        // Force GC and check LeakCanary
        System.gc()
        Thread.sleep(1000)

        // LeakCanary will report any leaks in the notification
    }
}
```

#### Memory Profiler (Android Studio)

1. Open Android Studio
2. Run → Profile app
3. Select "Memory" profiler
4. Run test scenario
5. Click "Dump Java heap" and analyze

**What to look for:**
- Native memory (JNI) should not continuously grow
- Java heap should return to baseline after GC
- No instances of `LexiconContext` remaining after test

---

## CI Integration

### GitHub Actions Workflow

Add to `.github/workflows/ci.yml`:

```yaml
  memory-test:
    name: Memory Leak Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y valgrind libasound2-dev cmake

      - name: Build memory test
        run: cargo build --release --test memory_stress_test

      - name: Run memory stress test
        run: cargo test --release --test memory_stress_test -- --test-threads=1

      - name: Run with valgrind (quick check)
        run: |
          TEST_BIN=$(find target/release/deps -name 'memory_stress_test-*' -type f -executable | head -1)
          valgrind --leak-check=summary --error-exitcode=1 \
            $TEST_BIN test_context_lifecycle_no_leak --test-threads=1
```

---

## Expected Memory Behavior

### Acceptable Patterns

| Behavior | Acceptable | Notes |
|----------|------------|-------|
| Initial allocation spike | ✓ | Runtime initialization |
| Stable plateau after warmup | ✓ | Expected steady state |
| Small growth (< 1MB/100 cycles) | ✓ | Fragmentation, caches |
| Full cleanup on exit | ✓ | All memory freed |

### Unacceptable Patterns

| Behavior | Issue | Action |
|----------|-------|--------|
| Continuous linear growth | Memory leak | Investigate allocation site |
| Exponential growth | Severe leak | Critical bug, fix immediately |
| Large retained memory after unload | Resource not freed | Check drop implementations |

### Known Memory Consumers

| Component | Typical Usage | Notes |
|-----------|---------------|-------|
| Whisper model | 75MB - 3GB | Depends on model size |
| SQLite connection | 1-5 MB | Page cache, prepared statements |
| Audio buffers | 1-10 MB | Ring buffers, resampling |
| TTS engine | 50-200 MB | Model and synthesis buffers |

---

## Troubleshooting

### Common Issues

#### 1. Valgrind reports "possibly lost"

Often false positives from:
- Thread-local storage
- Lazy initialization
- Runtime internals

**Resolution:** Focus on "definitely lost" first.

#### 2. RSS grows but valgrind shows no leaks

Causes:
- Memory fragmentation
- OS page caching
- Allocator behavior

**Resolution:** Check peak allocation vs. end allocation in heaptrack.

#### 3. Mobile tests show native memory growth

Common causes:
- JNI local references not released
- C strings not freed
- Callback closures retained

**Resolution:** Audit FFI boundary code for proper cleanup.

---

## References

- [Valgrind Manual](https://valgrind.org/docs/manual/manual.html)
- [Heaptrack Documentation](https://github.com/KDE/heaptrack)
- [Instruments User Guide](https://help.apple.com/instruments/mac/current/)
- [LeakCanary Documentation](https://square.github.io/leakcanary/)
- [Android Memory Profiler](https://developer.android.com/studio/profile/memory-profiler)
