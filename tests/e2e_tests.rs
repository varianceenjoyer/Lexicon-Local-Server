//! End-to-end integration tests for Lexicon Local Server.
//!
//! These tests verify the complete system behavior across all components.
//! They require the full workspace to be built and may take longer to run.

#![allow(dead_code)]

use std::path::PathBuf;

/// Get the project root directory.
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Get the path to test fixtures.
fn fixtures_dir() -> PathBuf {
    project_root().join("tests").join("fixtures")
}

/// Get a temporary directory for test data.
fn temp_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("lexicon-e2e-tests").join(name);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Clean up a test directory.
fn cleanup_test_dir(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

// =============================================================================
// Core Library Tests
// =============================================================================

mod core_tests {
    use super::*;

    #[test]
    fn test_config_default_directories() {
        use lexicon_core::LexiconConfig;

        let config = LexiconConfig::builder().build().unwrap();

        // Verify default directories are set
        assert!(!config.data_dir.as_os_str().is_empty());
        assert!(!config.model_dir.as_os_str().is_empty());
    }

    #[test]
    fn test_config_custom_directories() {
        use lexicon_core::LexiconConfig;

        let temp_dir = temp_test_dir("config-test");
        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .unwrap();

        assert_eq!(config.data_dir, temp_dir);
        assert_eq!(config.model_dir, temp_dir.join("models"));

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_context_creates_directories() {
        use lexicon_core::{LexiconConfig, LexiconContext};

        let temp_dir = temp_test_dir("context-test");
        let config = LexiconConfig::builder()
            .data_dir(temp_dir.clone())
            .model_dir(temp_dir.join("models"))
            .build()
            .unwrap();

        let _ctx = LexiconContext::new(config).unwrap();

        assert!(temp_dir.exists());
        assert!(temp_dir.join("models").exists());

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_platform_detection() {
        use lexicon_core::Platform;

        let platform = Platform::detect();

        // Should detect one of the supported platforms
        assert!(matches!(
            platform,
            Platform::MacOsArm64
                | Platform::MacOsX86
                | Platform::Windows
                | Platform::LinuxX86
                | Platform::LinuxArm64
                | Platform::Ios
                | Platform::IosSimulator
                | Platform::AndroidArm64
                | Platform::AndroidArm32
                | Platform::Unknown
        ));
    }
}

// =============================================================================
// Storage Layer Tests
// =============================================================================

#[cfg(feature = "storage")]
mod storage_tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_database_lifecycle() {
        use lexicon_core::Database;
        use lexicon_core::storage::{SessionMetadata, NewSegment};

        let temp_dir = temp_test_dir("db-lifecycle");
        let db_path = temp_dir.join("test.db");

        // Create database
        let db = Database::open(&db_path).unwrap();
        assert!(db_path.exists());

        // Create session
        let metadata = SessionMetadata::new().with_name("E2E Test Session");
        let session_id = db.create_session(&metadata).unwrap();

        // Add segments
        db.insert_segment(
            session_id,
            &NewSegment::new(0, 2000, "Hello, this is a test.")
                .with_confidence(0.95),
        )
        .unwrap();

        db.insert_segment(
            session_id,
            &NewSegment::new(2000, 4000, "Testing the database.")
                .with_confidence(0.92),
        )
        .unwrap();

        // Export SRT
        let srt = db.export_srt(session_id).unwrap();
        assert!(srt.contains("Hello, this is a test"));
        assert!(srt.contains("00:00:00,000 --> 00:00:02,000"));

        // Search
        let results = db.search("test").unwrap();
        assert!(!results.is_empty());

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_database_persistence() {
        use lexicon_core::Database;
        use lexicon_core::storage::SessionMetadata;

        let temp_dir = temp_test_dir("db-persistence");
        let db_path = temp_dir.join("persist.db");

        // Create and populate
        {
            let db = Database::open(&db_path).unwrap();
            db.create_session(&SessionMetadata::new().with_name("Persistent")).unwrap();
        }

        // Reopen and verify
        {
            let db = Database::open(&db_path).unwrap();
            let sessions = db.list_sessions().unwrap();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].name, Some("Persistent".to_string()));
        }

        cleanup_test_dir(&temp_dir);
    }
}

// =============================================================================
// Model Registry Tests
// =============================================================================

mod model_tests {
    use super::*;

    #[test]
    fn test_model_registry_initialization() {
        use lexicon_core::ModelRegistry;

        let temp_dir = temp_test_dir("model-registry");

        let registry = ModelRegistry::new(&temp_dir).unwrap();

        // Should have empty model list initially (no manifest loaded)
        let available = registry.list_models();
        assert!(available.is_empty(), "Should have no models without manifest");

        cleanup_test_dir(&temp_dir);
    }

    #[test]
    fn test_device_profile_detection() {
        use lexicon_core::DeviceProfile;

        let profile = DeviceProfile::detect();

        // Should detect valid values
        assert!(profile.available_memory_mb > 0);
        // platform should be detected
    }

    #[test]
    fn test_model_tier_selection() {
        use lexicon_core::ModelTier;
        use lexicon_core::DeviceProfile;

        let profile = DeviceProfile::detect();
        let tier = profile.recommended_tier();

        // Should recommend a valid tier
        assert!(matches!(
            tier,
            ModelTier::MobileLow | ModelTier::MobileMid | ModelTier::Desktop | ModelTier::Server
        ));
    }
}

// =============================================================================
// Audio Pipeline Tests
// =============================================================================

#[cfg(feature = "audio")]
mod audio_tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_audio_config_defaults() {
        use lexicon_core::AudioConfig;

        let config = AudioConfig::default();

        // AudioConfig has chunk_ms, vad_enabled, vad_mode, device_name fields
        assert!(config.chunk_ms > 0);
        assert!(config.vad_enabled);
    }

    #[test]
    fn test_audio_chunk_creation() {
        use lexicon_core::AudioChunk;

        let samples = vec![0.0f32; 16000]; // 1 second of silence
        // AudioChunk::new(samples, has_speech, timestamp_ms)
        let chunk = AudioChunk::new(samples.clone(), false, 0);

        assert_eq!(chunk.samples.len(), 16000);
        assert!(!chunk.has_speech); // No speech detected in silence
    }

    #[test]
    fn test_vad_config() {
        use lexicon_core::VoiceActivityDetector;

        // VAD should be creatable with a mode (0-3)
        let vad = VoiceActivityDetector::new(1);
        assert!(vad.is_ok());
    }

    #[test]
    fn test_resampler_creation() {
        use lexicon_core::Resampler;

        // Create resampler for 44.1kHz -> 16kHz
        let resampler = Resampler::new(44100, 1, 4410);
        assert!(resampler.is_ok());

        // Create resampler for 48kHz -> 16kHz
        let resampler = Resampler::new(48000, 1, 4800);
        assert!(resampler.is_ok());
    }
}

