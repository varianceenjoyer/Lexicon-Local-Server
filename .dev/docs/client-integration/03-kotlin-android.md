# Kotlin/Android Integration Guide

## Overview

Lexicon provides native Kotlin bindings via UniFFI. The bindings are auto-generated from the Rust code and provide type-safe, idiomatic Kotlin APIs with JNA-based FFI.

## Installation

### Gradle Setup

1. Build the library for Android targets:
   ```bash
   cd ../Lexicon-Local-Server
   ./scripts/build-android.sh
   cd ../Lexicon-Mobile
   ```

2. The build produces:
   - `jniLibs/` - Native libraries for each ABI (arm64-v8a, armeabi-v7a, x86_64, x86)
   - `kotlin/uniffi/lexicon/lexicon.kt` - Generated Kotlin bindings

3. Add to your Android project:

**build.gradle.kts (app module):**
```kotlin
android {
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

dependencies {
    implementation("net.java.dev.jna:jna:5.13.0@aar")
}
```

4. Copy files:
   - Native libraries to `app/src/main/jniLibs/<abi>/`
   - `lexicon.kt` to your Kotlin sources

### ABI Support

| ABI | Device Types |
|-----|--------------|
| arm64-v8a | Modern Android phones/tablets (64-bit) |
| armeabi-v7a | Older Android devices (32-bit) |
| x86_64 | Emulators, Chromebooks |
| x86 | Old emulators |

For most apps, include `arm64-v8a` and `armeabi-v7a`.

## API Reference

### Initialization

```kotlin
import uniffi.lexicon.*

// Initialize the library (call once at app startup)
try {
    val dataDir = context.filesDir.absolutePath
    initLexicon(dataDir = dataDir)
} catch (e: LexiconFfiException) {
    Log.e("Lexicon", "Initialization failed: ${e.message}")
}

// Get library version
val version = getVersion()
Log.d("Lexicon", "Lexicon version: $version")
```

### LexiconContext

The main context for managing models and platform info.

```kotlin
// Create context with configuration
val config = LexiconFfiConfig(
    dataDir = context.filesDir.absolutePath,
    modelDir = File(context.filesDir, "models").absolutePath,  // optional
    logLevel = "info"
)

try {
    val lexiconContext = LexiconContext(config)

    // Get platform information
    val platformInfo = lexiconContext.getPlatformInfo()
    Log.d("Lexicon", "Platform: ${platformInfo.platform}")
    Log.d("Lexicon", "Recommended tier: ${platformInfo.modelTier}")
    Log.d("Lexicon", "Available memory: ${platformInfo.availableMemoryMb} MB")
    Log.d("Lexicon", "Has GPU: ${platformInfo.hasGpu}")

    // List available models
    val models = lexiconContext.listModels()
    models.forEach { model ->
        Log.d("Lexicon", "${model.id}: ${model.name} (${model.sizeBytes} bytes)")
        Log.d("Lexicon", "  Downloaded: ${model.downloaded}")
        model.localPath?.let { Log.d("Lexicon", "  Path: $it") }
    }

    // Check if specific model is downloaded
    val isDownloaded = lexiconContext.isModelDownloaded("whisper-base")

    // Delete a model
    lexiconContext.deleteModel("whisper-base")

} catch (e: LexiconFfiException) {
    Log.e("Lexicon", "Error: ${e.message}")
}
```

### Transcriber

Speech-to-text transcription.

```kotlin
// Create transcriber (can throw LexiconFfiException)
val transcriber = Transcriber()

// Load a model
val modelPath = File(context.filesDir, "models/whisper-base.bin").absolutePath
transcriber.loadModel(modelPath)

// Check model status
val isLoaded = transcriber.isModelLoaded()
val modelName = transcriber.getModelName()  // String?

// Prepare audio data (16kHz mono f32 PCM)
val audioData: ByteArray = prepareAudioData()  // Your audio preprocessing

// Configure transcription
val config = TranscriptionFfiConfig(
    language = "en",           // null for auto-detect
    translate = false,         // true to translate to English
    numThreads = 4u,
    initialPrompt = null,      // Optional prompt for context
    temperature = 0.0f         // 0.0 for greedy, higher for sampling
)

// Transcribe
try {
    val result = transcriber.transcribe(audioData, config)

    Log.d("Lexicon", "Detected language: ${result.detectedLanguage ?: "unknown"}")
    Log.d("Lexicon", "Audio duration: ${result.audioDurationMs} ms")
    Log.d("Lexicon", "Processing time: ${result.processingTimeMs} ms")

    result.segments.forEach { segment ->
        Log.d("Lexicon", "[${segment.startMs} - ${segment.endMs}] ${segment.text}")
        segment.confidence?.let { Log.d("Lexicon", "  Confidence: $it") }
    }
} catch (e: LexiconFfiException) {
    Log.e("Lexicon", "Transcription failed: ${e.message}")
}

// Translate non-English audio to English
val translation = transcriber.translate(audioData, config)

// Unload model when done (frees memory)
transcriber.unloadModel()
```

