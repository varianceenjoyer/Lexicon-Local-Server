# Audio Format Requirements

## Required Format

All Lexicon APIs expect audio in the following format:

| Property | Value |
|----------|-------|
| **Sample Rate** | 16,000 Hz (16 kHz) |
| **Channels** | 1 (Mono) |
| **Bit Depth** | 32-bit |
| **Sample Format** | Float (-1.0 to 1.0) |
| **Byte Order** | Little-endian |

## Data Layout

Audio data is a continuous array of 32-bit float samples:

```
┌──────────────────┬──────────────────┬──────────────────┬─────┐
│    Sample 0      │    Sample 1      │    Sample 2      │ ... │
│   (4 bytes)      │   (4 bytes)      │   (4 bytes)      │     │
│  float32 LE      │  float32 LE      │  float32 LE      │     │
└──────────────────┴──────────────────┴──────────────────┴─────┘
```

Each sample is a IEEE 754 single-precision float, stored in little-endian byte order.

**Example:** A 1-second audio clip at 16 kHz requires:
- 16,000 samples
- 64,000 bytes (16,000 × 4 bytes per sample)

## Conversion Utilities

### Swift (iOS/macOS)

```swift
import AVFoundation
import Accelerate

/// Converts any audio file to 16kHz mono f32 PCM.
func convertToLexiconFormat(url: URL) throws -> Data {
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
    guard let converter = AVAudioConverter(
        from: file.processingFormat,
        to: targetFormat
    ) else {
        throw AudioError.converterCreationFailed
    }

    // Calculate output length
    let ratio = targetFormat.sampleRate / file.processingFormat.sampleRate
    let outputFrameCount = AVAudioFrameCount(Double(file.length) * ratio)

    // Create output buffer
    guard let outputBuffer = AVAudioPCMBuffer(
        pcmFormat: targetFormat,
        frameCapacity: outputFrameCount
    ) else {
        throw AudioError.bufferCreationFailed
    }

    // Read entire input file
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

    // Extract raw bytes
    let floatChannelData = outputBuffer.floatChannelData![0]
    let frameLength = Int(outputBuffer.frameLength)

    // Create Data from float array
    return Data(bytes: floatChannelData, count: frameLength * MemoryLayout<Float>.size)
}

/// Converts raw PCM buffer from microphone to Lexicon format.
func convertMicrophoneBuffer(_ buffer: AVAudioPCMBuffer) throws -> Data {
    // If already correct format, extract directly
    if buffer.format.sampleRate == 16000 &&
       buffer.format.channelCount == 1 &&
       buffer.format.commonFormat == .pcmFormatFloat32 {
        let data = buffer.floatChannelData![0]
        return Data(bytes: data, count: Int(buffer.frameLength) * 4)
    }

    // Otherwise convert
    guard let targetFormat = AVAudioFormat(
        commonFormat: .pcmFormatFloat32,
        sampleRate: 16000,
        channels: 1,
        interleaved: false
    ) else {
        throw AudioError.formatCreationFailed
    }

    guard let converter = AVAudioConverter(from: buffer.format, to: targetFormat) else {
        throw AudioError.converterCreationFailed
    }

    let ratio = 16000.0 / buffer.format.sampleRate
    let outputLength = AVAudioFrameCount(Double(buffer.frameLength) * ratio)

    guard let outputBuffer = AVAudioPCMBuffer(pcmFormat: targetFormat, frameCapacity: outputLength) else {
        throw AudioError.bufferCreationFailed
    }

    var error: NSError?
    converter.convert(to: outputBuffer, error: &error) { _, outStatus in
        outStatus.pointee = .haveData
        return buffer
    }

    if let error = error { throw error }

    return Data(bytes: outputBuffer.floatChannelData![0], count: Int(outputBuffer.frameLength) * 4)
}
```

### Kotlin (Android)

