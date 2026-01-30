# Swift/iOS Integration Guide

## Overview

Lexicon provides native Swift bindings via UniFFI. The bindings are auto-generated from the Rust code and provide type-safe, idiomatic Swift APIs.

## Installation

### Swift Package Manager

Add the XCFramework to your project:

1. Build the library for iOS targets:
   ```bash
   ./scripts/build-ios.sh
   ```

2. The build produces:
   - `LexiconFFI.xcframework` - Universal framework
   - `Sources/Lexicon/lexicon.swift` - Generated Swift bindings

3. Add to your Xcode project:
   - Drag `LexiconFFI.xcframework` to Frameworks
   - Add `lexicon.swift` to your Swift sources

### Manual Integration

Copy the following files to your project:
- `crates/lexicon-ffi/bindings/swift/lexicon.swift`
- `crates/lexicon-ffi/bindings/swift/lexiconFFI.h`
- `crates/lexicon-ffi/bindings/swift/lexiconFFI.modulemap`
- Built static library (`liblexicon_ffi.a`)

## API Reference

### Initialization

```swift
import Lexicon

// Initialize the library (call once at app startup)
do {
    let documentsDir = FileManager.default.urls(
        for: .documentDirectory,
        in: .userDomainMask
    ).first!.path

    try initLexicon(dataDir: documentsDir)
} catch let error as LexiconFfiError {
    print("Initialization failed: \(error)")
}

// Get library version
let version = getVersion()
print("Lexicon version: \(version)")
```

### LexiconContext

The main context for managing models and platform info.

```swift
// Create context with configuration
let config = LexiconFfiConfig(
    dataDir: documentsDir,
    modelDir: documentsDir + "/models",  // optional
    logLevel: "info"
)

do {
    let context = try LexiconContext(config: config)

    // Get platform information
    let platformInfo = context.getPlatformInfo()
    print("Platform: \(platformInfo.platform)")
    print("Recommended tier: \(platformInfo.modelTier)")
    print("Available memory: \(platformInfo.availableMemoryMb) MB")
    print("Has GPU: \(platformInfo.hasGpu)")

    // List available models
    let models = try context.listModels()
    for model in models {
        print("\(model.id): \(model.name) (\(model.sizeBytes) bytes)")
        print("  Downloaded: \(model.downloaded)")
        if let path = model.localPath {
            print("  Path: \(path)")
        }
    }

    // Check if specific model is downloaded
    let isDownloaded = try context.isModelDownloaded(modelId: "whisper-base")

    // Delete a model
    try context.deleteModel(modelId: "whisper-base")

} catch let error as LexiconFfiError {
    print("Error: \(error)")
}
```

### Transcriber

Speech-to-text transcription.

```swift
// Create transcriber
let transcriber = try Transcriber()

// Load a model
let modelPath = documentsDir + "/models/whisper-base.bin"
try transcriber.loadModel(modelPath: modelPath)

// Check model status
let isLoaded = transcriber.isModelLoaded()
let modelName = transcriber.getModelName()  // Optional<String>

// Prepare audio data (16kHz mono f32 PCM)
let audioData: Data = prepareAudioData()  // Your audio preprocessing

// Configure transcription
let config = TranscriptionFfiConfig(
    language: "en",           // nil for auto-detect
    translate: false,         // true to translate to English
    numThreads: 4,
    initialPrompt: nil,       // Optional prompt for context
    temperature: 0.0          // 0.0 for greedy, higher for sampling
)

// Transcribe
do {
    let result = try transcriber.transcribe(audioData: audioData, config: config)

    print("Detected language: \(result.detectedLanguage ?? "unknown")")
    print("Audio duration: \(result.audioDurationMs) ms")
    print("Processing time: \(result.processingTimeMs) ms")

    for segment in result.segments {
        print("[\(segment.startMs) - \(segment.endMs)] \(segment.text)")
        if let confidence = segment.confidence {
            print("  Confidence: \(confidence)")
        }
    }
} catch let error as LexiconFfiError {
    print("Transcription failed: \(error)")
}

// Translate non-English audio to English
let translation = try transcriber.translate(audioData: audioData, config: config)

// Unload model when done (frees memory)
transcriber.unloadModel()
```

### Synthesizer

Text-to-speech synthesis.