### Synthesizer

Text-to-speech synthesis.

```kotlin
// Create synthesizer (can throw LexiconFfiException)
val synthesizer = Synthesizer()

// Check if ready
val isReady = synthesizer.isReady()

// List available voices
val voices = synthesizer.listVoices()
voices.forEach { voice ->
    Log.d("Lexicon", "Voice: $voice")
}

// Configure synthesis
val ttsConfig = TtsFfiConfig(
    language = "en-US",
    voiceId = null,          // Use default voice
    rate = 1.0f,             // 0.5-2.0, normal = 1.0
    pitch = 0.0f,            // Pitch adjustment
    volume = 1.0f,           // 0.0-1.0
    useKokoro = true         // Use Kokoro engine (higher quality)
)

// Synthesize
try {
    val result = synthesizer.synthesize("Hello world!", ttsConfig)

    Log.d("Lexicon", "Sample rate: ${result.sampleRate}")
    Log.d("Lexicon", "Duration: ${result.durationMs} ms")
    Log.d("Lexicon", "Engine: ${result.engine}")

    // result.audioData is f32 PCM bytes
    val audioData = result.audioData
    playAudio(audioData, result.sampleRate)

} catch (e: LexiconFfiException) {
    Log.e("Lexicon", "Synthesis failed: ${e.message}")
}
```

### ModelDownloader

Async model downloading with progress and cancellation.

```kotlin
// Create downloader (can throw LexiconFfiException)
val modelDir = File(context.filesDir, "models").absolutePath
val downloader = ModelDownloader(modelDir)

// Implement callback
class MyDownloadCallback : DownloadCallback {
    override fun onProgress(progress: DownloadProgress) {
        val percent = progress.downloadedBytes.toDouble() / progress.totalBytes * 100
        Log.d("Lexicon", "Download ${progress.modelId}: ${String.format("%.1f", percent)}%")
        Log.d("Lexicon", "Speed: ${progress.speedBps / 1024} KB/s")
        progress.etaSeconds?.let { Log.d("Lexicon", "ETA: $it seconds") }
    }

    override fun onComplete(modelId: String) {
        Log.d("Lexicon", "Download complete: $modelId")
    }

    override fun onError(modelId: String, error: String) {
        Log.e("Lexicon", "Download failed: $modelId - $error")
    }

    override fun onCancelled(modelId: String) {
        Log.d("Lexicon", "Download cancelled: $modelId")
    }
}

// Start download
val callback = MyDownloadCallback()
downloader.downloadModel("whisper-base", callback)

// Cancel download (e.g., from a cancel button)
val wasCancelled = downloader.cancelDownload("whisper-base")

// Check if download is active
val isDownloading = downloader.isDownloading("whisper-base")
```

## Error Handling

All errors are thrown as `LexiconFfiException`:

```kotlin
sealed class LexiconFfiException(message: String) : Exception(message) {
    class InitializationFailed(message: String) : LexiconFfiException(message)
    class ConfigError(message: String) : LexiconFfiException(message)
    class ModelNotFound(message: String) : LexiconFfiException(message)
    class ModelLoadFailed(message: String) : LexiconFfiException(message)
    class TranscriptionFailed(message: String) : LexiconFfiException(message)
    class SynthesisFailed(message: String) : LexiconFfiException(message)
    class InvalidInput(message: String) : LexiconFfiException(message)
    class IoError(message: String) : LexiconFfiException(message)
    class DatabaseError(message: String) : LexiconFfiException(message)
}

// Handle specific errors
try {
    transcriber.loadModel(path)
} catch (e: LexiconFfiException.ModelNotFound) {
    // Model file doesn't exist - prompt download
    showDownloadDialog()
} catch (e: LexiconFfiException.ModelLoadFailed) {
    // Model file corrupted or incompatible
    showErrorDialog(e.message)
} catch (e: LexiconFfiException) {
    // Other errors
    Log.e("Lexicon", "Unexpected error: ${e.message}")
}
```

## Audio Preparation

Convert audio to required format (16kHz mono f32 PCM):

