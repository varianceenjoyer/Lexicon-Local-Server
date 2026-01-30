# Flutter/Dart Integration Guide

## Overview

Lexicon provides Flutter bindings via `flutter_rust_bridge`. This enables cross-platform development for iOS, Android, and desktop platforms from a single Dart codebase.

## Installation

### Package Setup

1. Build the library:
   ```bash
   ./scripts/build-flutter-android.sh
   ```

2. The build produces:
   - Native libraries for each platform
   - Generated Dart bindings in `lib/src/rust/`

3. Add to your `pubspec.yaml`:
   ```yaml
   dependencies:
     lexicon:
       path: ../path/to/lexicon-flutter
   ```

### Platform Configuration

**iOS (ios/Podfile):**
```ruby
platform :ios, '13.0'
```

**Android (android/app/build.gradle):**
```gradle
android {
    defaultConfig {
        minSdkVersion 21
        ndk {
            abiFilters 'arm64-v8a', 'armeabi-v7a', 'x86_64'
        }
    }
}
```

## API Reference

### Initialization

```dart
import 'package:lexicon/lexicon.dart';

// Initialize flutter_rust_bridge runtime (call in main)
void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(MyApp());
}

// Initialize Lexicon
Future<void> initializeLexicon() async {
  try {
    final appDir = await getApplicationDocumentsDirectory();
    initLexicon(dataDir: appDir.path);
  } catch (e) {
    print('Initialization failed: $e');
  }
}

// Get library version
String version = getVersion();
print('Lexicon version: $version');
```

### Platform Info

```dart
// Get platform information (sync call)
PlatformInfo platformInfo = getPlatformInfo();
print('Platform: ${platformInfo.platform}');
print('Recommended tier: ${platformInfo.modelTier}');
print('Available memory: ${platformInfo.availableMemoryMb} MB');
print('Has GPU: ${platformInfo.hasGpu}');
```

### Model Management

```dart
// List available models
Future<void> showModels() async {
  final modelDir = '${appDir.path}/models';
  final models = await listModels(modelDir: modelDir);

  for (final model in models) {
    print('${model.id}: ${model.name}');
    print('  Size: ${model.sizeBytes} bytes');
    print('  Downloaded: ${model.downloaded}');
    if (model.localPath != null) {
      print('  Path: ${model.localPath}');
    }
  }
}

// Check if model is downloaded (sync)
bool downloaded = isModelDownloaded(
  modelDir: modelDir,
  modelId: 'whisper-base',
);

// Delete a model
await deleteModel(modelDir: modelDir, modelId: 'whisper-base');
```

### Transcriber

Speech-to-text transcription with opaque Rust object.

```dart
class TranscriptionService {
  late Transcriber _transcriber;
  bool _isLoaded = false;

  Future<void> initialize(String modelPath) async {
    _transcriber = await Transcriber.newInstance();
    await _transcriber.loadModel(modelPath: modelPath);
    _isLoaded = true;
  }

  bool get isReady => _isLoaded && _transcriber.isModelLoaded();

  String? get modelName => _transcriber.getModelName();

  Future<TranscriptionResult> transcribe(Uint8List audioData, {
    String? language,
    bool translate = false,
    int numThreads = 4,
    String? initialPrompt,
    double temperature = 0.0,
  }) async {
    final config = TranscriptionConfig(
      language: language,
      translate: translate,
      numThreads: numThreads,
      initialPrompt: initialPrompt,
      temperature: temperature,
    );

    return await _transcriber.transcribe(
      audioData: audioData,
      config: config,
    );
  }

  Future<TranscriptionResult> translateToEnglish(Uint8List audioData) async {
    final config = TranscriptionConfig(
      translate: true,
      numThreads: 4,
      temperature: 0.0,
    );

    return await _transcriber.translate(
      audioData: audioData,
      config: config,
    );
  }

  void unload() {
    _transcriber.unloadModel();
    _isLoaded = false;
  }
}

// Usage
final service = TranscriptionService();
await service.initialize('${modelDir}/whisper-base.bin');

final result = await service.transcribe(audioData);
print('Detected language: ${result.detectedLanguage}');
print('Duration: ${result.audioDurationMs} ms');
print('Processing: ${result.processingTimeMs} ms');

for (final segment in result.segments) {
  print('[${segment.startMs}-${segment.endMs}] ${segment.text}');
  if (segment.confidence != null) {
    print('  Confidence: ${segment.confidence}');
  }
}
```

### Synthesizer

Text-to-speech synthesis.

