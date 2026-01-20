//! Memory Stress Test for Lexicon Local Server
//!
//! This test verifies that repeated create/destroy cycles don't leak memory.
//! It's designed to be run under valgrind or similar memory analysis tools.
//!
//! # Running with Valgrind (Linux)
//!
//! ```bash
//! cargo build --release --test memory_stress_test
//! valgrind --leak-check=full --show-leak-kinds=all \
//!     ./target/release/deps/memory_stress_test-* --test-threads=1
//! ```
//!
//! # Running with Heaptrack (Linux)
//!
//! ```bash
//! cargo build --release --test memory_stress_test
//! heaptrack ./target/release/deps/memory_stress_test-* --test-threads=1
//! heaptrack_gui heaptrack.memory_stress_test.*.gz
//! ```
//!
//! # Expected Behavior
//!
//! - Memory usage should stabilize after initial allocations
//! - No continuous growth in RSS over cycles
//! - All allocations should be freed on context destruction

use std::path::PathBuf;

/// Number of cycles for the stress test
const STRESS_TEST_CYCLES: usize = 100;

/// Get a temporary directory for test data
fn temp_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("lexicon-memory-test")
        .join(name);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Clean up test directory
fn cleanup_test_dir(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Get current process RSS (Resident Set Size) in bytes
#[cfg(target_os = "linux")]
fn get_rss_bytes() -> Option<usize> {
    use std::fs;

    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: usize = parts[1].parse().ok()?;
                return Some(kb * 1024);
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn get_rss_bytes() -> Option<usize> {
    // On non-Linux platforms, we can't easily get RSS
    // Return None to skip RSS-based assertions
    None
}

/// Format bytes as human-readable string
fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

// =============================================================================
// Context Lifecycle Tests
// =============================================================================

#[test]
fn test_context_lifecycle_no_leak() {
    use lexicon_core::{LexiconConfig, LexiconContext};

    let temp_dir = temp_test_dir("context-lifecycle");

    let initial_rss = get_rss_bytes();

    for cycle in 0..STRESS_TEST_CYCLES {
        let cycle_dir = temp_dir.join(format!("cycle-{}", cycle));
        let _ = std::fs::create_dir_all(&cycle_dir);

        // Create config and context
        let config = LexiconConfig::builder()
            .data_dir(cycle_dir.clone())
            .model_dir(cycle_dir.join("models"))
            .build()
            .expect("Failed to build config");

        let ctx = LexiconContext::new(config).expect("Failed to create context");

        // Use the context briefly
        let _ = ctx.config();

        // Context is dropped here
        drop(ctx);

        // Clean up cycle directory
        let _ = std::fs::remove_dir_all(&cycle_dir);

        // Log progress every 10 cycles
        if (cycle + 1) % 10 == 0 {
            if let Some(rss) = get_rss_bytes() {
                println!("Cycle {}: RSS = {}", cycle + 1, format_bytes(rss));
            }
        }
    }

    // Final RSS check
    if let (Some(initial), Some(final_rss)) = (initial_rss, get_rss_bytes()) {
        let growth = final_rss.saturating_sub(initial);
        println!("Initial RSS: {}", format_bytes(initial));
        println!("Final RSS: {}", format_bytes(final_rss));
        println!("Growth: {}", format_bytes(growth));

        // Allow up to 10MB growth (for runtime allocations, caches, etc.)
        // This is a sanity check, not a strict limit
        let max_allowed_growth = 10 * 1024 * 1024;
        assert!(
            growth < max_allowed_growth,
            "Memory grew by {} which exceeds {} limit",
            format_bytes(growth),
            format_bytes(max_allowed_growth)
        );
    }

    cleanup_test_dir(&temp_dir);
}

// =============================================================================
// Database Lifecycle Tests
// =============================================================================

#[cfg(feature = "storage")]
#[test]
fn test_database_lifecycle_no_leak() {
    use lexicon_core::Database;
    use lexicon_core::storage::{SessionMetadata, NewSegment};

    let temp_dir = temp_test_dir("database-lifecycle");

    let initial_rss = get_rss_bytes();

    for cycle in 0..STRESS_TEST_CYCLES {
        let db_path = temp_dir.join(format!("test-{}.db", cycle));

        // Create database
        let db = Database::open(&db_path).expect("Failed to open database");

        // Create session and add segments
        let session_id = db.create_session(
            &SessionMetadata::new().with_name(&format!("Session {}", cycle))
        ).expect("Failed to create session");

        // Add multiple segments
        for i in 0..10 {
            db.insert_segment(
                session_id,
                &NewSegment::new(
                    i * 1000,
                    (i + 1) * 1000,
                    &format!("Segment {} in cycle {}", i, cycle)
                ).with_confidence(0.95)
            ).expect("Failed to insert segment");
        }

        // Query the data
        let _segments = db.get_session_segments(session_id).expect("Failed to get segments");
        let _srt = db.export_srt(session_id).expect("Failed to export SRT");
        let _results = db.search("Segment").expect("Failed to search");

        // Database is dropped here
        drop(db);

        // Remove database file
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));

        // Log progress
        if (cycle + 1) % 10 == 0 {
            if let Some(rss) = get_rss_bytes() {
                println!("Cycle {}: RSS = {}", cycle + 1, format_bytes(rss));
            }
        }
    }

    // Final RSS check
    if let (Some(initial), Some(final_rss)) = (initial_rss, get_rss_bytes()) {
        let growth = final_rss.saturating_sub(initial);
        println!("Database test - Initial: {}, Final: {}, Growth: {}",
            format_bytes(initial), format_bytes(final_rss), format_bytes(growth));

        let max_allowed_growth = 10 * 1024 * 1024;
        assert!(growth < max_allowed_growth, "Memory leak detected: {} growth", format_bytes(growth));
    }

    cleanup_test_dir(&temp_dir);
}

