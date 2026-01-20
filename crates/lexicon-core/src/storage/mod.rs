//! Storage Layer - SQLite database with FTS5 search
//!
//! This module provides persistent storage for transcription sessions,
//! segments, model metadata, and application settings.
//!
//! # Features
//!
//! - WAL mode for concurrent read access
//! - FTS5 full-text search with BM25 ranking
//! - Millisecond-precision timestamps
//! - Blob storage threshold (files >100KB stored externally)
//! - Migration framework for schema evolution
//!
//! # Example
//!
//! ```no_run
//! use lexicon_core::storage::{Database, SessionMetadata, NewSegment};
//! use std::path::Path;
//!
//! let db = Database::open(Path::new("lexicon.db")).unwrap();
//!
//! // Create a session
//! let session_id = db.create_session(&SessionMetadata::new()
//!     .with_name("Meeting Notes")
//!     .with_language("en")).unwrap();
//!
//! // Add a segment
//! let segment = NewSegment::new(0, 5000, "Hello, world!");
//! db.insert_segment(session_id, &segment).unwrap();
//!
//! // Search across all segments
//! let results = db.search("hello").unwrap();
//! ```

mod migrations;
mod models;

pub use models::*;

use crate::error::DatabaseError;
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;
use tracing::{debug, info};

/// Result type for database operations
pub type DbResult<T> = std::result::Result<T, DatabaseError>;