```swift
// Create synthesizer
let synthesizer = try Synthesizer()

// Check if ready
let isReady = synthesizer.isReady()

// List available voices
let voices = synthesizer.listVoices()
for voice in voices {
    print("Voice: \(voice)")
}

// Configure synthesis
let ttsConfig = TtsFfiConfig(
    language: "en-US",
    voiceId: nil,          // Use default voice
    rate: 1.0,             // 0.5-2.0, normal = 1.0
    pitch: 0.0,            // Pitch adjustment
    volume: 1.0,           // 0.0-1.0
    useKokoro: true        // Use Kokoro engine (higher quality)
)

// Synthesize
do {
    let result = try synthesizer.synthesize(text: "Hello world!", config: ttsConfig)

    print("Sample rate: \(result.sampleRate)")
    print("Duration: \(result.durationMs) ms")
    print("Engine: \(result.engine)")

    // result.audioData is f32 PCM bytes
    let audioData = result.audioData
    playAudio(data: audioData, sampleRate: result.sampleRate)

} catch let error as LexiconFfiError {
    print("Synthesis failed: \(error)")
}
```

### ModelDownloader

Async model downloading with progress and cancellation.

```swift
// Create downloader
let modelDir = documentsDir + "/models"
let downloader = try ModelDownloader(modelDir: modelDir)

// Implement callback
class MyDownloadCallback: DownloadCallback {
    func onProgress(progress: DownloadProgress) {
        let percent = Double(progress.downloadedBytes) / Double(progress.totalBytes) * 100
        print("Download \(progress.modelId): \(String(format: "%.1f", percent))%")
        print("Speed: \(progress.speedBps / 1024) KB/s")
        if let eta = progress.etaSeconds {
            print("ETA: \(eta) seconds")
        }
    }

    func onComplete(modelId: String) {
        print("Download complete: \(modelId)")
    }

    func onError(modelId: String, error: String) {
        print("Download failed: \(modelId) - \(error)")
    }

    func onCancelled(modelId: String) {
        print("Download cancelled: \(modelId)")
    }
}

// Start download
let callback = MyDownloadCallback()
try downloader.downloadModel(modelId: "whisper-base", callback: callback)

// Cancel download (e.g., from a cancel button)
let wasCancelled = downloader.cancelDownload(modelId: "whisper-base")

// Check if download is active
let isDownloading = downloader.isDownloading("whisper-base")
```

## Error Handling

All errors are thrown as `LexiconFfiError`:

```swift
enum LexiconFfiError: Error {
    case InitializationFailed(String)
    case ConfigError(String)
    case ModelNotFound(String)
    case ModelLoadFailed(String)
    case TranscriptionFailed(String)
    case SynthesisFailed(String)
    case InvalidInput(String)
    case IoError(String)
    case DatabaseError(String)
}

// Handle specific errors
do {
    try transcriber.loadModel(modelPath: path)
} catch LexiconFfiError.ModelNotFound(let msg) {
    // Model file doesn't exist - prompt download
    showDownloadPrompt()
} catch LexiconFfiError.ModelLoadFailed(let msg) {
    // Model file corrupted or incompatible
    showErrorAlert(msg)
} catch {
    // Other errors
    print("Unexpected error: \(error)")
}
```

## Audio Preparation

Convert audio to required format (16kHz mono f32 PCM):

```swift
import AVFoundation

func prepareAudioData(from url: URL) throws -> Data {
    let file = try AVAudioFile(forReading: url)

    // Target format: 16kHz mono f32
    guard let targetFormat = AVAudioFormat(
        commonFormat: .pcmFormatFloat32,
        sampleRate: 16000,
        channels: 1,
        interleaved: false
    ) else {
        throw AudioError.formatCreationFailed
    }

    // Create converter
    guard let converter = AVAudioConverter(from: file.processingFormat, to: targetFormat) else {
        throw AudioError.converterCreationFailed
    }

    // Calculate output frame count
    let ratio = targetFormat.sampleRate / file.processingFormat.sampleRate
    let outputFrameCount = AVAudioFrameCount(Double(file.length) * ratio)

    // Create output buffer
    guard let outputBuffer = AVAudioPCMBuffer(
        pcmFormat: targetFormat,
        frameCapacity: outputFrameCount
    ) else {
        throw AudioError.bufferCreationFailed
    }

    // Read input
    let inputBuffer = AVAudioPCMBuffer(
        pcmFormat: file.processingFormat,
        frameCapacity: AVAudioFrameCount(file.length)
    )!
    try file.read(into: inputBuffer)

    // Convert
    var error: NSError?
    converter.convert(to: outputBuffer, error: &error) { inNumPackets, outStatus in
        outStatus.pointee = .haveData
        return inputBuffer
    }

    if let error = error {
        throw error
    }

    // Extract bytes
    let floatChannelData = outputBuffer.floatChannelData![0]
    let frameLength = Int(outputBuffer.frameLength)

    var data = Data(capacity: frameLength * 4)
    for i in 0..<frameLength {
        var sample = floatChannelData[i]
        data.append(contentsOf: withUnsafeBytes(of: &sample) { Array($0) })
    }

    return data
}
```

