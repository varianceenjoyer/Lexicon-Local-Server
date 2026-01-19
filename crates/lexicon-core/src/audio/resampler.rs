//! Audio Resampling using Rubato.
//!
//! This module provides high-quality resampling from common capture rates
//! (44.1kHz, 48kHz) to the 16kHz required by Whisper.

use rubato::{FastFixedIn, PolynomialDegree, Resampler as RubatoResampler};

use crate::error::{AudioError, LexiconError, Result};

use super::TARGET_SAMPLE_RATE;

/// Audio resampler for converting between sample rates.
///
/// Uses rubato's `FastFixedIn` resampler for efficient, high-quality
/// conversion from capture rates to 16kHz.
pub struct Resampler {
    resampler: FastFixedIn<f32>,
    input_rate: u32,
    output_rate: u32,
    channels: usize,
}

impl Resampler {
    /// Creates a new resampler for the given input sample rate.
    ///
    /// # Arguments
    ///
    /// * `input_rate` - Input sample rate (e.g., 44100, 48000)
    /// * `channels` - Number of channels (typically 1 for mono)
    /// * `chunk_size` - Expected input chunk size in samples
    ///
    /// # Errors
    ///
    /// Returns an error if the resampler cannot be constructed.
    pub fn new(input_rate: u32, channels: usize, chunk_size: usize) -> Result<Self> {
        let output_rate = TARGET_SAMPLE_RATE;
        let resample_ratio = output_rate as f64 / input_rate as f64;

        let resampler = FastFixedIn::new(
            resample_ratio,
            1.0, // No additional ratio adjustment
            PolynomialDegree::Cubic,
            chunk_size,
            channels,
        )
        .map_err(|e| {
            LexiconError::Audio(AudioError::ResamplerInitFailed {
                message: format!("Failed to create resampler: {e}"),
            })
        })?;

        Ok(Self {
            resampler,
            input_rate,
            output_rate,
            channels,
        })
    }

    /// Creates a resampler for 44.1kHz to 16kHz conversion.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Expected input chunk size in samples
    pub fn from_44100(chunk_size: usize) -> Result<Self> {
        Self::new(44100, 1, chunk_size)
    }

    /// Creates a resampler for 48kHz to 16kHz conversion.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Expected input chunk size in samples
    pub fn from_48000(chunk_size: usize) -> Result<Self> {
        Self::new(48000, 1, chunk_size)
    }

    /// Resamples audio from the input rate to 16kHz.
    ///
    /// # Arguments
    ///
    /// * `input` - Input samples at the configured input rate
    ///
    /// # Returns
    ///
    /// Resampled audio at 16kHz
    ///
    /// # Errors
    ///
    /// Returns an error if resampling fails.
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        // Convert input to the format rubato expects (Vec<Vec<f32>> for channels)
        let input_frames: Vec<Vec<f32>> = vec![input.to_vec()];

        let output_frames = self.resampler.process(&input_frames, None).map_err(|e| {
            LexiconError::Audio(AudioError::ResamplerError {
                message: format!("Resampling failed: {e}"),
            })
        })?;

        // Extract mono channel output
        Ok(output_frames.into_iter().next().unwrap_or_default())
    }

    /// Resamples and converts stereo to mono.
    ///
    /// # Arguments
    ///
    /// * `left` - Left channel samples
    /// * `right` - Right channel samples
    ///
    /// # Returns
    ///
    /// Mono audio at 16kHz (averaged from stereo)
    pub fn process_stereo(&mut self, left: &[f32], right: &[f32]) -> Result<Vec<f32>> {
        // Average stereo to mono first
        let mono: Vec<f32> = left
            .iter()
            .zip(right.iter())
            .map(|(l, r)| (l + r) * 0.5)
            .collect();

        self.process(&mono)
    }

    /// Returns the input sample rate.
    #[must_use]
    pub fn input_rate(&self) -> u32 {
        self.input_rate
    }

    /// Returns the output sample rate (always 16kHz).
    #[must_use]
    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }

    /// Returns the resampling ratio.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        self.output_rate as f64 / self.input_rate as f64
    }

    /// Calculates the expected output size for a given input size.
    #[must_use]
    pub fn output_size(&self, input_size: usize) -> usize {
        ((input_size as f64) * self.ratio()).ceil() as usize
    }

    /// Resets the resampler state.
    ///
    /// Call this when starting a new audio stream to clear internal buffers.
    pub fn reset(&mut self) {
        self.resampler.reset();
    }
}