```dart
class TtsService {
  late Synthesizer _synthesizer;

  Future<void> initialize() async {
    _synthesizer = await Synthesizer.newInstance();
  }

  bool get isReady => _synthesizer.isReady();

  List<String> get voices => _synthesizer.listVoices();

  Future<SynthesisResult> speak(String text, {
    String? language,
    String? voiceId,
    double rate = 1.0,
    double pitch = 0.0,
    double volume = 1.0,
    bool useKokoro = true,
  }) async {
    final config = TtsConfig(
      language: language,
      voiceId: voiceId,
      rate: rate,
      pitch: pitch,
      volume: volume,
      useKokoro: useKokoro,
    );

    return await _synthesizer.synthesize(text: text, config: config);
  }
}

// Usage
final tts = TtsService();
await tts.initialize();

final result = await tts.speak('Hello world!');
print('Sample rate: ${result.sampleRate}');
print('Duration: ${result.durationMs} ms');
print('Engine: ${result.engine}');

// Play audio (result.audioData is f32 PCM bytes)
await playAudio(result.audioData, result.sampleRate);
```

### Streaming Transcription

Real-time transcription with chunk-based processing.

```dart
class StreamingTranscriptionService {
  late StreamingTranscriber _transcriber;

  Future<void> initialize(String modelPath) async {
    _transcriber = await StreamingTranscriber.newInstance();
    await _transcriber.loadModel(modelPath: modelPath);
  }

  bool get isReady => _transcriber.isReady();
  bool get isStreaming => _transcriber.isStreaming();

  Future<List<TranscriptionSegment>> processChunk(
    Uint8List audioData,
    int timestampMs, {
    bool isFinal = false,
    String? language,
    bool translate = false,
  }) async {
    final chunk = AudioChunk(
      data: audioData,
      timestampMs: timestampMs,
      isFinal: isFinal,
    );

    final config = TranscriptionConfig(
      language: language,
      translate: translate,
      numThreads: 4,
      temperature: 0.0,
    );

    return await _transcriber.processChunk(chunk: chunk, config: config);
  }

  void stop() {
    _transcriber.stop();
  }
}

// Usage with microphone
class MicrophoneTranscriber extends StatefulWidget {
  @override
  _MicrophoneTranscriberState createState() => _MicrophoneTranscriberState();
}

class _MicrophoneTranscriberState extends State<MicrophoneTranscriber> {
  final _service = StreamingTranscriptionService();
  final _text = StringBuffer();
  int _timestampMs = 0;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    await _service.initialize(modelPath);
    // Start microphone capture
    startMicrophoneCapture(_onAudioChunk);
  }

  void _onAudioChunk(Uint8List audioData) async {
    final segments = await _service.processChunk(
      audioData,
      _timestampMs,
    );

    setState(() {
      for (final segment in segments) {
        _text.write(segment.text);
        _text.write(' ');
      }
    });

    // Advance timestamp (assuming 100ms chunks)
    _timestampMs += 100;
  }

  @override
  Widget build(BuildContext context) {
    return Text(_text.toString());
  }
}
```

### Streaming TTS

Chunked audio synthesis for progressive playback.

```dart
class StreamingTtsService {
  late StreamingSynthesizer _synthesizer;

  Future<void> initialize() async {
    _synthesizer = await StreamingSynthesizer.newInstance();
  }

  bool get isReady => _synthesizer.isReady();
  bool get isSynthesizing => _synthesizer.isSynthesizing();

  Future<List<TtsAudioChunk>> synthesizeChunked(
    String text, {
    int chunkMs = 100,
    String? language,
    double rate = 1.0,
    double pitch = 0.0,
    double volume = 1.0,
    bool useKokoro = true,
  }) async {
    final config = TtsConfig(
      language: language,
      rate: rate,
      pitch: pitch,
      volume: volume,
      useKokoro: useKokoro,
    );

    return await _synthesizer.synthesizeChunked(
      text: text,
      config: config,
      chunkMs: chunkMs,
    );
  }

  void stop() {
    _synthesizer.stop();
  }
}

// Usage - progressive playback
final tts = StreamingTtsService();
await tts.initialize();

final chunks = await tts.synthesizeChunked('This is a long sentence...');

for (final chunk in chunks) {
  // Play each chunk immediately
  await playAudioChunk(chunk.data, chunk.sampleRate);

  if (chunk.isFinal) {
    print('Synthesis complete');
  }
}
```

### Model Downloader

Async model downloading with progress tracking.

