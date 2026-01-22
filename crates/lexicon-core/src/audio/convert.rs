//! Audio Format Conversion using Symphonia.
//!
//! This module provides audio format conversion from common formats (MP3, OGG, FLAC, WAV, AAC)
//! to 16kHz mono f32 samples suitable for Whisper transcription.

use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{AudioError, LexiconError};

use super::Resampler;

/// Metadata about the original audio file.
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Original sample rate in Hz.
    pub original_sample_rate: u32,
    /// Original number of channels.
    pub original_channels: u16,
    /// Detected format name.
    pub format: String,
}

/// Error type for audio conversion operations.
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    /// The audio format is not supported.
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    /// Error decoding the audio file.
    #[error("Decoding error: {0}")]
    DecodingError(String),

    /// I/O error reading or writing files.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// No audio track found in the file.
    #[error("No audio track found in file")]
    NoAudioTrack,

    /// Resampling error.
    #[error("Resampling error: {0}")]
    ResamplingError(String),
}

impl From<SymphoniaError> for ConversionError {
    fn from(err: SymphoniaError) -> Self {
        match err {
            SymphoniaError::IoError(e) => ConversionError::IoError(e),
            SymphoniaError::Unsupported(_) => {
                ConversionError::UnsupportedFormat(err.to_string())
            }
            _ => ConversionError::DecodingError(err.to_string()),
        }
    }
}

impl From<ConversionError> for LexiconError {
    fn from(err: ConversionError) -> Self {
        LexiconError::Audio(AudioError::ConversionError {
            message: err.to_string(),
        })
    }
}

/// Audio format converter for transcription preprocessing.
///
/// Converts various audio formats to 16kHz mono f32 samples
/// suitable for Whisper transcription.
#[derive(Debug, Clone)]
pub struct AudioConverter {
    target_sample_rate: u32,
    #[allow(dead_code)]
    target_channels: u16,
}

impl Default for AudioConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioConverter {
    /// Target sample rate for Whisper (16kHz).
    pub const TARGET_SAMPLE_RATE: u32 = 16000;

    /// Creates a new audio converter with default settings (16kHz mono).
    #[must_use]
    pub fn new() -> Self {
        Self {
            target_sample_rate: Self::TARGET_SAMPLE_RATE,
            target_channels: 1,
        }
    }

    /// Creates an audio converter with custom target settings.
    #[must_use]
    pub fn with_settings(sample_rate: u32, channels: u16) -> Self {
        Self {
            target_sample_rate: sample_rate,
            target_channels: channels,
        }
    }

