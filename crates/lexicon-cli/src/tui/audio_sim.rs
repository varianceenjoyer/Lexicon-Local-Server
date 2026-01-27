//! Simulated Audio Source for Testing
//!
//! Provides a simulated audio source that streams audio from files, enabling
//! testing of audio capture and transcription features on platforms where real
//! audio hardware is unavailable (e.g., Android CLI without app context).
//!
//! # Features
//!
//! - Load audio from WAV, MP3, FLAC, OGG files
//! - Automatic resampling to 16kHz mono (Whisper format)
//! - Real-time streaming at natural playback speed
//! - Loop mode for continuous testing
//! - Level meter support (RMS calculation)
//!
//! # Example
//!
//! ```no_run
//! use std::path::PathBuf;
//! use tokio::sync::mpsc;
//!
//! let config = SimulatedAudioConfig {
//!     file_path: PathBuf::from("test.wav"),
//!     loop_playback: false,
//!     speed: 1.0,
//! };
//!
//! let mut source = SimulatedAudioSource::new(config).unwrap();
//! let (tx, rx) = mpsc::unbounded_channel();
//! source.start(tx).unwrap();
//! ```

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rubato::{FftFixedIn, Resampler};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Target sample rate for Whisper (16kHz).
const TARGET_SAMPLE_RATE: u32 = 16000;

/// Chunk size in samples (~100ms at 16kHz = 1600 samples).
const CHUNK_SAMPLES: usize = 1600;

/// Gain multiplier for level meter display.
const METER_GAIN: f32 = 8.0;

/// Configuration for the simulated audio source.
#[derive(Debug, Clone)]
pub struct SimulatedAudioConfig {
    /// Path to the audio file to stream.
    pub file_path: PathBuf,
    /// Whether to loop the audio file continuously.
    pub loop_playback: bool,
    /// Playback speed multiplier (1.0 = real-time, 2.0 = 2x speed).
    pub speed: f32,
}

impl Default for SimulatedAudioConfig {
    fn default() -> Self {
        Self {
            file_path: PathBuf::new(),
            loop_playback: false,
            speed: 1.0,
        }
    }
}

/// Errors that can occur with simulated audio.
#[derive(Debug, Error)]
pub enum AudioSimError {
    #[error("Failed to load audio file: {0}")]
    LoadError(String),
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),
    #[error("Playback error: {0}")]
    PlaybackError(String),
    #[error("Resampling error: {0}")]
    ResampleError(String),
}

/// A simulated audio source that streams audio from a file.
pub struct SimulatedAudioSource {
    config: SimulatedAudioConfig,
    /// Current audio level (0.0 - 1.0), stored as u32 bits of f32.
    level: Arc<AtomicU32>,
    /// Whether playback is active.
    playing: Arc<AtomicBool>,
    /// Current playback position in samples.
    position: Arc<AtomicU64>,
    /// Total duration in samples.
    total_samples: u64,
    /// Worker thread handle.
    worker_handle: Option<thread::JoinHandle<()>>,
    /// Shutdown signal.
    shutdown: Arc<AtomicBool>,
}

