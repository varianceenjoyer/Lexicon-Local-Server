# Error Handling Reference

## Overview

Lexicon uses typed errors across all platforms. This document covers error types, codes, and handling strategies.

## Error Types

### LexiconFfiError (UniFFI - Swift/Kotlin)

| Error | Description | Common Causes |
|-------|-------------|---------------|
| `InitializationFailed` | Library initialization failed | Invalid data directory, permission denied |
| `ConfigError` | Configuration error | Invalid config values, missing required fields |
| `ModelNotFound` | Model file not found | Model not downloaded, wrong path |
| `ModelLoadFailed` | Failed to load model | Corrupted model, incompatible version, OOM |
| `TranscriptionFailed` | Transcription error | Invalid audio format, model not loaded |
| `SynthesisFailed` | TTS synthesis error | TTS not initialized, engine error |
| `InvalidInput` | Invalid input data | Empty audio, wrong format |
| `IoError` | I/O operation failed | File access error, disk full |
| `DatabaseError` | Database error | SQLite error (storage feature) |

### LexiconFlutterError (Flutter/Dart)

| Error | Description | Common Causes |
|-------|-------------|---------------|
| `InitializationFailed` | Library initialization failed | Invalid data directory, permission denied |
| `ConfigError` | Configuration error | Invalid config values, missing required fields |
| `ModelNotFound` | Model file not found | Model not downloaded, wrong path |
| `ModelLoadFailed` | Failed to load model | Corrupted model, incompatible version, OOM |
| `TranscriptionFailed` | Transcription error | Invalid audio format, model not loaded |
| `SynthesisFailed` | TTS synthesis error | TTS not initialized, engine error |
| `InvalidInput` | Invalid input data | Empty audio, wrong format |
| `IoError` | I/O operation failed | File access error, disk full |
| `DownloadFailed` | Model download failed | Network error, server unavailable |
| `Cancelled` | Operation was cancelled | User cancelled download or operation |

### HTTP Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `BAD_REQUEST` | 400 | Invalid request format |
| `NOT_FOUND` | 404 | Resource not found |
| `INTERNAL_ERROR` | 500 | Server-side error |
| `SERVICE_UNAVAILABLE` | 503 | Feature not enabled or not ready |

## Platform-Specific Handling

### Swift

```swift
do {
    try transcriber.loadModel(modelPath: path)
} catch LexiconFfiError.ModelNotFound(let message) {
    // Model doesn't exist - offer to download
    presentDownloadDialog(message: "Model not found. Download now?")

} catch LexiconFfiError.ModelLoadFailed(let message) {
    // Model corrupted or incompatible
    if message.contains("memory") {
        presentAlert(
            title: "Insufficient Memory",
            message: "Try using a smaller model."
        )
    } else {
        presentAlert(
            title: "Model Load Failed",
            message: "The model file may be corrupted. Please re-download."
        )
        // Optionally delete and re-download
        try? context.deleteModel(modelId: modelId)
    }

} catch LexiconFfiError.InitializationFailed(let message) {
    // Library not initialized
    fatalError("Lexicon must be initialized at app startup: \(message)")

} catch {
    // Unexpected error
    presentAlert(title: "Error", message: error.localizedDescription)
}
```

### Kotlin

```kotlin
try {
    transcriber.loadModel(path)
} catch (e: LexiconFfiException.ModelNotFound) {
    // Model doesn't exist - offer to download
    showDownloadDialog("Model not found. Download now?")

} catch (e: LexiconFfiException.ModelLoadFailed) {
    // Model corrupted or incompatible
    when {
        e.message?.contains("memory") == true -> {
            showAlert(
                title = "Insufficient Memory",
                message = "Try using a smaller model."
            )
        }
        else -> {
            showAlert(
                title = "Model Load Failed",
                message = "The model file may be corrupted. Please re-download."
            )
            // Optionally delete and re-download
            runCatching { context.deleteModel(modelId) }
        }
    }

} catch (e: LexiconFfiException.InitializationFailed) {
    // Library not initialized
    throw IllegalStateException("Lexicon must be initialized at app startup: ${e.message}")

} catch (e: LexiconFfiException) {
    // Other Lexicon errors
    showAlert(title = "Error", message = e.message ?: "Unknown error")

} catch (e: Exception) {
    // Unexpected error
    Log.e("Lexicon", "Unexpected error", e)
    showAlert(title = "Error", message = e.message ?: "Unknown error")
}
```

