---
id: WP-TUI
type: work-plan
status: complete
phase: beta
created: 2026-01-23
updated: 2026-01-25
author: design-lead
consumers: [developer]
depends-on: [WP-CORE, WP-AUDIO, WP-STT, WP-DB, WP-CLI]
blocks: []
priority: high
tags: [tui, cli, audio, transcription, diagnostics]
---

# WP-TUI: Diagnostic TUI for Real-Time Transcription

> **Phase**: Beta | **Priority**: HIGH | **Status**: COMPLETE

## Summary

Provide a terminal user interface (TUI) for real-time audio diagnostics and live transcription. Users can select audio devices, monitor signal levels, and capture transcriptions with integrated SQLite persistence. The TUI serves both as a diagnostic tool and a primary transcription workflow.

---

## Feature Overview

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Lexicon Diagnostics                              [Device: MacBook Pro Mic ▼] │
├──────────────────────────────────────────────────────────────────────────────┤
│ Audio Level: ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁  │  Chars: 1,234 / 100,000  │  ● REC (Ctrl+T) │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  The quick brown fox jumps over the lazy dog. This is a live transcription  │
│  of speech being captured in real-time from the selected audio device.      │
│                                                                              │
│  [Transcription continues here as user speaks...]                           │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ [Copy] [Save] [View Saved] │ Ctrl+T: Toggle Record │ Ctrl+S: Save │ Esc: Exit│
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Scope

### Included
- CLI command to launch diagnostic TUI (`lexicon tui` or `lexicon diag`)
- Mouse support throughout the interface
- Audio device selection dropdown at top of screen
- Real-time audio signal level meter visualization
- Sticky toolbar showing level meter and character count (used/total)
- Ctrl+T toggle to start/stop transcription recording
- Scrollable text area for transcription output (up to 100k chars)
- Automatic buffer flush to SQLite when reaching 100k char limit
- Ctrl+S to save/rename transcript with text input or voice dictation
- Copy button to copy transcript to system clipboard
- Save button to persist transcript to SQLite
- Separate view for browsing saved transcriptions
- Keyboard navigation backups (Ctrl+arrows, Ctrl+A, Ctrl+X)

### Excluded
- Translation during live capture (future enhancement)
- TTS playback within TUI (separate workflow)
- Multi-language model switching (use config)
- Network/server mode diagnostics

---

## User Flow

### Main Diagnostic Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            LAUNCH TUI                                        │
│                        `lexicon tui`                                        │
└─────────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────────┐
                    │  Load audio devices         │
                    │  Show current selection     │
                    │  Display level meter        │
                    └─────────────────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────────┐
                    │  User selects device        │◄────── Mouse click / arrow keys
                    │  (dropdown at top)          │
                    └─────────────────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────────┐
                    │  Confirm audio input        │◄────── Level meter shows signal
                    │  (visual feedback)          │
                    └─────────────────────────────┘
                                   │
                                   ▼ Ctrl+T
                    ┌─────────────────────────────┐
                    │  Toggle recording ON        │
                    │  ● REC indicator shows      │
                    │  Transcription streams      │
                    └─────────────────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────────┐
                    │  Text fills buffer          │
                    │  Chars: X / 100,000         │
                    │  Auto-flush at limit        │
                    └─────────────────────────────┘
                                   │
                           ┌───────┴───────┐
                           │               │
                    Ctrl+S │               │ Ctrl+T (toggle off)
                           ▼               ▼
              ┌───────────────────┐  ┌─────────────────┐
              │  Save/Rename      │  │  Stop recording │
              │  dialog appears   │  │  Keep text      │
              └───────────────────┘  └─────────────────┘
                           │
                           ▼
              ┌───────────────────┐
              │  Type name or     │
              │  speak name (REC) │
              │  Enter to confirm │
              └───────────────────┘
                           │
                           ▼
              ┌───────────────────┐
              │  Transcript saved │
              │  to SQLite        │
              └───────────────────┘
```

### Saved Transcriptions View

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Saved Transcriptions                                        [Back to Main]  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ ● Meeting Notes 2026-01-23         │ 2,345 chars │ Jan 23, 3:45 PM     ││
│  │   "The quarterly report shows..."                                       ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │   Voice Memo                       │ 892 chars   │ Jan 22, 10:30 AM    ││
│  │   "Remember to call the client..." ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │   Untitled Recording               │ 5,678 chars │ Jan 21, 2:15 PM     ││
│  │   "This is a test of the..."                                            ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ [Open] [Delete] [Export] │ ↑↓: Navigate │ Enter: Open │ Esc: Back          │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Interface Contract

### TUI Application

```rust
// crates/lexicon-cli/src/tui/mod.rs

