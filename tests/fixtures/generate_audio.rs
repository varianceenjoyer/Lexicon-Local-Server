//! Audio fixture generator for Lexicon Local Server tests.
//!
//! Run with: cargo run --bin generate-fixtures
//!
//! Generates synthetic WAV files for testing the audio pipeline.

use std::f32::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

const SAMPLE_RATE: u32 = 16000;

fn main() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("audio");

    std::fs::create_dir_all(&fixtures_dir).expect("Failed to create fixtures directory");

    println!("Generating audio fixtures in {:?}", fixtures_dir);

    // 10 seconds of silence (for VAD testing)
    generate_silence(&fixtures_dir.join("silence_10s.wav"), 10.0);
    println!("  Created: silence_10s.wav");

    // 5 seconds of synthetic speech-like audio (tone bursts)
    generate_speech_like(&fixtures_dir.join("speech_like_5s.wav"), 5.0);
    println!("  Created: speech_like_5s.wav");

    // 2 seconds of sine wave (for audio pipeline testing)
    generate_sine_wave(&fixtures_dir.join("sine_440hz_2s.wav"), 440.0, 2.0);
    println!("  Created: sine_440hz_2s.wav");

    // 1 second of white noise
    generate_noise(&fixtures_dir.join("noise_1s.wav"), 1.0);
    println!("  Created: noise_1s.wav");

    println!("Done! Generated 4 audio fixtures.");
}

/// Write WAV header for 16-bit mono PCM
fn write_wav_header(writer: &mut BufWriter<File>, num_samples: u32) {
    let byte_rate = SAMPLE_RATE * 2; // 16-bit mono
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;

    // RIFF header
    writer.write_all(b"RIFF").unwrap();
    writer.write_all(&file_size.to_le_bytes()).unwrap();
    writer.write_all(b"WAVE").unwrap();

    // fmt chunk
    writer.write_all(b"fmt ").unwrap();
    writer.write_all(&16u32.to_le_bytes()).unwrap(); // chunk size
    writer.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    writer.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    writer.write_all(&SAMPLE_RATE.to_le_bytes()).unwrap();
    writer.write_all(&byte_rate.to_le_bytes()).unwrap();
    writer.write_all(&2u16.to_le_bytes()).unwrap(); // block align
    writer.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample

    // data chunk
    writer.write_all(b"data").unwrap();
    writer.write_all(&data_size.to_le_bytes()).unwrap();
}

fn write_samples(writer: &mut BufWriter<File>, samples: &[f32]) {
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let i16_sample = (clamped * 32767.0) as i16;
        writer.write_all(&i16_sample.to_le_bytes()).unwrap();
    }
}

fn generate_silence(path: &Path, duration_s: f32) {
    let num_samples = (SAMPLE_RATE as f32 * duration_s) as u32;
    let file = File::create(path).expect("Failed to create file");
    let mut writer = BufWriter::new(file);

    write_wav_header(&mut writer, num_samples);

    let samples = vec![0.0f32; num_samples as usize];
    write_samples(&mut writer, &samples);
}

fn generate_sine_wave(path: &Path, frequency: f32, duration_s: f32) {
    let num_samples = (SAMPLE_RATE as f32 * duration_s) as u32;
    let file = File::create(path).expect("Failed to create file");
    let mut writer = BufWriter::new(file);

    write_wav_header(&mut writer, num_samples);

    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            0.5 * (2.0 * PI * frequency * t).sin()
        })
        .collect();

    write_samples(&mut writer, &samples);
}

fn generate_noise(path: &Path, duration_s: f32) {
    let num_samples = (SAMPLE_RATE as f32 * duration_s) as u32;
    let file = File::create(path).expect("Failed to create file");
    let mut writer = BufWriter::new(file);

    write_wav_header(&mut writer, num_samples);

    // Simple pseudo-random noise using LCG
    let mut seed: u32 = 12345;
    let samples: Vec<f32> = (0..num_samples)
        .map(|_| {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let normalized = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            normalized * 0.3 // Reduce amplitude
        })
        .collect();

    write_samples(&mut writer, &samples);
}

fn generate_speech_like(path: &Path, duration_s: f32) {
    let num_samples = (SAMPLE_RATE as f32 * duration_s) as u32;
    let file = File::create(path).expect("Failed to create file");
    let mut writer = BufWriter::new(file);

    write_wav_header(&mut writer, num_samples);

    // Generate speech-like audio with:
    // - Fundamental frequency around 150Hz (typical speech)
    // - Harmonics
    // - Amplitude modulation to simulate syllables
    // - Occasional pauses

    let fundamental = 150.0;
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;

            // Syllable envelope (4 Hz modulation = ~4 syllables per second)
            let syllable_env = ((2.0 * PI * 4.0 * t).sin() * 0.5 + 0.5).powi(2);

            // Pause detection (silence every ~1.5 seconds for 0.2s)
            let pause_period = 1.5;
            let pause_duration = 0.2;
            let in_pause = (t % pause_period) < pause_duration;

            if in_pause {
                0.0
            } else {
                // Fundamental + harmonics
                let f0 = (2.0 * PI * fundamental * t).sin();
                let f1 = 0.5 * (2.0 * PI * fundamental * 2.0 * t).sin();
                let f2 = 0.25 * (2.0 * PI * fundamental * 3.0 * t).sin();
                let f3 = 0.125 * (2.0 * PI * fundamental * 4.0 * t).sin();

                let signal = f0 + f1 + f2 + f3;
                signal * syllable_env * 0.3 // Scale down
            }
        })
        .collect();

    write_samples(&mut writer, &samples);
}