```kotlin
import android.media.AudioFormat
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Converts an audio file to 16kHz mono f32 PCM.
 */
fun convertToLexiconFormat(file: File): ByteArray {
    val extractor = MediaExtractor()
    extractor.setDataSource(file.absolutePath)

    // Find audio track
    var audioTrack = -1
    for (i in 0 until extractor.trackCount) {
        val format = extractor.getTrackFormat(i)
        val mime = format.getString(MediaFormat.KEY_MIME) ?: continue
        if (mime.startsWith("audio/")) {
            audioTrack = i
            break
        }
    }
    require(audioTrack >= 0) { "No audio track found" }

    extractor.selectTrack(audioTrack)
    val format = extractor.getTrackFormat(audioTrack)
    val sampleRate = format.getInteger(MediaFormat.KEY_SAMPLE_RATE)
    val channelCount = format.getInteger(MediaFormat.KEY_CHANNEL_COUNT)

    // Decode to PCM
    val decodedSamples = decodeAudioTrack(extractor, format)
    extractor.release()

    // Resample to 16kHz
    val resampled = if (sampleRate != 16000) {
        resample(decodedSamples, sampleRate, 16000)
    } else {
        decodedSamples
    }

    // Convert to mono
    val mono = if (channelCount > 1) {
        convertToMono(resampled, channelCount)
    } else {
        resampled
    }

    // Convert to bytes (little-endian f32)
    return floatArrayToBytes(mono)
}

private fun decodeAudioTrack(extractor: MediaExtractor, format: MediaFormat): FloatArray {
    val mime = format.getString(MediaFormat.KEY_MIME)!!
    val decoder = MediaCodec.createDecoderByType(mime)
    decoder.configure(format, null, null, 0)
    decoder.start()

    val samples = mutableListOf<Float>()
    val bufferInfo = MediaCodec.BufferInfo()
    var sawEOS = false

    while (!sawEOS) {
        // Feed input
        val inputIndex = decoder.dequeueInputBuffer(10000)
        if (inputIndex >= 0) {
            val inputBuffer = decoder.getInputBuffer(inputIndex)!!
            val sampleSize = extractor.readSampleData(inputBuffer, 0)

            if (sampleSize < 0) {
                decoder.queueInputBuffer(inputIndex, 0, 0, 0,
                    MediaCodec.BUFFER_FLAG_END_OF_STREAM)
                sawEOS = true
            } else {
                decoder.queueInputBuffer(inputIndex, 0, sampleSize,
                    extractor.sampleTime, 0)
                extractor.advance()
            }
        }

        // Get output
        val outputIndex = decoder.dequeueOutputBuffer(bufferInfo, 10000)
        if (outputIndex >= 0) {
            val outputBuffer = decoder.getOutputBuffer(outputIndex)!!
            outputBuffer.order(ByteOrder.LITTLE_ENDIAN)

            // Assuming 16-bit PCM output
            val shortBuffer = outputBuffer.asShortBuffer()
            while (shortBuffer.hasRemaining()) {
                val sample = shortBuffer.get()
                samples.add(sample.toFloat() / 32768f)
            }

            decoder.releaseOutputBuffer(outputIndex, false)

            if (bufferInfo.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0) {
                break
            }
        }
    }

    decoder.stop()
    decoder.release()

    return samples.toFloatArray()
}

private fun resample(samples: FloatArray, fromRate: Int, toRate: Int): FloatArray {
    val ratio = toRate.toDouble() / fromRate
    val newLength = (samples.size * ratio).toInt()
    val result = FloatArray(newLength)

    for (i in result.indices) {
        val srcIndex = i / ratio
        val srcIndexInt = srcIndex.toInt()
        val frac = (srcIndex - srcIndexInt).toFloat()

        result[i] = if (srcIndexInt + 1 < samples.size) {
            samples[srcIndexInt] * (1 - frac) + samples[srcIndexInt + 1] * frac
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

private fun floatArrayToBytes(samples: FloatArray): ByteArray {
    val buffer = ByteBuffer.allocate(samples.size * 4)
    buffer.order(ByteOrder.LITTLE_ENDIAN)
    samples.forEach { buffer.putFloat(it) }
    return buffer.array()
}

/**
 * Converts AudioRecord buffer to Lexicon format.
 */
fun convertAudioRecordBuffer(
    buffer: FloatArray,  // From ENCODING_PCM_FLOAT
    sampleRate: Int
): ByteArray {
    // Resample if needed
    val resampled = if (sampleRate != 16000) {
        resample(buffer, sampleRate, 16000)
    } else {
        buffer
    }

    return floatArrayToBytes(resampled)
}

/**
 * Converts int16 PCM to Lexicon format.
 */
fun convertInt16ToLexiconFormat(
    int16Data: ByteArray,
    sampleRate: Int,
    channels: Int
): ByteArray {
    // Convert bytes to shorts
    val shortBuffer = ByteBuffer.wrap(int16Data)
        .order(ByteOrder.LITTLE_ENDIAN)
        .asShortBuffer()

    val samples = FloatArray(shortBuffer.remaining())
    for (i in samples.indices) {
        samples[i] = shortBuffer.get().toFloat() / 32768f
    }

    // Convert to mono
    val mono = if (channels > 1) {
        convertToMono(samples, channels)
    } else {
        samples
    }

    // Resample
    val resampled = if (sampleRate != 16000) {
        resample(mono, sampleRate, 16000)
    } else {
        mono
    }

    return floatArrayToBytes(resampled)
}
```

### Dart/Flutter

