//! End-to-end integration tests for Lexicon Local Server.
//!
//! These tests verify the complete system behavior across all components.
//! They require the full workspace to be built and may take longer to run.

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

        // Should have empty model list initially
        let available = registry.list_available();
        // May be empty or have defaults depending on implementation

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
        let tier = profile.suggested_tier();

        // Should suggest a valid tier
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
    use super::*;

    #[test]
    fn test_audio_config_defaults() {
        use lexicon_core::AudioConfig;

        let config = AudioConfig::default();

        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
    }

    #[test]
    fn test_audio_chunk_creation() {
        use lexicon_core::AudioChunk;

        let samples = vec![0.0f32; 16000]; // 1 second of silence
        let chunk = AudioChunk::new(samples.clone(), 0);

        assert_eq!(chunk.samples.len(), 16000);
        assert!(!chunk.has_speech); // No speech detected in silence
    }

    #[test]
    fn test_vad_config() {
        use lexicon_core::VoiceActivityDetector;

        // VAD should be creatable with default settings
        let vad = VoiceActivityDetector::new();
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
// Performance Benchmark Placeholders
// =============================================================================

#[cfg(feature = "bench")]
mod benchmarks {
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
