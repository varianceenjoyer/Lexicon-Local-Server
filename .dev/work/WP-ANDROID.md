# WP-ANDROID: Android Platform Integration

> **Status**: Blocked (Release Phase) | **Priority**: HIGH | **Phase**: Release
>
> **Research**: [R-THR-01](../design/research/R-THR-01-thermal-profiling.md), [R-ORT-01](../design/research/R-ORT-01-ort-mobile-builds.md)
>
> **Created**: 2026-01-23 | **Last Updated**: 2026-01-23

---

## Overview

Create Android shared library and Kotlin bindings for embedding Lexicon in Android apps. This enables on-device speech-to-text, translation, and TTS across the Android ecosystem.

**Target**: Android API 26+ (Android 8.0 Oreo and newer)

---

## Scope

### Included
- Shared library build for `aarch64-linux-android` (arm64-v8a)
- Shared library build for `armv7-linux-androideabi` (armeabi-v7a)
- AAR package creation
- Kotlin/JNI bindings via UniFFI
- Foreground service for background transcription
- Audio permission handling
- XNNPACK acceleration (NNAPI deprecated per R-ORT-01)
- Thermal monitoring (API 29+)
- Example Android app

### Excluded
- Play Store publication (user responsibility)
- Android-specific UI components
- Wear OS support
- Android Auto integration
- x86/x86_64 builds (emulator-only, low priority)

---

## Interface Contract

### Kotlin Package API

```kotlin
// com/lexicon/core/LexiconContext.kt

package com.lexicon.core

import android.content.Context

/**
 * Main entry point for Lexicon functionality
 */
class LexiconContext private constructor(
    private val handle: Long // Native pointer
) : AutoCloseable {

    companion object {
        /**
         * Initialize with Android context and configuration
         */
        @JvmStatic
        fun create(context: Context, config: LexiconConfig): LexiconContext

        init {
            System.loadLibrary("lexicon_ffi")
        }
    }

    /**
     * Load a transcription model
     */
    suspend fun loadModel(modelId: String)

    /**
     * Start a transcription session
     */
    fun startSession(): TranscriptionSession

    /**
     * Get available models for this device tier
     */
    fun availableModels(): List<ModelInfo>

    /**
     * Get current thermal headroom (API 29+)
     */
    val thermalHeadroom: Float

    /**
     * Callback for thermal state changes
     */
    var onThermalChange: ((Float) -> Unit)?

    override fun close()
}

/**
 * Transcription session
 */
class TranscriptionSession(private val handle: Long) : AutoCloseable {

    /**
     * Process audio samples (16kHz mono float)
     */
    fun processAudio(samples: FloatArray): List<Segment>

    /**
     * Finish session and get final results
     */
    fun finish(): List<Segment>

    /**
     * Export to SRT format
     */
    fun exportSRT(): String

    override fun close()
}

/**
 * Configuration
 */
data class LexiconConfig(
    val modelDir: String,
    val dataDir: String,
    val diarization: Boolean = false,
    val maxSpeakers: Int = 2,
    val preferredModel: String? = null
)

/**
 * Transcription segment
 */
data class Segment(
    val startMs: Long,
    val endMs: Long,
    val text: String,
    val confidence: Float?,
    val speakerId: Int?
)
```

### Native Kotlin Helpers

```kotlin
// com/lexicon/core/platform/ThermalMonitor.kt

package com.lexicon.core.platform

import android.content.Context
import android.os.Build
import android.os.PowerManager

/**
 * Monitor Android thermal state and notify Rust core
 */
class ThermalMonitor(context: Context) {
    private val powerManager = context.getSystemService(Context.POWER_SERVICE) as PowerManager

    /**
     * Get thermal headroom (0.0 = critical, 1.0 = cool)
     * Only available on API 29+
     */
    val thermalHeadroom: Float
        get() = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            powerManager.thermalHeadroom?.coerceIn(0f, 1f) ?: 1f
        } else {
            1f // No thermal data on older devices
        }

    /**
     * Should downgrade model based on thermal state
     */
    val shouldDowngradeModel: Boolean
        get() = thermalHeadroom < 0.7f

    /**
     * Should warn user about thermal state
     */
    val shouldWarnUser: Boolean
        get() = thermalHeadroom < 0.3f

    /**
     * Register for thermal status updates (API 29+)
     */
    fun registerCallback(callback: (Float) -> Unit) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            powerManager.addThermalStatusListener { status ->
                callback(thermalHeadroom)
            }
        }
    }
}
```