// =============================================================================
// Audio Pipeline Tests
// =============================================================================

#[cfg(feature = "audio")]
#[test]
fn test_audio_pipeline_no_leak() {
    use lexicon_core::{AudioChunk, VoiceActivityDetector, Resampler};

    let initial_rss = get_rss_bytes();

    for cycle in 0..STRESS_TEST_CYCLES {
        // Create VAD
        let mut vad = VoiceActivityDetector::new(1).expect("Failed to create VAD");

        // Create resampler (44.1kHz -> 16kHz)
        let mut resampler = Resampler::new(44100, 1, 4410).expect("Failed to create resampler");

        // Generate test audio (1 second at 44.1kHz)
        let input_samples: Vec<f32> = (0..44100)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        // Process through resampler
        let resampled = resampler.process(&input_samples).expect("Failed to resample");

        // Process chunks through VAD (160 samples = 10ms at 16kHz)
        for chunk_start in (0..resampled.len()).step_by(160) {
            let chunk_end = (chunk_start + 160).min(resampled.len());
            let chunk_samples = &resampled[chunk_start..chunk_end];

            if chunk_samples.len() == 160 {
                let _has_speech = vad.is_speech_f32(chunk_samples).unwrap_or(false);
            }
        }

        // Create audio chunks
        let _chunk = AudioChunk::new(resampled.clone(), false, cycle as i64 * 1000);

        // Log progress
        if (cycle + 1) % 10 == 0 {
            if let Some(rss) = get_rss_bytes() {
                println!("Cycle {}: RSS = {}", cycle + 1, format_bytes(rss));
            }
        }
    }

    // Final RSS check
    if let (Some(initial), Some(final_rss)) = (initial_rss, get_rss_bytes()) {
        let growth = final_rss.saturating_sub(initial);
        println!("Audio test - Initial: {}, Final: {}, Growth: {}",
            format_bytes(initial), format_bytes(final_rss), format_bytes(growth));

        let max_allowed_growth = 10 * 1024 * 1024;
        assert!(growth < max_allowed_growth, "Memory leak detected: {} growth", format_bytes(growth));
    }
}

// =============================================================================
// TTS Pipeline Tests
// =============================================================================

