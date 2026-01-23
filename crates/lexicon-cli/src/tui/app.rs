//! TUI Application
//!
//! Main application state and event loop for the diagnostic TUI.

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::mpsc;

use super::audio::AudioDeviceManager;
use super::buffer::TranscriptionBuffer;
use super::dialogs::{DialogAction, SaveDialog};
use super::transcription::{TranscriptionManager, TranscriptionManagerConfig, TranscriptionMessage};
use super::views::SavedTranscriptionsList;
use super::widgets::LevelMeter;
use super::Result;

/// TUI display modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiMode {
    /// Main diagnostic/transcription view.
    Main,
    /// Device selection dropdown.
    DeviceSelect,
    /// Save/rename dialog.
    SaveDialog,
    /// Browse saved transcriptions.
    SavedList,
}

/// Configuration for the TUI application.
pub struct TuiConfig {
    /// Buffer size in characters.
    pub buffer_size: usize,
    /// Enable mouse support.
    pub mouse_enabled: bool,
    /// Pre-selected device name.
    pub device_name: Option<String>,
    /// Path to the Whisper model file.
    pub model_path: Option<PathBuf>,
    /// Language code for transcription (e.g., "en", "auto").
    pub language: Option<String>,
    /// Whether to translate to English.
    pub translate: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100_000,
            mouse_enabled: true,
            device_name: None,
            model_path: None,
            language: Some("en".to_string()),
            translate: false,
        }
    }
}

/// Main TUI application.
pub struct TuiApp {
    /// Current view mode.
    mode: TuiMode,
    /// Audio device manager (for level monitoring when no transcription).
    audio: AudioDeviceManager,
    /// Transcription buffer.
    buffer: TranscriptionBuffer,
    /// Recording state.
    recording: bool,
    /// Whether the app should quit.
    should_quit: bool,
    /// Save dialog.
    save_dialog: SaveDialog,
    /// Saved transcriptions list.
    saved_list: SavedTranscriptionsList,
    /// Device dropdown open state.
    device_dropdown_open: bool,
    /// Selected device in dropdown (for navigation).
    dropdown_selection: usize,
    /// Status message to display.
    status_message: Option<(String, std::time::Instant)>,
    /// Mouse enabled.
    mouse_enabled: bool,
    /// Transcription manager (if model is available).
    transcription: Option<TranscriptionManager>,
    /// Receiver for transcription messages.
    transcription_rx: Option<mpsc::UnboundedReceiver<TranscriptionMessage>>,
    /// Whether transcription is enabled.
    transcription_enabled: bool,
    /// Model loading status message.
    model_status: Option<String>,
}

