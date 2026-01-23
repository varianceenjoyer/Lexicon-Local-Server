# Handoff: WP-TUI Diagnostic TUI Implementation

> **ID**: HO-TUI-001 | **Work Package**: WP-TUI
>
> **Created**: 2026-01-23 | **Status**: DB Integration Complete
>
> **Last Updated**: 2026-01-23

---

## Summary

Implement a terminal user interface (TUI) for real-time audio diagnostics and live transcription. Key features include audio device selection, signal level meter, recording toggle, transcription buffer with auto-flush, and saved transcription management.

**Complexity**: HIGH (multi-component feature with real-time audio, UI, and database integration)

---

## Quick Start

1. **Read the work package**: [.dev/work/WP-TUI.md](../work/WP-TUI.md)
2. **Add dependencies** to `lexicon-cli/Cargo.toml`
3. **Start with the core TUI scaffold** (app.rs, event loop)
4. **Add audio device enumeration** (reuse existing cpal infrastructure)
5. **Build level meter widget** first (quick visual feedback)
6. **Integrate transcription** (connect to STT engine)
7. **Add save/load functionality** (database operations)

---

## Implementation Order

### Phase 1: Project Setup and Core Scaffold (1-2 hours)

1. Add dependencies to `lexicon-cli/Cargo.toml`:

```toml
[dependencies]
# TUI framework
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }

# Clipboard (cross-platform)
arboard = "3.4"

# Async for event handling
tokio = { version = "1", features = ["sync", "macros", "rt-multi-thread"] }
```

2. Create TUI module structure:

```bash
mkdir -p crates/lexicon-cli/src/tui/{views,widgets,dialogs}
touch crates/lexicon-cli/src/tui/mod.rs
touch crates/lexicon-cli/src/tui/app.rs
touch crates/lexicon-cli/src/tui/audio.rs
touch crates/lexicon-cli/src/tui/buffer.rs
touch crates/lexicon-cli/src/tui/views/{mod,main,saved_list,transcript}.rs
touch crates/lexicon-cli/src/tui/widgets/{mod,level_meter,device_dropdown}.rs
touch crates/lexicon-cli/src/tui/dialogs/{mod,save}.rs
```

3. Create basic TUI module (`tui/mod.rs`):

```rust
mod app;
mod audio;
mod buffer;
mod views;
mod widgets;
mod dialogs;

pub use app::TuiApp;
pub use audio::AudioDeviceManager;
pub use buffer::TranscriptionBuffer;
```

4. Create app skeleton (`tui/app.rs`):

```rust
use std::io;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEvent, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    Terminal,
};

use super::audio::AudioDeviceManager;
use super::buffer::TranscriptionBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiMode {
    Main,
    DeviceSelect,
    SaveDialog,
    SavedList,
    TranscriptView,
}

pub struct TuiApp {
    mode: TuiMode,
    audio: AudioDeviceManager,
    buffer: TranscriptionBuffer,
    recording: bool,
    should_quit: bool,
}

impl TuiApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            mode: TuiMode::Main,
            audio: AudioDeviceManager::new()?,
            buffer: TranscriptionBuffer::new(100_000),
            recording: false,
            should_quit: false,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main loop
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // TODO: Implement rendering for each mode
        match self.mode {
            TuiMode::Main => self.render_main(frame, area),
            TuiMode::DeviceSelect => self.render_device_select(frame, area),
            TuiMode::SaveDialog => self.render_save_dialog(frame, area),
            TuiMode::SavedList => self.render_saved_list(frame, area),
            TuiMode::TranscriptView => self.render_transcript_view(frame, area),
        }
    }

    fn handle_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => self.handle_key(key.code, key.modifiers),
                Event::Mouse(mouse) => self.handle_mouse(mouse),
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            // Ctrl+T: Toggle recording
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.recording = !self.recording;
            }
            // Ctrl+S: Save dialog
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                self.mode = TuiMode::SaveDialog;
            }
            // Ctrl+L: Saved list
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.mode = TuiMode::SavedList;
            }
            // Esc: Back or quit
            (KeyCode::Esc, _) => {
                if self.mode == TuiMode::Main {
                    self.should_quit = true;
                } else {
                    self.mode = TuiMode::Main;
                }
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, _event: MouseEvent) {
        // TODO: Implement mouse handling
    }

    // Render methods (stubs)
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Lexicon Diagnostics")
            .borders(Borders::ALL);
        frame.render_widget(block, area);
    }

    fn render_device_select(&self, _frame: &mut Frame, _area: Rect) {}
    fn render_save_dialog(&self, _frame: &mut Frame, _area: Rect) {}
    fn render_saved_list(&self, _frame: &mut Frame, _area: Rect) {}
    fn render_transcript_view(&self, _frame: &mut Frame, _area: Rect) {}
}
```

