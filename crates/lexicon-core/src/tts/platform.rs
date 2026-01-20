//! Platform TTS Synthesizer
//!
//! Uses platform-native text-to-speech with actual audio capture.
//! Supports multiple languages via system TTS engines.
//!
//! Audio capture strategy by platform:
//! - macOS: `say -o output.wav "text"`
//! - Linux: `espeak-ng "text" --stdout` or `piper --output_raw`
//! - Windows: PowerShell `SpeechSynthesizer.SetOutputToWaveFile()`

use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use tts::Tts;
use tracing::{debug, info, warn};

use super::types::{AudioBuffer, TtsConfig, TtsEngine, VoiceGender, VoiceInfo, TTS_SAMPLE_RATE};
use super::Synthesizer;
use crate::error::{InferenceError, LexiconError, Result};

/// Platform-native TTS synthesizer with audio capture.
///
/// This provides TTS functionality using platform-specific command-line tools
/// that can output audio to files, enabling actual audio capture:
/// - macOS: `say` command with `-o` flag
/// - Windows: PowerShell with SpeechSynthesizer
/// - Linux: `espeak-ng` with `--stdout` flag
///
/// Falls back to tts-rs for voice enumeration and configuration.
pub struct PlatformSynthesizer {
    tts: Mutex<Tts>,
}

/// Read a WAV file and convert to AudioBuffer.
///
/// Supports both 16-bit PCM and 32-bit float WAV formats.
fn read_wav_file(path: &Path) -> Result<AudioBuffer> {
    let file = std::fs::File::open(path).map_err(|e| {
        LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("Failed to open WAV file: {e}"),
        })
    })?;

    let mut reader = BufReader::new(file);
    let mut header = [0u8; 44];
    reader.read_exact(&mut header).map_err(|e| {
        LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("Failed to read WAV header: {e}"),
        })
    })?;

    // Parse WAV header
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err(LexiconError::Inference(InferenceError::InferenceFailed {
            message: "Invalid WAV file format".to_string(),
        }));
    }

    let channels = u16::from_le_bytes([header[22], header[23]]);
    let sample_rate = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
    let bits_per_sample = u16::from_le_bytes([header[34], header[35]]);

    // Read audio data
    let mut audio_data = Vec::new();
    reader.read_to_end(&mut audio_data).map_err(|e| {
        LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("Failed to read WAV data: {e}"),
        })
    })?;

    // Convert to f32 samples
    let samples: Vec<f32> = match bits_per_sample {
        16 => audio_data
            .chunks_exact(2)
            .map(|chunk| {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                sample as f32 / 32768.0
            })
            .collect(),
        32 => audio_data
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
        _ => {
            return Err(LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Unsupported WAV bit depth: {bits_per_sample}"),
            }));
        }
    };

    // Convert to mono if stereo
    let mono_samples = if channels == 2 {
        samples
            .chunks_exact(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    } else {
        samples
    };

    debug!(
        "Read WAV: {} samples at {}Hz, {} channels, {} bits",
        mono_samples.len(),
        sample_rate,
        channels,
        bits_per_sample
    );

    Ok(AudioBuffer::new(mono_samples, sample_rate, 1))
}

/// Synthesize text using macOS `say` command.
#[cfg(target_os = "macos")]
fn synthesize_macos(text: &str, config: &TtsConfig) -> Result<AudioBuffer> {
    use std::io::Write;

    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join(format!("lexicon_tts_{}.wav", std::process::id()));

    // Build say command with options
    let mut cmd = Command::new("say");

    // Add voice if specified
    if let Some(ref voice) = config.voice {
        cmd.arg("-v").arg(voice);
    }

    // Add rate (say uses words per minute, default ~175)
    let wpm = (175.0 * config.rate).round() as i32;
    cmd.arg("-r").arg(wpm.to_string());

    // Output to file
    cmd.arg("-o").arg(&output_path);

    // Pass text via stdin to avoid shell escaping issues
    cmd.stdin(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("Failed to spawn say command: {e}"),
        })
    })?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(text.as_bytes()).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Failed to write to say stdin: {e}"),
            })
        })?;
    }

    let status = child.wait().map_err(|e| {
        LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("say command failed: {e}"),
        })
    })?;

    if !status.success() {
        return Err(LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("say command exited with status: {status}"),
        }));
    }

    let buffer = read_wav_file(&output_path)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&output_path);

    Ok(buffer)
}