impl TuiApp {
    /// Creates a new TUI application with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(TuiConfig::default())
    }

    /// Creates a new TUI application with the specified configuration.
    pub fn with_config(config: TuiConfig) -> Result<Self> {
        let audio = AudioDeviceManager::new()?;

        // Try to create transcription manager if model path is provided
        let (transcription, transcription_rx, transcription_enabled) =
            if let Some(model_path) = config.model_path {
                if model_path.exists() {
                    let (mut manager, rx) = TranscriptionManager::create();

                    let trans_config = TranscriptionManagerConfig {
                        model_path,
                        language: config.language.clone(),
                        translate: config.translate,
                        device_name: config.device_name.clone(),
                    };

                    match manager.start(trans_config) {
                        Ok(()) => (Some(manager), Some(rx), true),
                        Err(e) => {
                            tracing::warn!("Failed to start transcription: {}", e);
                            (None, None, false)
                        }
                    }
                } else {
                    tracing::warn!("Model file not found: {:?}", model_path);
                    (None, None, false)
                }
            } else {
                (None, None, false)
            };

        Ok(Self {
            mode: TuiMode::Main,
            audio,
            buffer: TranscriptionBuffer::new(config.buffer_size),
            recording: false,
            should_quit: false,
            save_dialog: SaveDialog::new(),
            saved_list: SavedTranscriptionsList::new(),
            device_dropdown_open: false,
            dropdown_selection: 0,
            status_message: None,
            mouse_enabled: config.mouse_enabled,
            transcription,
            transcription_rx,
            transcription_enabled,
            model_status: None,
        })
    }

    /// Runs the TUI event loop.
    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if self.mouse_enabled {
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        } else {
            execute!(stdout, EnterAlternateScreen)?;
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main loop
        let result = self.run_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        if self.mouse_enabled {
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
        } else {
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        }
        terminal.show_cursor()?;

        result
    }

    /// Main event loop.
    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            // Process transcription messages (non-blocking)
            self.process_transcription_messages();

            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

            // Poll for events
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_key(key.code, key.modifiers);
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    Event::Resize(_, _) => {
                        // Terminal will redraw automatically
                    }
                    _ => {}
                }
            }

            // Clear expired status messages
            if let Some((_, instant)) = &self.status_message {
                if instant.elapsed() > Duration::from_secs(3) {
                    self.status_message = None;
                }
            }
        }

        Ok(())
    }

    /// Handles key events.
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Handle save dialog first if visible
        if self.save_dialog.is_visible() {
            match self.save_dialog.handle_key(code, modifiers) {
                DialogAction::Confirm(name) => {
                    self.save_transcript(&name);
                    self.mode = TuiMode::Main;
                }
                DialogAction::Cancel => {
                    self.mode = TuiMode::Main;
                }
                DialogAction::None => {}
            }
            return;
        }

        // Handle device dropdown
        if self.device_dropdown_open {
            match code {
                KeyCode::Up => {
                    if self.dropdown_selection > 0 {
                        self.dropdown_selection -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.dropdown_selection < self.audio.device_count().saturating_sub(1) {
                        self.dropdown_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    let _ = self.audio.select_device(self.dropdown_selection);
                    self.device_dropdown_open = false;
                }
                KeyCode::Esc => {
                    self.device_dropdown_open = false;
                }
                _ => {}
            }
            return;
        }

        // Handle saved list mode
        if self.mode == TuiMode::SavedList {
            match code {
                KeyCode::Up => self.saved_list.select_prev(),
                KeyCode::Down => self.saved_list.select_next(),
                KeyCode::Enter => {
                    // Open selected transcript (would load from DB)
                    self.set_status("Opening transcript...");
                    self.mode = TuiMode::Main;
                }
                KeyCode::Char('d') | KeyCode::Delete => {
                    if let Some(_id) = self.saved_list.remove_selected() {
                        self.set_status("Transcript deleted");
                    }
                }
                KeyCode::Esc => {
                    self.mode = TuiMode::Main;
                }
                _ => {}
            }
            return;
        }

        // Main mode key handling
        match (code, modifiers) {
            // Ctrl+T: Toggle recording
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.toggle_recording();
            }
            // Ctrl+S: Save dialog
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                let default_name = chrono::Local::now().format("Recording %Y-%m-%d %H:%M").to_string();
                self.save_dialog.open_with_name(&default_name);
                self.mode = TuiMode::SaveDialog;
            }
            // Ctrl+L: Saved list
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.mode = TuiMode::SavedList;
            }
            // Ctrl+C: Copy to clipboard
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.copy_to_clipboard();
            }
            // Tab: Open device dropdown
            (KeyCode::Tab, _) => {
                self.dropdown_selection = self.audio.selected_index();
                self.device_dropdown_open = true;
            }
            // Scroll
            (KeyCode::Up, _) => {
                self.buffer.scroll_up(1);
            }
            (KeyCode::Down, _) => {
                self.buffer.scroll_down(1, 20);
            }
            (KeyCode::PageUp, _) => {
                self.buffer.scroll_up(10);
            }
            (KeyCode::PageDown, _) => {
                self.buffer.scroll_down(10, 20);
            }
            // Escape/Q: Quit
            (KeyCode::Esc, _) | (KeyCode::Char('q'), _) => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    /// Handles mouse events.
    fn handle_mouse(&mut self, mouse: event::MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.buffer.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                self.buffer.scroll_down(3, 20);
            }
            MouseEventKind::Down(_) => {
                // Could handle button clicks here
            }
            _ => {}
        }
    }

    /// Renders the current frame.
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        match self.mode {
            TuiMode::Main | TuiMode::DeviceSelect => self.render_main(frame, area),
            TuiMode::SaveDialog => {
                self.render_main(frame, area);
                self.save_dialog.render(area, frame);
            }
            TuiMode::SavedList => self.render_saved_list(frame, area),
        }

        // Render device dropdown if open
        if self.device_dropdown_open {
            self.render_device_dropdown(frame, area);
        }
    }

    /// Renders the main view.
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

        self.render_header(frame, chunks[0]);
        self.render_toolbar(frame, chunks[1]);
        self.render_transcription(frame, chunks[2]);
        self.render_footer(frame, chunks[3]);
    }

    /// Renders the header with device selector.
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let device_name = self.audio.selected_device()
            .map(|d| d.name.as_str())
            .unwrap_or("No device");

        // Truncate device name if too long
        let max_device_len = 30;
        let display_name = if device_name.len() > max_device_len {
            format!("{}...", &device_name[..max_device_len - 3])
        } else {
            device_name.to_string()
        };

        let title = format!(" Lexicon Diagnostics ");
        let device_indicator = format!("[Device: {} \u{25BC}]", display_name);

        let block = Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(title)
            .title_alignment(Alignment::Left);

        frame.render_widget(block, area);

        // Device selector on the right
        let device_x = area.x + area.width.saturating_sub(device_indicator.len() as u16 + 2);
        let device_span = Span::styled(
            device_indicator,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        );
        frame.render_widget(Paragraph::new(device_span), Rect::new(device_x, area.y + 1, area.width, 1));
    }

    /// Renders the toolbar with level meter and char count.
    fn render_toolbar(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),   // Left margin
                Constraint::Length(22),  // Level meter
                Constraint::Length(20),  // Char count
                Constraint::Min(10),     // Recording indicator
            ])
            .split(area);

        // Level meter - use transcription manager if available, otherwise audio manager
        let level = if let Some(ref trans) = self.transcription {
            trans.get_level()
        } else {
            self.audio.get_level()
        };
        let meter = LevelMeter::new(level).width(20);
        frame.render_widget(meter, Rect::new(chunks[1].x, chunks[1].y, 20, 1));

        // Char count
        let (used, total) = self.buffer.usage();
        let count_text = format!(" {:>6} / {:>6} chars", used, total);
        let count_style = if used > total * 9 / 10 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        frame.render_widget(
            Paragraph::new(count_text).style(count_style),
            chunks[2],
        );

        // Recording indicator - show model status if not transcription enabled
        let (rec_style, rec_text) = if self.recording {
            (
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                " \u{25CF} REC (Ctrl+T to stop)".to_string(),
            )
        } else if !self.transcription_enabled {
            (
                Style::default().fg(Color::Yellow),
                " \u{26A0} No model (Ctrl+T)".to_string(),
            )
        } else {
            (
                Style::default().fg(Color::DarkGray),
                " \u{25CB} Ready (Ctrl+T to start)".to_string(),
            )
        };
        frame.render_widget(
            Paragraph::new(rec_text).style(rec_style),
            chunks[3],
        );
    }

    /// Renders the transcription area.
    fn render_transcription(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Transcription ");

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let content = if self.buffer.is_empty() {
            if self.recording {
                "Listening... speak to see transcription here."
            } else {
                "Press Ctrl+T to start recording.\nTranscription will appear here."
            }
        } else {
            self.buffer.content()
        };

        let paragraph = Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .scroll((self.buffer.scroll_offset() as u16, 0))
            .style(if self.buffer.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            });

        frame.render_widget(paragraph, inner);
    }

    /// Renders the footer with buttons and shortcuts.
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);

        frame.render_widget(block, area);

        // Status message or help text
        let help_text = if let Some((msg, _)) = &self.status_message {
            Span::styled(format!(" {} ", msg), Style::default().fg(Color::Green))
        } else {
            Span::raw(" Ctrl+T: Record | Ctrl+S: Save | Ctrl+L: List | Ctrl+C: Copy | Tab: Device | Esc: Exit ")
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(help, Rect::new(area.x + 1, area.y + 1, area.width - 2, 1));
    }

    /// Renders the device dropdown.
    fn render_device_dropdown(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::Clear;

        let devices = self.audio.devices();
        if devices.is_empty() {
            return;
        }

        let dropdown_height = (devices.len() as u16 + 2).min(area.height / 2);
        let dropdown_width = 50.min(area.width - 4);
        let dropdown_x = area.x + area.width.saturating_sub(dropdown_width + 2);
        let dropdown_y = area.y + 3;

        let dropdown_area = Rect::new(dropdown_x, dropdown_y, dropdown_width, dropdown_height);

        // Clear background
        frame.render_widget(Clear, dropdown_area);

        let block = Block::default()
            .title(" Select Device ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(dropdown_area);
        frame.render_widget(block, dropdown_area);

        // Render device list
        for (i, device) in devices.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let is_selected = i == self.dropdown_selection;
            let is_current = i == self.audio.selected_index();

            let prefix = if is_current { "\u{2713} " } else { "  " };
            let name = if device.name.len() > (inner.width as usize - 4) {
                format!("{}...", &device.name[..(inner.width as usize - 7)])
            } else {
                device.name.clone()
            };

            let style = if is_selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };

            let line = Paragraph::new(format!("{}{}", prefix, name)).style(style);
            frame.render_widget(line, Rect::new(inner.x, inner.y + i as u16, inner.width, 1));
        }
    }

    /// Renders the saved transcriptions list.
    fn render_saved_list(&self, frame: &mut Frame, area: Rect) {
        let mut list = self.saved_list.clone();
        list.render(area, frame, true);
    }

    /// Saves the current transcript.
    fn save_transcript(&mut self, name: &str) {
        // In a full implementation, this would save to SQLite
        self.set_status(&format!("Saved: {}", name));
    }

    /// Copies the buffer to clipboard.
    fn copy_to_clipboard(&mut self) {
        use arboard::Clipboard;

        match Clipboard::new() {
            Ok(mut clipboard) => {
                let content = self.buffer.content().to_string();
                if content.is_empty() {
                    self.set_status("Nothing to copy");
                } else {
                    match clipboard.set_text(content) {
                        Ok(()) => self.set_status("Copied to clipboard"),
                        Err(e) => self.set_status(&format!("Clipboard error: {}", e)),
                    }
                }
            }
            Err(e) => {
                self.set_status(&format!("Clipboard error: {}", e));
            }
        }
    }

    /// Sets a status message.
    fn set_status(&mut self, message: &str) {
        self.status_message = Some((message.to_string(), std::time::Instant::now()));
    }

    /// Toggles recording state.
    fn toggle_recording(&mut self) {
        self.recording = !self.recording;

        if self.recording {
            // Start recording
            if let Some(ref trans) = self.transcription {
                trans.start_recording();
                self.set_status("Recording started");
            } else {
                self.set_status("Recording (no transcription model)");
            }
        } else {
            // Stop recording
            if let Some(ref trans) = self.transcription {
                trans.stop_recording();
            }
            self.set_status("Recording stopped");
        }
    }

    /// Processes pending transcription messages.
    fn process_transcription_messages(&mut self) {
        // Collect messages first to avoid borrow issues
        let mut messages = Vec::new();

        if let Some(ref mut rx) = self.transcription_rx {
            loop {
                match rx.try_recv() {
                    Ok(msg) => messages.push(msg),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.transcription_enabled = false;
                        break;
                    }
                }
            }
        }

        // Process collected messages
        for msg in messages {
            match msg {
                TranscriptionMessage::Text(text) => {
                    // Append transcription to buffer with space separator
                    if !self.buffer.is_empty() {
                        let _ = self.buffer.append(" ");
                    }
                    let flushed = self.buffer.append(&text);
                    if flushed {
                        self.set_status("Buffer auto-saved (100k limit)");
                    }
                }
                TranscriptionMessage::Error(err) => {
                    self.set_status(&format!("Error: {}", err));
                }
                TranscriptionMessage::ModelLoaded(name) => {
                    self.model_status = Some(format!("Model: {}", name));
                    self.set_status(&format!("Model loaded: {}", name));
                }
                TranscriptionMessage::ModelError(err) => {
                    self.transcription_enabled = false;
                    self.set_status(&format!("Model error: {}", err));
                }
                TranscriptionMessage::Processing => {
                    // Could show a processing indicator
                }
            }
        }
    }
}

impl Clone for SavedTranscriptionsList {
    fn clone(&self) -> Self {
        Self::with_items(self.items().to_vec())
    }
}