    /// Returns a list of supported audio format extensions.
    #[must_use]
    pub fn supported_formats() -> Vec<&'static str> {
        vec!["mp3", "ogg", "flac", "wav", "m4a", "aac", "opus"]
    }

    /// Check if a file extension is supported.
    #[must_use]
    pub fn is_supported(extension: &str) -> bool {
        let ext = extension.to_lowercase();
        Self::supported_formats().contains(&ext.as_str())
    }

    /// Convert an audio file to 16kHz mono f32 samples.
    ///
    /// This is the main entry point for audio conversion. It handles:
    /// - Format detection and decoding via Symphonia
    /// - Resampling to the target sample rate
    /// - Channel mixing (stereo to mono)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the audio file
    ///
    /// # Returns
    ///
    /// A vector of f32 samples at 16kHz mono, ready for Whisper.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, decoded, or converted.
    pub fn convert_to_samples(&self, path: &Path) -> std::result::Result<Vec<f32>, ConversionError> {
        self.convert_to_samples_with_progress(path, |_| {})
    }

    /// Convert an audio file to samples with progress reporting.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the audio file
    /// * `progress` - Callback receiving progress as a value from 0.0 to 1.0
    ///
    /// # Returns
    ///
    /// A vector of f32 samples at 16kHz mono.
    pub fn convert_to_samples_with_progress<F>(
        &self,
        path: &Path,
        mut progress: F,
    ) -> std::result::Result<Vec<f32>, ConversionError>
    where
        F: FnMut(f32),
    {
        // Open the media source
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint based on file extension
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        // Probe the format
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;

        // Find the first audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(ConversionError::NoAudioTrack)?;

        let track_id = track.id;

        // Get codec parameters
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);

        // Estimate total frames for progress reporting
        let total_frames = codec_params.n_frames.unwrap_or(0);
        let mut processed_frames: u64 = 0;

        // Create decoder
        let decoder_opts = DecoderOptions::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &decoder_opts)?;

        // Create resampler if needed
        let needs_resampling = sample_rate != self.target_sample_rate;
        let chunk_size = sample_rate as usize; // 1 second of audio
        let mut resampler = if needs_resampling {
            Some(
                Resampler::new(sample_rate, 1, chunk_size)
                    .map_err(|e| ConversionError::ResamplingError(e.to_string()))?,
            )
        } else {
            None
        };

        let mut all_samples: Vec<f32> = Vec::new();
        let mut mono_buffer: Vec<f32> = Vec::new();

        // Decode loop
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(SymphoniaError::ResetRequired) => {
                    // Reset decoder and continue
                    decoder.reset();
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            // Skip packets from other tracks
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet
            let decoded = match decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(SymphoniaError::DecodeError(_)) => continue, // Skip corrupted frames
                Err(e) => return Err(e.into()),
            };

            // Get samples from the decoded audio
            let spec = *decoded.spec();
            let num_frames = decoded.frames();
            let num_channels = spec.channels.count();

            // Create sample buffer
            let mut sample_buf = SampleBuffer::<f32>::new(
                decoded.capacity() as u64,
                *decoded.spec(),
            );
            sample_buf.copy_interleaved_ref(decoded);
            let samples = sample_buf.samples();

            // Convert to mono if needed
            if num_channels > 1 {
                // Average channels to mono
                for frame_samples in samples.chunks(num_channels) {
                    let mono_sample: f32 =
                        frame_samples.iter().sum::<f32>() / num_channels as f32;
                    mono_buffer.push(mono_sample);
                }
            } else {
                mono_buffer.extend_from_slice(samples);
            }

            // Resample if needed
            if let Some(ref mut rs) = resampler {
                // Process in chunks
                while mono_buffer.len() >= chunk_size {
                    let chunk: Vec<f32> = mono_buffer.drain(..chunk_size).collect();
                    let resampled = rs
                        .process(&chunk)
                        .map_err(|e| ConversionError::ResamplingError(e.to_string()))?;
                    all_samples.extend(resampled);
                }
            } else {
                all_samples.append(&mut mono_buffer);
            }

            // Update progress
            processed_frames += num_frames as u64;
            if total_frames > 0 {
                let pct = (processed_frames as f32 / total_frames as f32).min(1.0);
                progress(pct);
            }
        }

        // Process remaining samples in buffer
        if let Some(ref mut rs) = resampler {
            if !mono_buffer.is_empty() {
                let resampled = rs
                    .process_partial(&mono_buffer)
                    .map_err(|e| ConversionError::ResamplingError(e.to_string()))?;
                all_samples.extend(resampled);
            }
        } else {
            all_samples.append(&mut mono_buffer);
        }

        progress(1.0);

        Ok(all_samples)
    }

    /// Get metadata about an audio file without fully decoding it.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the audio file
    ///
    /// # Returns
    ///
    /// Metadata about the audio file including duration, sample rate, and channels.
    pub fn get_metadata(&self, path: &Path) -> std::result::Result<AudioMetadata, ConversionError> {
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;
        let format_name = format.metadata().current()
            .and_then(|m| m.tags().iter().find(|t| t.key == "format").map(|t| t.value.to_string()))
            .unwrap_or_else(|| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(ConversionError::NoAudioTrack)?;

        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        // Calculate duration
        let duration_ms = if let (Some(n_frames), Some(sr)) =
            (codec_params.n_frames, codec_params.sample_rate)
        {
            (n_frames * 1000) / sr as u64
        } else if let Some(tb) = codec_params.time_base {
            // Try to get duration from time base
            codec_params.n_frames.map(|f| {
                let duration = tb.calc_time(f);
                (duration.seconds * 1000) + (duration.frac * 1000.0) as u64
            }).unwrap_or(0)
        } else {
            0
        };

        Ok(AudioMetadata {
            duration_ms,
            original_sample_rate: sample_rate,
            original_channels: channels,
            format: format_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/audio")
            .join(name)
    }

    #[test]
    fn test_supported_formats() {
        let formats = AudioConverter::supported_formats();
        assert!(formats.contains(&"mp3"));
        assert!(formats.contains(&"wav"));
        assert!(formats.contains(&"ogg"));
        assert!(formats.contains(&"flac"));
    }

    #[test]
    fn test_is_supported() {
        assert!(AudioConverter::is_supported("mp3"));
        assert!(AudioConverter::is_supported("MP3"));
        assert!(AudioConverter::is_supported("wav"));
        assert!(!AudioConverter::is_supported("txt"));
        assert!(!AudioConverter::is_supported("pdf"));
    }

    #[test]
    fn test_converter_default() {
        let converter = AudioConverter::new();
        assert_eq!(converter.target_sample_rate, 16000);
        assert_eq!(converter.target_channels, 1);
    }

    #[test]
    fn test_convert_wav_file() {
        let converter = AudioConverter::new();
        let path = fixture_path("sine_440hz_2s.wav");

        if path.exists() {
            let result = converter.convert_to_samples(&path);
            assert!(result.is_ok(), "Failed to convert WAV: {:?}", result.err());

            let samples = result.unwrap();
            // 2 seconds at 16kHz = 32000 samples
            assert!(
                samples.len() >= 30000 && samples.len() <= 34000,
                "Expected ~32000 samples, got {}",
                samples.len()
            );
        }
    }

    #[test]
    fn test_convert_mp3_file() {
        let converter = AudioConverter::new();
        let path = fixture_path("dolly.mp3");

        if path.exists() {
            let result = converter.convert_to_samples(&path);
            assert!(result.is_ok(), "Failed to convert MP3: {:?}", result.err());

            let samples = result.unwrap();
            // Should have some samples
            assert!(!samples.is_empty(), "No samples produced from MP3");
        }
    }

    #[test]
    fn test_progress_callback() {
        let converter = AudioConverter::new();
        let path = fixture_path("sine_440hz_2s.wav");

        if path.exists() {
            let mut progress_values: Vec<f32> = Vec::new();
            let result = converter.convert_to_samples_with_progress(&path, |p| {
                progress_values.push(p);
            });

            assert!(result.is_ok());
            // Should have received some progress updates
            assert!(!progress_values.is_empty());
            // Last progress should be 1.0
            assert!((progress_values.last().unwrap() - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_get_metadata_wav() {
        let converter = AudioConverter::new();
        let path = fixture_path("sine_440hz_2s.wav");

        if path.exists() {
            let result = converter.get_metadata(&path);
            assert!(result.is_ok(), "Failed to get metadata: {:?}", result.err());

            let metadata = result.unwrap();
            // 2 second file
            assert!(
                metadata.duration_ms >= 1900 && metadata.duration_ms <= 2100,
                "Expected ~2000ms duration, got {}",
                metadata.duration_ms
            );
        }
    }

    #[test]
    fn test_nonexistent_file() {
        let converter = AudioConverter::new();
        let result = converter.convert_to_samples(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_err());
    }
}
