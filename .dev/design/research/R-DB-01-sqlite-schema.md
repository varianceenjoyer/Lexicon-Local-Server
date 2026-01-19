# Research Plan: SQLite Schema Design

**ID:** R-DB-01
**Status:** Ongoing
**Category:** Architecture
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** Schema design documentation

---

## Problem Statement

The project requires persistent storage for:
- Transcription history (text, timestamps, metadata)
- Audio recordings (optional, user-controlled)
- Model management (downloaded models, versions)
- User preferences and settings
- Export capabilities (various formats)

**Engineering Challenge:** Design a SQLite schema that efficiently stores transcription data with full-text search, supports audio storage, and enables flexible export.

---

## Research Questions

### RQ1: What data entities need persistence?

**Entities to model:**
- [ ] Transcription sessions
- [ ] Transcription segments (words/phrases with timestamps)
- [ ] Audio files (optional blob storage)
- [ ] Models (metadata, not the files themselves)
- [ ] User settings
- [ ] Export history

### RQ2: How should transcription data be structured?

**Areas to investigate:**
- [ ] Session-level vs segment-level storage
- [ ] Timestamp precision requirements
- [ ] Speaker diarization support (future)
- [ ] Language/translation storage
- [ ] Confidence scores

### RQ3: How should full-text search be implemented?

**Areas to investigate:**
- [ ] SQLite FTS5 capabilities
- [ ] Tokenization for different languages
- [ ] Search result ranking
- [ ] Highlight/snippet generation

### RQ4: What export formats should be supported?

**Formats to consider:**
- [ ] Plain text
- [ ] SRT (subtitles)
- [ ] VTT (web subtitles)
- [ ] JSON (structured)
- [ ] CSV (tabular)
- [ ] Word document (optional)

---

## Proposed Schema

### Core Tables

```sql
-- Transcription sessions
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    title TEXT,
    duration_ms INTEGER,
    language TEXT,
    model_id INTEGER REFERENCES models(id),
    audio_file_id INTEGER REFERENCES audio_files(id),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived', 'deleted'))
);

-- Transcription segments (words/phrases)
CREATE TABLE segments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    confidence REAL,
    speaker_id INTEGER,  -- For future diarization
    is_translated INTEGER NOT NULL DEFAULT 0,
    original_segment_id INTEGER REFERENCES segments(id)  -- For translations
);

-- Full-text search index
CREATE VIRTUAL TABLE segments_fts USING fts5(
    text,
    content='segments',
    content_rowid='id',
    tokenize='unicode61'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER segments_ai AFTER INSERT ON segments BEGIN
    INSERT INTO segments_fts(rowid, text) VALUES (new.id, new.text);
END;

CREATE TRIGGER segments_ad AFTER DELETE ON segments BEGIN
    INSERT INTO segments_fts(segments_fts, rowid, text) VALUES('delete', old.id, old.text);
END;

CREATE TRIGGER segments_au AFTER UPDATE ON segments BEGIN
    INSERT INTO segments_fts(segments_fts, rowid, text) VALUES('delete', old.id, old.text);
    INSERT INTO segments_fts(rowid, text) VALUES (new.id, new.text);
END;
```

### Audio Storage (Optional)

```sql
-- Audio files (blob storage)
CREATE TABLE audio_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    duration_ms INTEGER,
    sample_rate INTEGER,
    channels INTEGER,
    data BLOB  -- Or store path if external
);

-- Setting to control audio storage
-- audio_storage: 'none' | 'compressed' | 'original'
```

### Model Management

```sql
-- Downloaded models metadata
CREATE TABLE models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL CHECK (type IN ('stt', 'tts', 'translation')),
    version TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    file_hash TEXT NOT NULL,  -- SHA-256
    downloaded_at TEXT NOT NULL DEFAULT (datetime('now')),
    is_active INTEGER NOT NULL DEFAULT 1
);
```

### Settings

```sql
-- Key-value settings store
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Default settings
INSERT INTO settings (key, value) VALUES
    ('audio_storage', 'none'),
    ('default_language', 'en'),
    ('default_model', 'base.en'),
    ('theme', 'system');
```

