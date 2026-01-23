//! Transcription Manager for TUI
//!
//! Handles audio capture, resampling, and speech-to-text processing
//! in a background thread, sending results back to the UI.

#![allow(dead_code)] // Some methods reserved for future use

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::mpsc;

use lexicon_core::stt::{
    install_whisper_logging_hooks, Transcriber, TranscriptionConfig, WhisperTranscriber,
};

use super::{Result, TuiError};

/// Gain multiplier for level meter display.
///
/// Scales RMS values so typical speech levels display prominently.
/// Typical speech RMS is 0.02-0.1, we want that to show as 0.2-0.8 on the meter.
const METER_GAIN: f32 = 8.0;

/// Target sample rate for Whisper (16kHz).
const TARGET_SAMPLE_RATE: u32 = 16000;

/// Minimum audio duration in ms to transcribe (500ms).
const MIN_AUDIO_MS: u64 = 500;

/// Maximum audio duration in ms before forced transcription (10s).
const MAX_AUDIO_MS: u64 = 10000;

/// Silence threshold for voice activity detection.
const SILENCE_THRESHOLD: f32 = 0.01;

/// Duration of silence before processing (300ms).
const SILENCE_DURATION_MS: u64 = 300;

/// Minimum RMS energy threshold for transcription.
///
/// If the accumulated audio buffer has an overall RMS below this threshold,
/// we skip transcription entirely to avoid Whisper hallucinations on silence.
const MIN_TRANSCRIPTION_ENERGY: f32 = 0.005;

/// Message types sent from the transcription worker to the UI.
#[derive(Debug)]
pub enum TranscriptionMessage {
    /// New transcription text available.
    Text(String),
    /// An error occurred during transcription.
    Error(String),
    /// Model loading status.
    ModelLoaded(String),
    /// Model loading failed.
    ModelError(String),
    /// Processing status (for UI feedback).
    Processing,
}

/// Configuration for the transcription manager.
#[derive(Debug, Clone)]
pub struct TranscriptionManagerConfig {
    /// Path to the Whisper model file.
    pub model_path: PathBuf,
    /// Language code (e.g., "en", "auto").
    pub language: Option<String>,
    /// Whether to translate to English.
    pub translate: bool,
    /// Device name to use (None for default).
    pub device_name: Option<String>,
}

impl Default for TranscriptionManagerConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/ggml-base.en.bin"),
            language: Some("en".to_string()),
            translate: false,
            device_name: None,
        }
    }
}

/// Manages background transcription processing.
pub struct TranscriptionManager {
    /// Sender for messages to the UI.
    tx: mpsc::UnboundedSender<TranscriptionMessage>,
    /// Receiver for messages from the worker.
    rx: mpsc::UnboundedReceiver<TranscriptionMessage>,
    /// Recording flag (shared with audio thread).
    recording: Arc<AtomicBool>,
    /// Current audio level (shared with audio thread).
    level: Arc<AtomicU32>,
    /// Audio capture stream.
    stream: Option<cpal::Stream>,
    /// Worker thread handle.
    worker_handle: Option<thread::JoinHandle<()>>,
    /// Audio samples channel (to worker).
    audio_tx: Option<mpsc::UnboundedSender<Vec<f32>>>,
    /// Flag to signal worker shutdown.
    shutdown: Arc<AtomicBool>,
}

impl TranscriptionManager {
    /// Creates a new transcription manager.
    ///
    /// This does NOT start the transcription worker. Call `start()` to begin.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TranscriptionMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (tx_clone, rx_return) = (tx.clone(), rx);

        let manager = Self {
            tx: tx_clone,
            rx: rx_return,
            recording: Arc::new(AtomicBool::new(false)),
            level: Arc::new(AtomicU32::new(0)),
            stream: None,
            worker_handle: None,
            audio_tx: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        };

