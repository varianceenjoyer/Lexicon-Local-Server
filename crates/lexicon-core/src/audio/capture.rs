//! Audio Capture using cpal.
//!
//! This module provides cross-platform audio capture using the cpal crate,
//! with automatic resampling to 16kHz and optional voice activity detection.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;

use super::{AudioChunk, AudioConfig, AudioSource, Resampler, VoiceActivityDetector};
use super::TARGET_SAMPLE_RATE;
use crate::error::{AudioError, LexiconError, Result};

/// Audio source using cpal for cross-platform capture.
///
/// `CpalAudioSource` captures audio from the system's default or specified
/// input device, resamples it to 16kHz mono, and optionally performs voice
/// activity detection on each chunk.
pub struct CpalAudioSource {
    config: AudioConfig,
    device: Option<Device>,
    stream: Option<Stream>,

    // Ring buffer for audio data (producer is in stream callback, consumer is read_chunk)
    consumer: Option<ringbuf::HeapCons<f32>>,

    // Shared state between stream callback and main thread
    running: Arc<AtomicBool>,
    error_flag: Arc<AtomicBool>,
    timestamp_ms: Arc<AtomicI64>,

    // Processing components
    resampler: Option<Resampler>,
    vad: Option<VoiceActivityDetector>,

    // Stream parameters
    input_sample_rate: u32,
    input_channels: u16,
}

impl CpalAudioSource {
    /// Creates a new audio source with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No audio host is available
    /// - The specified device cannot be found
    /// - The device doesn't support any compatible format
    pub fn new(config: AudioConfig) -> Result<Self> {
        let host = cpal::default_host();

        let device = if let Some(ref name) = config.device_name {
            // Find device by name
            host.input_devices()
                .map_err(|e| LexiconError::Audio(AudioError::DeviceEnumerationFailed {
                    message: e.to_string(),
                }))?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .ok_or_else(|| LexiconError::Audio(AudioError::NoInputDevice))?
        } else {
            // Use default device
            host.default_input_device()
                .ok_or_else(|| LexiconError::Audio(AudioError::NoInputDevice))?
        };

        // Get supported config to determine input sample rate
        let supported_config = device
            .default_input_config()
            .map_err(|e| LexiconError::Audio(AudioError::UnsupportedFormat {
                format: e.to_string(),
            }))?;

        let input_sample_rate = supported_config.sample_rate().0;
        let input_channels = supported_config.channels();

        // Initialize VAD if enabled
        let vad = if config.vad_enabled {
            Some(VoiceActivityDetector::new(config.vad_mode)?)
        } else {
            None
        };

        Ok(Self {
            config,
            device: Some(device),
            stream: None,
            consumer: None,
            running: Arc::new(AtomicBool::new(false)),
            error_flag: Arc::new(AtomicBool::new(false)),
            timestamp_ms: Arc::new(AtomicI64::new(0)),
            resampler: None,
            vad,
            input_sample_rate,
            input_channels,
        })
    }

    /// Calculates the number of input samples needed for one chunk.
    fn input_chunk_samples(&self) -> usize {
        // Calculate how many input samples we need to produce one chunk at target rate
        let target_samples = (TARGET_SAMPLE_RATE * self.config.chunk_ms) / 1000;
        let input_samples_per_channel =
            (target_samples as f64 * self.input_sample_rate as f64 / TARGET_SAMPLE_RATE as f64).ceil() as usize;

        // Account for channels
        input_samples_per_channel * self.input_channels as usize
    }

    /// Converts interleaved multi-channel audio to mono.
    #[allow(dead_code)]
    fn to_mono(&self, samples: &[f32]) -> Vec<f32> {
        if self.input_channels == 1 {
            return samples.to_vec();
        }

        let channels = self.input_channels as usize;
        samples
            .chunks(channels)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    }
}

impl AudioSource for CpalAudioSource {
    fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let device = self.device.as_ref()
            .ok_or_else(|| LexiconError::Audio(AudioError::NoInputDevice))?;

        // Get supported config
        let supported_config = device
            .default_input_config()
            .map_err(|e| LexiconError::Audio(AudioError::UnsupportedFormat {
                format: e.to_string(),
            }))?;

        let sample_format = supported_config.sample_format();
        let stream_config: StreamConfig = supported_config.into();

