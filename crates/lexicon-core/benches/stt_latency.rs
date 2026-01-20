//! STT Latency Benchmarks
//!
//! Benchmarks for Speech-to-Text transcription latency.
//!
//! # Running
//!
//! ```bash
//! cargo bench -p lexicon-core --bench stt_latency --features stt
//! ```
//!
//! # Prerequisites
//!
//! Requires a Whisper model to be downloaded. The benchmark will skip
//! if no model is available.
//!
//! # Target Metrics
//!
//! - STT base model: p95 < 800ms for 5 seconds of audio
//! - STT tiny model: p95 < 400ms for 5 seconds of audio

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::path::PathBuf;

/// Generate synthetic speech-like audio for benchmarking
fn generate_test_audio(duration_secs: f32) -> Vec<f32> {
    let sample_rate = 16000.0;
    let num_samples = (duration_secs * sample_rate) as usize;

    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            // Mix of frequencies to simulate speech
            let f0 = (2.0 * std::f32::consts::PI * 150.0 * t).sin();
            let f1 = (2.0 * std::f32::consts::PI * 500.0 * t).sin() * 0.5;
            let f2 = (2.0 * std::f32::consts::PI * 1500.0 * t).sin() * 0.25;
            // Amplitude modulation
            let envelope = ((2.0 * std::f32::consts::PI * 4.0 * t).sin() * 0.3 + 0.7).max(0.0);
            (f0 + f1 + f2) * envelope * 0.3
        })
        .collect()
}

/// Find available Whisper model for benchmarking
fn find_model_path() -> Option<PathBuf> {
    // Check common locations for Whisper models
    let possible_paths = [
        // XDG data directory
        dirs::data_dir().map(|p| p.join("lexicon/models")),
        // Home directory
        dirs::home_dir().map(|p| p.join(".lexicon/models")),
        // Current directory
        Some(PathBuf::from("models")),
        // Test fixtures
        Some(PathBuf::from("tests/fixtures/models")),
    ];

    let model_names = [
        "whisper-tiny.bin",
        "whisper-tiny.en.bin",
        "whisper-base.bin",
        "whisper-base.en.bin",
        "ggml-tiny.bin",
        "ggml-tiny.en.bin",
        "ggml-base.bin",
        "ggml-base.en.bin",
    ];

    for base_path in possible_paths.into_iter().flatten() {
        for model_name in &model_names {
            let model_path = base_path.join(model_name);
            if model_path.exists() {
                return Some(model_path);
            }
        }
    }

    None
}

#[cfg(feature = "stt")]
mod stt_benchmarks {
    use super::*;
    use criterion::black_box;
    use lexicon_core::stt::{TranscriptionConfig, Transcriber};

    pub fn bench_stt_transcription(c: &mut Criterion) {
        let model_path = match find_model_path() {
            Some(p) => p,
            None => {
                eprintln!("No Whisper model found. Skipping STT benchmarks.");
                eprintln!("Download a model to run these benchmarks.");
                return;
            }
        };

        eprintln!("Using model: {}", model_path.display());

        // Create transcriber
        let mut transcriber = match Transcriber::new() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to create transcriber: {e}");
                return;
            }
        };

        if let Err(e) = transcriber.load_model(&model_path) {
            eprintln!("Failed to load model: {e}");
            return;
        }

        let mut group = c.benchmark_group("STT_Transcription");

        // Configure for realistic benchmark times
        group.sample_size(10); // Fewer samples since STT is slow
        group.measurement_time(std::time::Duration::from_secs(30));

        // Benchmark different audio durations
        for duration_secs in [1.0, 3.0, 5.0, 10.0] {
            let audio = generate_test_audio(duration_secs);
            let config = TranscriptionConfig::new();

            group.bench_with_input(
                BenchmarkId::new("duration", format!("{}s", duration_secs)),
                &audio,
                |b, audio| {
                    b.iter(|| {
                        let result = transcriber.transcribe(black_box(audio), &config);
                        black_box(result)
                    });
                },
            );
        }

        group.finish();
    }

    pub fn bench_stt_with_language(c: &mut Criterion) {
        let model_path = match find_model_path() {
            Some(p) => p,
            None => return,
        };

        let mut transcriber = match Transcriber::new() {
            Ok(t) => t,
            Err(_) => return,
        };

        if transcriber.load_model(&model_path).is_err() {
            return;
        }

        let mut group = c.benchmark_group("STT_Language");
        group.sample_size(10);

        let audio = generate_test_audio(5.0);

        // Benchmark with different language hints
        for lang in ["en", "auto"] {
            let config = TranscriptionConfig::new().with_language(lang);

            group.bench_with_input(
                BenchmarkId::new("lang", lang),
                &audio,
                |b, audio| {
                    b.iter(|| {
                        let result = transcriber.transcribe(black_box(audio), &config);
                        black_box(result)
                    });
                },
            );
        }

        group.finish();
    }
}

#[cfg(feature = "stt")]
criterion_group!(
    stt_benches,
    stt_benchmarks::bench_stt_transcription,
    stt_benchmarks::bench_stt_with_language,
);

#[cfg(feature = "stt")]
criterion_main!(stt_benches);

// Stub for when stt feature is disabled
#[cfg(not(feature = "stt"))]
fn main() {
    eprintln!("STT benchmarks require the 'stt' feature. Run with:");
    eprintln!("  cargo bench -p lexicon-core --bench stt_latency --features stt");
    eprintln!("");
    eprintln!("Note: A Whisper model must be downloaded for benchmarks to run.");
}

// Helper module for finding directories (minimal implementation)
mod dirs {
    use std::path::PathBuf;

    pub fn data_dir() -> Option<PathBuf> {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|h| h.join(".local/share")))
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}