/// Main database handle for Lexicon storage.
///
/// Provides CRUD operations for sessions, segments, models, settings,
/// and audio files. Supports FTS5 full-text search with BM25 ranking.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the specified path.
    ///
    /// Enables WAL mode for concurrent reads and creates schema if needed.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the database cannot be opened or migrated.
    pub fn open(path: &Path) -> DbResult<Self> {
        info!("Opening database at {:?}", path);

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        )
        .map_err(|e| DatabaseError::OpenFailed {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut db = Self { conn };
        db.configure()?;
        db.migrate()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the database cannot be initialized.
    pub fn open_in_memory() -> DbResult<Self> {
        debug!("Opening in-memory database");

        let conn = Connection::open_in_memory().map_err(|e| DatabaseError::OpenFailed {
            path: std::path::PathBuf::from(":memory:"),
            source: e,
        })?;

        let mut db = Self { conn };
        db.configure()?;
        db.migrate()?;

        Ok(db)
    }

    /// Configure database pragmas.
    fn configure(&self) -> DbResult<()> {
        // Enable WAL mode for concurrent reads
        self.conn
            .execute_batch(
                "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            ",
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to configure database: {e}"),
            })?;

        debug!("Database configured with WAL mode");
        Ok(())
    }

    /// Run database migrations.
    fn migrate(&mut self) -> DbResult<()> {
        migrations::run_migrations(&self.conn)
    }

    /// Get the current schema version.
    pub fn schema_version(&self) -> DbResult<i32> {
        let version: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to get schema version: {e}"),
            })?;
        Ok(version)
    }

    // -------------------------------------------------------------------------
    // Session Operations
    // -------------------------------------------------------------------------

    /// Create a new transcription session.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the session cannot be created.
    pub fn create_session(&self, metadata: &SessionMetadata) -> DbResult<SessionId> {
        let now = chrono::Utc::now().timestamp_millis();

        self.conn
            .execute(
                "INSERT INTO sessions (name, language, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)",
                params![metadata.name, metadata.language, now,],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to create session: {e}"),
            })?;

        let id = self.conn.last_insert_rowid();
        debug!("Created session with id {}", id);
        Ok(SessionId(id))
    }

    /// Get a session by ID.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the session doesn't exist.
    pub fn get_session(&self, id: SessionId) -> DbResult<Session> {
        self.conn
            .query_row(
                "SELECT id, name, language, created_at, updated_at FROM sessions WHERE id = ?1",
                params![id.0],
                |row| {
                    Ok(Session {
                        id: SessionId(row.get(0)?),
                        name: row.get(1)?,
                        language: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DatabaseError::NotFound {
                    entity: "session".to_string(),
                    id: id.0.to_string(),
                },
                other => DatabaseError::QueryFailed {
                    message: format!("Failed to get session: {other}"),
                },
            })
    }

    /// List all sessions, ordered by most recent first.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn list_sessions(&self) -> DbResult<Vec<Session>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, language, created_at, updated_at
             FROM sessions ORDER BY updated_at DESC, id DESC",
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to prepare sessions query: {e}"),
            })?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(Session {
                    id: SessionId(row.get(0)?),
                    name: row.get(1)?,
                    language: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to query sessions: {e}"),
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to collect sessions: {e}"),
            })?;

        Ok(sessions)
    }

    /// Update a session's metadata.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the session doesn't exist.
    pub fn update_session(&self, id: SessionId, metadata: &SessionMetadata) -> DbResult<()> {
        let now = chrono::Utc::now().timestamp_millis();

        let rows = self
            .conn
            .execute(
                "UPDATE sessions SET name = ?1, language = ?2, updated_at = ?3 WHERE id = ?4",
                params![metadata.name, metadata.language, now, id.0,],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to update session: {e}"),
            })?;

        if rows == 0 {
            return Err(DatabaseError::NotFound {
                entity: "session".to_string(),
                id: id.0.to_string(),
            });
        }
        Ok(())
    }

    /// Delete a session and all its segments (cascade).
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the session doesn't exist.
    pub fn delete_session(&self, id: SessionId) -> DbResult<()> {
        let rows = self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id.0])
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to delete session: {e}"),
            })?;

        if rows == 0 {
            return Err(DatabaseError::NotFound {
                entity: "session".to_string(),
                id: id.0.to_string(),
            });
        }

        debug!("Deleted session {} and its segments", id.0);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Segment Operations
    // -------------------------------------------------------------------------

    /// Insert a transcription segment.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the session doesn't exist.
    pub fn insert_segment(
        &self,
        session_id: SessionId,
        segment: &NewSegment,
    ) -> DbResult<SegmentId> {
        // Verify session exists
        self.get_session(session_id)?;

        self.conn
            .execute(
                "INSERT INTO segments (session_id, start_ms, end_ms, text, confidence, language, speaker_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id.0,
                    segment.start_ms,
                    segment.end_ms,
                    segment.text,
                    segment.confidence,
                    segment.language,
                    segment.speaker_id,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to insert segment: {e}"),
            })?;

        let id = self.conn.last_insert_rowid();

        // Update session's updated_at timestamp
        let now = chrono::Utc::now().timestamp_millis();
        self.conn
            .execute(
                "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
                params![now, session_id.0],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to update session timestamp: {e}"),
            })?;

        debug!("Inserted segment {} for session {}", id, session_id.0);
        Ok(SegmentId(id))
    }

    /// Get a segment by ID.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the segment doesn't exist.
    pub fn get_segment(&self, id: SegmentId) -> DbResult<Segment> {
        self.conn
            .query_row(
                "SELECT id, session_id, start_ms, end_ms, text, confidence, language, speaker_id
                 FROM segments WHERE id = ?1",
                params![id.0],
                |row| {
                    Ok(Segment {
                        id: SegmentId(row.get(0)?),
                        session_id: SessionId(row.get(1)?),
                        start_ms: row.get(2)?,
                        end_ms: row.get(3)?,
                        text: row.get(4)?,
                        confidence: row.get(5)?,
                        language: row.get(6)?,
                        speaker_id: row.get(7)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DatabaseError::NotFound {
                    entity: "segment".to_string(),
                    id: id.0.to_string(),
                },
                other => DatabaseError::QueryFailed {
                    message: format!("Failed to get segment: {other}"),
                },
            })
    }

    /// Get all segments for a session, ordered by start time.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn get_session_segments(&self, session_id: SessionId) -> DbResult<Vec<Segment>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, session_id, start_ms, end_ms, text, confidence, language, speaker_id
             FROM segments WHERE session_id = ?1 ORDER BY start_ms ASC",
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to prepare segments query: {e}"),
            })?;

        let segments = stmt
            .query_map(params![session_id.0], |row| {
                Ok(Segment {
                    id: SegmentId(row.get(0)?),
                    session_id: SessionId(row.get(1)?),
                    start_ms: row.get(2)?,
                    end_ms: row.get(3)?,
                    text: row.get(4)?,
                    confidence: row.get(5)?,
                    language: row.get(6)?,
                    speaker_id: row.get(7)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to query segments: {e}"),
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to collect segments: {e}"),
            })?;

        Ok(segments)
    }

    /// Update a segment.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the segment doesn't exist.
    pub fn update_segment(&self, id: SegmentId, segment: &NewSegment) -> DbResult<()> {
        let rows = self
            .conn
            .execute(
                "UPDATE segments SET start_ms = ?1, end_ms = ?2, text = ?3, 
             confidence = ?4, language = ?5, speaker_id = ?6 WHERE id = ?7",
                params![
                    segment.start_ms,
                    segment.end_ms,
                    segment.text,
                    segment.confidence,
                    segment.language,
                    segment.speaker_id,
                    id.0,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to update segment: {e}"),
            })?;

        if rows == 0 {
            return Err(DatabaseError::NotFound {
                entity: "segment".to_string(),
                id: id.0.to_string(),
            });
        }
        Ok(())
    }

    /// Delete a segment.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the segment doesn't exist.
    pub fn delete_segment(&self, id: SegmentId) -> DbResult<()> {
        let rows = self
            .conn
            .execute("DELETE FROM segments WHERE id = ?1", params![id.0])
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to delete segment: {e}"),
            })?;

        if rows == 0 {
            return Err(DatabaseError::NotFound {
                entity: "segment".to_string(),
                id: id.0.to_string(),
            });
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Search Operations (FTS5)
    // -------------------------------------------------------------------------

    /// Full-text search across segments.
    ///
    /// Uses SQLite FTS5 with BM25 ranking for relevance-ordered results.
    /// Supports FTS5 query syntax including phrase matching and boolean operators.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the search query is invalid or fails.
    pub fn search(&self, query: &str) -> DbResult<Vec<SearchResult>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.id, s.session_id, s.start_ms, s.end_ms, s.text, s.confidence,
                    s.language, s.speaker_id, sess.name as session_name,
                    bm25(segments_fts) as rank
             FROM segments_fts fts
             JOIN segments s ON fts.rowid = s.id
             JOIN sessions sess ON s.session_id = sess.id
             WHERE segments_fts MATCH ?1
             ORDER BY rank",
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to prepare search query: {e}"),
            })?;

        let results = stmt
            .query_map(params![query], |row| {
                Ok(SearchResult {
                    segment: Segment {
                        id: SegmentId(row.get(0)?),
                        session_id: SessionId(row.get(1)?),
                        start_ms: row.get(2)?,
                        end_ms: row.get(3)?,
                        text: row.get(4)?,
                        confidence: row.get(5)?,
                        language: row.get(6)?,
                        speaker_id: row.get(7)?,
                    },
                    session_name: row.get(8)?,
                    rank: row.get(9)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to execute search: {e}"),
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to collect search results: {e}"),
            })?;

        debug!("Search '{}' returned {} results", query, results.len());
        Ok(results)
    }

    // -------------------------------------------------------------------------
    // Export Operations
    // -------------------------------------------------------------------------

    /// Export a session's segments as SRT subtitle format.
    ///
    /// The SRT format includes:
    /// - Sequential index numbers (1-based)
    /// - Timestamps in HH:MM:SS,mmm --> HH:MM:SS,mmm format
    /// - Text content
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::NotFound` if the session has no segments.
    pub fn export_srt(&self, session_id: SessionId) -> DbResult<String> {
        let segments = self.get_session_segments(session_id)?;

        if segments.is_empty() {
            return Err(DatabaseError::NotFound {
                entity: "segments for session".to_string(),
                id: session_id.0.to_string(),
            });
        }

        let mut srt = String::new();

        for (index, segment) in segments.iter().enumerate() {
            // SRT index (1-based)
            srt.push_str(&format!("{}\n", index + 1));

            // Timestamps: HH:MM:SS,mmm --> HH:MM:SS,mmm
            let start = format_srt_timestamp(segment.start_ms);
            let end = format_srt_timestamp(segment.end_ms);
            srt.push_str(&format!("{start} --> {end}\n"));

            // Text content
            srt.push_str(&segment.text);
            srt.push_str("\n\n");
        }

        Ok(srt)
    }

    // -------------------------------------------------------------------------
    // Model Registry Operations
    // -------------------------------------------------------------------------

    /// Register or update a model in the registry.
    ///
    /// Uses UPSERT to insert new models or update existing ones.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the operation fails.
    pub fn upsert_model(&self, model: &ModelInfo) -> DbResult<()> {
        self.conn
            .execute(
                "INSERT INTO models (id, name, size_bytes, hash_sha256, downloaded_at, verified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                size_bytes = excluded.size_bytes,
                hash_sha256 = excluded.hash_sha256,
                downloaded_at = excluded.downloaded_at,
                verified = excluded.verified",
                params![
                    model.id,
                    model.name,
                    model.size_bytes,
                    model.hash_sha256,
                    model.downloaded_at,
                    model.verified,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to upsert model: {e}"),
            })?;

        debug!("Upserted model {}", model.id);
        Ok(())
    }

    /// Get a model by ID.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn get_model(&self, id: &str) -> DbResult<Option<ModelInfo>> {
        let result = self.conn.query_row(
            "SELECT id, name, size_bytes, hash_sha256, downloaded_at, verified
             FROM models WHERE id = ?1",
            params![id],
            |row| {
                Ok(ModelInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    size_bytes: row.get(2)?,
                    hash_sha256: row.get(3)?,
                    downloaded_at: row.get(4)?,
                    verified: row.get(5)?,
                })
            },
        );

        match result {
            Ok(model) => Ok(Some(model)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed {
                message: format!("Failed to get model: {e}"),
            }),
        }
    }

    /// List all models.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn list_models(&self) -> DbResult<Vec<ModelInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, size_bytes, hash_sha256, downloaded_at, verified FROM models")
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to prepare models query: {e}"),
            })?;

        let models = stmt
            .query_map([], |row| {
                Ok(ModelInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    size_bytes: row.get(2)?,
                    hash_sha256: row.get(3)?,
                    downloaded_at: row.get(4)?,
                    verified: row.get(5)?,
                })
            })
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to query models: {e}"),
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to collect models: {e}"),
            })?;

        Ok(models)
    }

    /// Delete a model from the registry.
    ///
    /// Returns `true` if the model was deleted, `false` if it didn't exist.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn delete_model(&self, id: &str) -> DbResult<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM models WHERE id = ?1", params![id])
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to delete model: {e}"),
            })?;
        Ok(rows > 0)
    }

    // -------------------------------------------------------------------------
    // Settings Operations
    // -------------------------------------------------------------------------

    /// Get a setting value.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn get_setting(&self, key: &str) -> DbResult<Option<String>> {
        let result = self.conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed {
                message: format!("Failed to get setting: {e}"),
            }),
        }
    }

    /// Set a setting value.
    ///
    /// Uses UPSERT to insert or update the setting.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the operation fails.
    pub fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        self.conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to set setting: {e}"),
            })?;
        Ok(())
    }

    /// Delete a setting.
    ///
    /// Returns `true` if the setting was deleted, `false` if it didn't exist.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn delete_setting(&self, key: &str) -> DbResult<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM settings WHERE key = ?1", params![key])
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to delete setting: {e}"),
            })?;
        Ok(rows > 0)
    }

    // -------------------------------------------------------------------------
    // Audio File Operations
    // -------------------------------------------------------------------------

    /// Store audio file metadata (blob stored externally if > 100KB).
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the operation fails.
    pub fn store_audio_file(&self, audio: &AudioFileInfo) -> DbResult<i64> {
        self.conn
            .execute(
                "INSERT INTO audio_files (session_id, filename, size_bytes, mime_type, 
             duration_ms, sample_rate, channels, external_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    audio.session_id.map(|s| s.0),
                    audio.filename,
                    audio.size_bytes,
                    audio.mime_type,
                    audio.duration_ms,
                    audio.sample_rate,
                    audio.channels,
                    audio.external_path,
                ],
            )
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to store audio file: {e}"),
            })?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get audio file by ID.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn get_audio_file(&self, id: i64) -> DbResult<Option<AudioFileInfo>> {
        let result = self.conn.query_row(
            "SELECT id, session_id, filename, size_bytes, mime_type, duration_ms, 
             sample_rate, channels, external_path
             FROM audio_files WHERE id = ?1",
            params![id],
            |row| {
                Ok(AudioFileInfo {
                    id: Some(row.get(0)?),
                    session_id: row.get::<_, Option<i64>>(1)?.map(SessionId),
                    filename: row.get(2)?,
                    size_bytes: row.get(3)?,
                    mime_type: row.get(4)?,
                    duration_ms: row.get(5)?,
                    sample_rate: row.get(6)?,
                    channels: row.get(7)?,
                    external_path: row.get(8)?,
                })
            },
        );

        match result {
            Ok(audio) => Ok(Some(audio)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryFailed {
                message: format!("Failed to get audio file: {e}"),
            }),
        }
    }

    /// Check if WAL mode is enabled.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the query fails.
    pub fn is_wal_mode(&self) -> DbResult<bool> {
        let mode: String = self
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(|e| DatabaseError::QueryFailed {
                message: format!("Failed to check journal mode: {e}"),
            })?;
        Ok(mode.to_lowercase() == "wal")
    }
}

/// Format milliseconds as SRT timestamp (HH:MM:SS,mmm).
fn format_srt_timestamp(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let millis = ms % 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

#[cfg(test)]
mod tests;