### System Profiler (Kotlin)

```kotlin
// com/lexicon/core/platform/SystemProfiler.kt

package com.lexicon.core.platform

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import android.os.StatFs

/**
 * System capability detection for Android
 */
object SystemProfiler {

    fun detect(context: Context): SystemProfile {
        val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val memInfo = ActivityManager.MemoryInfo()
        activityManager.getMemoryInfo(memInfo)

        return SystemProfile(
            ramGB = memInfo.totalMem.toFloat() / 1_073_741_824f,
            availableRamGB = memInfo.availMem.toFloat() / 1_073_741_824f,
            cpuCores = Runtime.getRuntime().availableProcessors(),
            cpuArch = Build.SUPPORTED_ABIS.firstOrNull() ?: "unknown",
            platform = "android",
            androidApiLevel = Build.VERSION.SDK_INT
        )
    }

    fun recommendedModel(context: Context): String {
        val profile = detect(context)

        return when {
            profile.ramGB < 3f -> "whisper-tiny.en"
            profile.ramGB < 4f -> "whisper-tiny.en"
            profile.ramGB < 6f -> "whisper-base.en"
            else -> "whisper-base.en" // Limit for battery/thermal
        }
    }

    fun getFreeDiskSpace(path: String): Long {
        val stat = StatFs(path)
        return stat.availableBlocksLong * stat.blockSizeLong
    }
}

data class SystemProfile(
    val ramGB: Float,
    val availableRamGB: Float,
    val cpuCores: Int,
    val cpuArch: String,
    val platform: String,
    val androidApiLevel: Int
)
```

### Foreground Service

```kotlin
// com/lexicon/core/service/TranscriptionService.kt

package com.lexicon.core.service

import android.app.Notification
import android.app.Service
import android.content.Intent
import android.os.IBinder

/**
 * Foreground service for background transcription
 */
class TranscriptionService : Service() {

    companion object {
        const val NOTIFICATION_ID = 1001
        const val CHANNEL_ID = "lexicon_transcription"
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = createNotification()
        startForeground(NOTIFICATION_ID, notification)

        // Start transcription work
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun createNotification(): Notification {
        // Create notification channel and notification
        // See Android documentation for details
    }
}
```

---

## Acceptance Criteria

### Build
- [ ] Shared library compiles for `aarch64-linux-android`
- [ ] Shared library compiles for `armv7-linux-androideabi`
- [ ] AAR package builds with Gradle
- [ ] No warnings in release build
- [ ] ProGuard rules preserve FFI classes

### FFI Integration
- [ ] UniFFI generates Kotlin bindings
- [ ] All public APIs accessible from Kotlin
- [ ] Errors map to Kotlin exceptions
- [ ] Coroutine support for async operations
- [ ] No memory leaks (LeakCanary)

### Functionality
- [ ] Load Whisper model on device
- [ ] Transcribe audio file successfully
- [ ] Real-time transcription from microphone
- [ ] TTS synthesis works
- [ ] Database persists sessions

### Platform Integration
- [ ] RECORD_AUDIO permission request works
- [ ] Foreground service keeps transcription active
- [ ] XNNPACK acceleration active
- [ ] Thermal headroom monitoring works (API 29+)
- [ ] Model downgrade at headroom < 0.7

### Performance
- [ ] Memory usage <500MB on low-end (2GB) devices
- [ ] Memory usage <1GB on mid-range devices
- [ ] Transcription latency <800ms (base model)
- [ ] No ANR during transcription