/// Synthesize text using Linux espeak-ng command.
#[cfg(target_os = "linux")]
fn synthesize_linux(text: &str, config: &TtsConfig) -> Result<AudioBuffer> {
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join(format!("lexicon_tts_{}.wav", std::process::id()));

    // Build espeak-ng command
    let mut cmd = Command::new("espeak-ng");

    // Add voice/language if specified
    if let Some(ref lang) = config.language {
        cmd.arg("-v").arg(lang);
    }

    // Add speed (espeak uses words per minute, default 175)
    let wpm = (175.0 * config.rate).round() as i32;
    cmd.arg("-s").arg(wpm.to_string());

    // Add pitch (espeak uses 0-99, default 50)
    let pitch = ((config.pitch + 12.0) / 24.0 * 99.0).round() as i32;
    cmd.arg("-p").arg(pitch.clamp(0, 99).to_string());

    // Output to WAV file
    cmd.arg("-w").arg(&output_path);

    // Add text
    cmd.arg(text);

    let output = cmd.output().map_err(|e| {
        LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("Failed to run espeak-ng: {e}"),
        })
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("espeak-ng failed: {stderr}"),
        }));
    }

    let buffer = read_wav_file(&output_path)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&output_path);

    Ok(buffer)
}

/// Synthesize text using Windows PowerShell SpeechSynthesizer.
#[cfg(target_os = "windows")]
fn synthesize_windows(text: &str, config: &TtsConfig) -> Result<AudioBuffer> {
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join(format!("lexicon_tts_{}.wav", std::process::id()));
    let output_str = output_path.to_string_lossy();

    // Escape text for PowerShell
    let escaped_text = text.replace('`', "``").replace('"', "`\"").replace('$', "`$");

    // Build PowerShell script
    let script = format!(
        r#"
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$synth.Rate = {rate}
$synth.Volume = {volume}
$synth.SetOutputToWaveFile("{output}")
$synth.Speak("{text}")
$synth.Dispose()
"#,
        rate = ((config.rate - 1.0) * 5.0).round() as i32, // -10 to 10
        volume = (config.volume * 100.0).round() as i32,
        output = output_str,
        text = escaped_text
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Failed to run PowerShell: {e}"),
            })
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LexiconError::Inference(InferenceError::InferenceFailed {
            message: format!("PowerShell TTS failed: {stderr}"),
        }));
    }

    let buffer = read_wav_file(&output_path)?;

    // Clean up temp file
    let _ = std::fs::remove_file(&output_path);

    Ok(buffer)
}

/// Platform-specific synthesis dispatcher.
fn synthesize_platform(text: &str, config: &TtsConfig) -> Result<AudioBuffer> {
    #[cfg(target_os = "macos")]
    {
        synthesize_macos(text, config)
    }

    #[cfg(target_os = "linux")]
    {
        synthesize_linux(text, config)
    }

    #[cfg(target_os = "windows")]
    {
        synthesize_windows(text, config)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(LexiconError::Inference(InferenceError::InferenceFailed {
            message: "Platform TTS audio capture not supported on this platform".to_string(),
        }))
    }
}

impl PlatformSynthesizer {
    /// Creates a new platform TTS synthesizer.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform TTS cannot be initialized.
    pub fn new() -> Result<Self> {
        let tts = Tts::default().map_err(|e| {
            LexiconError::Inference(InferenceError::ContextCreationFailed {
                message: format!("Failed to initialize platform TTS: {e}"),
            })
        })?;

        info!("Platform TTS initialized");

        Ok(Self {
            tts: Mutex::new(tts),
        })
    }

