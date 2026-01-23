//! Tests for the storage layer.

use super::*;
use tempfile::TempDir;

/// Helper to create a test database.
fn test_db() -> Database {
    Database::open_in_memory().expect("Failed to create in-memory database")
}

// =============================================================================
// Database Configuration Tests
// =============================================================================

#[test]
fn test_db_creates_schema() {
    let db = test_db();

    // Verify schema version is set
    let version = db.schema_version().unwrap();
    assert!(version > 0);
}

#[test]
fn test_db_opens_file() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create database
    let db = Database::open(&db_path).unwrap();
    assert!(db_path.exists());

    // Reopen database
    drop(db);
    let db = Database::open(&db_path).unwrap();
    assert!(db.schema_version().unwrap() > 0);
}

#[test]
fn test_wal_mode_enabled() {
    let db = test_db();
    // In-memory databases may not support WAL mode, but the call should succeed
    let result = db.is_wal_mode();
    assert!(result.is_ok());
}

// =============================================================================
// Session CRUD Tests
// =============================================================================

#[test]
fn test_session_crud() {
    let db = test_db();

    // Create
    let metadata = SessionMetadata::new()
        .with_name("Test Session")
        .with_language("en");
    let session_id = db.create_session(&metadata).unwrap();

    // Read
    let session = db.get_session(session_id).unwrap();
    assert_eq!(session.id, session_id);
    assert_eq!(session.name, Some("Test Session".to_string()));
    assert_eq!(session.language, Some("en".to_string()));

    // Update
    let updated_metadata = SessionMetadata::new()
        .with_name("Updated Session")
        .with_language("es");
    db.update_session(session_id, &updated_metadata).unwrap();

    let session = db.get_session(session_id).unwrap();
    assert_eq!(session.name, Some("Updated Session".to_string()));
    assert_eq!(session.language, Some("es".to_string()));

    // Delete
    db.delete_session(session_id).unwrap();
    let result = db.get_session(session_id);
    assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
}

#[test]
fn test_session_list() {
    let db = test_db();

    // Create multiple sessions
    let id1 = db
        .create_session(&SessionMetadata::new().with_name("Session 1"))
        .unwrap();
    let id2 = db
        .create_session(&SessionMetadata::new().with_name("Session 2"))
        .unwrap();
    let id3 = db
        .create_session(&SessionMetadata::new().with_name("Session 3"))
        .unwrap();

    // List sessions (should be ordered by updated_at DESC)
    let sessions = db.list_sessions().unwrap();
    assert_eq!(sessions.len(), 3);

    // Most recent should be first
    assert_eq!(sessions[0].id, id3);
    assert_eq!(sessions[1].id, id2);
    assert_eq!(sessions[2].id, id1);
}

#[test]
fn test_session_not_found() {
    let db = test_db();

    let result = db.get_session(SessionId(999));
    assert!(matches!(result, Err(DatabaseError::NotFound { entity, id }) 
        if entity == "session" && id == "999"));
}

// =============================================================================
// Segment CRUD Tests
// =============================================================================

#[test]
fn test_segment_crud() {
    let db = test_db();

    // Create session first
    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    // Create segment
    let segment = NewSegment::new(0, 5000, "Hello, world!")
        .with_confidence(0.95)
        .with_language("en")
        .with_speaker(1);
    let segment_id = db.insert_segment(session_id, &segment).unwrap();

    // Read segment
    let stored = db.get_segment(segment_id).unwrap();
    assert_eq!(stored.id, segment_id);
    assert_eq!(stored.session_id, session_id);
    assert_eq!(stored.start_ms, 0);
    assert_eq!(stored.end_ms, 5000);
    assert_eq!(stored.text, "Hello, world!");
    assert_eq!(stored.confidence, Some(0.95));
    assert_eq!(stored.language, Some("en".to_string()));
    assert_eq!(stored.speaker_id, Some(1));

    // Update segment
    let updated = NewSegment::new(0, 6000, "Hello, updated world!")
        .with_confidence(0.98);
    db.update_segment(segment_id, &updated).unwrap();

    let stored = db.get_segment(segment_id).unwrap();
    assert_eq!(stored.end_ms, 6000);
    assert_eq!(stored.text, "Hello, updated world!");
    assert_eq!(stored.confidence, Some(0.98));

    // Delete segment
    db.delete_segment(segment_id).unwrap();
    let result = db.get_segment(segment_id);
    assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
}