### Indexes

```sql
-- Performance indexes
CREATE INDEX idx_segments_session ON segments(session_id);
CREATE INDEX idx_segments_time ON segments(session_id, start_ms);
CREATE INDEX idx_sessions_created ON sessions(created_at DESC);
CREATE INDEX idx_sessions_status ON sessions(status) WHERE status != 'deleted';
```

---

## Agent Tasks

### Task 1: Research SQLite best practices
- [ ] SQLite performance characteristics
- [ ] Blob storage vs external file trade-offs
- [ ] FTS5 tokenization options
- [ ] WAL mode benefits
- [ ] Vacuum and maintenance

### Task 2: Define entity relationships
- [ ] Finalize session → segment relationship
- [ ] Define audio storage strategy
- [ ] Define model metadata structure
- [ ] Plan for future features (diarization, translation)

### Task 3: Design search capabilities
- [ ] Implement FTS5 schema
- [ ] Test search across languages
- [ ] Define result ranking
- [ ] Implement snippet/highlight extraction

### Task 4: Design export functions
- [ ] Plain text export query
- [ ] SRT export query
- [ ] VTT export query
- [ ] JSON export structure
- [ ] CSV export structure

### Task 5: Implement and test schema
- [ ] Create migration files
- [ ] Write rusqlite integration
- [ ] Test CRUD operations
- [ ] Test search performance
- [ ] Test export functions

---

## Export Query Examples

### Plain Text Export
```sql
SELECT text
FROM segments
WHERE session_id = ?
ORDER BY start_ms;
```

### SRT Export
```sql
SELECT
    ROW_NUMBER() OVER (ORDER BY start_ms) as seq,
    start_ms,
    end_ms,
    text
FROM segments
WHERE session_id = ?
ORDER BY start_ms;
```

Format output as:
```
1
00:00:01,234 --> 00:00:03,456
Hello, world.

2
00:00:03,789 --> 00:00:05,012
This is a test.
```

### Full-Text Search
```sql
SELECT
    s.id,
    s.title,
    snippet(segments_fts, 0, '<mark>', '</mark>', '...', 32) as snippet,
    seg.start_ms
FROM segments_fts
JOIN segments seg ON segments_fts.rowid = seg.id
JOIN sessions s ON seg.session_id = s.id
WHERE segments_fts MATCH ?
ORDER BY rank;
```

---

## Success Criteria

- [ ] Schema supports all required entities
- [ ] FTS5 search working across segments
- [ ] Audio blob storage optional and configurable
- [ ] Export queries for all formats implemented
- [ ] Performance acceptable for 10,000+ segments
- [ ] Migration strategy documented

---

## Rust Integration

### rusqlite Usage
```rust
use rusqlite::{Connection, params};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(Self { conn })
    }

    pub fn create_session(&self, title: Option<&str>) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO sessions (title) VALUES (?)",
            params![title],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn add_segment(
        &self,
        session_id: i64,
        start_ms: i64,
        end_ms: i64,
        text: &str,
        confidence: Option<f64>,
    ) -> Result<i64, rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO segments (session_id, start_ms, end_ms, text, confidence)
             VALUES (?, ?, ?, ?, ?)",
            params![session_id, start_ms, end_ms, text, confidence],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.title, snippet(segments_fts, 0, '<mark>', '</mark>', '...', 32)
             FROM segments_fts
             JOIN segments seg ON segments_fts.rowid = seg.id
             JOIN sessions s ON seg.session_id = s.id
             WHERE segments_fts MATCH ?
             ORDER BY rank
             LIMIT 50"
        )?;
        // ...
    }
}
```

---

## Dependencies

- None (can be done independently)

---

## References

- SQLite FTS5: https://sqlite.org/fts5.html
- rusqlite: https://crates.io/crates/rusqlite
- SQLite Best Practices: https://sqlite.org/docs.html
- SRT Format: https://docs.fileformat.com/video/srt/
- WebVTT Format: https://developer.mozilla.org/en-US/docs/Web/API/WebVTT_API

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