### Phase 2: Audio Device Manager (1 hour)

Create `tui/audio.rs`:

```rust
use cpal::traits::{DeviceTrait, HostTrait};
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub id: String,
    pub is_default: bool,
}

pub struct AudioDeviceManager {
    devices: Vec<AudioDeviceInfo>,
    selected: usize,
    level: Arc<AtomicU32>,  // Stored as u32 bits of f32
}

impl AudioDeviceManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let devices = Self::enumerate_devices()?;
        let default_idx = devices.iter().position(|d| d.is_default).unwrap_or(0);

        Ok(Self {
            devices,
            selected: default_idx,
            level: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn enumerate_devices() -> Result<Vec<AudioDeviceInfo>, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let default_name = host.default_input_device()
            .and_then(|d| d.name().ok());

        let mut devices = Vec::new();

        for device in host.input_devices()? {
            if let Ok(name) = device.name() {
                devices.push(AudioDeviceInfo {
                    id: name.clone(),
                    is_default: Some(&name) == default_name.as_ref(),
                    name,
                });
            }
        }

        Ok(devices)
    }

    pub fn devices(&self) -> &[AudioDeviceInfo] {
        &self.devices
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn select_device(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if index < self.devices.len() {
            self.selected = index;
            // TODO: Rebuild audio stream with new device
        }
        Ok(())
    }

    pub fn selected_device(&self) -> Option<&AudioDeviceInfo> {
        self.devices.get(self.selected)
    }

    /// Get current audio level (0.0 - 1.0)
    pub fn get_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// Update level from audio callback (called from audio thread)
    pub fn set_level(&self, level: f32) {
        self.level.store(level.to_bits(), Ordering::Relaxed);
    }
}
```

### Phase 3: Level Meter Widget (30 min)

Create `tui/widgets/level_meter.rs`:

```rust
use ratatui::prelude::*;
use ratatui::widgets::Widget;

pub struct LevelMeter {
    level: f32,
    width: u16,
}

impl LevelMeter {
    pub fn new(level: f32) -> Self {
        Self {
            level: level.clamp(0.0, 1.0),
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
        let blocks = ['░', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

        let filled_full = (self.level * self.width as f32) as u16;
        let filled_partial = ((self.level * self.width as f32).fract() * 8.0) as usize;

        for x in 0..self.width.min(area.width) {
            let (char, style) = if x < filled_full {
                let color = Self::color_for_position(x, self.width);
                ('█', Style::default().fg(color))
            } else if x == filled_full && filled_partial > 0 {
                let color = Self::color_for_position(x, self.width);
                (blocks[filled_partial], Style::default().fg(color))
            } else {
                ('░', Style::default().fg(Color::DarkGray))
            };

            if area.y < buf.area.height && area.x + x < buf.area.width {
                buf.get_mut(area.x + x, area.y).set_char(char).set_style(style);
            }
        }
    }
}

impl LevelMeter {
    fn color_for_position(x: u16, width: u16) -> Color {
        let ratio = x as f32 / width as f32;
        if ratio > 0.9 {
            Color::Red
        } else if ratio > 0.7 {
            Color::Yellow
        } else {
            Color::Green
        }
    }
}
```

### Phase 4: Transcription Buffer (30 min)

Create `tui/buffer.rs`:

```rust
pub struct TranscriptionBuffer {
    text: String,
    max_chars: usize,
    flush_count: usize,
}

impl TranscriptionBuffer {
    pub fn new(max_chars: usize) -> Self {
        Self {
            text: String::new(),
            max_chars,
            flush_count: 0,
        }
    }

    /// Append text, returns true if auto-flush occurred
    pub fn append(&mut self, text: &str) -> bool {
        self.text.push_str(text);

        if self.text.len() >= self.max_chars {
            self.flush_count += 1;
            // TODO: Flush to database
            self.text.clear();
            return true;
        }

        false
    }

    pub fn content(&self) -> &str {
        &self.text
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn usage(&self) -> (usize, usize) {
        (self.char_count(), self.max_chars)
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }
}
```

### Phase 5: Main View Layout (1 hour)

Update `render_main` in `app.rs`:

```rust
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::layout::{Layout, Constraint, Direction};

fn render_main(&self, frame: &mut Frame, area: Rect) {
    // Layout: Header (3) + Toolbar (1) + Content (rest) + Footer (3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header with device selector
            Constraint::Length(1),  // Toolbar (meter + char count)
            Constraint::Min(10),    // Transcription area
            Constraint::Length(3),  // Footer with buttons
        ])
        .split(area);

    // Header
    self.render_header(frame, chunks[0]);

    // Toolbar
    self.render_toolbar(frame, chunks[1]);

    // Transcription area
    self.render_transcription(frame, chunks[2]);

    // Footer
    self.render_footer(frame, chunks[3]);
}

fn render_header(&self, frame: &mut Frame, area: Rect) {
    let device_name = self.audio.selected_device()
        .map(|d| d.name.as_str())
        .unwrap_or("No device");

    let title = format!(" Lexicon Diagnostics                              [Device: {} ▼] ", device_name);

    let block = Block::default()
        .title(title)
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

    frame.render_widget(block, area);
}

fn render_toolbar(&self, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),  // Level meter
            Constraint::Length(25),  // Char count
            Constraint::Min(10),     // Recording indicator
        ])
        .split(area);

    // Level meter
    let meter = LevelMeter::new(self.audio.get_level()).width(20);
    let meter_area = Rect::new(chunks[0].x + 1, chunks[0].y, 20, 1);
    frame.render_widget(meter, meter_area);

    // Char count
    let (used, total) = self.buffer.usage();
    let count_text = format!(" Chars: {} / {} ", used, total);
    let count = Paragraph::new(count_text);
    frame.render_widget(count, chunks[1]);

    // Recording indicator
    let rec_text = if self.recording {
        Span::styled(" ● REC (Ctrl+T) ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" ○ Ready (Ctrl+T) ", Style::default().fg(Color::DarkGray))
    };
    let rec = Paragraph::new(rec_text);
    frame.render_widget(rec, chunks[2]);
}

fn render_transcription(&self, frame: &mut Frame, area: Rect) {
    let text = self.buffer.content();
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn render_footer(&self, frame: &mut Frame, area: Rect) {
    let help = " [Copy] [Save] [View Saved] │ Ctrl+T: Toggle Record │ Ctrl+S: Save │ Esc: Exit ";
    let paragraph = Paragraph::new(help)
        .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT));

    frame.render_widget(paragraph, area);
}
```

### Phase 6: CLI Command Integration (30 min)

Add to `main.rs`:

```rust
mod tui;

#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...

    /// Launch diagnostic TUI for real-time transcription
    Tui {
        /// Pre-select audio device by name
        #[arg(long)]
        device: Option<String>,

        /// Override default transcription model
        #[arg(long)]
        model: Option<String>,

        /// Disable mouse support
        #[arg(long)]
        no_mouse: bool,

        /// Buffer size before auto-flush (default: 100000)
        #[arg(long, default_value = "100000")]
        buffer_size: usize,
    },

    /// Alias for tui
    Diag {
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        no_mouse: bool,
        #[arg(long, default_value = "100000")]
        buffer_size: usize,
    },
}

// In main():
Commands::Tui { device, model, no_mouse, buffer_size } |
Commands::Diag { device, model, no_mouse, buffer_size } => {
    let mut app = tui::TuiApp::new()?;
    app.run()?;
}
```

### Phase 7: Save Dialog (1 hour)

Create `tui/dialogs/save.rs`:

```rust
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Clear};
use crossterm::event::{KeyCode, KeyModifiers};

pub struct SaveDialog {
    input: String,
    cursor: usize,
    selection_start: Option<usize>,
}

pub enum DialogAction {
    None,
    Confirm(String),
    Cancel,
}

impl SaveDialog {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            selection_start: None,
        }
    }

    pub fn with_default(name: &str) -> Self {
        Self {
            input: name.to_string(),
            cursor: name.len(),
            selection_start: None,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> DialogAction {
        match (code, modifiers) {
            (KeyCode::Enter, _) => {
                DialogAction::Confirm(self.input.clone())
            }
            (KeyCode::Esc, _) => {
                DialogAction::Cancel
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                // Select all
                self.selection_start = Some(0);
                self.cursor = self.input.len();
                DialogAction::None
            }
            (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                // Cut (delete selection)
                if let Some(start) = self.selection_start.take() {
                    let (from, to) = if start < self.cursor {
                        (start, self.cursor)
                    } else {
                        (self.cursor, start)
                    };
                    self.input.drain(from..to);
                    self.cursor = from;
                }
                DialogAction::None
            }
            (KeyCode::Left, KeyModifiers::CONTROL) => {
                // Move word left
                self.cursor = self.find_word_boundary_left();
                DialogAction::None
            }
            (KeyCode::Right, KeyModifiers::CONTROL) => {
                // Move word right
                self.cursor = self.find_word_boundary_right();
                DialogAction::None
            }
            (KeyCode::Left, _) => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                self.selection_start = None;
                DialogAction::None
            }
            (KeyCode::Right, _) => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
                self.selection_start = None;
                DialogAction::None
            }
            (KeyCode::Backspace, _) => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
                DialogAction::None
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
                DialogAction::None
            }
            _ => DialogAction::None,
        }
    }

    pub fn append_voice(&mut self, text: &str) {
        self.input.push_str(text);
        self.cursor = self.input.len();
    }

    fn find_word_boundary_left(&self) -> usize {
        let chars: Vec<char> = self.input.chars().collect();
        let mut pos = self.cursor.saturating_sub(1);

        // Skip whitespace
        while pos > 0 && chars[pos].is_whitespace() {
            pos -= 1;
        }
        // Skip word chars
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        pos
    }

    fn find_word_boundary_right(&self) -> usize {
        let chars: Vec<char> = self.input.chars().collect();
        let mut pos = self.cursor;

        // Skip word chars
        while pos < chars.len() && !chars[pos].is_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        pos
    }

    pub fn render(&self, area: Rect, frame: &mut Frame) {
        // Center the dialog
        let dialog_width = 50;
        let dialog_height = 5;
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Dialog box
        let block = Block::default()
            .title(" Save Transcript ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Input field
        let input_area = Rect::new(inner.x + 1, inner.y + 1, inner.width - 2, 1);
        let input_text = format!("Name: {}", self.input);
        let paragraph = Paragraph::new(input_text);
        frame.render_widget(paragraph, input_area);

        // Show cursor
        frame.set_cursor_position((input_area.x + 6 + self.cursor as u16, input_area.y));
    }
}
```

### Phase 8: Saved Transcriptions List (1 hour)

See work package for full implementation. Key operations:
- Load list from database
- Render scrollable list
- Handle selection, deletion, export

### Phase 9: Database Integration (1 hour)

Add schema migration and CRUD operations for `saved_transcripts` table.

---

## Key Files

| File | Purpose |
|------|---------|
| `crates/lexicon-cli/src/tui/mod.rs` | TUI module root |
| `crates/lexicon-cli/src/tui/app.rs` | Main application state and event loop |
| `crates/lexicon-cli/src/tui/audio.rs` | Audio device enumeration and level monitoring |
| `crates/lexicon-cli/src/tui/buffer.rs` | Transcription buffer with auto-flush |
| `crates/lexicon-cli/src/tui/widgets/level_meter.rs` | Audio level visualization |
| `crates/lexicon-cli/src/tui/dialogs/save.rs` | Save/rename dialog |
| `crates/lexicon-cli/src/tui/views/saved_list.rs` | Saved transcriptions browser |

---

## Testing Commands