#[test]
fn test_segment_cascade_delete() {
    let db = test_db();

    // Create session with segments
    let session_id = db.create_session(&SessionMetadata::new()).unwrap();
    let seg1 = db
        .insert_segment(session_id, &NewSegment::new(0, 1000, "First"))
        .unwrap();
    let seg2 = db
        .insert_segment(session_id, &NewSegment::new(1000, 2000, "Second"))
        .unwrap();

    // Verify segments exist
    assert!(db.get_segment(seg1).is_ok());
    assert!(db.get_segment(seg2).is_ok());

    // Delete session
    db.delete_session(session_id).unwrap();

    // Segments should be deleted too (cascade)
    assert!(matches!(
        db.get_segment(seg1),
        Err(DatabaseError::NotFound { .. })
    ));
    assert!(matches!(
        db.get_segment(seg2),
        Err(DatabaseError::NotFound { .. })
    ));
}

#[test]
fn test_get_session_segments() {
    let db = test_db();

    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    // Insert segments out of order
    db.insert_segment(session_id, &NewSegment::new(5000, 10000, "Third"))
        .unwrap();
    db.insert_segment(session_id, &NewSegment::new(0, 2500, "First"))
        .unwrap();
    db.insert_segment(session_id, &NewSegment::new(2500, 5000, "Second"))
        .unwrap();

    // Get segments (should be ordered by start_ms)
    let segments = db.get_session_segments(session_id).unwrap();
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].text, "First");
    assert_eq!(segments[1].text, "Second");
    assert_eq!(segments[2].text, "Third");
}

#[test]
fn test_segment_requires_valid_session() {
    let db = test_db();

    let segment = NewSegment::new(0, 1000, "Test");
    let result = db.insert_segment(SessionId(999), &segment);

    assert!(matches!(result, Err(DatabaseError::NotFound { entity, .. }) if entity == "session"));
}

// =============================================================================
// FTS5 Search Tests
// =============================================================================

#[test]
fn test_fts5_indexes_text() {
    let db = test_db();

    let session_id = db
        .create_session(&SessionMetadata::new().with_name("Search Test"))
        .unwrap();

    db.insert_segment(
        session_id,
        &NewSegment::new(0, 1000, "The quick brown fox jumps over the lazy dog"),
    )
    .unwrap();

    // Search should find the segment
    let results = db.search("quick").unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].segment.text.contains("quick"));
}

#[test]
fn test_fts5_ranked_results() {
    let db = test_db();

    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    // Insert segments with varying relevance
    // Using longer document that mentions "quick" once
    db.insert_segment(
        session_id,
        &NewSegment::new(0, 1000, "The brown fox is quick and jumps over the lazy dog"),
    )
    .unwrap();
    // Using shorter document that mentions "quick" multiple times
    db.insert_segment(
        session_id,
        &NewSegment::new(1000, 2000, "quick quick quick"),
    )
    .unwrap();
    db.insert_segment(
        session_id,
        &NewSegment::new(2000, 3000, "The lazy dog sleeps"),
    )
    .unwrap();

    // Search should return results ordered by relevance (BM25)
    let results = db.search("quick").unwrap();
    assert_eq!(results.len(), 2);

    // Both results should contain "quick" - BM25 should rank them by relevance
    assert!(results.iter().all(|r| r.segment.text.to_lowercase().contains("quick")));

    // The segment with more "quick" occurrences and shorter length should rank higher
    // BM25 favors higher term frequency and shorter documents
    assert!(results[0].segment.text == "quick quick quick");
}