use ratatui::prelude::*;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEvent};

/// Main TUI application state
pub struct TuiApp {
    /// Current view mode
    mode: TuiMode,
    /// Audio device manager
    audio: AudioDeviceManager,
    /// Transcription buffer
    buffer: TranscriptionBuffer,
    /// Recording state
    recording: bool,
    /// Selected device index
    selected_device: usize,
    /// Device dropdown open
    device_dropdown_open: bool,
}

/// TUI display modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiMode {
    /// Main diagnostic/transcription view
    Main,
    /// Device selection dropdown
    DeviceSelect,
    /// Save/rename dialog
    SaveDialog,
    /// Browse saved transcriptions
    SavedList,
    /// View single saved transcript
    TranscriptView,
}

impl TuiApp {
    /// Create new TUI application
    pub fn new() -> Result<Self, TuiError>;

    /// Run the TUI event loop
    pub async fn run(&mut self) -> Result<(), TuiError>;

    /// Handle keyboard event
    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Action, TuiError>;

    /// Handle mouse event
    fn handle_mouse(&mut self, event: MouseEvent) -> Result<Action, TuiError>;

    /// Render current frame
    fn render(&self, frame: &mut Frame);
}

/// Actions that can be triggered by user input
#[derive(Debug, Clone)]
pub enum Action {
    None,
    Quit,
    ToggleRecording,
    OpenDeviceDropdown,
    SelectDevice(usize),
    OpenSaveDialog,
    SaveTranscript(String),
    CopyToClipboard,
    OpenSavedList,
    ViewTranscript(i64),
    DeleteTranscript(i64),
    Back,
}
```

### Audio Device Manager

```rust
// crates/lexicon-cli/src/tui/audio.rs

use cpal::traits::{DeviceTrait, HostTrait};

/// Manages audio device enumeration and selection
pub struct AudioDeviceManager {
    /// Available input devices
    devices: Vec<AudioDeviceInfo>,
    /// Currently selected device index
    selected: usize,
    /// Current audio level (0.0 - 1.0)
    level: f32,
    /// Audio stream handle
    stream: Option<AudioStream>,
}

/// Information about an audio device
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    /// Device name for display
    pub name: String,
    /// Unique device identifier
    pub id: String,
    /// Whether this is the system default
    pub is_default: bool,
    /// Sample rate
    pub sample_rate: u32,
}

impl AudioDeviceManager {
    /// Enumerate all available input devices
    pub fn enumerate_devices() -> Result<Vec<AudioDeviceInfo>, AudioError>;

    /// Select a device by index
    pub fn select_device(&mut self, index: usize) -> Result<(), AudioError>;

    /// Get current audio level (for meter display)
    pub fn get_level(&self) -> f32;

    /// Start audio capture
    pub fn start_capture(&mut self) -> Result<AudioReceiver, AudioError>;

    /// Stop audio capture
    pub fn stop_capture(&mut self);

    /// Get selected device info
    pub fn selected_device(&self) -> &AudioDeviceInfo;
}
```

### Transcription Buffer

```rust
// crates/lexicon-cli/src/tui/buffer.rs

/// Text buffer for live transcription with auto-flush
pub struct TranscriptionBuffer {
    /// Current text content
    text: String,
    /// Maximum buffer size before auto-flush
    max_chars: usize,
    /// Database connection for flushing
    db: Database,
    /// Current session ID (if recording)
    session_id: Option<i64>,
}

impl TranscriptionBuffer {
    /// Create new buffer with 100k char limit
    pub fn new(db: Database) -> Self {
        Self {
            text: String::new(),
            max_chars: 100_000,
            db,
            session_id: None,
        }
    }

    /// Append text to buffer, auto-flush if needed
    pub fn append(&mut self, text: &str) -> Result<(), BufferError>;

    /// Get current character count
    pub fn char_count(&self) -> usize;

    /// Get buffer usage as (used, total)
    pub fn usage(&self) -> (usize, usize);

    /// Clear buffer (after save or flush)
    pub fn clear(&mut self);

    /// Get buffer content for display
    pub fn content(&self) -> &str;

    /// Force flush to database
    pub fn flush(&mut self) -> Result<(), BufferError>;