```kotlin
import android.media.AudioFormat
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder

fun prepareAudioData(audioFile: File): ByteArray {
    // Extract and decode audio
    val extractor = MediaExtractor()
    extractor.setDataSource(audioFile.absolutePath)

    // Find audio track
    var audioTrackIndex = -1
    for (i in 0 until extractor.trackCount) {
        val format = extractor.getTrackFormat(i)
        val mime = format.getString(MediaFormat.KEY_MIME)
        if (mime?.startsWith("audio/") == true) {
            audioTrackIndex = i
            break
        }
    }

    require(audioTrackIndex >= 0) { "No audio track found" }
    extractor.selectTrack(audioTrackIndex)

    val format = extractor.getTrackFormat(audioTrackIndex)
    val sampleRate = format.getInteger(MediaFormat.KEY_SAMPLE_RATE)
    val channelCount = format.getInteger(MediaFormat.KEY_CHANNEL_COUNT)

    // Decode audio
    val decodedSamples = decodeAudio(extractor, format)

    // Resample to 16kHz if needed
    val resampledSamples = if (sampleRate != 16000) {
        resample(decodedSamples, sampleRate, 16000)
    } else {
        decodedSamples
    }

    // Convert to mono if needed
    val monoSamples = if (channelCount > 1) {
        convertToMono(resampledSamples, channelCount)
    } else {
        resampledSamples
    }

    // Convert to f32 byte array
    return floatsToBytes(monoSamples)
}

private fun floatsToBytes(samples: FloatArray): ByteArray {
    val buffer = ByteBuffer.allocate(samples.size * 4)
    buffer.order(ByteOrder.LITTLE_ENDIAN)
    samples.forEach { buffer.putFloat(it) }
    return buffer.array()
}

private fun resample(samples: FloatArray, fromRate: Int, toRate: Int): FloatArray {
    val ratio = toRate.toDouble() / fromRate
    val newLength = (samples.size * ratio).toInt()
    val result = FloatArray(newLength)

    for (i in result.indices) {
        val srcIndex = i / ratio
        val srcIndexInt = srcIndex.toInt()
        val frac = srcIndex - srcIndexInt

        result[i] = if (srcIndexInt + 1 < samples.size) {
            (samples[srcIndexInt] * (1 - frac) + samples[srcIndexInt + 1] * frac).toFloat()
        } else {
            samples[srcIndexInt]
        }
    }

    return result
}

private fun convertToMono(samples: FloatArray, channels: Int): FloatArray {
    val monoLength = samples.size / channels
    val result = FloatArray(monoLength)

    for (i in 0 until monoLength) {
        var sum = 0f
        for (c in 0 until channels) {
            sum += samples[i * channels + c]
        }
        result[i] = sum / channels
    }

    return result
}
```

## Real-time Streaming Example

For real-time transcription from microphone:

```kotlin
import android.Manifest
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import kotlinx.coroutines.*
import java.nio.ByteBuffer
import java.nio.ByteOrder

class RealtimeTranscriber(
    private val context: Context,
    modelPath: String
) {
    // Note: Constructor throws LexiconFfiException if model loading fails
    private val transcriber = Transcriber().apply {
        loadModel(modelPath)
    }

    private var audioRecord: AudioRecord? = null
    private var isRecording = false
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val sampleRate = 16000
    private val channelConfig = AudioFormat.CHANNEL_IN_MONO
    private val audioFormat = AudioFormat.ENCODING_PCM_FLOAT
    private val bufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)

    // Accumulate audio for 3 seconds before transcribing
    private val accumulationMs = 3000
    private val samplesPerAccumulation = (sampleRate * accumulationMs / 1000)

    fun startRecording(onSegment: (TranscriptionSegment) -> Unit) {
        if (isRecording) return

        audioRecord = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            sampleRate,
            channelConfig,
            audioFormat,
            bufferSize
        )

        isRecording = true
        audioRecord?.startRecording()

        scope.launch {
            val accumulator = mutableListOf<Float>()
            val buffer = FloatArray(bufferSize / 4)

            while (isRecording) {
                val read = audioRecord?.read(buffer, 0, buffer.size, AudioRecord.READ_BLOCKING) ?: 0
                if (read > 0) {
                    accumulator.addAll(buffer.take(read).toList())

                    if (accumulator.size >= samplesPerAccumulation) {
                        val samples = accumulator.toFloatArray()
                        accumulator.clear()

                        try {
                            val audioData = floatsToBytes(samples)
                            val config = TranscriptionFfiConfig(
                                language = null,
                                translate = false,
                                numThreads = 4u,
                                initialPrompt = null,
                                temperature = 0.0f
                            )

                            val result = transcriber.transcribe(audioData, config)
                            result.segments.forEach { segment ->
                                withContext(Dispatchers.Main) {
                                    onSegment(segment)
                                }
                            }
                        } catch (e: LexiconFfiException) {
                            Log.e("Lexicon", "Transcription error: ${e.message}")
                        }
                    }
                }
            }
        }
    }

    fun stopRecording() {
        isRecording = false
        audioRecord?.stop()
        audioRecord?.release()
        audioRecord = null
    }

    fun release() {
        stopRecording()
        transcriber.unloadModel()
        scope.cancel()
    }

    private fun floatsToBytes(samples: FloatArray): ByteArray {
        val buffer = ByteBuffer.allocate(samples.size * 4)
        buffer.order(ByteOrder.LITTLE_ENDIAN)
        samples.forEach { buffer.putFloat(it) }
        return buffer.array()
    }
}

// Usage
class MainActivity : AppCompatActivity() {
    private lateinit var transcriber: RealtimeTranscriber

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val modelPath = File(filesDir, "models/whisper-base.bin").absolutePath
        transcriber = RealtimeTranscriber(this, modelPath)

        // Request microphone permission first
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PERMISSION_GRANTED) {
            startTranscription()
        } else {
            requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 1)
        }
    }

    private fun startTranscription() {
        transcriber.startRecording { segment ->
            // Update UI with transcribed text
            textView.append(segment.text + " ")
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        transcriber.release()
    }
}
```