#[test]
fn test_fts5_unicode_search() {
    let db = test_db();

    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    db.insert_segment(
        session_id,
        &NewSegment::new(0, 1000, "Bonjour, comment allez-vous?"),
    )
    .unwrap();
    db.insert_segment(session_id, &NewSegment::new(1000, 2000, "Hola, como estas?"))
        .unwrap();

    let results = db.search("Bonjour").unwrap();
    assert_eq!(results.len(), 1);

    let results = db.search("como").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_fts5_search_includes_session_name() {
    let db = test_db();

    let session_id = db
        .create_session(&SessionMetadata::new().with_name("Meeting Notes"))
        .unwrap();

    db.insert_segment(session_id, &NewSegment::new(0, 1000, "Discuss the agenda"))
        .unwrap();

    let results = db.search("agenda").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session_name, Some("Meeting Notes".to_string()));
}

// =============================================================================
// SRT Export Tests
// =============================================================================

#[test]
fn test_export_srt_format() {
    let db = test_db();

    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    db.insert_segment(session_id, &NewSegment::new(0, 2500, "First subtitle"))
        .unwrap();
    db.insert_segment(session_id, &NewSegment::new(3000, 5500, "Second subtitle"))
        .unwrap();

    let srt = db.export_srt(session_id).unwrap();

    // Check format
    let expected = "1\n00:00:00,000 --> 00:00:02,500\nFirst subtitle\n\n2\n00:00:03,000 --> 00:00:05,500\nSecond subtitle\n\n";
    assert_eq!(srt, expected);
}

#[test]
fn test_export_srt_timestamps() {
    let db = test_db();

    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    // Test timestamp at 1 hour, 23 minutes, 45 seconds, 678 milliseconds
    let ms = (1 * 3600 + 23 * 60 + 45) * 1000 + 678;
    db.insert_segment(session_id, &NewSegment::new(ms, ms + 1000, "Test"))
        .unwrap();

    let srt = db.export_srt(session_id).unwrap();

    assert!(srt.contains("01:23:45,678"));
}

#[test]
fn test_export_srt_empty_session() {
    let db = test_db();

    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    let result = db.export_srt(session_id);
    assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
}

// =============================================================================
// Model Registry Tests
// =============================================================================

#[test]
fn test_model_upsert_and_get() {
    let db = test_db();

    let model = ModelInfo {
        id: "whisper-tiny-q5_1".to_string(),
        name: "Whisper Tiny (Q5_1)".to_string(),
        size_bytes: 40_000_000,
        hash_sha256: "abc123def456".to_string(),
        downloaded_at: Some(chrono::Utc::now().timestamp_millis()),
        verified: true,
    };

    // Insert
    db.upsert_model(&model).unwrap();

    // Get
    let stored = db.get_model("whisper-tiny-q5_1").unwrap().unwrap();
    assert_eq!(stored.id, model.id);
    assert_eq!(stored.name, model.name);
    assert_eq!(stored.verified, true);

    // Upsert (update)
    let updated = ModelInfo {
        verified: false,
        ..model
    };
    db.upsert_model(&updated).unwrap();

    let stored = db.get_model("whisper-tiny-q5_1").unwrap().unwrap();
    assert_eq!(stored.verified, false);
}

#[test]
fn test_model_list() {
    let db = test_db();

    let model1 = ModelInfo {
        id: "model-1".to_string(),
        name: "Model 1".to_string(),
        size_bytes: 100,
        hash_sha256: "hash1".to_string(),
        downloaded_at: None,
        verified: false,
    };
    let model2 = ModelInfo {
        id: "model-2".to_string(),
        name: "Model 2".to_string(),
        size_bytes: 200,
        hash_sha256: "hash2".to_string(),
        downloaded_at: None,
        verified: false,
    };

    db.upsert_model(&model1).unwrap();
    db.upsert_model(&model2).unwrap();

    let models = db.list_models().unwrap();
    assert_eq!(models.len(), 2);
}