// =============================================================================
// TTS Engine Tests
// =============================================================================

#[cfg(feature = "tts")]
mod tts_tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_tts_config_defaults() {
        use lexicon_core::TtsConfig;

        let config = TtsConfig::new();

        assert_eq!(config.rate, 1.0);
        assert_eq!(config.pitch, 0.0);
        assert_eq!(config.volume, 1.0);
        assert!(config.fallback_enabled);
    }

    #[test]
    fn test_tts_config_builder() {
        use lexicon_core::{TtsConfig, TtsEngine};

        let config = TtsConfig::new()
            .with_engine(TtsEngine::Platform)
            .with_rate(1.5)
            .with_language("es-ES");

        assert_eq!(config.preferred_engine, TtsEngine::Platform);
        assert!((config.rate - 1.5).abs() < f32::EPSILON);
        assert_eq!(config.language, Some("es-ES".to_string()));
    }

    #[test]
    fn test_audio_buffer_operations() {
        use lexicon_core::AudioBuffer;

        let samples = vec![0.0f32; 24000]; // 1 second at 24kHz
        let buffer = AudioBuffer::new(samples, 24000, 1);

        assert_eq!(buffer.duration_ms(), 1000);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 24000);
    }

    #[test]
    fn test_audio_buffer_to_i16() {
        use lexicon_core::AudioBuffer;

        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let buffer = AudioBuffer::new(samples, 24000, 1);
        let i16_samples = buffer.to_i16();

        assert_eq!(i16_samples[0], 0);
        assert!(i16_samples[1] > 16000); // ~0.5 * 32767
        assert!(i16_samples[2] < -16000);
    }
}

// =============================================================================
// STT Engine Tests
// =============================================================================

#[cfg(feature = "stt")]
mod stt_tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_transcription_config_defaults() {
        use lexicon_core::TranscriptionConfig;

        let config = TranscriptionConfig::new();