## Best Practices

### Memory Management

```kotlin
class TranscriptionService : Service() {
    private var transcriber: Transcriber? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // Load model when service starts
        transcriber = Transcriber().apply {
            loadModel(modelPath)
        }
        return START_STICKY
    }

    override fun onDestroy() {
        // Unload model when service stops
        transcriber?.unloadModel()
        transcriber = null
        super.onDestroy()
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        if (level >= TRIM_MEMORY_MODERATE) {
            // System is low on memory - unload model if not actively transcribing
            if (!isTranscribing) {
                transcriber?.unloadModel()
            }
        }
    }
}
```

### Model Selection

```kotlin
@Throws(LexiconFfiException::class)
fun selectAppropriateModel(context: Context): String {
    val config = LexiconFfiConfig(
        dataDir = context.filesDir.absolutePath,
        modelDir = null,
        logLevel = "info"
    )
    val lexiconContext = LexiconContext(config)  // Can throw
    val platform = lexiconContext.getPlatformInfo()

    return when (platform.modelTier) {
        "MobileLow" -> "whisper-tiny"
        "MobileMid" -> "whisper-base"
        "Desktop" -> "whisper-small"
        "Server" -> "whisper-medium"
        else -> "whisper-base"
    }
}
```

### Background Processing

```kotlin
class TranscriptionWorker(
    context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        val audioPath = inputData.getString("audio_path") ?: return Result.failure()

        return try {
            val transcriber = Transcriber()
            transcriber.loadModel(getModelPath())

            val audioData = prepareAudioData(File(audioPath))
            val config = TranscriptionFfiConfig(
                language = null,
                translate = false,
                numThreads = 4u,
                initialPrompt = null,
                temperature = 0.0f
            )

            val result = transcriber.transcribe(audioData, config)
            transcriber.unloadModel()

            val output = workDataOf(
                "text" to result.segments.joinToString(" ") { it.text }
            )
            Result.success(output)
        } catch (e: Exception) {
            Log.e("TranscriptionWorker", "Failed: ${e.message}")
            Result.failure()
        }
    }
}

// Enqueue work
val request = OneTimeWorkRequestBuilder<TranscriptionWorker>()
    .setInputData(workDataOf("audio_path" to audioFile.absolutePath))
    .build()

WorkManager.getInstance(context).enqueue(request)
```

## Troubleshooting

### Common Issues

**"UnsatisfiedLinkError" on startup:**
- Verify native libraries are in correct `jniLibs/<abi>/` directory
- Check that JNA dependency is added
- Ensure device ABI matches available libraries

**"Model not found" error:**
- Verify model file exists at specified path
- Check file permissions
- Ensure model was fully downloaded

**High memory usage:**
- Use smaller model (tiny/base)
- Unload model when not in use
- Process shorter audio chunks

**Slow transcription:**
- Reduce `numThreads` on older devices
- Use smaller model
- Consider using WorkManager for background processing

**Audio format errors:**
- Verify 16kHz sample rate
- Ensure mono channel
- Check f32 PCM format (ENCODING_PCM_FLOAT)
- Verify little-endian byte order

### ProGuard/R8 Configuration

If using code shrinking, add to `proguard-rules.pro`:

```proguard
-keep class uniffi.lexicon.** { *; }
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { *; }
```