impl SimulatedAudioSource {
    /// Creates a new simulated audio source.
    ///
    /// This loads and validates the audio file but does not start streaming.
    pub fn new(config: SimulatedAudioConfig) -> Result<Self, AudioSimError> {
        // Validate file exists
        if !config.file_path.exists() {
            return Err(AudioSimError::LoadError(format!(
                "File not found: {}",
                config.file_path.display()
            )));
        }

        // Probe the file to get duration
        let total_samples = probe_audio_duration(&config.file_path)?;

        Ok(Self {
            config,
            level: Arc::new(AtomicU32::new(0)),
            playing: Arc::new(AtomicBool::new(false)),
            position: Arc::new(AtomicU64::new(0)),
            total_samples,
            worker_handle: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Starts streaming audio to the provided sender.
    ///
    /// Audio is sent as `Vec<f32>` chunks at 16kHz mono (Whisper format).
    /// Each chunk is approximately 100ms of audio (1600 samples).
    pub fn start(
        &mut self,
        audio_tx: mpsc::UnboundedSender<Vec<f32>>,
    ) -> Result<(), AudioSimError> {
        if self.playing.load(Ordering::SeqCst) {
            return Err(AudioSimError::PlaybackError(
                "Already playing".to_string(),
            ));
        }

        self.shutdown.store(false, Ordering::SeqCst);
        self.playing.store(true, Ordering::SeqCst);
        self.position.store(0, Ordering::SeqCst);

        let config = self.config.clone();
        let level = self.level.clone();
        let playing = self.playing.clone();
        let position = self.position.clone();
        let shutdown = self.shutdown.clone();

        self.worker_handle = Some(thread::spawn(move || {
            if let Err(e) = stream_audio_file(
                &config,
                audio_tx,
                level,
                playing,
                position,
                shutdown,
            ) {
                warn!("Simulated audio error: {}", e);
            }
        }));

        info!(
            "Started simulated audio from: {}",
            self.config.file_path.display()
        );

        Ok(())
    }

    /// Stops streaming.
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.playing.store(false, Ordering::SeqCst);

        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        info!("Stopped simulated audio");
    }

    /// Returns the current audio level (0.0 - 1.0).
    pub fn get_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// Returns whether the source is currently playing.
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::SeqCst)
    }

    /// Returns total duration of the audio file.
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.total_samples as f64 / TARGET_SAMPLE_RATE as f64)
    }

    /// Returns current playback position.
    pub fn position(&self) -> Duration {
        let samples = self.position.load(Ordering::Relaxed);
        Duration::from_secs_f64(samples as f64 / TARGET_SAMPLE_RATE as f64)
    }

    /// Returns the file path being played.
    pub fn file_path(&self) -> &PathBuf {
        &self.config.file_path
    }
}

impl Drop for SimulatedAudioSource {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Probe audio file to get duration in samples at 16kHz.
fn probe_audio_duration(path: &PathBuf) -> Result<u64, AudioSimError> {
    let file = std::fs::File::open(path)
        .map_err(|e| AudioSimError::LoadError(format!("Cannot open file: {}", e)))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AudioSimError::UnsupportedFormat(format!("Probe failed: {}", e)))?;

    let format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| AudioSimError::LoadError("No audio track found".to_string()))?;

    // Calculate duration in samples at 16kHz
    let codec_params = &track.codec_params;
    let source_sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let n_frames = codec_params.n_frames.unwrap_or(0);

    // Convert to 16kHz sample count
    let duration_secs = n_frames as f64 / source_sample_rate as f64;
    let samples_16k = (duration_secs * TARGET_SAMPLE_RATE as f64) as u64;

    debug!(
        "Audio file: {} samples at {}Hz -> {} samples at 16kHz ({:.1}s)",
        n_frames, source_sample_rate, samples_16k, duration_secs
    );

    Ok(samples_16k)
}

/// Stream audio from file to channel with real-time pacing.
fn stream_audio_file(
    config: &SimulatedAudioConfig,
    audio_tx: mpsc::UnboundedSender<Vec<f32>>,
    level: Arc<AtomicU32>,
    playing: Arc<AtomicBool>,
    position: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
) -> Result<(), AudioSimError> {
    loop {
        // Load and decode the audio file
        let samples = load_and_resample_audio(&config.file_path)?;

        if samples.is_empty() {
            warn!("Audio file is empty");
            break;
        }

        info!(
            "Loaded {} samples ({:.1}s) from {}",
            samples.len(),
            samples.len() as f64 / TARGET_SAMPLE_RATE as f64,
            config.file_path.display()
        );

        // Stream chunks with real-time pacing
        let chunk_duration = Duration::from_secs_f64(
            CHUNK_SAMPLES as f64 / TARGET_SAMPLE_RATE as f64 / config.speed as f64,
        );

        let mut offset = 0;
        let start_time = Instant::now();
        let mut expected_time = Duration::ZERO;

        while offset < samples.len() && !shutdown.load(Ordering::SeqCst) {
            let end = (offset + CHUNK_SAMPLES).min(samples.len());
            let chunk: Vec<f32> = samples[offset..end].to_vec();

            // Calculate RMS level for the chunk
            let rms = calculate_rms(&chunk);
            let display_level = (rms * METER_GAIN).clamp(0.0, 1.0);
            level.store(display_level.to_bits(), Ordering::Relaxed);

            // Update position
            position.store((offset + chunk.len()) as u64, Ordering::Relaxed);

            // Send chunk to transcription pipeline
            if audio_tx.send(chunk).is_err() {
                debug!("Audio channel closed, stopping simulation");
                break;
            }

            offset += CHUNK_SAMPLES;

            // Real-time pacing with drift correction
            expected_time += chunk_duration;
            let elapsed = start_time.elapsed();
            if expected_time > elapsed {
                thread::sleep(expected_time - elapsed);
            }
        }

        // Check if we should loop
        if !config.loop_playback || shutdown.load(Ordering::SeqCst) {
            break;
        }

        info!("Looping audio playback");
        position.store(0, Ordering::Relaxed);
    }

    playing.store(false, Ordering::SeqCst);
    level.store(0.0_f32.to_bits(), Ordering::Relaxed);

    // Send empty chunk to signal completion
    let _ = audio_tx.send(Vec::new());

    Ok(())
}