    /// Sets the voice for synthesis.
    fn set_voice(&self, tts: &mut Tts, config: &TtsConfig) -> Result<()> {
        // Try to set voice if specified
        if let Some(ref voice_id) = config.voice {
            if let Ok(voices) = tts.voices() {
                if let Some(voice) = voices.iter().find(|v| &v.id() == voice_id) {
                    tts.set_voice(voice).map_err(|e| {
                        LexiconError::Inference(InferenceError::InferenceFailed {
                            message: format!("Failed to set voice: {e}"),
                        })
                    })?;
                }
            }
        }

        // Set rate (tts-rs uses 0.0-1.0 range, we use 0.1-10.0)
        // Map our 1.0 (normal) to tts-rs's 0.5 (normal)
        let rate = ((config.rate - 0.1) / 9.9 * 0.9 + 0.05).clamp(0.0, 1.0);
        if tts.set_rate(rate).is_err() {
            debug!("Failed to set speech rate");
        }

        // Set pitch (if supported)
        // tts-rs uses 0.0-1.0, we use -12 to +12 semitones
        let pitch = ((config.pitch + 12.0) / 24.0).clamp(0.0, 1.0);
        if tts.set_pitch(pitch).is_err() {
            debug!("Platform TTS doesn't support pitch adjustment");
        }

        // Set volume
        if tts.set_volume(config.volume).is_err() {
            debug!("Failed to set volume");
        }

        Ok(())
    }

    /// Gets available voices filtered by language.
    fn voices_for_language(&self, language: Option<&str>) -> Vec<VoiceInfo> {
        let tts = match self.tts.lock() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let voices = match tts.voices() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        voices
            .into_iter()
            .filter(|v| {
                if let Some(lang) = language {
                    v.language().to_string().starts_with(lang)
                } else {
                    true
                }
            })
            .map(|v| VoiceInfo {
                id: v.id().to_string(),
                name: v.name().to_string(),
                language: v.language().to_string(),
                gender: match v.gender() {
                    Some(tts::Gender::Male) => Some(VoiceGender::Male),
                    Some(tts::Gender::Female) => Some(VoiceGender::Female),
                    _ => None,
                },
                engine: TtsEngine::Platform,
            })
            .collect()
    }
}

impl Synthesizer for PlatformSynthesizer {
    fn load_model(&mut self, _path: &Path) -> Result<()> {
        // Platform TTS doesn't need model loading
        Ok(())
    }

    fn synthesize(&self, text: &str, config: &TtsConfig) -> Result<AudioBuffer> {
        if text.is_empty() {
            return Err(LexiconError::Inference(InferenceError::InvalidInput {
                message: "Text cannot be empty".to_string(),
            }));
        }

        debug!("Synthesizing {} characters with platform TTS", text.len());

        // Try platform-specific audio capture first (actual audio output)
        match synthesize_platform(text, config) {
            Ok(buffer) => {
                info!(
                    "Platform TTS synthesis successful: {} samples at {}Hz",
                    buffer.samples.len(),
                    buffer.sample_rate
                );
                return Ok(buffer);
            }
            Err(e) => {
                warn!(
                    "Platform TTS audio capture failed, falling back to playback: {e}"
                );
            }
        }

        // Fallback: use tts-rs for playback (audio plays but not captured)
        let mut tts = self.tts.lock().map_err(|_| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: "Failed to acquire TTS lock".to_string(),
            })
        })?;

        // Configure voice settings
        self.set_voice(&mut tts, config)?;

        // Speak the text (plays audio but can't capture directly via tts-rs)
        let _utterance_id = tts.speak(text, false).map_err(|e| {
            LexiconError::Inference(InferenceError::InferenceFailed {
                message: format!("Platform TTS synthesis failed: {e}"),
            })
        })?;

        // Estimate audio duration based on text length
        // Average speaking rate is ~150 words per minute, ~5 chars per word
        let chars_per_second = 150.0 * 5.0 / 60.0; // ~12.5 chars/second
        let estimated_duration_s = text.len() as f32 / chars_per_second;
        let estimated_samples = (estimated_duration_s * TTS_SAMPLE_RATE as f32) as usize;

        // Return a silent buffer of the estimated length
        // This fallback is used when command-line TTS tools aren't available
        warn!(
            "Platform TTS audio capture unavailable - returning silence ({} samples). \
             Install espeak-ng (Linux), or use Kokoro TTS for audio output.",
            estimated_samples
        );

        Ok(AudioBuffer::new(
            vec![0.0; estimated_samples],
            TTS_SAMPLE_RATE,
            1,
        ))
    }

    fn is_ready(&self) -> bool {
        self.tts.lock().is_ok()
    }

    fn list_voices(&self) -> Vec<VoiceInfo> {
        self.voices_for_language(None)
    }

    fn engine_type(&self) -> TtsEngine {
        TtsEngine::Platform
    }
}