        assert_eq!(config.language, None); // Auto-detect
        assert!(!config.translate);
        assert!(!config.enable_diarization);
    }

    #[test]
    fn test_transcription_config_builder() {
        use lexicon_core::TranscriptionConfig;

        let config = TranscriptionConfig::new()
            .with_language("en")
            .with_translation()
            .with_diarization(Some(4));

        assert_eq!(config.language, Some("en".to_string()));
        assert!(config.translate);
        assert!(config.enable_diarization);
        assert_eq!(config.max_speakers, Some(4));
    }

    #[test]
    fn test_segment_creation() {
        use lexicon_core::Segment;

        let segment = Segment::new(0, 2000, "Hello world")
            .with_confidence(0.95)
            .with_language("en");

        assert_eq!(segment.start_ms, 0);
        assert_eq!(segment.end_ms, 2000);
        assert_eq!(segment.text, "Hello world");
        assert_eq!(segment.confidence, Some(0.95));
        assert_eq!(segment.duration_ms(), 2000);
    }

    #[test]
    fn test_transcription_result() {
        use lexicon_core::{TranscriptionResult, Segment};

        let segments = vec![
            Segment::new(0, 1000, "First"),
            Segment::new(1000, 2000, "Second"),
        ];

        let result = TranscriptionResult::new(segments, 2000, 500)
            .with_detected_language("en");

        assert_eq!(result.full_text(), "First Second");
        assert_eq!(result.real_time_factor(), 0.25);
        assert_eq!(result.detected_language, Some("en".to_string()));
    }
}

// =============================================================================
// Cross-Component Integration Tests
// =============================================================================

mod integration_tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    #[cfg(all(feature = "storage", feature = "stt"))]
    fn test_session_with_segments_workflow() {
        use lexicon_core::Database;
        use lexicon_core::storage::{SessionMetadata, NewSegment};
        use lexicon_core::{Segment, TranscriptionResult};

        let temp_dir = temp_test_dir("session-workflow");
        let db = Database::open(&temp_dir.join("test.db")).unwrap();

        // Simulate: Create session from transcription result
        let transcription = TranscriptionResult::new(
            vec![
                Segment::new(0, 2500, "Hello, this is a test transcription."),
                Segment::new(2500, 5000, "It has multiple segments."),
            ],
            5000,
            1000,
        );

        let session_id = db.create_session(&SessionMetadata::new().with_name("Transcription")).unwrap();

        for seg in &transcription.segments {
            db.insert_segment(
                session_id,
                &NewSegment::new(seg.start_ms, seg.end_ms, &seg.text)
                    .with_confidence(seg.confidence.unwrap_or(1.0) as f64),
            )
            .unwrap();
        }

        // Verify
        let segments = db.get_session_segments(session_id).unwrap();
        assert_eq!(segments.len(), 2);

        let srt = db.export_srt(session_id).unwrap();
        assert!(srt.contains("Hello, this is a test"));
        assert!(srt.contains("00:00:02,500 --> 00:00:05,000"));

        cleanup_test_dir(&temp_dir);
    }
}

// =============================================================================
// MVP Validation Tests
// =============================================================================

#[cfg(feature = "storage")]
mod mvp_validation_tests {
    #[allow(unused_imports)]
    use super::*;

    /// Tests FTS5 search finds text across multiple sessions.
    /// This is an MVP Exit Criteria requirement.
    #[test]
    fn test_fts5_search_across_sessions() {
        use lexicon_core::Database;
        use lexicon_core::storage::{SessionMetadata, NewSegment};

        let temp_dir = temp_test_dir("fts5-cross-session");
        let db = Database::open(&temp_dir.join("test.db")).unwrap();

        // Create multiple sessions with different content
        let session1 = db.create_session(&SessionMetadata::new().with_name("Meeting Notes")).unwrap();
        db.insert_segment(session1, &NewSegment::new(0, 2000, "The quarterly budget report shows growth")).unwrap();
        db.insert_segment(session1, &NewSegment::new(2000, 4000, "Revenue increased by fifteen percent")).unwrap();

        let session2 = db.create_session(&SessionMetadata::new().with_name("Podcast Episode")).unwrap();
        db.insert_segment(session2, &NewSegment::new(0, 3000, "Today we discuss the impact of AI on business")).unwrap();
        db.insert_segment(session2, &NewSegment::new(3000, 6000, "Technology is changing how we work")).unwrap();

        let session3 = db.create_session(&SessionMetadata::new().with_name("Interview")).unwrap();
        db.insert_segment(session3, &NewSegment::new(0, 2500, "Tell me about your experience with budget management")).unwrap();
        db.insert_segment(session3, &NewSegment::new(2500, 5000, "I have worked with quarterly reports for years")).unwrap();

        // Search for term that appears in multiple sessions
        let results = db.search("quarterly").unwrap();
        assert!(results.len() >= 2, "FTS5 should find 'quarterly' in multiple sessions");

        // Verify results come from different sessions
        let unique_sessions: std::collections::HashSet<_> = results.iter().map(|r| r.segment.session_id).collect();
        assert!(unique_sessions.len() >= 2, "Results should come from at least 2 different sessions");

        // Search for term that appears in one session
        let ai_results = db.search("AI").unwrap();
        assert!(!ai_results.is_empty(), "FTS5 should find 'AI'");

        // Search for phrase
        let budget_results = db.search("budget").unwrap();
        assert!(budget_results.len() >= 2, "FTS5 should find 'budget' in multiple places");

        cleanup_test_dir(&temp_dir);
    }

