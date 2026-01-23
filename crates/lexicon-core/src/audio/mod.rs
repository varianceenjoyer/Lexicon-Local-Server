//! Audio Pipeline - Capture, VAD, and Resampling
//!
//! This module provides cross-platform audio capture with voice activity detection
//! and resampling to 16kHz mono for speech-to-text processing.
//!
//! # Architecture
//!
//! ```text
//! Microphone (44.1/48kHz) → Ring Buffer → VAD → Resample → AudioChunk (16kHz)
//! ```
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::audio::{AudioSource, CpalAudioSource, AudioConfig};
//!
//! let config = AudioConfig::default();
//! let mut source = CpalAudioSource::new(config)?;
//!
//! source.start()?;
//! loop {
//!     let chunk = source.read_chunk()?;
//!     if chunk.has_speech {
//!         // Process speech data
//!         println!("Speech detected: {} samples", chunk.samples.len());
//!     }
//! }
//! # Ok::<(), lexicon_core::error::LexiconError>(())
//! ```
//!
//! # Platform Support
//!
//! | Platform | Backend | Notes |
//! |----------|---------|-------|
//! | macOS | CoreAudio | Fully supported |
//! | Windows | WASAPI | Fully supported |
//! | Linux | ALSA | Requires libasound2-dev |

pub mod bluetooth;
mod capture;
mod resampler;
mod vad;

#[cfg(feature = "audio-convert")]
mod convert;

pub use capture::CpalAudioSource;
pub use resampler::Resampler;
pub use vad::VoiceActivityDetector;

#[cfg(feature = "audio-convert")]
pub use convert::{AudioConverter, AudioMetadata, ConversionError};

use crate::error::{AudioError, LexiconError, Result};

/// Target sample rate for STT engines (Whisper requires 16kHz).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Default chunk duration in milliseconds.
pub const DEFAULT_CHUNK_MS: u32 = 100;

/// A chunk of processed audio ready for STT.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// Audio samples in 16kHz mono f32 format, normalized to [-1.0, 1.0].
    pub samples: Vec<f32>,

    /// Whether voice activity was detected in this chunk.
    pub has_speech: bool,

    /// Timestamp of the chunk start in milliseconds since stream start.
    pub timestamp_ms: i64,

    /// Duration of this chunk in milliseconds.
    pub duration_ms: u32,
}

impl AudioChunk {
    /// Creates a new audio chunk.
    #[must_use]
    pub fn new(samples: Vec<f32>, has_speech: bool, timestamp_ms: i64) -> Self {
        let duration_ms = (samples.len() as u32 * 1000) / TARGET_SAMPLE_RATE;
        Self {
            samples,
            has_speech,
            timestamp_ms,
            duration_ms,
        }
    }

    /// Returns true if this chunk contains no samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns the number of samples in this chunk.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

/// Configuration for audio capture.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Target chunk duration in milliseconds.
    pub chunk_ms: u32,

    /// Enable voice activity detection.
    pub vad_enabled: bool,

    /// VAD aggressiveness level (0-3, higher = more aggressive filtering).
    pub vad_mode: u8,

    /// Preferred input device name (None = default device).
    pub device_name: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            chunk_ms: DEFAULT_CHUNK_MS,
            vad_enabled: true,
            vad_mode: 2,
            device_name: None,
        }
    }
}

impl AudioConfig {
    /// Creates a new audio configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the chunk duration in milliseconds.
    #[must_use]
    pub fn with_chunk_ms(mut self, ms: u32) -> Self {
        self.chunk_ms = ms;
        self
    }

    /// Enables or disables VAD.
    #[must_use]
    pub fn with_vad(mut self, enabled: bool) -> Self {
        self.vad_enabled = enabled;
        self
    }

    /// Sets VAD aggressiveness (0-3).
    #[must_use]
    pub fn with_vad_mode(mut self, mode: u8) -> Self {
        self.vad_mode = mode.min(3);
        self
    }

    /// Sets the preferred input device name.
    #[must_use]
    pub fn with_device(mut self, name: impl Into<String>) -> Self {
        self.device_name = Some(name.into());
        self
    }
}

/// Trait for audio sources that provide chunks of processed audio.
///
/// Implementations must:
/// - Capture audio from the platform's audio system
/// - Resample to 16kHz mono
/// - Perform VAD if enabled
/// - Handle device disconnection gracefully
pub trait AudioSource {
    /// Starts audio capture.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No audio input device is available
    /// - The device cannot be opened
    /// - The stream cannot be started
    fn start(&mut self) -> Result<()>;

    /// Stops audio capture.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream cannot be stopped cleanly.
    fn stop(&mut self) -> Result<()>;