/// Load audio file and resample to 16kHz mono.
fn load_and_resample_audio(path: &PathBuf) -> Result<Vec<f32>, AudioSimError> {
    let file = std::fs::File::open(path)
        .map_err(|e| AudioSimError::LoadError(format!("Cannot open file: {}", e)))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AudioSimError::UnsupportedFormat(format!("Probe failed: {}", e)))?;

    let mut format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| AudioSimError::LoadError("No audio track found".to_string()))?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let source_sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params.channels.map(|c| c.count()).unwrap_or(2);

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| AudioSimError::UnsupportedFormat(format!("Decoder error: {}", e)))?;

    // Collect all samples
    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                warn!("Error reading packet: {}", e);
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(e) => {
                warn!("Error decoding packet: {}", e);
                continue;
            }
        };

        // Convert to f32 samples
        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;
        let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
        sample_buf.copy_interleaved_ref(decoded);

        // Convert to mono by averaging channels
        let samples = sample_buf.samples();
        for chunk in samples.chunks(channels) {
            let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
            all_samples.push(mono);
        }
    }

    debug!(
        "Decoded {} samples at {}Hz ({} channels)",
        all_samples.len(),
        source_sample_rate,
        channels
    );

    // Resample to 16kHz if needed
    if source_sample_rate != TARGET_SAMPLE_RATE {
        all_samples = resample_audio(&all_samples, source_sample_rate, TARGET_SAMPLE_RATE)?;
        debug!("Resampled to {} samples at 16kHz", all_samples.len());
    }

    Ok(all_samples)
}

/// Resample audio using rubato.
fn resample_audio(
    input: &[f32],
    source_rate: u32,
    target_rate: u32,
) -> Result<Vec<f32>, AudioSimError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let ratio = target_rate as f64 / source_rate as f64;

    // Create resampler
    let mut resampler = FftFixedIn::<f32>::new(
        source_rate as usize,
        target_rate as usize,
        1024, // chunk size
        2,    // sub chunks
        1,    // channels (mono)
    )
    .map_err(|e| AudioSimError::ResampleError(format!("Resampler init error: {}", e)))?;

    // Process in chunks
    let chunk_size = 1024;
    let mut output = Vec::with_capacity((input.len() as f64 * ratio * 1.1) as usize);

    for chunk in input.chunks(chunk_size) {
        // Pad last chunk if needed
        let mut input_chunk = vec![chunk.to_vec()];
        if chunk.len() < chunk_size {
            input_chunk[0].resize(chunk_size, 0.0);
        }

        match resampler.process(&input_chunk, None) {
            Ok(resampled) => {
                if !resampled.is_empty() && !resampled[0].is_empty() {
                    output.extend_from_slice(&resampled[0]);
                }
            }
            Err(e) => {
                warn!("Resampling error: {}", e);
            }
        }
    }

    Ok(output)
}