```bash
# Build and run TUI
cargo run -p lexicon-cli -- tui

# Run TUI with specific device
cargo run -p lexicon-cli -- tui --device "MacBook Pro Microphone"

# Run without mouse support
cargo run -p lexicon-cli -- tui --no-mouse

# Run unit tests
cargo test -p lexicon-cli tui

# Test level meter widget
cargo test -p lexicon-cli level_meter

# Test buffer overflow
cargo test -p lexicon-cli buffer
```

---

## Implementation Progress (2026-01-23)

### Completed
- [x] TUI module structure created (`crates/lexicon-cli/src/tui/`)
- [x] `lexicon tui` and `lexicon diag` CLI commands with options
- [x] Audio device manager with real-time level monitoring via cpal
- [x] Level meter widget with color-coded visualization
- [x] Transcription buffer with scroll support
- [x] Save dialog with text input and selection
- [x] Saved transcriptions list view
- [x] Main event loop with keyboard and mouse handling
- [x] All core keyboard shortcuts (Ctrl+T/S/L/C)
- [x] Device dropdown widget
- [x] Database schema migration v2 for `saved_transcripts` table
- [x] Model types: `SavedTranscriptId`, `SavedTranscript`, `NewSavedTranscript`
- [x] Database CRUD operations for saved transcripts
- [x] FTS5 full-text search for saved transcripts
- [x] TranscriptionManager with background worker thread
- [x] Audio capture and resampling to 16kHz for Whisper
- [x] Voice activity detection for intelligent transcription timing
- [x] Channel-based communication between audio thread and UI
- [x] All tests passing (44 CLI tests, 40 storage tests, 6 migration tests, 32 TUI tests)
- [x] TranscriptionManager wired into TuiApp event loop
- [x] CLI arguments for model path, language, and translate options
- [x] Auto-detection of model files from config directory
- [x] Database integration in TuiApp (save, load, delete)
- [x] Type conversion from storage::SavedTranscript to views::SavedTranscript
- [x] Save dialog saves to database with full persistence
- [x] Delete from saved list deletes from database
- [x] Open transcript loads content from database into buffer
- [x] Database path passed from CLI to TuiConfig

### Remaining
- [ ] Voice input in save dialog
- [ ] Export functionality
- [ ] Test with actual Whisper model on hardware

---

## Acceptance Checklist

### Core TUI
- [x] TUI launches with `lexicon tui` command
- [x] Mouse support enabled by default
- [x] Clean exit with Esc key
- [x] Keyboard shortcuts documented in footer

### Audio
- [x] Device dropdown shows all input devices
- [x] Current device displayed at top
- [x] Device selection changes audio source
- [x] Level meter updates in real-time

### Recording
- [x] Ctrl+T toggles recording
- [x] Recording indicator visible (● REC)
- [ ] Transcription streams to text area (needs STT integration)
- [x] Char counter updates

### Buffer
- [x] 100k char limit enforced
- [x] Auto-flush triggers at limit
- [ ] User notified of auto-flush

### Database
- [x] Schema migration v2 for `saved_transcripts` table
- [x] FTS5 full-text search index
- [x] CRUD operations (create, read, update, delete)
- [x] Character count tracking
- [x] Session association (optional)
- [x] Connect TUI views to database operations

### Save/Rename
- [x] Ctrl+S opens dialog
- [x] Text input works
- [ ] Voice input works (if recording)
- [x] Keyboard shortcuts work (Ctrl+A, Ctrl+X, Ctrl+arrows)
- [x] Enter confirms, Esc cancels
- [x] Saves to database on confirm

### Copy
- [x] Copy button works
- [x] Text copied to system clipboard
- [x] Visual confirmation

### Saved List
- [x] Ctrl+L opens saved list
- [x] List shows name, preview, date
- [x] Selection with arrow keys
- [x] Enter opens transcript (loads from database into buffer)
- [x] Delete removes transcript (deletes from database)
- [x] Back returns to main

---

## Notes

- Use `crossterm` for cross-platform terminal handling
- Test on Windows, macOS, and Linux terminals
- Consider terminal size constraints (min 80x24)
- Handle terminal resize events gracefully
- Audio level should use RMS or peak detection

---

*This is a HIGH priority feature for Beta. Estimated effort: 8-12 hours.*
