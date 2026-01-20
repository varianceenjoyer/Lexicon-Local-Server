//! Audio Pipeline Benchmarks
//!
//! Benchmarks for audio processing components:
//! - Voice Activity Detection (VAD)
//! - Audio resampling
//! - Audio chunk processing
//!
//! # Running
//!
//! ```bash
//! cargo bench -p lexicon-core --bench audio_pipeline
//! ```
//!
//! # Target Metrics
//!
//! - VAD: p95 < 100ms for 1 second of audio
//! - Resampling: < 50ms for 1 second 44.1kHz → 16kHz

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Generate synthetic audio samples (sine wave)
fn generate_audio_samples(num_samples: usize, frequency: f32, sample_rate: f32) -> Vec<f32> {
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5
        })
        .collect()
}

/// Generate silence
fn generate_silence(num_samples: usize) -> Vec<f32> {
    vec![0.0; num_samples]
}

/// Generate speech-like audio (mix of frequencies with amplitude modulation)
fn generate_speech_like(num_samples: usize, sample_rate: f32) -> Vec<f32> {
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            // Fundamental frequency around 150Hz (male voice)
            let f0 = (2.0 * std::f32::consts::PI * 150.0 * t).sin();
            // First formant ~500Hz
            let f1 = (2.0 * std::f32::consts::PI * 500.0 * t).sin() * 0.5;
            // Second formant ~1500Hz
            let f2 = (2.0 * std::f32::consts::PI * 1500.0 * t).sin() * 0.25;
            // Amplitude modulation (syllable rhythm)
            let envelope = ((2.0 * std::f32::consts::PI * 4.0 * t).sin() * 0.3 + 0.7).max(0.0);
            (f0 + f1 + f2) * envelope * 0.3
        })
        .collect()
}

#[cfg(feature = "audio")]
mod vad_benchmarks {
    use super::*;
    use lexicon_core::VoiceActivityDetector;

    pub fn bench_vad_silence(c: &mut Criterion) {
        let mut group = c.benchmark_group("VAD");

        // Test different audio durations
        for duration_ms in [10, 100, 1000] {
            let num_samples = 16000 * duration_ms / 1000; // 16kHz
            let samples = generate_silence(num_samples);

            group.throughput(Throughput::Elements(num_samples as u64));
            group.bench_with_input(
                BenchmarkId::new("silence", format!("{}ms", duration_ms)),
                &samples,
                |b, samples| {
                    let mut vad = VoiceActivityDetector::new(2).unwrap();
                    b.iter(|| {
                        black_box(vad.speech_ratio(black_box(samples)))
                    });
                },
            );
        }

        group.finish();
    }

    pub fn bench_vad_speech(c: &mut Criterion) {
        let mut group = c.benchmark_group("VAD");

        for duration_ms in [10, 100, 1000] {
            let num_samples = 16000 * duration_ms / 1000;
            let samples = generate_speech_like(num_samples, 16000.0);

            group.throughput(Throughput::Elements(num_samples as u64));
            group.bench_with_input(
                BenchmarkId::new("speech", format!("{}ms", duration_ms)),
                &samples,
                |b, samples| {
                    let mut vad = VoiceActivityDetector::new(2).unwrap();
                    b.iter(|| {
                        black_box(vad.speech_ratio(black_box(samples)))
                    });
                },
            );
        }

        group.finish();
    }

    pub fn bench_vad_modes(c: &mut Criterion) {
        let mut group = c.benchmark_group("VAD_Modes");

        let samples = generate_speech_like(16000, 16000.0); // 1 second

        for mode in 0..=3 {
            group.bench_with_input(
                BenchmarkId::new("mode", mode),
                &samples,
                |b, samples| {
                    let mut vad = VoiceActivityDetector::new(mode).unwrap();
                    b.iter(|| {
                        black_box(vad.speech_ratio(black_box(samples)))
                    });
                },
            );
        }

        group.finish();
    }
}

#[cfg(feature = "audio")]
mod resampler_benchmarks {
    use super::*;
    use lexicon_core::Resampler;

    pub fn bench_resample_44100_to_16000(c: &mut Criterion) {
        let mut group = c.benchmark_group("Resampler");

        // Test different audio durations
        for duration_ms in [100, 500, 1000, 5000] {
            let num_samples = 44100 * duration_ms / 1000;
            let samples = generate_audio_samples(num_samples, 440.0, 44100.0);

            group.throughput(Throughput::Elements(num_samples as u64));
            group.bench_with_input(
                BenchmarkId::new("44.1kHz_to_16kHz", format!("{}ms", duration_ms)),
                &samples,
                |b, samples| {
                    let mut resampler = Resampler::new(44100, 1, 4410).unwrap();
                    b.iter(|| {
                        black_box(resampler.process(black_box(samples)).unwrap())
                    });
                },
            );
        }

        group.finish();
    }

    pub fn bench_resample_48000_to_16000(c: &mut Criterion) {
        let mut group = c.benchmark_group("Resampler");

        for duration_ms in [100, 500, 1000] {
            let num_samples = 48000 * duration_ms / 1000;
            let samples = generate_audio_samples(num_samples, 440.0, 48000.0);

            group.throughput(Throughput::Elements(num_samples as u64));
            group.bench_with_input(
                BenchmarkId::new("48kHz_to_16kHz", format!("{}ms", duration_ms)),
                &samples,
                |b, samples| {
                    let mut resampler = Resampler::new(48000, 1, 4800).unwrap();
                    b.iter(|| {
                        black_box(resampler.process(black_box(samples)).unwrap())
                    });
                },
            );
        }

        group.finish();
    }
}

#[cfg(feature = "audio")]
mod audio_chunk_benchmarks {
    use super::*;
    use lexicon_core::AudioChunk;

    pub fn bench_audio_chunk_creation(c: &mut Criterion) {
        let mut group = c.benchmark_group("AudioChunk");

        for num_samples in [480, 1600, 16000, 80000] {
            let samples = generate_audio_samples(num_samples, 440.0, 16000.0);

            group.throughput(Throughput::Elements(num_samples as u64));
            group.bench_with_input(
                BenchmarkId::new("create", num_samples),
                &samples,
                |b, samples| {
                    b.iter(|| {
                        black_box(AudioChunk::new(
                            black_box(samples.clone()),
                            false,
                            0,
                        ))
                    });
                },
            );
        }

        group.finish();
    }
}

// Criterion configuration
#[cfg(feature = "audio")]
criterion_group!(
    vad_benches,
    vad_benchmarks::bench_vad_silence,
    vad_benchmarks::bench_vad_speech,
    vad_benchmarks::bench_vad_modes,
);

#[cfg(feature = "audio")]
criterion_group!(
    resampler_benches,
    resampler_benchmarks::bench_resample_44100_to_16000,
    resampler_benchmarks::bench_resample_48000_to_16000,
);

#[cfg(feature = "audio")]
criterion_group!(
    audio_chunk_benches,
    audio_chunk_benchmarks::bench_audio_chunk_creation,
);

#[cfg(feature = "audio")]
criterion_main!(vad_benches, resampler_benches, audio_chunk_benches);

// Stub for when audio feature is disabled
#[cfg(not(feature = "audio"))]
fn main() {
    eprintln!("Audio benchmarks require the 'audio' feature. Run with:");
    eprintln!("  cargo bench -p lexicon-core --bench audio_pipeline --features audio");
}