#[cfg(feature = "tts")]
#[test]
fn test_tts_types_no_leak() {
    use lexicon_core::{AudioBuffer, TtsConfig, TtsEngine};
    use lexicon_core::tts::streaming_channel;

    let initial_rss = get_rss_bytes();

    for cycle in 0..STRESS_TEST_CYCLES {
        // Create TTS config variations
        let _config1 = TtsConfig::new()
            .with_engine(TtsEngine::Platform)
            .with_rate(1.0)
            .with_language("en-US");

        let _config2 = TtsConfig::new()
            .with_engine(TtsEngine::Kokoro)
            .with_rate(1.5)
            .with_pitch(0.5);

        // Create and manipulate audio buffers
        let samples: Vec<f32> = (0..24000)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let buffer = AudioBuffer::new(samples.clone(), 24000, 1);
        let _duration = buffer.duration_ms();
        let _i16_samples = buffer.to_i16();

        // Test streaming channel
        let (mut sender, receiver) = streaming_channel(24000);

        for i in 0..5 {
            let chunk: Vec<f32> = (0..2400).map(|j| ((i * 2400 + j) as f32 * 0.01).sin()).collect();
            let _ = sender.send(chunk, i == 4);
        }

        drop(sender);
        let _chunks: Vec<_> = receiver.collect();

        // Log progress
        if (cycle + 1) % 10 == 0 {
            if let Some(rss) = get_rss_bytes() {
                println!("Cycle {}: RSS = {}", cycle + 1, format_bytes(rss));
            }
        }
    }

    // Final RSS check
    if let (Some(initial), Some(final_rss)) = (initial_rss, get_rss_bytes()) {
        let growth = final_rss.saturating_sub(initial);
        println!("TTS test - Initial: {}, Final: {}, Growth: {}",
            format_bytes(initial), format_bytes(final_rss), format_bytes(growth));

        let max_allowed_growth = 10 * 1024 * 1024;
        assert!(growth < max_allowed_growth, "Memory leak detected: {} growth", format_bytes(growth));
    }
}

// =============================================================================
// Combined Stress Test
// =============================================================================

#[test]
#[cfg(all(feature = "storage", feature = "audio", feature = "tts"))]
fn test_combined_stress_no_leak() {
    use lexicon_core::{LexiconConfig, LexiconContext, Database, AudioBuffer};
    use lexicon_core::storage::{SessionMetadata, NewSegment};

    let temp_dir = temp_test_dir("combined-stress");
    let initial_rss = get_rss_bytes();

    println!("Running combined stress test with {} cycles...", STRESS_TEST_CYCLES);

    for cycle in 0..STRESS_TEST_CYCLES {
        let cycle_dir = temp_dir.join(format!("cycle-{}", cycle));
        let _ = std::fs::create_dir_all(&cycle_dir);

        // 1. Create context
        let config = LexiconConfig::builder()
            .data_dir(cycle_dir.clone())
            .model_dir(cycle_dir.join("models"))
            .build()
            .expect("Failed to build config");

        let _ctx = LexiconContext::new(config).expect("Failed to create context");

        // 2. Use database
        let db_path = cycle_dir.join("test.db");
        let db = Database::open(&db_path).expect("Failed to open database");

        let session_id = db.create_session(
            &SessionMetadata::new().with_name(&format!("Combined {}", cycle))
        ).expect("Failed to create session");

        for i in 0..5 {
            db.insert_segment(
                session_id,
                &NewSegment::new(i * 1000, (i + 1) * 1000, &format!("Seg {}", i))
            ).expect("Failed to insert");
        }

        let _srt = db.export_srt(session_id).expect("Failed to export");
        drop(db);

        // 3. Create audio buffers
        let samples: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.01).sin()).collect();
        let _buffer = AudioBuffer::new(samples, 16000, 1);

        // Clean up
        let _ = std::fs::remove_dir_all(&cycle_dir);

        // Log progress
        if (cycle + 1) % 20 == 0 {
            if let Some(rss) = get_rss_bytes() {
                println!("Cycle {}/{}: RSS = {}", cycle + 1, STRESS_TEST_CYCLES, format_bytes(rss));
            }
        }
    }

    // Final RSS check
    if let (Some(initial), Some(final_rss)) = (initial_rss, get_rss_bytes()) {
        let growth = final_rss.saturating_sub(initial);
        println!("\n=== Combined Stress Test Results ===");
        println!("Cycles: {}", STRESS_TEST_CYCLES);
        println!("Initial RSS: {}", format_bytes(initial));
        println!("Final RSS: {}", format_bytes(final_rss));
        println!("Growth: {}", format_bytes(growth));

        let max_allowed_growth = 20 * 1024 * 1024; // 20MB for combined test
        assert!(growth < max_allowed_growth,
            "Memory leak detected: {} growth exceeds {} limit",
            format_bytes(growth), format_bytes(max_allowed_growth));

        println!("✓ No significant memory leak detected");
    } else {
        println!("Note: RSS monitoring not available on this platform");
    }

    cleanup_test_dir(&temp_dir);
}