#[test]
fn test_model_delete() {
    let db = test_db();

    let model = ModelInfo {
        id: "to-delete".to_string(),
        name: "To Delete".to_string(),
        size_bytes: 100,
        hash_sha256: "hash".to_string(),
        downloaded_at: None,
        verified: false,
    };

    db.upsert_model(&model).unwrap();
    assert!(db.get_model("to-delete").unwrap().is_some());

    let deleted = db.delete_model("to-delete").unwrap();
    assert!(deleted);
    assert!(db.get_model("to-delete").unwrap().is_none());

    // Delete non-existent returns false
    let deleted = db.delete_model("non-existent").unwrap();
    assert!(!deleted);
}

// =============================================================================
// Settings Tests
// =============================================================================

#[test]
fn test_settings_crud() {
    let db = test_db();

    // Set
    db.set_setting("theme", "dark").unwrap();
    db.set_setting("language", "en").unwrap();

    // Get
    assert_eq!(db.get_setting("theme").unwrap(), Some("dark".to_string()));
    assert_eq!(
        db.get_setting("language").unwrap(),
        Some("en".to_string())
    );
    assert_eq!(db.get_setting("non-existent").unwrap(), None);

    // Update (upsert)
    db.set_setting("theme", "light").unwrap();
    assert_eq!(db.get_setting("theme").unwrap(), Some("light".to_string()));

    // Delete
    let deleted = db.delete_setting("theme").unwrap();
    assert!(deleted);
    assert_eq!(db.get_setting("theme").unwrap(), None);

    // Delete non-existent
    let deleted = db.delete_setting("non-existent").unwrap();
    assert!(!deleted);
}

// =============================================================================
// Audio File Tests
// =============================================================================

#[test]
fn test_audio_file_store_and_get() {
    let db = test_db();

    let session_id = db.create_session(&SessionMetadata::new()).unwrap();

    let audio = AudioFileInfo {
        id: None,
        session_id: Some(session_id),
        filename: "recording.wav".to_string(),
        size_bytes: 1_000_000,
        mime_type: "audio/wav".to_string(),
        duration_ms: Some(60_000),
        sample_rate: Some(44100),
        channels: Some(2),
        external_path: Some("/data/audio/recording.wav".to_string()),
    };

    let id = db.store_audio_file(&audio).unwrap();

    let stored = db.get_audio_file(id).unwrap().unwrap();
    assert_eq!(stored.filename, "recording.wav");
    assert_eq!(stored.session_id, Some(session_id));
    assert_eq!(stored.duration_ms, Some(60_000));
    assert_eq!(stored.sample_rate, Some(44100));
}

#[test]
fn test_audio_file_blob_threshold() {
    let small = AudioFileInfo {
        id: None,
        session_id: None,
        filename: "small.wav".to_string(),
        size_bytes: 50_000, // 50KB - under threshold
        mime_type: "audio/wav".to_string(),
        duration_ms: None,
        sample_rate: None,
        channels: None,
        external_path: None,
    };

    let large = AudioFileInfo {
        id: None,
        session_id: None,
        filename: "large.wav".to_string(),
        size_bytes: 200_000, // 200KB - over threshold
        mime_type: "audio/wav".to_string(),
        duration_ms: None,
        sample_rate: None,
        channels: None,
        external_path: None,
    };

    assert!(!small.should_store_externally());
    assert!(large.should_store_externally());
}

// =============================================================================
// Concurrent Access Tests
// =============================================================================

#[test]
fn test_concurrent_reads() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("concurrent.db");

    // Create database and add data
    let db = Database::open(&db_path).unwrap();
    let session_id = db.create_session(&SessionMetadata::new().with_name("Concurrent")).unwrap();
    db.insert_segment(session_id, &NewSegment::new(0, 1000, "Test data"))
        .unwrap();
    drop(db);

    let db_path = Arc::new(db_path);
    let mut handles = vec![];

    // Spawn multiple readers
    for _ in 0..10 {
        let path = Arc::clone(&db_path);
        handles.push(thread::spawn(move || {
            let db = Database::open(&path).unwrap();
            let sessions = db.list_sessions().unwrap();
            assert_eq!(sessions.len(), 1);
        }));
    }

    // Wait for all readers
    for handle in handles {
        handle.join().unwrap();
    }
}