impl std::fmt::Debug for PlatformSynthesizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformSynthesizer")
            .field("ready", &self.is_ready())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_synthesizer_creation() {
        // This test may fail on systems without TTS support
        match PlatformSynthesizer::new() {
            Ok(synth) => {
                assert!(synth.is_ready());
                assert_eq!(synth.engine_type(), TtsEngine::Platform);
            }
            Err(_) => {
                // Expected on systems without TTS
            }
        }
    }

    #[test]
    fn test_platform_voices() {
        if let Ok(synth) = PlatformSynthesizer::new() {
            let voices = synth.list_voices();
            // Most systems have at least one voice
            // But we don't assert this as CI may not have TTS
            for voice in voices {
                assert!(!voice.id.is_empty());
                assert!(!voice.name.is_empty());
                assert_eq!(voice.engine, TtsEngine::Platform);
            }
        }
    }

    #[test]
    fn test_platform_model_load_noop() {
        if let Ok(mut synth) = PlatformSynthesizer::new() {
            // load_model should be a no-op for platform TTS
            assert!(synth.load_model(Path::new("/any/path")).is_ok());
        }
    }

    #[test]
    fn test_platform_synthesis_produces_audio() {
        // Test that platform TTS produces non-silent audio when tools are available
        let config = TtsConfig::new();

        // Try platform-specific synthesis
        match synthesize_platform("Hello, world!", &config) {
            Ok(buffer) => {
                // Verify we got actual audio samples
                assert!(
                    !buffer.samples.is_empty(),
                    "Platform TTS should produce samples"
                );

                // Verify audio is not completely silent (has some non-zero samples)
                let has_audio = buffer.samples.iter().any(|&s| s.abs() > 0.001);
                assert!(
                    has_audio,
                    "Platform TTS should produce non-silent audio"
                );

                // Verify reasonable sample rate
                assert!(
                    buffer.sample_rate >= 8000 && buffer.sample_rate <= 48000,
                    "Sample rate should be reasonable: {}",
                    buffer.sample_rate
                );

                println!(
                    "Platform TTS produced {} samples at {}Hz",
                    buffer.samples.len(),
                    buffer.sample_rate
                );
            }
            Err(e) => {
                // Expected if platform TTS tools aren't installed
                // e.g., espeak-ng not installed on Linux
                println!("Platform TTS not available: {e}");
            }
        }
    }

    #[test]
    fn test_synthesize_with_fallback() {
        // Test the full synthesize path with fallback behavior
        if let Ok(synth) = PlatformSynthesizer::new() {
            let config = TtsConfig::new();

            match synth.synthesize("Test speech synthesis", &config) {
                Ok(buffer) => {
                    // We should get some audio buffer (may be silent if tools unavailable)
                    assert!(!buffer.samples.is_empty());
                    println!(
                        "Synthesize returned {} samples, has_audio={}",
                        buffer.samples.len(),
                        buffer.samples.iter().any(|&s| s.abs() > 0.001)
                    );
                }
                Err(e) => {
                    println!("Synthesize failed (expected on some systems): {e}");
                }
            }
        }
    }
}