    /// Validates that SRT export produces valid SRT format.
    /// This is an MVP Exit Criteria requirement.
    #[test]
    fn test_srt_format_validation() {
        use lexicon_core::Database;
        use lexicon_core::storage::{SessionMetadata, NewSegment};

        let temp_dir = temp_test_dir("srt-validation");
        let db = Database::open(&temp_dir.join("test.db")).unwrap();

        // Create session with segments that test edge cases
        let session = db.create_session(&SessionMetadata::new().with_name("SRT Test")).unwrap();

        // Add segments with various timing scenarios
        db.insert_segment(session, &NewSegment::new(0, 1500, "First subtitle line.")).unwrap();
        db.insert_segment(session, &NewSegment::new(1500, 3200, "Second line with special chars: café, naïve, résumé")).unwrap();
        db.insert_segment(session, &NewSegment::new(3200, 5678, "Third line with precise timing.")).unwrap();
        db.insert_segment(session, &NewSegment::new(10000, 62500, "Line crossing the one-minute mark.")).unwrap();
        db.insert_segment(session, &NewSegment::new(3661000, 3662500, "Line at 1:01:01.")).unwrap();

        let srt = db.export_srt(session).unwrap();

        // Validate SRT format:
        // 1. Sequence numbers start at 1 and increment
        assert!(srt.contains("1\n"), "SRT should start with sequence number 1");
        assert!(srt.contains("\n2\n"), "SRT should have sequence number 2");
        assert!(srt.contains("\n3\n"), "SRT should have sequence number 3");

        // 2. Timestamp format: HH:MM:SS,mmm --> HH:MM:SS,mmm
        assert!(srt.contains("00:00:00,000 --> 00:00:01,500"), "First timing should be correct");
        assert!(srt.contains("00:00:01,500 --> 00:00:03,200"), "Second timing should be correct");
        assert!(srt.contains("00:00:03,200 --> 00:00:05,678"), "Third timing with milliseconds");
        assert!(srt.contains("00:00:10,000 --> 00:01:02,500"), "Minute boundary timing");
        assert!(srt.contains("01:01:01,000 --> 01:01:02,500"), "Hour timing");

        // 3. Text content preserved
        assert!(srt.contains("First subtitle line."), "First text preserved");
        assert!(srt.contains("café"), "Special characters preserved");
        assert!(srt.contains("Line at 1:01:01."), "Long timing text preserved");

        // 4. Blank lines between entries (SRT format requirement)
        let entries: Vec<&str> = srt.trim().split("\n\n").collect();
        assert_eq!(entries.len(), 5, "Should have 5 SRT entries separated by blank lines");

        // 5. Each entry should have exactly 3 parts: number, timing, text
        for (i, entry) in entries.iter().enumerate() {
            let lines: Vec<&str> = entry.lines().collect();
            assert!(lines.len() >= 3, "Entry {} should have at least 3 lines", i + 1);

            // First line should be a number
            assert!(lines[0].parse::<u32>().is_ok(), "Entry {} should start with sequence number", i + 1);

            // Second line should contain -->
            assert!(lines[1].contains(" --> "), "Entry {} should have timing with ' --> '", i + 1);
        }

        cleanup_test_dir(&temp_dir);
    }