// =============================================================================
// MVP Validation Tests
// =============================================================================

/// Tests FTS5 search finds text across multiple sessions.
/// This is an MVP Exit Criteria requirement.
#[test]
fn test_fts5_search_across_sessions() {
    let db = test_db();

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
    assert!(results.len() >= 2, "FTS5 should find 'quarterly' in multiple sessions, found: {}", results.len());

    // Verify results come from different sessions
    let unique_sessions: std::collections::HashSet<_> = results.iter().map(|r| r.segment.session_id).collect();
    assert!(unique_sessions.len() >= 2, "Results should come from at least 2 different sessions, found: {}", unique_sessions.len());

    // Search for term that appears in one session
    let ai_results = db.search("AI").unwrap();
    assert!(!ai_results.is_empty(), "FTS5 should find 'AI'");

    // Search for term that appears multiple times
    let budget_results = db.search("budget").unwrap();
    assert!(budget_results.len() >= 2, "FTS5 should find 'budget' in multiple places, found: {}", budget_results.len());
}

/// Validates that SRT export produces valid SRT format.
/// This is an MVP Exit Criteria requirement.
#[test]
fn test_srt_format_validation() {
    let db = test_db();

    // Create session with segments that test edge cases
    let session = db.create_session(&SessionMetadata::new().with_name("SRT Test")).unwrap();

    // Add segments with various timing scenarios
    db.insert_segment(session, &NewSegment::new(0, 1500, "First subtitle line.")).unwrap();
    db.insert_segment(session, &NewSegment::new(1500, 3200, "Second line with special chars: café, naïve")).unwrap();
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
}

/// Test that migrations are idempotent - running them multiple times doesn't break the schema.
#[test]
fn test_db_migrations_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("migrations-idempotent.db");

    // Create and populate database with initial schema
    let session_id = {
        let db = Database::open(&db_path).unwrap();

        // Create some data
        let meta = SessionMetadata::new().with_name("Migration Test");
        let session_id = db.create_session(&meta).unwrap();
        db.insert_segment(
            session_id,
            &NewSegment::new(0, 1000, "Test segment"),
        ).unwrap();

        session_id
    };

    // Reopen database (migrations run again on open)
    {
        let db = Database::open(&db_path).unwrap();

        // Data should still be intact
        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, Some("Migration Test".to_string()));

        // Segments should be preserved
        let segments = db.get_session_segments(session_id).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Test segment");
    }

    // Open a third time to ensure stability
    {
        let db = Database::open(&db_path).unwrap();
        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
    }

    // temp_dir automatically cleaned up when dropped
}

// =============================================================================
// Saved Transcript Tests (TUI)
// =============================================================================

#[test]
fn test_saved_transcript_crud() {
    let db = test_db();

    // Create
    let transcript = NewSavedTranscript::new("Meeting Notes", "This is the transcript content.");
    let id = db.create_saved_transcript(&transcript).unwrap();

    // Read
    let stored = db.get_saved_transcript(id).unwrap();
    assert_eq!(stored.id, id);
    assert_eq!(stored.name, "Meeting Notes");
    assert_eq!(stored.content, "This is the transcript content.");
    assert_eq!(stored.char_count, 31); // "This is the transcript content." = 31 chars
    assert!(stored.session_id.is_none());

    // Update
    let updated = NewSavedTranscript::new("Updated Notes", "New content here.");
    db.update_saved_transcript(id, &updated).unwrap();

    let stored = db.get_saved_transcript(id).unwrap();
    assert_eq!(stored.name, "Updated Notes");
    assert_eq!(stored.content, "New content here.");
    assert_eq!(stored.char_count, 17);

    // Delete
    db.delete_saved_transcript(id).unwrap();
    let result = db.get_saved_transcript(id);
    assert!(matches!(result, Err(DatabaseError::NotFound { .. })));
}