```dart
class ModelDownloadService {
  late ModelDownloader _downloader;

  Future<void> initialize(String modelDir) async {
    _downloader = await ModelDownloader.newInstance(modelDir: modelDir);
  }

  String get modelDir => _downloader.getModelDir();
  bool get isCancelled => _downloader.isCancelled();

  void cancel() {
    _downloader.cancel();
  }

  void reset() {
    _downloader.reset();
  }

  Stream<DownloadEvent> downloadModel(String modelId) async* {
    final events = await _downloader.downloadModel(modelId: modelId);

    for (final event in events) {
      yield event;
    }
  }
}

// Usage with progress UI
class DownloadScreen extends StatefulWidget {
  @override
  _DownloadScreenState createState() => _DownloadScreenState();
}

class _DownloadScreenState extends State<DownloadScreen> {
  final _service = ModelDownloadService();
  double _progress = 0.0;
  String _status = 'Ready';

  Future<void> _startDownload() async {
    await _service.initialize(modelDir);

    await for (final event in _service.downloadModel('whisper-base')) {
      if (event is DownloadEventProgress) {
        final progress = event.field0;
        setState(() {
          _progress = progress.downloadedBytes / progress.totalBytes;
          _status = 'Downloading: ${(progress.speedBps / 1024).toStringAsFixed(0)} KB/s';
          if (progress.etaSeconds != null) {
            _status += ' - ${progress.etaSeconds}s remaining';
          }
        });
      } else if (event is DownloadEventComplete) {
        setState(() {
          _progress = 1.0;
          _status = 'Complete: ${event.path}';
        });
      } else if (event is DownloadEventError) {
        setState(() {
          _status = 'Error: ${event.error}';
        });
      } else if (event is DownloadEventCancelled) {
        setState(() {
          _status = 'Cancelled';
        });
      }
    }
  }

  void _cancelDownload() {
    _service.cancel();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        LinearProgressIndicator(value: _progress),
        Text(_status),
        Row(
          children: [
            ElevatedButton(
              onPressed: _startDownload,
              child: Text('Download'),
            ),
            ElevatedButton(
              onPressed: _cancelDownload,
              child: Text('Cancel'),
            ),
          ],
        ),
      ],
    );
  }
}
```

## Error Handling

Errors are thrown as `LexiconFlutterError` variants:

```dart
// LexiconFlutterError variants:
// - LexiconFlutterError.InitializationFailed(String)
// - LexiconFlutterError.ConfigError(String)
// - LexiconFlutterError.ModelNotFound(String)
// - LexiconFlutterError.ModelLoadFailed(String)
// - LexiconFlutterError.TranscriptionFailed(String)
// - LexiconFlutterError.SynthesisFailed(String)
// - LexiconFlutterError.InvalidInput(String)
// - LexiconFlutterError.IoError(String)
// - LexiconFlutterError.DownloadFailed(String)
// - LexiconFlutterError.Cancelled

// Handle errors using pattern matching
try {
  await transcriber.loadModel(modelPath: path);
} on LexiconFlutterError catch (e) {
  if (e is LexiconFlutterErrorModelNotFound) {
    // Show download dialog
    showDownloadDialog();
  } else if (e is LexiconFlutterErrorModelLoadFailed) {
    // Model corrupted
    showErrorDialog('Model file may be corrupted: ${e.field0}');
  } else if (e is LexiconFlutterErrorCancelled) {
    // Operation cancelled
    showSnackBar('Operation cancelled');
  } else {
    showErrorDialog(e.toString());
  }
}
```

## Audio Preparation

Convert audio to required format (16kHz mono f32 PCM):

```dart
import 'dart:typed_data';
import 'package:ffmpeg_kit_flutter/ffmpeg_kit.dart';

Future<Uint8List> prepareAudioData(String inputPath) async {
  // Use FFmpeg to convert to raw PCM
  final outputPath = '${Directory.systemTemp.path}/audio_16k.pcm';

  await FFmpegKit.execute(
    '-i "$inputPath" '
    '-ar 16000 '      // 16kHz sample rate
    '-ac 1 '          // Mono
    '-f f32le '       // 32-bit float little-endian
    '-y "$outputPath"'
  );

  final file = File(outputPath);
  return await file.readAsBytes();
}

// Alternative: manual resampling
Uint8List convertToF32PCM(List<int> int16Samples, int sampleRate) {
  // Resample to 16kHz
  final ratio = 16000 / sampleRate;
  final newLength = (int16Samples.length * ratio).toInt();

  final floats = Float32List(newLength);
  for (int i = 0; i < newLength; i++) {
    final srcIndex = (i / ratio).floor();
    // Convert int16 to float (-1.0 to 1.0)
    floats[i] = int16Samples[srcIndex] / 32768.0;
  }

  return floats.buffer.asUint8List();
}
```

## Real-time Microphone Recording

Using `record` package:

```dart
import 'package:record/record.dart';

class MicrophoneCapture {
  final _recorder = AudioRecorder();
  final _transcriber = StreamingTranscriptionService();
  final _buffer = <int>[];
  int _timestampMs = 0;

  Future<void> start(Function(String) onText) async {
    await _transcriber.initialize(modelPath);

    // Configure recorder for 16kHz mono
    await _recorder.start(
      RecordConfig(
        encoder: AudioEncoder.pcm16bits,
        sampleRate: 16000,
        numChannels: 1,
      ),
      path: '', // Stream mode
    );

    // Process audio stream
    final stream = await _recorder.startStream(
      RecordConfig(
        encoder: AudioEncoder.pcm16bits,
        sampleRate: 16000,
        numChannels: 1,
      ),
    );

    stream.listen((data) async {
      _buffer.addAll(data);

      // Process every 3 seconds (48000 samples at 16kHz int16)
      if (_buffer.length >= 96000) {
        final audioData = _convertToF32(_buffer.take(96000).toList());
        _buffer.removeRange(0, 96000);

        final segments = await _transcriber.processChunk(
          audioData,
          _timestampMs,
        );

        for (final segment in segments) {
          onText(segment.text);
        }

        _timestampMs += 3000;
      }
    });
  }

  Future<void> stop() async {
    await _recorder.stop();
    _transcriber.stop();
  }

  Uint8List _convertToF32(List<int> int16Bytes) {
    // Convert byte pairs to int16, then to float
    final samples = <double>[];
    for (int i = 0; i < int16Bytes.length - 1; i += 2) {
      final int16 = (int16Bytes[i + 1] << 8) | int16Bytes[i];
      final signed = int16 > 32767 ? int16 - 65536 : int16;
      samples.add(signed / 32768.0);
    }

    final floats = Float32List.fromList(samples.map((e) => e.toDouble()).toList());
    return floats.buffer.asUint8List();
  }
}
```

## Best Practices

### State Management

```dart
// Using Riverpod
final transcriptionServiceProvider = Provider((ref) {
  final service = TranscriptionService();
  ref.onDispose(() => service.unload());
  return service;
});

final transcriptionStateProvider = StateNotifierProvider<TranscriptionNotifier, TranscriptionState>((ref) {
  final service = ref.watch(transcriptionServiceProvider);
  return TranscriptionNotifier(service);
});

class TranscriptionNotifier extends StateNotifier<TranscriptionState> {
  final TranscriptionService _service;

  TranscriptionNotifier(this._service) : super(TranscriptionState.initial());

  Future<void> initialize(String modelPath) async {
    state = state.copyWith(status: TranscriptionStatus.loading);
    try {
      await _service.initialize(modelPath);
      state = state.copyWith(status: TranscriptionStatus.ready);
    } catch (e) {
      state = state.copyWith(status: TranscriptionStatus.error, error: e.toString());
    }
  }

  Future<void> transcribe(Uint8List audioData) async {
    state = state.copyWith(status: TranscriptionStatus.processing);
    try {
      final result = await _service.transcribe(audioData);
      state = state.copyWith(
        status: TranscriptionStatus.ready,
        lastResult: result,
      );
    } catch (e) {
      state = state.copyWith(status: TranscriptionStatus.error, error: e.toString());
    }
  }
}
```

### Memory Management

```dart
class TranscriptionScreen extends StatefulWidget {
  @override
  _TranscriptionScreenState createState() => _TranscriptionScreenState();
}

class _TranscriptionScreenState extends State<TranscriptionScreen> with WidgetsBindingObserver {
  final _service = TranscriptionService();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _service.initialize(modelPath);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _service.unload();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused) {
      // Unload model when app goes to background
      _service.unload();
    } else if (state == AppLifecycleState.resumed) {
      // Reload model when app returns
      _service.initialize(modelPath);
    }
  }
}
```

### Model Selection

```dart
String selectAppropriateModel() {
  final platform = getPlatformInfo();

  switch (platform.modelTier) {
    case 'MobileLow':
      return 'whisper-tiny';
    case 'MobileMid':
      return 'whisper-base';
    case 'Desktop':
      return 'whisper-small';
    case 'Server':
      return 'whisper-medium';
    default:
      return 'whisper-base';
  }
}
```

## Troubleshooting

### Common Issues

**"Failed to load dynamic library":**
- Ensure native libraries are properly bundled
- Check platform-specific build configuration
- Verify ABI filters on Android

**"Model not found" error:**
- Verify model file path is correct
- Check file exists and is readable
- Ensure model was fully downloaded

**High memory usage:**
- Use smaller model (tiny/base)
- Unload model when not in use
- Process shorter audio chunks

**Slow transcription:**
- Reduce `numThreads` on older devices
- Use smaller model
- Consider processing in isolate

**Audio format errors:**
- Verify 16kHz sample rate
- Ensure mono channel
- Check f32 PCM format (Float32List)
- Verify little-endian byte order