        // Calculate buffer size - enough for several chunks
        let chunk_samples = self.input_chunk_samples();
        let buffer_size = chunk_samples * 4; // 4 chunks worth of buffer

        // Create ring buffer
        let rb = HeapRb::<f32>::new(buffer_size);
        let (mut producer, consumer) = rb.split();
        self.consumer = Some(consumer);

        // Create resampler
        let input_chunk_mono = chunk_samples / self.input_channels as usize;
        self.resampler = Some(Resampler::new(
            self.input_sample_rate,
            1, // mono output
            input_chunk_mono,
        )?);

        // Shared state for callback
        let running = self.running.clone();
        let error_flag = self.error_flag.clone();

        // Error callback
        let error_flag_clone = error_flag.clone();
        let err_fn = move |err| {
            tracing::error!("Audio stream error: {}", err);
            error_flag_clone.store(true, Ordering::SeqCst);
        };

        // Build stream based on sample format
        let stream = match sample_format {
            SampleFormat::F32 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if running.load(Ordering::SeqCst) {
                            // Push samples to ring buffer, dropping old samples if full
                            for &sample in data {
                                let _ = producer.try_push(sample);
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if running.load(Ordering::SeqCst) {
                            for &sample in data {
                                let f_sample = sample as f32 / 32768.0;
                                let _ = producer.try_push(f_sample);
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::U16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if running.load(Ordering::SeqCst) {
                            for &sample in data {
                                let f_sample = (sample as f32 - 32768.0) / 32768.0;
                                let _ = producer.try_push(f_sample);
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => {
                return Err(LexiconError::Audio(AudioError::UnsupportedFormat {
                    format: format!("Unsupported sample format: {sample_format:?}"),
                }));
            }
        }
        .map_err(|e| LexiconError::Audio(AudioError::StreamError {
            message: e.to_string(),
        }))?;

        // Start the stream
        stream.play().map_err(|e| LexiconError::Audio(AudioError::StreamError {
            message: e.to_string(),
        }))?;

        self.stream = Some(stream);
        self.running.store(true, Ordering::SeqCst);
        self.error_flag.store(false, Ordering::SeqCst);
        self.timestamp_ms.store(0, Ordering::SeqCst);

        tracing::info!(
            "Audio capture started: {}Hz {}ch -> {}Hz mono",
            self.input_sample_rate,
            self.input_channels,
            TARGET_SAMPLE_RATE
        );

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);

        // Drop stream to stop capture
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }

        // Clear consumer
        self.consumer = None;
        self.resampler = None;

        tracing::info!("Audio capture stopped");

        Ok(())
    }

    fn read_chunk(&mut self) -> Result<AudioChunk> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(LexiconError::Audio(AudioError::StreamNotStarted));
        }

        // Check for stream error
        if self.error_flag.load(Ordering::SeqCst) {
            return Err(LexiconError::Audio(AudioError::DeviceDisconnected));
        }

        // Get values needed before mutable borrows
        let chunk_samples = self.input_chunk_samples();
        let timeout = Duration::from_millis(self.config.chunk_ms as u64 * 2);
        let input_channels = self.input_channels;

        // Check components exist
        if self.consumer.is_none() || self.resampler.is_none() {
            return Err(LexiconError::Audio(AudioError::StreamNotStarted));
        }

        // Wait for enough samples
        let start = std::time::Instant::now();
        {
            let consumer = self.consumer.as_ref().unwrap();
            while consumer.occupied_len() < chunk_samples {
                if !self.running.load(Ordering::SeqCst) {
                    return Err(LexiconError::Audio(AudioError::StreamNotStarted));
                }

                if self.error_flag.load(Ordering::SeqCst) {
                    return Err(LexiconError::Audio(AudioError::DeviceDisconnected));
                }

                if start.elapsed() > timeout {
                    if consumer.occupied_len() > 0 {
                        break;
                    }
                    return Err(LexiconError::Audio(AudioError::BufferError {
                        kind: "underrun".to_string(),
                    }));
                }

                std::thread::sleep(Duration::from_millis(1));
            }
        }

        // Read samples from ring buffer
        let input_samples = {
            let consumer = self.consumer.as_mut().unwrap();
            let available = consumer.occupied_len().min(chunk_samples);
            let mut samples = vec![0.0f32; available];
            consumer.pop_slice(&mut samples);
            samples
        };

        // Convert to mono (inline to avoid borrow issues)
        let mono_samples: Vec<f32> = if input_channels == 1 {
            input_samples
        } else {
            let channels = input_channels as usize;
            input_samples
                .chunks(channels)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect()
        };

        // Resample to 16kHz
        let resampler = self.resampler.as_mut().unwrap();
        let resampled = resampler.process(&mono_samples)?;

        // Get timestamp and advance
        let timestamp = self.timestamp_ms.load(Ordering::SeqCst);
        let duration_ms = (resampled.len() as u32 * 1000) / TARGET_SAMPLE_RATE;
        self.timestamp_ms.fetch_add(duration_ms as i64, Ordering::SeqCst);

        // Run VAD if enabled
        let has_speech = if let Some(ref mut vad) = self.vad {
            vad.has_speech(&resampled, 0.3) // 30% speech threshold
        } else {
            true // If VAD disabled, assume speech
        };

        Ok(AudioChunk::new(resampled, has_speech, timestamp))
    }

    fn config(&self) -> &AudioConfig {
        &self.config
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for CpalAudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalAudioSource")
            .field("config", &self.config)
            .field("running", &self.running.load(Ordering::SeqCst))
            .field("input_sample_rate", &self.input_sample_rate)
            .field("input_channels", &self.input_channels)
            .finish()
    }
}

impl Drop for CpalAudioSource {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_source_creation() {
        // This test may fail in CI without audio devices, so we handle both cases
        let config = AudioConfig::default();
        let result = CpalAudioSource::new(config);