### Dart/Flutter

```dart
try {
  await transcriber.loadModel(modelPath: path);
} on LexiconFlutterError catch (e) {
  // LexiconFlutterError is a sealed class with typed variants
  if (e is LexiconFlutterErrorModelNotFound) {
    // Model doesn't exist - offer to download
    await showDownloadDialog(context);
  } else if (e is LexiconFlutterErrorModelLoadFailed) {
    // Model corrupted or incompatible
    if (e.message.contains('memory')) {
      showSnackBar(context, 'Insufficient memory. Try a smaller model.');
    } else {
      showSnackBar(context, 'Model corrupted. Please re-download.');
      await deleteModel(modelDir: modelDir, modelId: modelId);
    }
  } else if (e is LexiconFlutterErrorInitializationFailed) {
    // Library not initialized - shouldn't happen if init called in main()
    throw StateError('Lexicon not initialized: ${e.message}');
  } else if (e is LexiconFlutterErrorCancelled) {
    // Operation was cancelled
    showSnackBar(context, 'Operation cancelled');
  } else {
    showSnackBar(context, 'Error: ${e.toString()}');
  }
} catch (e) {
  // Unexpected error
  showSnackBar(context, 'Unexpected error: $e');
}
```

### HTTP/JavaScript

```javascript
async function transcribe(audioData) {
  try {
    const response = await fetch('/api/v1/transcribe', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ audio: audioData })
    });

    if (!response.ok) {
      const error = await response.json();

      switch (error.code) {
        case 'BAD_REQUEST':
          // Invalid request format
          console.error('Invalid request:', error.error);
          showAlert('Invalid audio format. Please check your recording.');
          break;

        case 'SERVICE_UNAVAILABLE':
          // Model not loaded or feature disabled
          if (error.error.includes('Model not available')) {
            await loadModel();
            return transcribe(audioData); // Retry
          }
          showAlert('Service not available. Please try again later.');
          break;

        case 'INTERNAL_ERROR':
          // Server error
          console.error('Server error:', error.error);
          showAlert('Server error. Please try again.');
          break;

        default:
          showAlert(`Error: ${error.error}`);
      }
      return null;
    }

    return await response.json();
  } catch (error) {
    // Network error
    console.error('Network error:', error);
    showAlert('Network error. Please check your connection.');
    return null;
  }
}
```

## Error Recovery Strategies

### Automatic Retry

For transient errors (network issues, temporary resource unavailability):

```swift
func transcribeWithRetry(audio: Data, maxRetries: Int = 3) async throws -> TranscriptionFfiResult {
    var lastError: Error?

    for attempt in 1...maxRetries {
        do {
            return try transcriber.transcribe(audioData: audio, config: config)
        } catch LexiconFfiError.TranscriptionFailed(let msg) where msg.contains("temporary") {
            lastError = error
            let delay = Double(attempt) * 0.5  // Exponential backoff
            try await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
            continue
        } catch {
            throw error  // Non-retryable error
        }
    }

    throw lastError!
}
```

### Model Fallback

If a larger model fails to load due to memory:

```swift
func loadBestAvailableModel() throws {
    let models = ["whisper-small", "whisper-base", "whisper-tiny"]

    for modelId in models {
        let path = modelDir.appendingPathComponent("\(modelId).bin").path
        guard FileManager.default.fileExists(atPath: path) else { continue }

        do {
            try transcriber.loadModel(modelPath: path)
            print("Loaded model: \(modelId)")
            return
        } catch LexiconFfiError.ModelLoadFailed(let msg) where msg.contains("memory") {
            print("Model \(modelId) too large, trying smaller...")
            continue
        }
    }

    throw LexiconFfiError.ModelNotFound("No suitable model found")
}
```