    /// Save with name
    pub fn save_as(&mut self, name: &str) -> Result<i64, BufferError>;
}
```

### Level Meter Widget

```rust
// crates/lexicon-cli/src/tui/widgets/level_meter.rs

use ratatui::prelude::*;
use ratatui::widgets::Widget;

/// Audio level meter visualization
pub struct LevelMeter {
    /// Current level (0.0 - 1.0)
    level: f32,
    /// Peak hold level
    peak: f32,
    /// Meter width in characters
    width: u16,
}

impl LevelMeter {
    pub fn new(level: f32) -> Self {
        Self {
            level,
            peak: level,
            width: 20,
        }
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }
}

impl Widget for LevelMeter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render using block characters: ▁▂▃▄▅▆▇█
        // Green for normal levels, yellow for moderate, red for clipping
        let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let filled = (self.level * self.width as f32) as u16;

        for x in 0..self.width.min(area.width) {
            let char = if x < filled {
                let intensity = (x as f32 / self.width as f32 * 7.0) as usize;
                blocks[intensity.min(7)]
            } else {
                '░'
            };

            let color = if x as f32 / self.width as f32 > 0.9 {
                Color::Red
            } else if x as f32 / self.width as f32 > 0.7 {
                Color::Yellow
            } else {
                Color::Green
            };

            buf.set_string(
                area.x + x,
                area.y,
                char.to_string(),
                Style::default().fg(color),
            );
        }
    }
}
```

### Save Dialog

```rust
// crates/lexicon-cli/src/tui/dialogs/save.rs

/// Save/rename dialog with text or voice input
pub struct SaveDialog {
    /// Current input text
    input: String,
    /// Cursor position
    cursor: usize,
    /// Whether voice input is active
    voice_input: bool,
}

impl SaveDialog {
    pub fn new() -> Self;

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> DialogAction;

    /// Handle voice transcription input (when recording active)
    pub fn append_voice(&mut self, text: &str);

    /// Get current input value
    pub fn value(&self) -> &str;

    /// Render the dialog
    pub fn render(&self, area: Rect, frame: &mut Frame);
}

pub enum DialogAction {
    None,
    Confirm(String),
    Cancel,
    /// Text selection actions
    SelectAll,
    Cut,
    /// Move cursor
    MoveLeft,
    MoveRight,
    MoveWordLeft,
    MoveWordRight,
}
```

### Saved Transcriptions List

```rust
// crates/lexicon-cli/src/tui/views/saved_list.rs

/// View for browsing saved transcriptions
pub struct SavedTranscriptionsList {
    /// List of saved transcriptions
    items: Vec<SavedTranscript>,
    /// Currently selected index
    selected: usize,
    /// Scroll offset
    scroll: usize,
}

/// Summary of a saved transcript
#[derive(Debug, Clone)]
pub struct SavedTranscript {
    pub id: i64,
    pub name: String,
    pub preview: String,  // First ~50 chars
    pub char_count: usize,
    pub created_at: DateTime<Utc>,
}

impl SavedTranscriptionsList {
    /// Load from database
    pub fn load(db: &Database) -> Result<Self, DbError>;

    /// Navigate selection
    pub fn select_next(&mut self);
    pub fn select_prev(&mut self);

    /// Get selected transcript ID
    pub fn selected_id(&self) -> Option<i64>;

    /// Delete selected transcript
    pub fn delete_selected(&mut self, db: &Database) -> Result<(), DbError>;

    /// Render the list
    pub fn render(&self, area: Rect, frame: &mut Frame);
}
```

---

## CLI Command

```
lexicon tui
    Launch the diagnostic TUI for real-time transcription

    OPTIONS:
        --device <NAME>       Pre-select audio device by name
        --model <MODEL>       Override default transcription model
        --no-mouse            Disable mouse support
        --buffer-size <N>     Set buffer size (default: 100000)

lexicon diag
    Alias for `lexicon tui`

KEYBOARD SHORTCUTS:
    Ctrl+T          Toggle recording on/off
    Ctrl+S          Save transcript (opens rename dialog)
    Ctrl+C          Copy transcript to clipboard
    Ctrl+L          Open saved transcriptions list
    Tab / Shift+Tab Navigate UI elements
    Esc             Close dialog / Exit TUI
    ↑/↓             Navigate lists / device selection
    Enter           Confirm selection / Open item

IN RENAME DIALOG:
    Ctrl+A          Select all text
    Ctrl+X          Cut selection (delete)
    Ctrl+←/→        Move cursor by word
    Enter           Confirm name and save
    Esc             Cancel