        // Either succeeds or fails with device-related errors in CI/headless environments
        match result {
            Ok(source) => {
                assert!(!source.is_running());
            }
            Err(LexiconError::Audio(AudioError::NoInputDevice)) => {
                // Expected in CI/headless environments
            }
            Err(LexiconError::Audio(AudioError::UnsupportedFormat { .. })) => {
                // May occur when device is not available
            }
            Err(LexiconError::Audio(AudioError::DeviceEnumerationFailed { .. })) => {
                // May occur on systems without audio subsystem
            }
            Err(LexiconError::Audio(AudioError::DeviceConfigFailed { .. })) => {
                // May occur when device configuration is not available
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    #[test]
    fn test_to_mono_passthrough() {
        // Create a minimal source for testing the to_mono function
        let source = CpalAudioSource {
            config: AudioConfig::default(),
            device: None,
            stream: None,
            consumer: None,
            running: Arc::new(AtomicBool::new(false)),
            error_flag: Arc::new(AtomicBool::new(false)),
            timestamp_ms: Arc::new(AtomicI64::new(0)),
            resampler: None,
            vad: None,
            input_sample_rate: 48000,
            input_channels: 1,
        };

        let samples = vec![1.0, 2.0, 3.0];
        let mono = source.to_mono(&samples);
        assert_eq!(mono, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_to_mono_stereo() {
        let source = CpalAudioSource {
            config: AudioConfig::default(),
            device: None,
            stream: None,
            consumer: None,
            running: Arc::new(AtomicBool::new(false)),
            error_flag: Arc::new(AtomicBool::new(false)),
            timestamp_ms: Arc::new(AtomicI64::new(0)),
            resampler: None,
            vad: None,
            input_sample_rate: 48000,
            input_channels: 2,
        };

        // Stereo: L=0.0,R=1.0, L=0.5,R=0.5, L=1.0,R=0.0
        let samples = vec![0.0, 1.0, 0.5, 0.5, 1.0, 0.0];
        let mono = source.to_mono(&samples);
        assert_eq!(mono, vec![0.5, 0.5, 0.5]);
    }

    #[test]
    fn test_read_chunk_not_started() {
        let config = AudioConfig::default();

        // Create source without starting
        let mut source = CpalAudioSource {
            config,
            device: None,
            stream: None,
            consumer: None,
            running: Arc::new(AtomicBool::new(false)),
            error_flag: Arc::new(AtomicBool::new(false)),
            timestamp_ms: Arc::new(AtomicI64::new(0)),
            resampler: None,
            vad: None,
            input_sample_rate: 48000,
            input_channels: 1,
        };

        let result = source.read_chunk();
        assert!(matches!(
            result,
            Err(LexiconError::Audio(AudioError::StreamNotStarted))
        ));
    }
}
