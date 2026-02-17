//! RNNoise-based noise reduction via nnnoiseless.
//!
//! Provides real-time noise suppression as a preprocessing step before STT.
//! Uses the nnnoiseless crate (Rust port of Xiph's RNNoise).
//!
//! # Sample Rate Handling
//!
//! nnnoiseless requires 48kHz input. Since our pipeline delivers 16kHz audio,
//! this module internally upsamples to 48kHz using rubato (high-quality polyphase),
//! denoises, then downsamples back to 16kHz.
//!
//! # Sample Scaling
//!
//! nnnoiseless expects samples in 16-bit PCM range [-32768, 32767]. Our pipeline
//! uses normalized floats [-1.0, 1.0], so we scale up before processing and
//! scale back down after.
//!
//! # Strength Control
//!
//! The `strength` parameter (0.0–1.0) controls denoising intensity via dry/wet
//! blending. VAD probability from RNNoise adaptively preserves more original
//! signal during speech to avoid distortion.

use nnnoiseless::DenoiseState;
use rubato::{FastFixedIn, PolynomialDegree, Resampler};

/// Frame size for nnnoiseless (480 samples at 48kHz = 10ms).
const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;

/// Corresponding frame size at 16kHz (160 samples = 10ms).
const FRAME_SIZE_16K: usize = FRAME_SIZE / 3;

/// Scale factor: nnnoiseless expects 16-bit PCM range.
const PCM_SCALE: f32 = 32767.0;

/// EMA smoothing factor for VAD probability.
const VAD_SMOOTHING: f32 = 0.8;

/// Maximum dry signal preservation during speech (at full strength).
/// During high-VAD frames, up to this fraction of the original signal is kept
/// to preserve speech clarity.
const MAX_SPEECH_DRY: f32 = 0.3;

/// Stateful noise reduction processor.
///
/// Maintains internal RNNoise state and rubato resamplers across calls for
/// streaming use. Each instance should be used for a single audio stream.
pub struct Denoiser {
    state: Box<DenoiseState<'static>>,
    upsampler: FastFixedIn<f32>,
    downsampler: FastFixedIn<f32>,
    /// Leftover 16kHz samples from previous call that didn't fill a complete frame.
    remainder: Vec<f32>,
    /// Denoising strength: 0.0 = bypass, 1.0 = maximum denoising.
    strength: f32,
    /// Smoothed VAD probability from RNNoise.
    smoothed_vad: f32,
}

impl Denoiser {
    /// Create a new denoiser instance with full strength.
    pub fn new() -> Self {
        Self::with_strength(1.0)
    }

    /// Create a new denoiser with the given strength (0.0–1.0).
    pub fn with_strength(strength: f32) -> Self {
        let strength = strength.clamp(0.0, 1.0);

        // 16kHz → 48kHz (ratio 3.0), chunk size = 160 (one 10ms frame at 16kHz)
        let upsampler = FastFixedIn::new(
            3.0,
            1.0,
            PolynomialDegree::Cubic,
            FRAME_SIZE_16K,
            1,
        )
        .expect("Failed to create upsampler");

        // 48kHz → 16kHz (ratio 1/3), chunk size = 480 (one 10ms frame at 48kHz)
        let downsampler = FastFixedIn::new(
            1.0 / 3.0,
            1.0,
            PolynomialDegree::Cubic,
            FRAME_SIZE,
            1,
        )
        .expect("Failed to create downsampler");

        Self {
            state: DenoiseState::new(),
            upsampler,
            downsampler,
            remainder: Vec::with_capacity(FRAME_SIZE_16K),
            strength,
            smoothed_vad: 0.0,
        }
    }