```dart
import 'dart:typed_data';

/// Converts int16 PCM bytes to Lexicon format.
Uint8List convertInt16ToLexiconFormat(
  Uint8List int16Data,
  int sampleRate,
  int channels,
) {
  // Convert bytes to int16 samples
  final int16List = Int16List.view(int16Data.buffer);
  final samples = Float32List(int16List.length);

  for (int i = 0; i < int16List.length; i++) {
    samples[i] = int16List[i] / 32768.0;
  }

  // Convert to mono if needed
  Float32List mono;
  if (channels > 1) {
    final monoLength = samples.length ~/ channels;
    mono = Float32List(monoLength);
    for (int i = 0; i < monoLength; i++) {
      double sum = 0;
      for (int c = 0; c < channels; c++) {
        sum += samples[i * channels + c];
      }
      mono[i] = sum / channels;
    }
  } else {
    mono = samples;
  }

  // Resample to 16kHz if needed
  Float32List resampled;
  if (sampleRate != 16000) {
    final ratio = 16000 / sampleRate;
    final newLength = (mono.length * ratio).toInt();
    resampled = Float32List(newLength);

    for (int i = 0; i < newLength; i++) {
      final srcIndex = i / ratio;
      final srcIndexInt = srcIndex.toInt();
      final frac = srcIndex - srcIndexInt;

      if (srcIndexInt + 1 < mono.length) {
        resampled[i] = (mono[srcIndexInt] * (1 - frac) +
            mono[srcIndexInt + 1] * frac);
      } else {
        resampled[i] = mono[srcIndexInt];
      }
    }
  } else {
    resampled = mono;
  }

  // Return as bytes
  return resampled.buffer.asUint8List();
}

/// Converts float samples to Lexicon format bytes.
Uint8List floatSamplesToBytes(List<double> samples) {
  final float32List = Float32List.fromList(samples);
  return float32List.buffer.asUint8List();
}
```

### JavaScript (Web)

```javascript
/**
 * Converts AudioBuffer to Lexicon format (16kHz mono f32 PCM).
 * @param {AudioBuffer} audioBuffer - Input audio buffer
 * @returns {ArrayBuffer} - Raw f32 PCM bytes
 */
async function convertToLexiconFormat(audioBuffer) {
  // Create offline context at 16kHz
  const offlineContext = new OfflineAudioContext(
    1,  // mono
    Math.ceil(audioBuffer.duration * 16000),
    16000
  );

  // Create source
  const source = offlineContext.createBufferSource();
  source.buffer = audioBuffer;
  source.connect(offlineContext.destination);
  source.start();

  // Render
  const renderedBuffer = await offlineContext.startRendering();

  // Extract channel data (already Float32Array)
  const channelData = renderedBuffer.getChannelData(0);

  // Return as ArrayBuffer (already little-endian on most systems)
  return channelData.buffer;
}

/**
 * Converts file to Lexicon format.
 * @param {File} file - Audio file
 * @returns {ArrayBuffer} - Raw f32 PCM bytes
 */
async function fileToLexiconFormat(file) {
  const audioContext = new AudioContext();
  const arrayBuffer = await file.arrayBuffer();
  const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);
  return convertToLexiconFormat(audioBuffer);
}

/**
 * Converts Base64 to Lexicon format for HTTP API.
 * @param {ArrayBuffer} buffer - f32 PCM buffer
 * @returns {string} - Base64 encoded string
 */
function bufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

// Real-time recording example
class LexiconRecorder {
  constructor() {
    this.audioContext = null;
    this.mediaStream = null;
    this.processor = null;
    this.chunks = [];
  }

  async start(onChunk) {
    this.audioContext = new AudioContext({ sampleRate: 16000 });
    this.mediaStream = await navigator.mediaDevices.getUserMedia({
      audio: {
        channelCount: 1,
        sampleRate: 16000,
        echoCancellation: true,
        noiseSuppression: true
      }
    });

    const source = this.audioContext.createMediaStreamSource(this.mediaStream);

    // Using AudioWorklet for better performance
    await this.audioContext.audioWorklet.addModule('audio-processor.js');
    this.processor = new AudioWorkletNode(this.audioContext, 'audio-processor');

    this.processor.port.onmessage = (event) => {
      const float32Data = event.data;
      onChunk(float32Data.buffer);
    };

    source.connect(this.processor);
    this.processor.connect(this.audioContext.destination);
  }

  stop() {
    if (this.processor) {
      this.processor.disconnect();
    }
    if (this.mediaStream) {
      this.mediaStream.getTracks().forEach(track => track.stop());
    }
    if (this.audioContext) {
      this.audioContext.close();
    }
  }
}

// audio-processor.js (AudioWorklet)
// class AudioProcessor extends AudioWorkletProcessor {
//   process(inputs, outputs) {
//     const input = inputs[0];
//     if (input.length > 0) {
//       const channelData = input[0];
//       this.port.postMessage(new Float32Array(channelData));
//     }
//     return true;
//   }
// }
// registerProcessor('audio-processor', AudioProcessor);
```