/// Calculate RMS (root mean square) of audio samples.
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_calculate_rms() {
        // Silence
        assert_eq!(calculate_rms(&[0.0, 0.0, 0.0]), 0.0);

        // Unit signal
        let rms = calculate_rms(&[1.0, 1.0, 1.0, 1.0]);
        assert!((rms - 1.0).abs() < 0.001);

        // Known RMS: sin wave has RMS of 1/sqrt(2) ≈ 0.707
        let samples: Vec<f32> = (0..1000)
            .map(|i| (i as f32 * 0.01).sin())
            .collect();
        let rms = calculate_rms(&samples);
        assert!((rms - 0.707).abs() < 0.1);
    }

    #[test]
    fn test_config_default() {
        let config = SimulatedAudioConfig::default();
        assert!(!config.loop_playback);
        assert!((config.speed - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_source_file_not_found() {
        let config = SimulatedAudioConfig {
            file_path: PathBuf::from("/nonexistent/file.wav"),
            ..Default::default()
        };
        let result = SimulatedAudioSource::new(config);
        assert!(matches!(result, Err(AudioSimError::LoadError(_))));
    }

    #[test]
    fn test_create_simple_wav() {
        // Create a simple WAV file for testing
        let mut temp = NamedTempFile::new().unwrap();

        // WAV header for 16kHz mono 16-bit, 1 second of silence
        let sample_rate: u32 = 16000;
        let num_samples: u32 = 16000; // 1 second
        let data_size: u32 = num_samples * 2; // 16-bit = 2 bytes per sample

        // RIFF header
        temp.write_all(b"RIFF").unwrap();
        temp.write_all(&(36 + data_size).to_le_bytes()).unwrap();
        temp.write_all(b"WAVE").unwrap();

        // fmt chunk
        temp.write_all(b"fmt ").unwrap();
        temp.write_all(&16u32.to_le_bytes()).unwrap(); // chunk size
        temp.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
        temp.write_all(&1u16.to_le_bytes()).unwrap(); // mono
        temp.write_all(&sample_rate.to_le_bytes()).unwrap();
        temp.write_all(&(sample_rate * 2).to_le_bytes()).unwrap(); // byte rate
        temp.write_all(&2u16.to_le_bytes()).unwrap(); // block align
        temp.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample

        // data chunk
        temp.write_all(b"data").unwrap();
        temp.write_all(&data_size.to_le_bytes()).unwrap();

        // Silence (zeros)
        for _ in 0..num_samples {
            temp.write_all(&0i16.to_le_bytes()).unwrap();
        }
        temp.flush().unwrap();

        // Test loading
        let config = SimulatedAudioConfig {
            file_path: temp.path().to_path_buf(),
            ..Default::default()
        };

        let source = SimulatedAudioSource::new(config);
        assert!(source.is_ok(), "Failed to load WAV: {:?}", source.err());

        let source = source.unwrap();
        assert!(!source.is_playing());

        // Duration should be approximately 1 second
        let duration = source.duration();
        assert!(duration.as_secs_f64() > 0.9 && duration.as_secs_f64() < 1.1,
            "Duration was {:?}", duration);
    }

    #[test]
    fn test_load_fixture_wav() {
        // Load the woodchuck.wav fixture if it exists
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/audio/woodchuck.wav");

        if !fixture_path.exists() {
            eprintln!("Skipping test: fixture not found at {:?}", fixture_path);
            return;
        }

        let config = SimulatedAudioConfig {
            file_path: fixture_path,
            loop_playback: false,
            speed: 1.0,
        };

        let source = SimulatedAudioSource::new(config);
        assert!(source.is_ok(), "Failed to load fixture WAV: {:?}", source.err());

        let source = source.unwrap();
        assert!(source.duration().as_secs() > 0, "Duration should be > 0");
    }

    #[test]
    fn test_load_fixture_mp3() {
        // Load the dolly.mp3 fixture if it exists
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/audio/dolly.mp3");

        if !fixture_path.exists() {
            eprintln!("Skipping test: fixture not found at {:?}", fixture_path);
            return;
        }

        let config = SimulatedAudioConfig {
            file_path: fixture_path,
            loop_playback: false,
            speed: 1.0,
        };

        let source = SimulatedAudioSource::new(config);
        assert!(source.is_ok(), "Failed to load fixture MP3: {:?}", source.err());

        let source = source.unwrap();
        assert!(source.duration().as_secs() > 0, "Duration should be > 0");
    }
}
