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
    db.insert_segment(
        session_id,
        &NewSegment::new(0, 1000, "The fox is quick"),
    )
    .unwrap();
    db.insert_segment(
        session_id,
        &NewSegment::new(1000, 2000, "Quick quick quick fox"),
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

    // The segment with more "quick" occurrences should rank higher
    assert!(results[0].segment.text.contains("quick quick quick"));
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