### Graceful Degradation

When transcription fails, provide partial results or alternatives:

```swift
func transcribeSafely(audio: Data) -> String {
    do {
        let result = try transcriber.transcribe(audioData: audio, config: config)
        return result.segments.map(\.text).joined(separator: " ")
    } catch LexiconFfiError.TranscriptionFailed {
        // Log for debugging
        print("Transcription failed, audio may be corrupt or too short")
        return "[Transcription unavailable]"
    } catch {
        return "[Error: \(error.localizedDescription)]"
    }
}
```

## Common Error Scenarios

### Scenario: App First Launch

```swift
func initializeOnFirstLaunch() async throws {
    // 1. Initialize library
    do {
        try initLexicon(dataDir: dataDir)
    } catch LexiconFfiError.InitializationFailed(let msg) {
        // Check permissions
        if msg.contains("permission") {
            throw AppError.permissionDenied
        }
        throw AppError.initFailed(msg)
    }

    // 2. Check for models
    let context = try LexiconContext(config: config)
    let models = try context.listModels()

    if !models.contains(where: { $0.downloaded }) {
        // No models - need to download
        await presentModelDownloadScreen()
    }
}
```

### Scenario: Background Transcription

```swift
func transcribeInBackground(audioURL: URL) {
    let taskId = UIApplication.shared.beginBackgroundTask {
        // Background time expired
        self.handleBackgroundExpiration()
    }

    Task {
        defer { UIApplication.shared.endBackgroundTask(taskId) }

        do {
            let audio = try loadAudio(from: audioURL)
            let result = try transcriber.transcribe(audioData: audio, config: config)
            await saveResult(result)
        } catch LexiconFfiError.TranscriptionFailed {
            // Save for later retry
            await queueForRetry(audioURL)
        } catch {
            await logError(error)
        }
    }
}
```

### Scenario: Memory Pressure

```swift
func handleMemoryWarning() {
    // Unload model to free memory
    transcriber.unloadModel()

    // Notify user
    showAlert("Memory low. Transcription paused.")

    // Track state
    modelUnloadedDueToMemory = true
}

func resumeAfterMemoryWarning() {
    guard modelUnloadedDueToMemory else { return }

    // Try to reload with smaller model
    do {
        try loadBestAvailableModel()
        modelUnloadedDueToMemory = false
    } catch {
        // Still not enough memory
        showAlert("Please close other apps and try again.")
    }
}
```

## Logging Errors

### Structured Logging

```swift
func logError(_ error: Error, context: [String: Any] = [:]) {
    var info = context
    info["timestamp"] = Date().timeIntervalSince1970
    info["device"] = UIDevice.current.model
    info["osVersion"] = UIDevice.current.systemVersion

    if let lexiconError = error as? LexiconFfiError {
        info["errorType"] = String(describing: type(of: lexiconError))
        info["errorMessage"] = lexiconError.localizedDescription
    }

    // Send to analytics/crash reporting
    Analytics.logError(info)
}
```

### Debug Information

When reporting issues, include:

1. **Platform info:** `getPlatformInfo()` result
2. **Model info:** Which model was being used
3. **Audio info:** Duration, format, source
4. **Error details:** Full error message
5. **Reproduction steps:** What led to the error

```swift
func generateDebugReport(error: Error) -> String {
    let platform = context.getPlatformInfo()
    return """
    === Lexicon Debug Report ===
    Version: \(getVersion())
    Platform: \(platform.platform)
    Memory: \(platform.availableMemoryMb) MB
    GPU: \(platform.hasGpu)

    Error Type: \(type(of: error))
    Error: \(error)

    Model: \(transcriber.getModelName() ?? "none")
    ===========================
    """
}
```
