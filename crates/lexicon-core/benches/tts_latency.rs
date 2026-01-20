//! TTS Latency Benchmarks
//!
//! Benchmarks for Text-to-Speech synthesis latency.
//!
//! # Running
//!
//! ```bash
//! cargo bench -p lexicon-core --bench tts_latency --features tts
//! ```
//!
//! # Target Metrics
//!
//! - TTS first chunk latency: < 200ms
//! - TTS total synthesis: < 1s for short sentences

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Sample texts of varying lengths for benchmarking
const SHORT_TEXT: &str = "Hello, world!";
const MEDIUM_TEXT: &str = "The quick brown fox jumps over the lazy dog. This is a test of the text-to-speech system.";
const LONG_TEXT: &str = "In the beginning, there was silence. Then came the spoken word, transforming thoughts into sound waves that traveled through the air. Today, we have machines that can speak for us, converting written text into natural-sounding speech. This technology has revolutionized accessibility and human-computer interaction.";

#[cfg(feature = "tts")]
mod tts_benchmarks {
    use super::*;
    use lexicon_core::tts::{AudioBuffer, TtsConfig, TtsEngine};

    pub fn bench_tts_config_creation(c: &mut Criterion) {
        let mut group = c.benchmark_group("TTS_Config");

        group.bench_function("default", |b| {
            b.iter(|| {
                black_box(TtsConfig::new())
            });
        });

        group.bench_function("with_options", |b| {
            b.iter(|| {
                black_box(
                    TtsConfig::new()
                        .with_engine(TtsEngine::Platform)
                        .with_rate(1.0)
                        .with_pitch(0.0)
                        .with_language("en-US")
                )
            });
        });

        group.finish();
    }

    pub fn bench_audio_buffer_operations(c: &mut Criterion) {
        let mut group = c.benchmark_group("TTS_AudioBuffer");

        // Benchmark buffer creation
        for num_samples in [2400, 24000, 240000] {
            let samples: Vec<f32> = (0..num_samples)
                .map(|i| (i as f32 * 0.01).sin() * 0.5)
                .collect();

            group.throughput(Throughput::Elements(num_samples as u64));

            group.bench_with_input(
                BenchmarkId::new("create", num_samples),
                &samples,
                |b, samples| {
                    b.iter(|| {
                        black_box(AudioBuffer::new(
                            black_box(samples.clone()),
                            24000,
                            1,
                        ))
                    });
                },
            );
        }

        // Benchmark duration calculation
        let samples: Vec<f32> = (0..24000).map(|i| (i as f32 * 0.01).sin()).collect();
        let buffer = AudioBuffer::new(samples, 24000, 1);

        group.bench_function("duration_ms", |b| {
            b.iter(|| {
                black_box(buffer.duration_ms())
            });
        });

        // Benchmark i16 conversion
        group.bench_function("to_i16_24k_samples", |b| {
            b.iter(|| {
                black_box(buffer.to_i16())
            });
        });

        group.finish();
    }

    pub fn bench_streaming_channel(c: &mut Criterion) {
        use lexicon_core::tts::streaming_channel;

        let mut group = c.benchmark_group("TTS_Streaming");

        // Benchmark channel creation
        group.bench_function("channel_create", |b| {
            b.iter(|| {
                black_box(streaming_channel(24000))
            });
        });

        // Benchmark sending chunks
        for chunk_size in [480, 2400, 4800] {
            let chunk: Vec<f32> = (0..chunk_size).map(|i| (i as f32 * 0.01).sin()).collect();

            group.throughput(Throughput::Elements(chunk_size as u64));
            group.bench_with_input(
                BenchmarkId::new("send_chunk", chunk_size),
                &chunk,
                |b, chunk| {
                    b.iter(|| {
                        let (mut sender, _receiver) = streaming_channel(24000);
                        black_box(sender.send(black_box(chunk.clone()), false))
                    });
                },
            );
        }

        // Benchmark receiving
        group.bench_function("receive_10_chunks", |b| {
            b.iter(|| {
                let (mut sender, receiver) = streaming_channel(24000);
                let chunk: Vec<f32> = (0..2400).map(|i| (i as f32 * 0.01).sin()).collect();

                for i in 0..10 {
                    let _ = sender.send(chunk.clone(), i == 9);
                }
                drop(sender);

                let chunks: Vec<_> = receiver.collect();
                black_box(chunks)
            });
        });

        group.finish();
    }

    pub fn bench_platform_tts_synthesis(c: &mut Criterion) {
        use lexicon_core::tts::PlatformSynthesizer;
        use lexicon_core::tts::Synthesizer;

        // Try to create platform synthesizer
        let synth = match PlatformSynthesizer::new() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Platform TTS not available: {e}");
                eprintln!("Skipping platform TTS benchmarks.");
                return;
            }
        };

        let mut group = c.benchmark_group("TTS_Platform");
        group.sample_size(10); // Fewer samples since TTS can be slow
        group.measurement_time(std::time::Duration::from_secs(20));

        let config = TtsConfig::new();

        // Benchmark different text lengths
        for (name, text) in [
            ("short", SHORT_TEXT),
            ("medium", MEDIUM_TEXT),
            ("long", LONG_TEXT),
        ] {
            group.throughput(Throughput::Bytes(text.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("synthesize", name),
                &text,
                |b, &text| {
                    b.iter(|| {
                        let result = synth.synthesize(black_box(text), &config);
                        black_box(result)
                    });
                },
            );
        }

        group.finish();
    }
}

#[cfg(feature = "tts")]
criterion_group!(
    tts_config_benches,
    tts_benchmarks::bench_tts_config_creation,
);

#[cfg(feature = "tts")]
criterion_group!(
    tts_buffer_benches,
    tts_benchmarks::bench_audio_buffer_operations,
);

#[cfg(feature = "tts")]
criterion_group!(
    tts_streaming_benches,
    tts_benchmarks::bench_streaming_channel,
);

#[cfg(feature = "tts")]
criterion_group!(
    tts_platform_benches,
    tts_benchmarks::bench_platform_tts_synthesis,
);

#[cfg(feature = "tts")]
criterion_main!(
    tts_config_benches,
    tts_buffer_benches,
    tts_streaming_benches,
    tts_platform_benches,
);

// Stub for when tts feature is disabled
#[cfg(not(feature = "tts"))]
fn main() {
    eprintln!("TTS benchmarks require the 'tts' feature. Run with:");
    eprintln!("  cargo bench -p lexicon-core --bench tts_latency --features tts");
}