impl std::fmt::Debug for Resampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resampler")
            .field("input_rate", &self.input_rate)
            .field("output_rate", &self.output_rate)
            .field("channels", &self.channels)
            .field("ratio", &self.ratio())
            .finish()
    }
}

/// Converts interleaved stereo samples to separate channels.
#[must_use]
pub fn deinterleave_stereo(interleaved: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut left = Vec::with_capacity(interleaved.len() / 2);
    let mut right = Vec::with_capacity(interleaved.len() / 2);

    for chunk in interleaved.chunks(2) {
        if chunk.len() == 2 {
            left.push(chunk[0]);
            right.push(chunk[1]);
        }
    }

    (left, right)
}

/// Converts stereo to mono by averaging channels.
#[must_use]
pub fn stereo_to_mono(interleaved: &[f32]) -> Vec<f32> {
    interleaved
        .chunks(2)
        .map(|chunk| {
            if chunk.len() == 2 {
                (chunk[0] + chunk[1]) * 0.5
            } else {
                chunk[0]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_creation_44100() {
        let resampler = Resampler::from_44100(4410); // 100ms at 44.1kHz
        assert!(resampler.is_ok());

        let r = resampler.unwrap();
        assert_eq!(r.input_rate(), 44100);
        assert_eq!(r.output_rate(), 16000);
    }

    #[test]
    fn test_resampler_creation_48000() {
        let resampler = Resampler::from_48000(4800); // 100ms at 48kHz
        assert!(resampler.is_ok());

        let r = resampler.unwrap();
        assert_eq!(r.input_rate(), 48000);
        assert_eq!(r.output_rate(), 16000);
    }

    #[test]
    fn test_resampler_ratio() {
        let resampler = Resampler::from_48000(4800).unwrap();
        let ratio = resampler.ratio();

        // 16000 / 48000 = 1/3
        assert!((ratio - (1.0 / 3.0)).abs() < 0.001);
    }

    #[test]
    fn test_output_size_calculation() {
        let resampler = Resampler::from_48000(4800).unwrap();

        // 4800 samples at 48kHz should become ~1600 at 16kHz
        let expected = 1600;
        let actual = resampler.output_size(4800);

        // Allow some tolerance for rounding
        assert!((actual as i32 - expected as i32).abs() <= 10);
    }

    #[test]
    fn test_resampler_process() {
        let mut resampler = Resampler::from_48000(4800).unwrap();

        // Create a simple sine wave at 48kHz
        let input: Vec<f32> = (0..4800)
            .map(|i| (i as f32 * 0.01).sin())
            .collect();

        let output = resampler.process(&input);
        assert!(output.is_ok());

        let output = output.unwrap();
        // Output should be roughly 1/3 the size
        assert!(output.len() > 1500 && output.len() < 1700);
    }

    #[test]
    fn test_deinterleave_stereo() {
        let interleaved = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let (left, right) = deinterleave_stereo(&interleaved);

        assert_eq!(left, vec![1.0, 3.0, 5.0]);
        assert_eq!(right, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_stereo_to_mono() {
        let interleaved = vec![0.0, 1.0, 0.5, 0.5, 1.0, 0.0];
        let mono = stereo_to_mono(&interleaved);

        assert_eq!(mono, vec![0.5, 0.5, 0.5]);
    }
}