```

---

## Database Schema Extensions

```sql
-- Saved transcriptions table (extends existing schema)
CREATE TABLE IF NOT EXISTS saved_transcripts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    char_count INTEGER NOT NULL,
    session_id INTEGER REFERENCES sessions(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_saved_transcripts_created
    ON saved_transcripts(created_at DESC);

-- Full-text search for saved transcripts
CREATE VIRTUAL TABLE IF NOT EXISTS saved_transcripts_fts USING fts5(
    name,
    content,
    content='saved_transcripts',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS saved_transcripts_ai AFTER INSERT ON saved_transcripts BEGIN
    INSERT INTO saved_transcripts_fts(rowid, name, content)
    VALUES (new.id, new.name, new.content);
END;

CREATE TRIGGER IF NOT EXISTS saved_transcripts_ad AFTER DELETE ON saved_transcripts BEGIN
    INSERT INTO saved_transcripts_fts(saved_transcripts_fts, rowid, name, content)
    VALUES ('delete', old.id, old.name, old.content);
END;
```

---

## Acceptance Criteria

### Audio Device Selection
- [ ] Device dropdown shows all available input devices at top of TUI
- [ ] Current device clearly indicated when dropdown closed
- [ ] Mouse click opens/closes dropdown
- [ ] Arrow keys navigate device list
- [ ] Enter selects device, Esc closes without change
- [ ] Changing device updates audio stream without restart

### Level Meter
- [ ] Real-time audio level visualization using block characters
- [ ] Updates at least 10 times per second
- [ ] Color coding: green (normal), yellow (moderate), red (clipping)
- [ ] Visible in sticky toolbar

### Recording Toggle
- [ ] Ctrl+T toggles recording on/off
- [ ] Clear visual indicator when recording (● REC)
- [ ] Transcription streams to text area when recording
- [ ] Recording state persists across device changes

### Transcription Buffer
- [ ] Text area shows live transcription
- [ ] Scrollable when content exceeds view
- [ ] Character counter shows used/total (e.g., "1,234 / 100,000")
- [ ] Auto-flush to SQLite at 100k char limit
- [ ] Clear notification when auto-flush occurs

### Save/Rename Dialog
- [ ] Ctrl+S opens save dialog with text input
- [ ] User can type filename normally
- [ ] Voice input works when recording is active
- [ ] Ctrl+A selects all, Ctrl+X cuts selection
- [ ] Ctrl+←/→ moves cursor by word
- [ ] Enter confirms, Esc cancels
- [ ] Transcript saved to SQLite with name

### Copy to Clipboard
- [ ] Copy button (mouse) copies transcript
- [ ] Ctrl+C copies transcript (when not in input field)
- [ ] Visual confirmation of copy

### Saved Transcriptions View
- [ ] Accessible via button or Ctrl+L
- [ ] List shows name, preview, char count, date
- [ ] Arrow keys navigate list
- [ ] Enter opens selected transcript
- [ ] Delete option removes transcript
- [ ] Export option saves to file
- [ ] Back button returns to main view

### Mouse Support
- [ ] All UI elements clickable
- [ ] Scroll wheel works in text areas
- [ ] Device dropdown responds to clicks
- [ ] Buttons highlight on hover

### Keyboard Navigation
- [ ] Tab cycles through UI elements
- [ ] All actions have keyboard shortcuts
- [ ] Focus indicator visible
- [ ] Esc closes dialogs/dropdowns

---

## Dependencies

### Requires
- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal control
- `copypasta` or `arboard` - Clipboard access
- `cpal` - Audio device enumeration (already in use)
- `rusqlite` - Database (already in use)
- WP-AUDIO - Audio capture infrastructure
- WP-STT - Transcription engine
- WP-DB - Database schema

### Blocks
- None (standalone feature)

---

## Files

### New Files
- `crates/lexicon-cli/src/tui/mod.rs` - TUI module root
- `crates/lexicon-cli/src/tui/app.rs` - Main application state
- `crates/lexicon-cli/src/tui/audio.rs` - Audio device manager
- `crates/lexicon-cli/src/tui/buffer.rs` - Transcription buffer
- `crates/lexicon-cli/src/tui/views/mod.rs` - View modules
- `crates/lexicon-cli/src/tui/views/main.rs` - Main diagnostic view
- `crates/lexicon-cli/src/tui/views/saved_list.rs` - Saved transcriptions list
- `crates/lexicon-cli/src/tui/views/transcript.rs` - Single transcript view
- `crates/lexicon-cli/src/tui/widgets/mod.rs` - Custom widgets
- `crates/lexicon-cli/src/tui/widgets/level_meter.rs` - Audio level meter
- `crates/lexicon-cli/src/tui/widgets/device_dropdown.rs` - Device selector
- `crates/lexicon-cli/src/tui/dialogs/mod.rs` - Dialog modules
- `crates/lexicon-cli/src/tui/dialogs/save.rs` - Save/rename dialog

### Modified Files
- `crates/lexicon-cli/src/main.rs` - Add `tui` subcommand
- `crates/lexicon-cli/Cargo.toml` - Add ratatui, crossterm, clipboard deps
- `crates/lexicon-core/src/db/schema.rs` - Add saved_transcripts table

---

## Test Plan

### Unit Tests

| Test | Input | Expected Output |
|------|-------|-----------------|
| `buffer_append` | Text chunks | Buffer accumulates correctly |
| `buffer_auto_flush` | 100k+ chars | Auto-flush triggered, buffer cleared |
| `level_meter_render` | Level 0.5 | Half-filled meter |
| `level_meter_colors` | Level 0.95 | Red coloring |
| `device_enumerate` | System devices | List of available devices |
| `dialog_text_input` | Key events | Correct text handling |
| `dialog_selection` | Ctrl+A, Ctrl+X | Select all, cut works |

### Integration Tests

| Test | Scenario | Pass Criteria |
|------|----------|---------------|
| `tui_launches` | `lexicon tui` | TUI renders without crash |
| `device_switch` | Select different device | Audio stream switches |
| `record_toggle` | Ctrl+T twice | Recording starts/stops |
| `save_transcript` | Ctrl+S, type name, Enter | Saved to database |
| `load_saved` | Open saved list | Shows saved transcripts |
| `clipboard_copy` | Click Copy | Text in clipboard |

### Manual Tests

| Test | Procedure | Pass Criteria |
|------|-----------|---------------|
| Level meter responsiveness | Speak into mic | Meter moves in real-time |
| Mouse interactions | Click all elements | All respond correctly |
| Long recording | Record 5 minutes | No buffer issues |
| Device hot-swap | Unplug/replug mic | Handles gracefully |

---

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Clipboard access varies by platform | MEDIUM | MEDIUM | Use arboard crate with fallbacks |
| Terminal doesn't support mouse | LOW | LOW | --no-mouse flag, keyboard shortcuts |
| Audio device enumeration fails | MEDIUM | LOW | Graceful error, retry option |
| Buffer overflow on fast speech | LOW | LOW | Streaming append, async flush |
| Transcription latency visible | MEDIUM | MEDIUM | Indicate processing state |

---

## Research References

- **R-TUI-01**: Terminal UI framework comparison (ratatui vs cursive vs tui-rs)
- **R-AUD-01**: cpal audio capture patterns
- **R-DB-01**: SQLite schema design

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work package created |
| 2026-01-23 | Feature complete - all implementation items done |
| 2026-01-25 | Hardware verification complete - MVP spec validated on Windows and bash builds |

---

## Implementation Notes

### 2026-01-23: Core TUI Scaffolding Complete

**Implemented:**
- TUI module structure (`crates/lexicon-cli/src/tui/`)
- `lexicon tui` and `lexicon diag` CLI commands with options
- Audio device manager with real-time level monitoring
- Level meter widget with color-coded visualization
- Transcription buffer with scroll support
- Save dialog with text input and selection
- Saved transcriptions list view
- Main event loop with keyboard and mouse handling
- All core keyboard shortcuts (Ctrl+T, Ctrl+S, Ctrl+L, Ctrl+C)

**Dependencies added:**
- ratatui 0.29
- crossterm 0.28
- arboard 3.4
- cpal (for audio capture)

**Build status:** Compiles successfully with warnings (unused code paths for future features)

### 2026-01-23: Feature Complete

**All features implemented:**
- [x] Integration with WhisperTranscriber for real-time STT
- [x] Database integration for saving/loading transcripts
- [x] Database schema migration for saved_transcripts table
- [x] Voice input in save dialog
- [x] Export functionality (export selected transcript to .txt file)
- [x] Polish and cleanup of warnings (dead_code suppressed, clippy fixes applied)

**Status:** Feature complete. Awaiting hardware testing for final validation.
