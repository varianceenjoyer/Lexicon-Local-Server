//! Database migrations for the storage layer.
//!
//! Migrations are applied in order based on version number. Each migration
//! is idempotent and can be safely re-run.

use crate::error::DatabaseError;
use rusqlite::Connection;
use tracing::{debug, info};

/// Current schema version.
const CURRENT_VERSION: i32 = 1;

/// Run all pending migrations.
pub fn run_migrations(conn: &Connection) -> Result<(), DatabaseError> {
    let current_version = get_version(conn)?;
    info!(
        "Database schema version: {}, target: {}",
        current_version, CURRENT_VERSION
    );

    if current_version >= CURRENT_VERSION {
        debug!("Database schema is up to date");
        return Ok(());
    }

    // Run migrations in order
    for version in (current_version + 1)..=CURRENT_VERSION {
        info!("Applying migration to version {}", version);
        apply_migration(conn, version)?;
        set_version(conn, version)?;
    }

    info!("Database migrations complete");
    Ok(())
}

/// Get the current schema version.
fn get_version(conn: &Connection) -> Result<i32, DatabaseError> {
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| DatabaseError::MigrationFailed {
            message: format!("Failed to get schema version: {e}"),
        })?;
    Ok(version)
}

/// Set the schema version.
fn set_version(conn: &Connection, version: i32) -> Result<(), DatabaseError> {
    conn.execute(&format!("PRAGMA user_version = {version}"), [])
        .map_err(|e| DatabaseError::MigrationFailed {
            message: format!("Failed to set schema version: {e}"),
        })?;
    Ok(())
}

/// Apply a specific migration version.
fn apply_migration(conn: &Connection, version: i32) -> Result<(), DatabaseError> {
    match version {
        1 => migration_v1(conn),
        _ => Err(DatabaseError::MigrationFailed {
            message: format!("Unknown migration version: {version}"),
        }),
    }
}

/// Migration v1: Initial schema.
///
/// Creates the core tables:
/// - sessions: Transcription sessions
/// - segments: Individual transcription segments with FTS5 index
/// - models: Model registry
/// - settings: Key-value settings
/// - audio_files: Audio file metadata
fn migration_v1(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r"
        -- Sessions table
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            language TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        -- Create index for listing sessions by date
        CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at DESC);

        -- Segments table with millisecond precision timestamps
        CREATE TABLE IF NOT EXISTS segments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            start_ms INTEGER NOT NULL,
            end_ms INTEGER NOT NULL,
            text TEXT NOT NULL,
            confidence REAL,
            language TEXT,
            speaker_id INTEGER
        );

        -- Create indexes for segment queries
        CREATE INDEX IF NOT EXISTS idx_segments_session_id ON segments(session_id);
        CREATE INDEX IF NOT EXISTS idx_segments_start_ms ON segments(session_id, start_ms);

        -- FTS5 virtual table for full-text search
        CREATE VIRTUAL TABLE IF NOT EXISTS segments_fts USING fts5(
            text,
            content='segments',
            content_rowid='id'
        );

        -- Triggers to keep FTS5 index in sync with segments table
        CREATE TRIGGER IF NOT EXISTS segments_ai AFTER INSERT ON segments BEGIN
            INSERT INTO segments_fts(rowid, text) VALUES (new.id, new.text);
        END;

        CREATE TRIGGER IF NOT EXISTS segments_ad AFTER DELETE ON segments BEGIN
            INSERT INTO segments_fts(segments_fts, rowid, text) VALUES('delete', old.id, old.text);
        END;

        CREATE TRIGGER IF NOT EXISTS segments_au AFTER UPDATE ON segments BEGIN
            INSERT INTO segments_fts(segments_fts, rowid, text) VALUES('delete', old.id, old.text);
            INSERT INTO segments_fts(rowid, text) VALUES (new.id, new.text);
        END;

        -- Models registry
        CREATE TABLE IF NOT EXISTS models (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            hash_sha256 TEXT NOT NULL,
            downloaded_at INTEGER,
            verified INTEGER NOT NULL DEFAULT 0
        );

        -- Settings key-value store
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Audio files metadata
        CREATE TABLE IF NOT EXISTS audio_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER REFERENCES sessions(id) ON DELETE SET NULL,
            filename TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            mime_type TEXT NOT NULL,
            duration_ms INTEGER,
            sample_rate INTEGER,
            channels INTEGER,
            external_path TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_audio_files_session_id ON audio_files(session_id);
        ",
    )
    .map_err(|e| DatabaseError::MigrationFailed {
        message: format!("Migration v1 failed: {e}"),
    })?;

    debug!("Migration v1 applied successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // Run migrations twice - should not fail
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Check version is set correctly
        let version = get_version(&conn).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn test_schema_created() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Check tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"segments".to_string()));
        assert!(tables.contains(&"models".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"audio_files".to_string()));
    }

    #[test]
    fn test_fts5_table_created() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Check FTS5 virtual table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='segments_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }
}