    /// Reads the next chunk of processed audio.
    ///
    /// This method blocks until a complete chunk is available or an error occurs.
    ///
    /// # Returns
    ///
    /// An `AudioChunk` containing:
    /// - 16kHz mono f32 samples
    /// - VAD result (if enabled)
    /// - Timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The stream has not been started
    /// - The device was disconnected
    /// - An I/O error occurred
    fn read_chunk(&mut self) -> Result<AudioChunk>;

    /// Returns the current configuration.
    fn config(&self) -> &AudioConfig;

    /// Returns true if the audio source is currently capturing.
    fn is_running(&self) -> bool;
}

/// Lists available audio input devices.
///
/// # Errors
///
/// Returns an error if device enumeration fails.
pub fn list_input_devices() -> Result<Vec<String>> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let devices = host.input_devices().map_err(|e| {
        LexiconError::Audio(AudioError::DeviceEnumerationFailed {
            message: e.to_string(),
        })
    })?;

    let names: Vec<String> = devices
        .filter_map(|d| d.name().ok())
        .collect();

    Ok(names)
}

/// Returns the name of the default audio input device.
///
/// # Errors
///
/// Returns an error if no default device is available.
pub fn default_input_device() -> Result<String> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| {
        LexiconError::Audio(AudioError::NoInputDevice)
    })?;

    device.name().map_err(|e| {
        LexiconError::Audio(AudioError::DeviceEnumerationFailed {
            message: e.to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_chunk_creation() {
        let samples = vec![0.0f32; 1600]; // 100ms at 16kHz
        let chunk = AudioChunk::new(samples, true, 0);

        assert_eq!(chunk.len(), 1600);
        assert!(chunk.has_speech);
        assert_eq!(chunk.duration_ms, 100);
    }

    #[test]
    fn test_audio_config_builder() {
        let config = AudioConfig::new()
            .with_chunk_ms(200)
            .with_vad(false)
            .with_vad_mode(3)
            .with_device("Test Device");

        assert_eq!(config.chunk_ms, 200);
        assert!(!config.vad_enabled);
        assert_eq!(config.vad_mode, 3);
        assert_eq!(config.device_name, Some("Test Device".to_string()));
    }

    #[test]
    fn test_vad_mode_clamped() {
        let config = AudioConfig::new().with_vad_mode(10);
        assert_eq!(config.vad_mode, 3); // Clamped to max
    }

    #[test]
    fn test_audio_chunk_empty() {
        let chunk = AudioChunk::new(vec![], false, 0);
        assert!(chunk.is_empty());
        assert_eq!(chunk.len(), 0);
        assert_eq!(chunk.duration_ms, 0);
    }

    #[test]
    fn test_audio_chunk_duration_calculation() {
        // 16000 samples = 1 second at 16kHz
        let samples = vec![0.0f32; 16000];
        let chunk = AudioChunk::new(samples, true, 1000);

        assert_eq!(chunk.duration_ms, 1000);
        assert_eq!(chunk.timestamp_ms, 1000);
        assert!(chunk.has_speech);
    }

    #[test]
    fn test_list_input_devices_does_not_panic() {
        // This test verifies the function doesn't panic even without audio devices
        // In CI environments, this may return an empty list or an error
        let result = list_input_devices();
        // We don't assert on the contents since that depends on hardware,
        // but the function should not panic
        match result {
            Ok(devices) => {
                // Success case: devices is a list (possibly empty)
                assert!(devices.iter().all(|d| !d.is_empty() || devices.is_empty()));
            }
            Err(_) => {
                // Error case: acceptable in CI environments without audio
            }
        }
    }

    #[test]
    fn test_default_input_device_does_not_panic() {
        // This test verifies the function doesn't panic
        // It may return an error if no default device exists (e.g., in CI)
        let result = default_input_device();
        match result {
            Ok(device_name) => {
                // Success case: we got a device name
                assert!(!device_name.is_empty());
            }
            Err(e) => {
                // Error case: acceptable in CI environments without audio
                // Verify it's the expected error type
                match e {
                    crate::error::LexiconError::Audio(crate::error::AudioError::NoInputDevice) => {}
                    crate::error::LexiconError::Audio(crate::error::AudioError::DeviceEnumerationFailed { .. }) => {}
                    _ => panic!("Unexpected error type: {}", e),
                }
            }
        }
    }

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();

        assert_eq!(config.chunk_ms, DEFAULT_CHUNK_MS);
        assert!(config.vad_enabled);
        assert_eq!(config.vad_mode, 2);
        assert!(config.device_name.is_none());
    }

    #[test]
    fn test_target_sample_rate_constant() {
        assert_eq!(TARGET_SAMPLE_RATE, 16_000);
    }

    #[test]
    fn test_default_chunk_ms_constant() {
        assert_eq!(DEFAULT_CHUNK_MS, 100);
    }
}