    /// Tests database handles concurrent reads while maintaining data integrity.
    /// This validates WAL mode is working correctly.
    #[test]
    fn test_database_concurrent_reads() {
        use lexicon_core::Database;
        use lexicon_core::storage::{SessionMetadata, NewSegment};
        use std::sync::Arc;
        use std::thread;

        let temp_dir = temp_test_dir("concurrent-reads");
        let db_path = temp_dir.join("test.db");

        // Set up data
        let db = Database::open(&db_path).unwrap();
        let session = db.create_session(&SessionMetadata::new().with_name("Concurrent Test")).unwrap();

        for i in 0..20 {
            db.insert_segment(
                session,
                &NewSegment::new(i * 1000, (i + 1) * 1000, format!("Segment number {}", i)),
            ).unwrap();
        }
        drop(db); // Close the initial connection

        // Spawn multiple reader threads
        let db_path = Arc::new(db_path);
        let mut handles = vec![];

        for thread_id in 0..5 {
            let path = Arc::clone(&db_path);
            let handle = thread::spawn(move || {
                let db = Database::open(&*path).unwrap();

                // Each thread performs multiple reads
                for _ in 0..10 {
                    let sessions = db.list_sessions().unwrap();
                    assert!(!sessions.is_empty(), "Thread {} should see sessions", thread_id);

                    let segments = db.get_session_segments(session).unwrap();
                    assert_eq!(segments.len(), 20, "Thread {} should see all segments", thread_id);

                    let results = db.search("Segment").unwrap();
                    assert!(!results.is_empty(), "Thread {} search should find results", thread_id);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread should complete without panic");
        }

        cleanup_test_dir(&temp_dir);
    }

    /// Tests diarization assigns speaker IDs to segments.
    /// This is a Beta Exit Criteria but validates the implementation is working.
    #[test]
    #[cfg(feature = "stt")]
    fn test_diarization_speaker_assignment() {
        use lexicon_core::stt::{assign_speakers, DiarizationConfig, Segment};

        // Create test audio with two distinct "speakers" (different energy levels)
        let mut audio = Vec::with_capacity(64000); // 4 seconds at 16kHz

        // Speaker 1 (loud): 0-1s
        for i in 0..16000 {
            audio.push((i as f32 * 0.1).sin() * 0.8);
        }
        // Speaker 2 (quiet): 1-2s
        for i in 0..16000 {
            audio.push((i as f32 * 0.15).sin() * 0.2);
        }
        // Speaker 1 (loud): 2-3s
        for i in 0..16000 {
            audio.push((i as f32 * 0.1).sin() * 0.8);
        }
        // Speaker 2 (quiet): 3-4s
        for i in 0..16000 {
            audio.push((i as f32 * 0.15).sin() * 0.2);
        }

        let segments = vec![
            Segment::new(0, 1000, "First speaker here"),
            Segment::new(1000, 2000, "Second speaker now"),
            Segment::new(2000, 3000, "Back to first speaker"),
            Segment::new(3000, 4000, "Second speaker again"),
        ];

        let config = DiarizationConfig::new().with_max_speakers(2);
        let result = assign_speakers(segments, &audio, &config);

        // All segments should have speaker IDs
        for seg in &result {
            assert!(seg.speaker_id.is_some(), "All segments should have speaker_id");
        }

        // Same "speaker" segments should have matching IDs
        assert_eq!(result[0].speaker_id, result[2].speaker_id, "First and third should be same speaker");
        assert_eq!(result[1].speaker_id, result[3].speaker_id, "Second and fourth should be same speaker");

        // Different "speakers" should have different IDs
        assert_ne!(result[0].speaker_id, result[1].speaker_id, "Different speakers should have different IDs");
    }

    /// Tests TTS streaming API returns chunks.
    #[test]
    #[cfg(feature = "tts")]
    fn test_tts_streaming_returns_chunks() {
        use lexicon_core::tts::streaming_channel;

        let (mut sender, receiver) = streaming_channel(24000);

        // Simulate streaming synthesis
        sender.send(vec![0.0; 2400], false).unwrap(); // 100ms chunk
        sender.send(vec![0.5; 2400], false).unwrap();
        sender.send(vec![1.0; 1200], true).unwrap(); // Final chunk

        drop(sender);

        // Collect all chunks
        let chunks: Vec<_> = receiver.collect();

        assert_eq!(chunks.len(), 3, "Should receive 3 chunks");
        assert!(!chunks[0].is_final);
        assert!(!chunks[1].is_final);
        assert!(chunks[2].is_final);

        // Verify sequence numbers
        assert_eq!(chunks[0].sequence, 0);
        assert_eq!(chunks[1].sequence, 1);
        assert_eq!(chunks[2].sequence, 2);
    }
}

// =============================================================================
// Performance Benchmark Placeholders
// =============================================================================

#[cfg(feature = "bench")]
mod benchmarks {
    #[allow(unused_imports)]
    use super::*;

    // Placeholder for performance benchmarks
    // These would use criterion or similar benchmarking framework

    #[test]
    fn benchmark_placeholder() {
        // Actual benchmarks would measure:
        // - Transcription latency (target: <800ms for base model)
        // - TTS synthesis time (target: <200ms first byte)
        // - Database query performance
        // - Memory usage
    }
}