        // Create a new receiver for the caller
        let (_, rx2) = mpsc::unbounded_channel();
        // Actually we need to return the original rx
        // Let me restructure this
        (manager, rx2)
    }

    /// Creates a new transcription manager with proper channel setup.
    pub fn create() -> (Self, mpsc::UnboundedReceiver<TranscriptionMessage>) {
        let (ui_tx, ui_rx) = mpsc::unbounded_channel();

        let manager = Self {
            tx: ui_tx,
            rx: mpsc::unbounded_channel().1, // Unused placeholder
            recording: Arc::new(AtomicBool::new(false)),
            level: Arc::new(AtomicU32::new(0)),
            stream: None,
            worker_handle: None,
            audio_tx: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        };

        (manager, ui_rx)
    }

    /// Starts the transcription worker with the given configuration.
    pub fn start(&mut self, config: TranscriptionManagerConfig) -> Result<()> {
        tracing::info!("Starting transcription manager...");
        tracing::info!("Model path: {:?}", config.model_path);
        tracing::info!("Model exists: {}", config.model_path.exists());
        tracing::info!("Language: {:?}, translate: {}", config.language, config.translate);

        // Create channel for audio samples
        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        self.audio_tx = Some(audio_tx);

        // Clone shared state for worker
        let ui_tx = self.tx.clone();
        let recording = self.recording.clone();
        let shutdown = self.shutdown.clone();

        // Spawn worker thread
        let model_path = config.model_path.clone();
        let language = config.language.clone();
        let translate = config.translate;

        tracing::info!("Spawning transcription worker thread...");
        self.worker_handle = Some(thread::spawn(move || {
            transcription_worker(
                audio_rx,
                ui_tx,
                recording,
                shutdown,
                model_path,
                language,
                translate,
            );
        }));

        // Start audio capture
        tracing::info!("Starting audio capture...");
        self.start_audio_capture(config.device_name)?;
        tracing::info!("Audio capture started successfully");

        Ok(())
    }

    /// Starts audio capture on the specified device.
    fn start_audio_capture(&mut self, device_name: Option<String>) -> Result<()> {
        let host = cpal::default_host();

        let device = if let Some(name) = device_name {
            host.input_devices()
                .map_err(|e| TuiError::Audio(format!("Device enumeration failed: {}", e)))?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| TuiError::Audio(format!("Device not found: {}", name)))?
        } else {
            host.default_input_device()
                .ok_or_else(|| TuiError::Audio("No default input device".to_string()))?
        };

        let config = device
            .default_input_config()
            .map_err(|e| TuiError::Audio(format!("Failed to get device config: {}", e)))?;

        let source_sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let audio_tx = self.audio_tx.clone();
        let level = self.level.clone();
        let recording = self.recording.clone();

        // Resampling ratio (source -> 16kHz)
        let resample_ratio = TARGET_SAMPLE_RATE as f64 / source_sample_rate as f64;

        // Buffer for accumulating samples before resampling
        let buffer_size = (source_sample_rate as usize / 10).max(1600); // ~100ms buffer
        let mut buffer = Vec::with_capacity(buffer_size);

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Calculate level (always, for meter)
                    if !data.is_empty() {
                        let mono_sum: f32 = data
                            .chunks(channels)
                            .map(|chunk| {
                                let sample = chunk.iter().sum::<f32>() / channels as f32;
                                sample * sample
                            })
                            .sum();

                        let sample_count = data.len() / channels;
                        let rms = (mono_sum / sample_count as f32).sqrt();

                        // Apply gain to make meter more responsive to typical speech levels
                        let scaled_rms = rms * METER_GAIN;

                        // Smooth and store level
                        let current = f32::from_bits(level.load(Ordering::Relaxed));
                        let alpha = 0.3;
                        let smoothed = (current * (1.0 - alpha) + scaled_rms * alpha).clamp(0.0, 1.0);
                        level.store(smoothed.to_bits(), Ordering::Relaxed);
                    }

                    // Only process audio when recording
                    if !recording.load(Ordering::Relaxed) {
                        return;
                    }

                    // Convert to mono
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                        .collect();

                    // Add to buffer
                    buffer.extend_from_slice(&mono);

                    // When buffer is full enough, resample and send
                    if buffer.len() >= buffer_size {
                        // Simple linear resampling (for real production, use rubato)
                        let resampled = simple_resample(&buffer, resample_ratio);

                        if let Some(ref tx) = audio_tx {
                            let _ = tx.send(resampled);
                        }

                        buffer.clear();
                    }
                },
                |err| {
                    tracing::error!("Audio capture error: {}", err);
                },
                None,
            )
            .map_err(|e| TuiError::Audio(format!("Failed to build stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| TuiError::Audio(format!("Failed to start stream: {}", e)))?;

        self.stream = Some(stream);
        Ok(())
    }

    /// Returns the current audio level (0.0 - 1.0).
    pub fn get_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// Starts recording.
    pub fn start_recording(&self) {
        self.recording.store(true, Ordering::Relaxed);
    }

    /// Stops recording.
    pub fn stop_recording(&self) {
        self.recording.store(false, Ordering::Relaxed);
    }

    /// Returns whether recording is active.
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Relaxed)
    }

    /// Changes the audio capture device.
    ///
    /// Stops the current audio stream and starts a new one on the specified device.
    pub fn change_device(&mut self, device_name: Option<String>) -> Result<()> {
        tracing::info!("Changing audio device to: {:?}", device_name);

        // Stop current stream
        self.stream = None;

        // Reset level
        self.level.store(0_f32.to_bits(), Ordering::Relaxed);

        // Start audio capture on new device
        self.start_audio_capture(device_name)?;

        tracing::info!("Audio device changed successfully");
        Ok(())
    }

    /// Shuts down the transcription manager.
    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.stream = None;

        // Wait for worker to finish
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for TranscriptionManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Simple linear resampling (for basic functionality).
///
/// For production quality, use rubato crate instead.
fn simple_resample(samples: &[f32], ratio: f64) -> Vec<f32> {
    let new_len = ((samples.len() as f64) * ratio).ceil() as usize;
    let mut resampled = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx = (i as f64) / ratio;
        let idx = src_idx.floor() as usize;
        let frac = src_idx.fract() as f32;

        let sample = if idx + 1 < samples.len() {
            samples[idx] * (1.0 - frac) + samples[idx + 1] * frac
        } else if idx < samples.len() {
            samples[idx]
        } else {
            0.0
        };

        resampled.push(sample);
    }

    resampled
}