    /// Set denoising strength (0.0 = bypass, 1.0 = maximum).
    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength.clamp(0.0, 1.0);
    }

    /// Get current denoising strength.
    pub fn strength(&self) -> f32 {
        self.strength
    }

    /// Denoise 16kHz f32 audio samples.
    ///
    /// Accepts any number of normalized float samples at 16kHz. Internally
    /// upsamples to 48kHz via rubato, scales to PCM range, processes through
    /// RNNoise, scales back, and downsamples to 16kHz.
    ///
    /// Returns denoised samples (may be slightly fewer than input due to framing).
    pub fn process(&mut self, samples_16k: &[f32]) -> Vec<f32> {
        if samples_16k.is_empty() {
            return Vec::new();
        }

        // Full bypass when strength is zero
        if self.strength == 0.0 {
            return samples_16k.to_vec();
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
        let mut output_48k = [0.0f32; FRAME_SIZE];

        for frame_idx in 0..complete_frames {
            let start = frame_idx * FRAME_SIZE_16K;
            let frame_16k = &total_16k[start..start + FRAME_SIZE_16K];

            // Upsample 16kHz → 48kHz via rubato
            let input_frames = vec![frame_16k.to_vec()];
            let upsampled = self
                .upsampler
                .process(&input_frames, None)
                .expect("Upsampling failed");
            let up_samples = &upsampled[0];

            // Prepare input at PCM scale for nnnoiseless
            let mut input_48k = [0.0f32; FRAME_SIZE];
            let copy_len = up_samples.len().min(FRAME_SIZE);
            for i in 0..copy_len {
                input_48k[i] = up_samples[i] * PCM_SCALE;
            }

            // Process through RNNoise — returns VAD probability
            let vad = self.state.process_frame(&mut output_48k, &input_48k);

            // Scale back to normalized floats
            for sample in &mut output_48k {
                *sample /= PCM_SCALE;
            }

            // Smooth VAD with EMA
            self.smoothed_vad = VAD_SMOOTHING * self.smoothed_vad + (1.0 - VAD_SMOOTHING) * vad;

            // VAD-adaptive dry/wet blending:
            // - During speech (high VAD): preserve some original signal to avoid distortion
            // - During noise-only (low VAD): use full denoised signal
            let speech_dry = self.smoothed_vad * MAX_SPEECH_DRY * self.strength;
            let wet = self.strength * (1.0 - speech_dry);
            let dry = 1.0 - wet;

            // Blend: apply wet/dry mix at 48kHz
            let up_for_blend = &upsampled[0];
            for i in 0..FRAME_SIZE.min(up_for_blend.len()) {
                output_48k[i] = wet * output_48k[i] + dry * up_for_blend[i];
            }

            // Downsample 48kHz → 16kHz via rubato
            let down_input = vec![output_48k[..FRAME_SIZE].to_vec()];
            let downsampled = self
                .downsampler
                .process(&down_input, None)
                .expect("Downsampling failed");
            output.extend_from_slice(&downsampled[0]);
        }

        output
    }

    /// Reset the denoiser state. Call when starting a new audio stream.
    pub fn reset(&mut self) {
        self.state = DenoiseState::new();
        self.remainder.clear();
        self.smoothed_vad = 0.0;
        self.upsampler.reset();
        self.downsampler.reset();
    }
}

impl Default for Denoiser {
    fn default() -> Self {
        Self::new()
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
    fn test_denoiser_with_strength() {
        let denoiser = Denoiser::with_strength(0.5);
        assert!((denoiser.strength() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_strength_clamping() {
        let mut denoiser = Denoiser::new();
        denoiser.set_strength(1.5);
        assert!((denoiser.strength() - 1.0).abs() < f32::EPSILON);
        denoiser.set_strength(-0.5);
        assert!(denoiser.strength().abs() < f32::EPSILON);
    }

    #[test]
    fn test_process_silence() {
        let mut denoiser = Denoiser::new();
        let silence = vec![0.0f32; 1600]; // 100ms at 16kHz
        let result = denoiser.process(&silence);
        // Rubato may produce slightly different sample counts due to polyphase filtering
        assert!(
            result.len() >= 1500 && result.len() <= 1700,
            "Expected ~1600 samples, got {}",
            result.len()
        );
        // Output should be near-silent
        let max_abs: f32 = result.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(
            max_abs < 0.05,
            "Denoised silence should be near-zero, got {max_abs}"
        );
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
        assert!(!result.is_empty(), "Should output samples for one complete frame");
    }

    #[test]
    fn test_process_empty() {
        let mut denoiser = Denoiser::new();
        let result = denoiser.process(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_bypass_at_zero_strength() {
        let mut denoiser = Denoiser::with_strength(0.0);
        let input = vec![0.5f32; 1600];
        let result = denoiser.process(&input);
        // At strength 0, output should be identical to input
        assert_eq!(result.len(), input.len());
        for (a, b) in result.iter().zip(input.iter()) {
            assert!((a - b).abs() < f32::EPSILON);
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
        assert!(!result.is_empty());
    }
}
