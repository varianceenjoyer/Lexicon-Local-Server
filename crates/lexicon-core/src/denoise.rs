//! RNNoise-based noise reduction via nnnoiseless.
//!
//! Provides real-time noise suppression as a preprocessing step before STT.
//! Uses the nnnoiseless crate (Rust port of Xiph's RNNoise).
//!
//! # Sample Rate Handling
//!
//! nnnoiseless requires 48kHz input. Since our pipeline delivers 16kHz audio,
//! this module internally upsamples to 48kHz, denoises, then downsamples back
//! to 16kHz. The 3x ratio (16kHz ↔ 48kHz) allows simple integer resampling.

use nnnoiseless::DenoiseState;

/// Frame size for nnnoiseless (480 samples at 48kHz = 10ms).
const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;

/// Corresponding frame size at 16kHz (160 samples = 10ms).
const FRAME_SIZE_16K: usize = FRAME_SIZE / 3;

/// Stateful noise reduction processor.
///
/// Maintains internal RNNoise state across calls for streaming use.
/// Each instance should be used for a single audio stream.
pub struct Denoiser {
    state: Box<DenoiseState<'static>>,
    /// Leftover 16kHz samples from previous call that didn't fill a complete frame.
    remainder: Vec<f32>,
}

impl Denoiser {
    /// Create a new denoiser instance.
    pub fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            remainder: Vec::with_capacity(FRAME_SIZE_16K),
        }
    }

    /// Denoise 16kHz f32 audio samples in-place.
    ///
    /// Accepts any number of samples at 16kHz. Internally upsamples to 48kHz,
    /// processes through RNNoise in 480-sample frames, then downsamples back.
    /// Returns denoised samples (may be slightly fewer than input due to framing).
    pub fn process(&mut self, samples_16k: &[f32]) -> Vec<f32> {
        if samples_16k.is_empty() {
            return Vec::new();
        }

        // Prepend any remainder from previous call
        let total_16k: Vec<f32> = if self.remainder.is_empty() {
            samples_16k.to_vec()
        } else {
            let mut combined = std::mem::take(&mut self.remainder);
            combined.extend_from_slice(samples_16k);
            combined
        };

        // Calculate how many complete 10ms frames we can process
        let complete_frames = total_16k.len() / FRAME_SIZE_16K;
        if complete_frames == 0 {
            self.remainder = total_16k;
            return Vec::new();
        }

        let samples_to_process = complete_frames * FRAME_SIZE_16K;

        // Save remainder for next call
        if samples_to_process < total_16k.len() {
            self.remainder = total_16k[samples_to_process..].to_vec();
        }

        let mut output = Vec::with_capacity(samples_to_process);
        let mut input_48k = [0.0f32; FRAME_SIZE];
        let mut output_48k = [0.0f32; FRAME_SIZE];

        for frame_idx in 0..complete_frames {
            let start = frame_idx * FRAME_SIZE_16K;
            let frame_16k = &total_16k[start..start + FRAME_SIZE_16K];

            // Upsample 16kHz → 48kHz (3x linear interpolation)
            upsample_3x(frame_16k, &mut input_48k);

            // Process through RNNoise (separate input/output buffers)
            self.state.process_frame(&mut output_48k, &input_48k);

            // Downsample 48kHz → 16kHz (3x decimation)
            for i in 0..FRAME_SIZE_16K {
                output.push(output_48k[i * 3]);
            }
        }

        output
    }

    /// Reset the denoiser state. Call when starting a new audio stream.
    pub fn reset(&mut self) {
        self.state = DenoiseState::new();
        self.remainder.clear();
    }
}

impl Default for Denoiser {
    fn default() -> Self {
        Self::new()
    }
}

/// Upsample 160 samples at 16kHz to 480 samples at 48kHz using linear interpolation.
fn upsample_3x(input: &[f32], output: &mut [f32; FRAME_SIZE]) {
    debug_assert_eq!(input.len(), FRAME_SIZE_16K);

    for i in 0..FRAME_SIZE_16K {
        let sample = input[i];
        let next = if i + 1 < FRAME_SIZE_16K {
            input[i + 1]
        } else {
            sample
        };

        let base = i * 3;
        output[base] = sample;
        output[base + 1] = sample + (next - sample) / 3.0;
        output[base + 2] = sample + 2.0 * (next - sample) / 3.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_denoiser_creation() {
        let _denoiser = Denoiser::new();
    }

    #[test]
    fn test_process_silence() {
        let mut denoiser = Denoiser::new();
        let silence = vec![0.0f32; 1600]; // 100ms at 16kHz
        let result = denoiser.process(&silence);
        // Should return 1600 samples (10 complete frames of 160)
        assert_eq!(result.len(), 1600);
        // Output should be near-silent
        let max_abs: f32 = result.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(max_abs < 0.01, "Denoised silence should be near-zero, got {max_abs}");
    }

    #[test]
    fn test_process_partial_frame() {
        let mut denoiser = Denoiser::new();
        // 100 samples = less than one frame (160)
        let samples = vec![0.0f32; 100];
        let result = denoiser.process(&samples);
        assert!(result.is_empty(), "Partial frame should return empty");

        // Send 60 more to complete the frame
        let more = vec![0.0f32; 60];
        let result = denoiser.process(&more);
        assert_eq!(result.len(), 160, "Should output one complete frame");
    }

    #[test]
    fn test_process_empty() {
        let mut denoiser = Denoiser::new();
        let result = denoiser.process(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_upsample_3x() {
        let input = [1.0f32; FRAME_SIZE_16K];
        let mut output = [0.0f32; FRAME_SIZE];
        upsample_3x(&input, &mut output);
        // Constant input should produce constant output
        for s in &output {
            assert!((s - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_reset() {
        let mut denoiser = Denoiser::new();
        // Process some audio
        let samples = vec![0.1f32; 1600];
        denoiser.process(&samples);
        // Reset and verify clean state
        denoiser.reset();
        let silence = vec![0.0f32; 1600];
        let result = denoiser.process(&silence);
        assert_eq!(result.len(), 1600);
    }
}