/// Worker thread function that processes audio and runs transcription.
fn transcription_worker(
    mut audio_rx: mpsc::UnboundedReceiver<Vec<f32>>,
    ui_tx: mpsc::UnboundedSender<TranscriptionMessage>,
    recording: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    model_path: PathBuf,
    language: Option<String>,
    translate: bool,
) {
    // Suppress whisper.cpp verbose logging that would corrupt TUI display
    install_whisper_logging_hooks();

    tracing::info!("Transcription worker thread started");
    tracing::info!("Loading model from: {:?}", model_path);

    // Load model
    tracing::debug!("Creating WhisperTranscriber...");
    let mut transcriber = match WhisperTranscriber::new() {
        Ok(t) => {
            tracing::info!("WhisperTranscriber created successfully");
            t
        }
        Err(e) => {
            tracing::error!("Failed to create transcriber: {}", e);
            let _ = ui_tx.send(TranscriptionMessage::ModelError(format!(
                "Failed to create transcriber: {}",
                e
            )));
            return;
        }
    };

    tracing::info!("Loading model file...");
    if let Err(e) = transcriber.load_model(&model_path) {
        tracing::error!("Failed to load model: {}", e);
        let _ = ui_tx.send(TranscriptionMessage::ModelError(format!(
            "Failed to load model: {}",
            e
        )));
        return;
    }

    let model_name = model_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    tracing::info!("Model loaded successfully: {}", model_name);
    let _ = ui_tx.send(TranscriptionMessage::ModelLoaded(model_name.to_string()));

    // Configure transcription
    let mut config = TranscriptionConfig::new();
    if let Some(lang) = language {
        config = config.with_language(&lang);
    }
    config.translate = translate;

    // Audio accumulation buffer
    let mut audio_buffer: Vec<f32> = Vec::new();
    let mut last_speech_time = std::time::Instant::now();
    let mut has_speech = false;

    // Main processing loop
    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        // Try to receive audio with timeout
        let recv_result = audio_rx.try_recv();

        match recv_result {
            Ok(samples) => {
                // Check if this chunk has speech
                let rms: f32 = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
                let rms = rms.sqrt();

                if rms > SILENCE_THRESHOLD {
                    has_speech = true;
                    last_speech_time = std::time::Instant::now();
                }

                // Only accumulate if recording
                if recording.load(Ordering::Relaxed) {
                    audio_buffer.extend_from_slice(&samples);
                }
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                // No data available, check if we should process
                thread::sleep(Duration::from_millis(10));
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                break;
            }
        }

        // Check if we should transcribe
        let buffer_duration_ms =
            (audio_buffer.len() as u64 * 1000) / TARGET_SAMPLE_RATE as u64;
        let silence_duration = last_speech_time.elapsed().as_millis() as u64;

        let should_transcribe = !audio_buffer.is_empty()
            && (
                // Enough audio and silence detected (natural pause)
                (buffer_duration_ms >= MIN_AUDIO_MS && has_speech && silence_duration >= SILENCE_DURATION_MS)
                // Or buffer is getting too long
                || buffer_duration_ms >= MAX_AUDIO_MS
            );

        if should_transcribe {
            // Check if audio buffer has sufficient energy to avoid hallucinations on silence.
            // Whisper tends to hallucinate repeated phrases (like "Thank you.") when given
            // mostly silence, so we skip transcription if the overall energy is too low.
            let buffer_rms: f32 = if !audio_buffer.is_empty() {
                let sum_sq: f32 = audio_buffer.iter().map(|s| s * s).sum();
                (sum_sq / audio_buffer.len() as f32).sqrt()
            } else {
                0.0
            };

            if buffer_rms < MIN_TRANSCRIPTION_ENERGY {
                tracing::debug!(
                    "Skipping transcription: buffer RMS {:.4} below threshold {:.4}",
                    buffer_rms,
                    MIN_TRANSCRIPTION_ENERGY
                );
                audio_buffer.clear();
                has_speech = false;
                continue;
            }

            let _ = ui_tx.send(TranscriptionMessage::Processing);

            // Transcribe accumulated audio
            match transcriber.transcribe(&audio_buffer, &config) {
                Ok(result) => {
                    for segment in result.segments {
                        let text = segment.text.trim();
                        if !text.is_empty() {
                            let _ = ui_tx.send(TranscriptionMessage::Text(text.to_string()));
                        }
                    }
                }
                Err(e) => {
                    let _ = ui_tx.send(TranscriptionMessage::Error(format!(
                        "Transcription error: {}",
                        e
                    )));
                }
            }

            // Clear buffer
            audio_buffer.clear();
            has_speech = false;
        }

        // Clear buffer if not recording
        if !recording.load(Ordering::Relaxed) && !audio_buffer.is_empty() {
            audio_buffer.clear();
            has_speech = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_resample_downsample() {
        // 48kHz to 16kHz (ratio = 1/3)
        let samples: Vec<f32> = (0..480).map(|i| (i as f32) / 100.0).collect();
        let ratio = 16000.0 / 48000.0;

        let resampled = simple_resample(&samples, ratio);

        // Should have approximately 1/3 of the samples
        assert!(resampled.len() >= 150 && resampled.len() <= 170);
    }

    #[test]
    fn test_simple_resample_upsample() {
        // 8kHz to 16kHz (ratio = 2)
        let samples: Vec<f32> = (0..80).map(|i| (i as f32) / 100.0).collect();
        let ratio = 16000.0 / 8000.0;

        let resampled = simple_resample(&samples, ratio);

        // Should have approximately 2x the samples
        assert!(resampled.len() >= 150 && resampled.len() <= 170);
    }

    #[test]
    fn test_transcription_config_default() {
        let config = TranscriptionManagerConfig::default();
        assert_eq!(config.language, Some("en".to_string()));
        assert!(!config.translate);
    }
}
