//! Voice Activity Detection using WebRTC VAD.
//!
//! This module wraps the webrtc-vad crate to provide speech detection
//! for audio chunks.

use webrtc_vad::{SampleRate, Vad, VadMode};

use crate::error::{AudioError, LexiconError, Result};

/// Voice Activity Detector using WebRTC VAD algorithm.
///
/// The VAD processes audio frames and determines if speech is present.
/// It operates on 16kHz audio in 10ms, 20ms, or 30ms frames.
pub struct VoiceActivityDetector {
    vad: Vad,
    sample_rate: SampleRate,
}

impl VoiceActivityDetector {
    /// Creates a new VAD with the specified aggressiveness mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - Aggressiveness level (0-3):
    ///   - 0: Quality mode (least aggressive, fewer false negatives)
    ///   - 1: Low bitrate mode
    ///   - 2: Aggressive mode
    ///   - 3: Very aggressive mode (most aggressive, more false negatives)
    ///
    /// # Errors
    ///
    /// Returns an error if the VAD cannot be initialized.
    pub fn new(mode: u8) -> Result<Self> {
        let vad_mode = match mode {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            _ => VadMode::VeryAggressive,
        };

        let mut vad = Vad::new();
        vad.set_mode(vad_mode);

        Ok(Self {
            vad,
            sample_rate: SampleRate::Rate16kHz,
        })
    }

    /// Detects if speech is present in an audio frame.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples at 16kHz. Length must be 160, 320, or 480
    ///   samples (10ms, 20ms, or 30ms frames).
    ///
    /// # Returns
    ///
    /// `true` if speech is detected, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame size is invalid.
    pub fn is_speech(&mut self, samples: &[i16]) -> Result<bool> {
        // WebRTC VAD requires specific frame sizes
        let valid_sizes = [160, 320, 480]; // 10ms, 20ms, 30ms at 16kHz
        if !valid_sizes.contains(&samples.len()) {
            return Err(LexiconError::Audio(AudioError::InvalidFrameSize {
                expected: "160, 320, or 480".to_string(),
                actual: samples.len(),
            }));
        }

        self.vad
            .is_voice_segment(samples)
            .map_err(|_| LexiconError::Audio(AudioError::VadError {
                message: "VAD processing failed".to_string(),
            }))
    }

    /// Detects speech in f32 samples by converting to i16.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples at 16kHz as f32 in range [-1.0, 1.0].
    ///   Length must be 160, 320, or 480 samples.
    ///
    /// # Returns
    ///
    /// `true` if speech is detected, `false` otherwise.
    pub fn is_speech_f32(&mut self, samples: &[f32]) -> Result<bool> {
        let i16_samples: Vec<i16> = samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        self.is_speech(&i16_samples)
    }

    /// Processes a longer audio buffer and returns the proportion of frames with speech.
    ///
    /// This method divides the buffer into 10ms frames and processes each one,
    /// returning the ratio of frames that contain speech.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples at 16kHz as f32. Can be any length.
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 1.0 representing the proportion of speech frames.
    pub fn speech_ratio(&mut self, samples: &[f32]) -> f32 {
        if samples.len() < 160 {
            return 0.0;
        }

        let frame_size = 160; // 10ms at 16kHz
        let num_frames = samples.len() / frame_size;

        if num_frames == 0 {
            return 0.0;
        }

        let mut speech_frames = 0;

        for i in 0..num_frames {
            let start = i * frame_size;
            let end = start + frame_size;
            let frame = &samples[start..end];

            if let Ok(is_speech) = self.is_speech_f32(frame) {
                if is_speech {
                    speech_frames += 1;
                }
            }
        }

        speech_frames as f32 / num_frames as f32
    }

    /// Returns true if the speech ratio exceeds the threshold.
    ///
    /// # Arguments
    ///
    /// * `samples` - Audio samples at 16kHz as f32.
    /// * `threshold` - Minimum speech ratio to consider as having speech (0.0-1.0).
    ///
    /// # Returns
    ///
    /// `true` if speech ratio >= threshold.
    pub fn has_speech(&mut self, samples: &[f32], threshold: f32) -> bool {
        self.speech_ratio(samples) >= threshold
    }
}

impl std::fmt::Debug for VoiceActivityDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceActivityDetector")
            .field("sample_rate", &"16kHz")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_creation() {
        let vad = VoiceActivityDetector::new(2);
        assert!(vad.is_ok());
    }

    #[test]
    fn test_vad_silence() {
        let mut vad = VoiceActivityDetector::new(2).unwrap();

        // Silent frame (160 samples = 10ms at 16kHz)
        let silence: Vec<i16> = vec![0; 160];
        let result = vad.is_speech(&silence);

        assert!(result.is_ok());
        // Silence should not be detected as speech
        assert!(!result.unwrap());
    }

    #[test]
    fn test_vad_invalid_frame_size() {
        let mut vad = VoiceActivityDetector::new(2).unwrap();

        // Invalid frame size
        let samples: Vec<i16> = vec![0; 100];
        let result = vad.is_speech(&samples);

        assert!(result.is_err());
    }

    #[test]
    fn test_vad_f32_conversion() {
        let mut vad = VoiceActivityDetector::new(2).unwrap();

        // Silent frame as f32
        let silence: Vec<f32> = vec![0.0; 160];
        let result = vad.is_speech_f32(&silence);

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_speech_ratio_empty() {
        let mut vad = VoiceActivityDetector::new(2).unwrap();

        // Too short for even one frame
        let short: Vec<f32> = vec![0.0; 100];
        let ratio = vad.speech_ratio(&short);

        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_speech_ratio_silence() {
        let mut vad = VoiceActivityDetector::new(2).unwrap();

        // Multiple silent frames
        let silence: Vec<f32> = vec![0.0; 1600]; // 100ms
        let ratio = vad.speech_ratio(&silence);

        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_has_speech_threshold() {
        let mut vad = VoiceActivityDetector::new(2).unwrap();

        let silence: Vec<f32> = vec![0.0; 1600];

        // With 0.0 threshold, even silence passes
        assert!(vad.has_speech(&silence, 0.0));

        // With any positive threshold, silence fails
        assert!(!vad.has_speech(&silence, 0.1));
    }
}