## Real-time Streaming Example

For real-time transcription from microphone:

```swift
import AVFoundation

class RealtimeTranscriber {
    private let transcriber: Transcriber
    private let audioEngine = AVAudioEngine()
    private var audioBuffer = Data()
    private let bufferDurationMs: Int = 3000  // Process every 3 seconds

    init(modelPath: String) throws {
        self.transcriber = try Transcriber()
        try transcriber.loadModel(modelPath: modelPath)
    }

    func startRecording() throws {
        let inputNode = audioEngine.inputNode
        let recordingFormat = inputNode.outputFormat(forBus: 0)

        // Install tap
        inputNode.installTap(onBus: 0, bufferSize: 1024, format: recordingFormat) { [weak self] buffer, time in
            self?.processAudioBuffer(buffer)
        }

        audioEngine.prepare()
        try audioEngine.start()
    }

    func stopRecording() {
        audioEngine.stop()
        audioEngine.inputNode.removeTap(onBus: 0)
    }

    private func processAudioBuffer(_ buffer: AVAudioPCMBuffer) {
        // Convert to 16kHz mono f32 and accumulate
        // When buffer reaches bufferDurationMs, transcribe and clear

        // ... conversion code ...

        if audioBuffer.count >= requiredBytes {
            transcribeChunk()
        }
    }

    private func transcribeChunk() {
        let config = TranscriptionFfiConfig(
            language: nil,
            translate: false,
            numThreads: 4,
            initialPrompt: nil,
            temperature: 0.0
        )

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }

            do {
                let result = try self.transcriber.transcribe(
                    audioData: self.audioBuffer,
                    config: config
                )

                DispatchQueue.main.async {
                    self.handleTranscriptionResult(result)
                }

                self.audioBuffer.removeAll()
            } catch {
                print("Transcription error: \(error)")
            }
        }
    }

    private func handleTranscriptionResult(_ result: TranscriptionFfiResult) {
        for segment in result.segments {
            print("Transcribed: \(segment.text)")
        }
    }
}
```

## Best Practices

### Memory Management

```swift
// Models consume significant memory - unload when not needed
func onEnterBackground() {
    transcriber.unloadModel()
}

func onEnterForeground() {
    try? transcriber.loadModel(modelPath: modelPath)
}
```

### Model Selection

```swift
func selectAppropriateModel() -> String {
    let context = try! LexiconContext(config: config)
    let platform = context.getPlatformInfo()

    switch platform.modelTier {
    case "MobileLow":
        return "whisper-tiny"
    case "MobileMid":
        return "whisper-base"
    case "Desktop":
        return "whisper-small"
    case "Server":
        return "whisper-medium"
    default:
        return "whisper-base"
    }
}
```

### Background Processing

```swift
// Use background tasks for long operations
func transcribeLongAudio(_ audioData: Data) {
    let taskId = UIApplication.shared.beginBackgroundTask {
        // Handle expiration
    }

    DispatchQueue.global(qos: .userInitiated).async {
        defer {
            UIApplication.shared.endBackgroundTask(taskId)
        }

        // Transcription code here
    }
}
```

## Troubleshooting

### Common Issues

**"Model not found" error:**
- Verify model file exists at specified path
- Check file permissions
- Ensure model was fully downloaded (not corrupted)

**High memory usage:**
- Use smaller model (tiny/base)
- Unload model when not in use
- Process shorter audio chunks

**Slow transcription:**
- Reduce `numThreads` on older devices
- Use smaller model
- Enable GPU acceleration (if available)

**Audio format errors:**
- Verify 16kHz sample rate
- Ensure mono channel
- Check f32 PCM format
- Verify little-endian byte order