### Example App
- [ ] Builds and runs on device
- [ ] Demonstrates all major features
- [ ] Includes usage documentation

---

## Dependencies

### Requires
- WP-FFI (UniFFI bindings)
- Android Studio with NDK
- Android device for testing
- `cargo-ndk` for cross-compilation

### Blocks
- Play Store distribution
- Production Android apps

---

## Files

### New Files
- `android/lexicon-android/` - AAR library project
- `android/lexicon-android/src/main/kotlin/com/lexicon/core/` - Kotlin wrapper
- `android/lexicon-example/` - Example Android app
- `scripts/build-android.sh` - Build script

### Modified Files
- `crates/lexicon-ffi/Cargo.toml` - Android target configuration
- `.github/workflows/release.yml` - Android build job

---

## Build Process

### Prerequisites

```bash
# Install Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# Install cargo-ndk
cargo install cargo-ndk

# Set NDK path
export ANDROID_NDK_HOME=/path/to/ndk
```

### Build Commands

```bash
# Build for arm64-v8a
cargo ndk -t arm64-v8a -o android/lexicon-android/src/main/jniLibs build --release -p lexicon-ffi

# Build for armeabi-v7a
cargo ndk -t armeabi-v7a -o android/lexicon-android/src/main/jniLibs build --release -p lexicon-ffi

# Build AAR
cd android/lexicon-android
./gradlew assembleRelease
```

### UniFFI Kotlin Generation

```bash
# Generate Kotlin bindings
cargo run --bin uniffi-bindgen generate \
    --library target/release/liblexicon_ffi.so \
    --language kotlin \
    --out-dir android/lexicon-android/src/main/kotlin
```

---

## Test Plan

### Unit Tests (Kotlin)

| Test | Input | Expected Output |
|------|-------|-----------------|
| `testContextCreation` | Valid config | Context instance |
| `testModelLoading` | Model ID | No exception |
| `testTranscription` | Audio samples | Segments list |
| `testThermalHeadroom` | API 29+ device | Valid float |
| `testSystemProfile` | Context | Valid profile |

### Integration Tests

| Test | Scenario | Pass Criteria |
|------|----------|---------------|
| `testFileTranscription` | Transcribe WAV | Valid SRT output |
| `testMicrophoneCapture` | Record and transcribe | Real-time segments |
| `testForegroundService` | App backgrounded | Transcription continues |
| `testThermalDowngrade` | Simulate thermal | Model downgrade |

### Device Testing Matrix

| Device | API Level | RAM | Model | Expected Performance |
|--------|-----------|-----|-------|----------------------|
| Pixel 6 | 31+ | 8GB | base | <700ms latency |
| Samsung A52 | 30+ | 6GB | base | <800ms latency |
| Redmi Note 10 | 29+ | 4GB | tiny | <500ms latency |
| Moto G7 | 28 | 2GB | tiny | <600ms latency |

---

## Implementation Notes

### Foreground Service Permission

Add to AndroidManifest.xml:
```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MICROPHONE" />

<service
    android:name=".service.TranscriptionService"
    android:foregroundServiceType="microphone"
    android:exported="false" />
```

### XNNPACK Acceleration

whisper.cpp uses XNNPACK for CPU optimization on Android:
- NNAPI deprecated (R-ORT-01)
- XNNPACK provides 35-70% power reduction
- Works on all Android devices

### Memory Management

Android has variable memory limits:
- Register `ComponentCallbacks2` for memory pressure
- Unload model on `TRIM_MEMORY_RUNNING_LOW`
- Use `android:largeHeap="true"` if needed (not recommended)

### ProGuard Rules

```proguard
# Keep FFI classes
-keep class com.lexicon.core.** { *; }
-keepclassmembers class * {
    native <methods>;
}
```

---

## Research References

- **R-ORT-01**: ort crate mobile builds, NNAPI deprecation
- **R-THR-01**: Thermal profiling and management
- **R-AUD-01**: cpal audio capture on Android

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work package created for Release phase |

---

*Blocked until Release phase. Requires Android NDK for development.*
