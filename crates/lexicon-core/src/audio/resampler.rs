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
    chunk_size: usize,
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
            chunk_size,
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
    /// **Important:** The input must have exactly `chunk_size` samples (as specified
    /// during construction). For the final chunk of a file which may be smaller,
    /// use [`process_partial`] instead.
    ///
    /// # Arguments
    ///
    /// * `input` - Input samples at the configured input rate (must be exactly chunk_size)
    ///
    /// # Returns
    ///
    /// Resampled audio at 16kHz
    ///
    /// # Errors
    ///
    /// Returns an error if resampling fails or if input size doesn't match chunk_size.
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

    /// Resamples a partial chunk of audio (typically the final chunk of a file).
    ///
    /// Unlike [`process`], this method accepts input of any size up to `chunk_size`.
    /// Use this for the final chunk when processing a file that doesn't divide evenly
    /// into `chunk_size` samples.
    ///
    /// # Arguments
    ///
    /// * `input` - Input samples at the configured input rate (any size up to chunk_size)
    ///
    /// # Returns
    ///
    /// Resampled audio at 16kHz
    ///
    /// # Errors
    ///
    /// Returns an error if resampling fails.
    pub fn process_partial(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        // Convert input to the format rubato expects (Vec<Vec<f32>> for channels)
        let input_frames: Vec<Vec<f32>> = vec![input.to_vec()];

        // FastFixedIn requires output buffer sized for the full chunk_size, not the partial input.
        // Use output_frames_max() which returns the maximum output frames the resampler can produce.
        let output_size = self.resampler.output_frames_max();
        let mut output_buffer: Vec<Vec<f32>> = vec![vec![0.0; output_size]; self.channels];

        let (_, num_out) = self
            .resampler
            .process_partial_into_buffer(Some(&input_frames), &mut output_buffer, None)
            .map_err(|e| {
                LexiconError::Audio(AudioError::ResamplerError {
                    message: format!("Resampling partial chunk failed: {e}"),
                })
            })?;

        // Extract mono channel output, truncated to actual output size
        let mut result = output_buffer.into_iter().next().unwrap_or_default();

        // Calculate expected output based on actual input size and resampling ratio
        // The resampler may report more output frames than actually correspond to input
        let ratio = self.output_rate as f64 / self.input_rate as f64;
        let expected_output = ((input.len() as f64) * ratio).ceil() as usize;

        // Use the minimum of reported output and expected output to avoid extra samples
        let actual_output = num_out.min(expected_output);
        result.truncate(actual_output);
        Ok(result)
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    fn test_resampler_process_partial() {
        // Create resampler with 1 second chunk size (44100 samples)
        let mut resampler = Resampler::from_44100(44100).unwrap();

        // Process a partial chunk (simulating end of file with ~0.765 seconds)
        let partial_input: Vec<f32> = (0..33765)
            .map(|i| (i as f32 * 0.01).sin())
            .collect();

        let output = resampler.process_partial(&partial_input);
        assert!(output.is_ok(), "process_partial should handle partial chunks");

        let output = output.unwrap();
        // 33765 samples at 44.1kHz ≈ 0.765 seconds
        // At 16kHz, should be roughly 0.765 * 16000 ≈ 12240 samples
        let expected_min = 12000;
        let expected_max = 12500;
        assert!(
            output.len() >= expected_min && output.len() <= expected_max,
            "Expected output between {expected_min}-{expected_max}, got {}",
            output.len()
        );
    }

    #[test]
    fn test_resampler_process_partial_very_short() {
        // Create resampler with 1 second chunk size
        let mut resampler = Resampler::from_48000(48000).unwrap();

        // Process a very short chunk (<100ms)
        let short_input: Vec<f32> = (0..4000)
            .map(|i| (i as f32 * 0.01).sin())
            .collect();

        let output = resampler.process_partial(&short_input);
        assert!(output.is_ok(), "process_partial should handle very short chunks");

        let output = output.unwrap();
        // 4000 samples at 48kHz ≈ 83ms
        // At 16kHz, should be roughly 83ms * 16 = ~1333 samples
        assert!(
            output.len() >= 1200 && output.len() <= 1500,
            "Expected output around 1333 samples, got {}",
            output.len()
        );
    }

    #[test]
    fn test_resampler_chunked_file_simulation() {
        // Simulate processing a file that doesn't divide evenly into chunk_size
        let chunk_size = 44100; // 1 second at 44.1kHz
        let mut resampler = Resampler::from_44100(chunk_size).unwrap();

        // Simulate a 2.765 second file (2 full chunks + 1 partial)
        let total_samples = 44100 * 2 + 33765; // 121965 samples
        let full_input: Vec<f32> = (0..total_samples)
            .map(|i| (i as f32 * 0.001).sin())
            .collect();

        let mut output = Vec::new();
        let chunks: Vec<&[f32]> = full_input.chunks(chunk_size).collect();
        let num_chunks = chunks.len();

        for (i, chunk) in chunks.into_iter().enumerate() {
            let is_last = i == num_chunks - 1;
            let is_partial = chunk.len() < chunk_size;

            let resampled = if is_last && is_partial {
                resampler.process_partial(chunk).unwrap()
            } else {
                resampler.process(chunk).unwrap()
            };
            output.extend(resampled);
        }

        // 2.765 seconds at 16kHz should be roughly 44240 samples
        let expected_min = 43000;
        let expected_max = 45000;
        assert!(
            output.len() >= expected_min && output.len() <= expected_max,
            "Expected total output between {expected_min}-{expected_max}, got {}",
            output.len()
        );
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