#[test]
fn test_saved_transcript_with_session() {
    let db = test_db();

    // Create a session first
    let session_id = db
        .create_session(&SessionMetadata::new().with_name("Test Session"))
        .unwrap();

    // Create transcript linked to session
    let transcript = NewSavedTranscript::new("Session Transcript", "Content from session.")
        .with_session(session_id);
    let id = db.create_saved_transcript(&transcript).unwrap();

    let stored = db.get_saved_transcript(id).unwrap();
    assert_eq!(stored.session_id, Some(session_id));
}

#[test]
fn test_saved_transcript_list() {
    let db = test_db();

    // Create multiple transcripts
    let _id1 = db
        .create_saved_transcript(&NewSavedTranscript::new("First", "Content 1"))
        .unwrap();
    let _id2 = db
        .create_saved_transcript(&NewSavedTranscript::new("Second", "Content 2"))
        .unwrap();
    let _id3 = db
        .create_saved_transcript(&NewSavedTranscript::new("Third", "Content 3"))
        .unwrap();

    // List transcripts (ordered by created_at DESC)
    let transcripts = db.list_saved_transcripts().unwrap();
    assert_eq!(transcripts.len(), 3);

    // All three transcripts should be present
    let names: Vec<&str> = transcripts.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"First"));
    assert!(names.contains(&"Second"));
    assert!(names.contains(&"Third"));
}

#[test]
fn test_saved_transcript_not_found() {
    let db = test_db();

    let result = db.get_saved_transcript(SavedTranscriptId(999));
    assert!(matches!(
        result,
        Err(DatabaseError::NotFound { entity, id }) if entity == "saved_transcript" && id == "999"
    ));
}

#[test]
fn test_saved_transcript_fts_search() {
    let db = test_db();

    // Create transcripts with searchable content
    db.create_saved_transcript(&NewSavedTranscript::new(
        "Meeting Notes",
        "The quarterly budget report shows significant growth in revenue.",
    ))
    .unwrap();
    db.create_saved_transcript(&NewSavedTranscript::new(
        "Interview Transcript",
        "The candidate discussed their experience with project management.",
    ))
    .unwrap();
    db.create_saved_transcript(&NewSavedTranscript::new(
        "Lecture Notes",
        "Today's lecture covered the fundamentals of machine learning.",
    ))
    .unwrap();

    // Search by content
    let results = db.search_saved_transcripts("budget").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Meeting Notes");

    // Search by name
    let results = db.search_saved_transcripts("Interview").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Interview Transcript");

    // Search term in multiple transcripts
    let results = db.search_saved_transcripts("the").unwrap();
    assert!(results.len() >= 2, "Should find 'the' in multiple transcripts");
}

#[test]
fn test_saved_transcript_char_count() {
    let db = test_db();

    // Test with various character counts including unicode
    let content = "Héllo wörld! 你好";
    let transcript = NewSavedTranscript::new("Unicode Test", content);
    let id = db.create_saved_transcript(&transcript).unwrap();

    let stored = db.get_saved_transcript(id).unwrap();
    // Character count should match the number of unicode characters
    assert_eq!(stored.char_count, content.chars().count() as i64);
}

#[test]
fn test_saved_transcript_preview() {
    // Test the preview method on SavedTranscript
    let short_transcript = SavedTranscript {
        id: SavedTranscriptId(1),
        name: "Short".to_string(),
        content: "Short content.".to_string(),
        char_count: 14,
        session_id: None,
        created_at: "2026-01-23T00:00:00".to_string(),
        updated_at: "2026-01-23T00:00:00".to_string(),
    };

    let long_transcript = SavedTranscript {
        id: SavedTranscriptId(2),
        name: "Long".to_string(),
        content: "This is a very long transcript content that exceeds fifty characters and should be truncated.".to_string(),
        char_count: 92,
        session_id: None,
        created_at: "2026-01-23T00:00:00".to_string(),
        updated_at: "2026-01-23T00:00:00".to_string(),
    };

    // Short content should not be truncated
    assert_eq!(short_transcript.preview(), "Short content.");

    // Long content should be truncated with "..."
    let preview = long_transcript.preview();
    assert!(preview.ends_with("..."));
    assert_eq!(preview.len(), 50); // 47 chars + "..."
}