### Python

```python
import numpy as np
from scipy.io import wavfile
from scipy.signal import resample
import base64

def convert_to_lexicon_format(filepath: str) -> bytes:
    """
    Convert any audio file to Lexicon format (16kHz mono f32 PCM).

    Args:
        filepath: Path to audio file

    Returns:
        Raw bytes of f32 PCM samples
    """
    # Read audio file
    sample_rate, audio = wavfile.read(filepath)

    # Convert to float32 normalized to [-1, 1]
    if audio.dtype == np.int16:
        audio = audio.astype(np.float32) / 32768.0
    elif audio.dtype == np.int32:
        audio = audio.astype(np.float32) / 2147483648.0
    elif audio.dtype == np.uint8:
        audio = (audio.astype(np.float32) - 128) / 128.0
    elif audio.dtype != np.float32:
        audio = audio.astype(np.float32)

    # Convert to mono if stereo
    if len(audio.shape) > 1:
        audio = np.mean(audio, axis=1)

    # Resample to 16kHz if needed
    if sample_rate != 16000:
        num_samples = int(len(audio) * 16000 / sample_rate)
        audio = resample(audio, num_samples)

    # Ensure float32
    audio = audio.astype(np.float32)

    # Return raw bytes (numpy is little-endian by default on x86)
    return audio.tobytes()


def to_base64(audio_bytes: bytes) -> str:
    """Convert raw audio bytes to base64 for HTTP API."""
    return base64.b64encode(audio_bytes).decode('utf-8')


def from_base64(base64_str: str) -> np.ndarray:
    """Convert base64 string back to float32 array."""
    audio_bytes = base64.b64decode(base64_str)
    return np.frombuffer(audio_bytes, dtype=np.float32)


# Example usage
if __name__ == '__main__':
    import requests

    # Convert audio file
    audio_bytes = convert_to_lexicon_format('recording.wav')
    audio_base64 = to_base64(audio_bytes)

    # Send to API
    response = requests.post(
        'http://localhost:3000/api/v1/transcribe',
        json={
            'audio': audio_base64,
            'language': 'en'
        }
    )

    result = response.json()
    for segment in result['segments']:
        print(f"[{segment['start_ms']}-{segment['end_ms']}] {segment['text']}")
```

## Format Verification

To verify your audio is in the correct format:

```python
import numpy as np

def verify_lexicon_format(audio_bytes: bytes) -> dict:
    """Verify audio is in correct Lexicon format."""
    # Should be divisible by 4 (f32 = 4 bytes)
    if len(audio_bytes) % 4 != 0:
        return {'valid': False, 'error': 'Length not divisible by 4'}

    # Parse as float32
    samples = np.frombuffer(audio_bytes, dtype=np.float32)

    # Check range
    min_val, max_val = samples.min(), samples.max()
    in_range = -1.0 <= min_val and max_val <= 1.0

    # Calculate duration at 16kHz
    duration_ms = len(samples) * 1000 / 16000

    return {
        'valid': in_range,
        'num_samples': len(samples),
        'duration_ms': duration_ms,
        'min_value': float(min_val),
        'max_value': float(max_val),
        'error': None if in_range else 'Sample values out of range [-1, 1]'
    }
```

## Common Issues

### Issue: Audio too quiet or too loud

Normalize audio to optimal levels:

```python
def normalize_audio(samples: np.ndarray, target_db: float = -20.0) -> np.ndarray:
    """Normalize audio to target dB level."""
    rms = np.sqrt(np.mean(samples ** 2))
    if rms == 0:
        return samples

    current_db = 20 * np.log10(rms)
    gain = 10 ** ((target_db - current_db) / 20)

    # Apply gain with clipping prevention
    normalized = samples * gain
    return np.clip(normalized, -1.0, 1.0)
```

### Issue: DC offset

Remove DC offset before processing:

```python
def remove_dc_offset(samples: np.ndarray) -> np.ndarray:
    """Remove DC offset from audio."""
    return samples - np.mean(samples)
```

### Issue: Resampling artifacts

Use high-quality resampling for best results:

```python
from scipy.signal import resample_poly

def high_quality_resample(samples: np.ndarray, from_rate: int, to_rate: int) -> np.ndarray:
    """High-quality resampling using polyphase filter."""
    from math import gcd
    g = gcd(from_rate, to_rate)
    up = to_rate // g
    down = from_rate // g
    return resample_poly(samples, up, down)
```
