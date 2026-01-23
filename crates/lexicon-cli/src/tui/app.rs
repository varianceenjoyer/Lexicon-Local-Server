//! TUI Application
//!
//! Main application state and event loop for the diagnostic TUI.

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::mpsc;

use lexicon_core::storage::{Database, NewSavedTranscript, SavedTranscriptId};

use super::audio::AudioDeviceManager;
use super::buffer::TranscriptionBuffer;
use super::dialogs::{ConfirmAction, ConfirmDialog, DialogAction, SaveDialog};
use super::transcription::{TranscriptionManager, TranscriptionManagerConfig, TranscriptionMessage};
use super::views::{SavedTranscript, SavedTranscriptionsList};
use super::widgets::LevelMeter;
use super::Result;

#[allow(unused_imports)]
use super::TuiError;

/// TUI display modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // DeviceSelect reserved for future use
pub enum TuiMode {
    /// Main diagnostic/transcription view.
    Main,
    /// Device selection dropdown.
    DeviceSelect,
    /// Save/rename dialog.
    SaveDialog,
    /// Browse saved transcriptions.
    SavedList,
    /// Confirmation dialog (e.g., save before new).
    ConfirmDialog,
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
    /// Path to the database file.
    pub database_path: Option<PathBuf>,
    /// Whether to prompt before creating a new transcription with unsaved changes.
    pub confirm_new_transcription: bool,
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
            database_path: None,
            confirm_new_transcription: true,
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
    /// Confirmation dialog (for new transcription).
    confirm_dialog: ConfirmDialog,
    /// Whether the current buffer has been saved since last modification.
    buffer_saved: bool,
    /// Whether to create a new transcription after saving (set by confirm dialog flow).
    pending_new_after_save: bool,
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
    /// Database connection for saving transcripts.
    database: Option<Database>,
    /// Whether to prompt before creating a new transcription with unsaved changes.
    confirm_new_transcription: bool,
    /// Pending delete transcript ID (for delete confirmation flow).
    pending_delete_id: Option<i64>,
}

impl TuiApp {
    /// Creates a new TUI application with default configuration.
    #[allow(dead_code)] // Used for testing and future API
    pub fn new() -> Result<Self> {
        Self::with_config(TuiConfig::default())
    }

    /// Creates a new TUI application with the specified configuration.
    pub fn with_config(config: TuiConfig) -> Result<Self> {
        tracing::info!("Creating TUI application...");
        tracing::debug!("TUI config: model_path={:?}, language={:?}, translate={}",
            config.model_path, config.language, config.translate);

        let audio = AudioDeviceManager::new()?;
        tracing::info!("Audio device manager initialized");

        // Try to create transcription manager if model path is provided
        let (transcription, transcription_rx, transcription_enabled) =
            if let Some(model_path) = config.model_path {
                tracing::info!("Model path provided: {:?}", model_path);
                tracing::info!("Model file exists: {}", model_path.exists());

                if model_path.exists() {
                    tracing::info!("Creating transcription manager...");
                    let (mut manager, rx) = TranscriptionManager::create();

                    let trans_config = TranscriptionManagerConfig {
                        model_path: model_path.clone(),
                        language: config.language.clone(),
                        translate: config.translate,
                        device_name: config.device_name.clone(),
                    };

                    match manager.start(trans_config) {
                        Ok(()) => {
                            tracing::info!("Transcription manager started successfully");
                            (Some(manager), Some(rx), true)
                        }
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
                tracing::info!("No model path provided, transcription disabled");
                (None, None, false)
            };

        tracing::info!(
            "TUI initialization complete: transcription_enabled={}",
            transcription_enabled
        );

        // Try to open database if path is provided
        let database = if let Some(db_path) = config.database_path {
            match Database::open(&db_path) {
                Ok(db) => Some(db),
                Err(e) => {
                    tracing::warn!("Failed to open database: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Load saved transcripts if database is available
        let saved_list = if let Some(ref db) = database {
            match db.list_saved_transcripts() {
                Ok(transcripts) => SavedTranscriptionsList::with_items(
                    transcripts.into_iter().map(SavedTranscript::from).collect()
                ),
                Err(_) => SavedTranscriptionsList::new(),
            }
        } else {
            SavedTranscriptionsList::new()
        };

        Ok(Self {
            mode: TuiMode::Main,
            audio,
            buffer: TranscriptionBuffer::new(config.buffer_size),
            recording: false,
            should_quit: false,
            save_dialog: SaveDialog::new(),
            confirm_dialog: ConfirmDialog::new(),
            buffer_saved: true, // Empty buffer is considered "saved"
            pending_new_after_save: false,
            saved_list,
            device_dropdown_open: false,
            dropdown_selection: 0,
            status_message: None,
            mouse_enabled: config.mouse_enabled,
            transcription,
            transcription_rx,
            transcription_enabled,
            model_status: None,
            database,
            confirm_new_transcription: config.confirm_new_transcription,
            pending_delete_id: None,
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
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
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
        // Handle confirm dialog first if visible
        if self.confirm_dialog.is_visible() {
            match self.confirm_dialog.handle_key(code, modifiers) {
                ConfirmAction::Yes => {
                    // Check if this is a delete confirmation
                    if let Some(id) = self.pending_delete_id.take() {
                        // User confirmed delete
                        self.saved_list.remove_selected();
                        if let Some(ref db) = self.database {
                            if let Err(e) = db.delete_saved_transcript(SavedTranscriptId(id)) {
                                self.set_status(&format!("Delete failed: {e}"));
                            } else {
                                self.set_status("Transcript deleted");
                            }
                        } else {
                            self.set_status("Transcript deleted");
                        }
                        self.mode = TuiMode::SavedList;
                    } else {
                        // User wants to save first, open save dialog
                        self.pending_new_after_save = true;
                        let default_name =
                            chrono::Local::now().format("Recording %Y-%m-%d %H:%M").to_string();
                        self.save_dialog.open_with_name(&default_name);
                        self.mode = TuiMode::SaveDialog;
                    }
                }
                ConfirmAction::No => {
                    // Check if this is a delete confirmation
                    if self.pending_delete_id.take().is_some() {
                        // User cancelled delete
                        self.mode = TuiMode::SavedList;
                    } else {
                        // User doesn't want to save, just create new
                        self.create_new_transcription();
                        self.mode = TuiMode::Main;
                    }
                }
                ConfirmAction::Cancel => {
                    // User cancelled
                    if self.pending_delete_id.take().is_some() {
                        self.mode = TuiMode::SavedList;
                    } else {
                        self.mode = TuiMode::Main;
                    }
                }
                ConfirmAction::None => {}
            }
            return;
        }

        // Handle save dialog if visible
        if self.save_dialog.is_visible() {
            match self.save_dialog.handle_key(code, modifiers) {
                DialogAction::Confirm(name) => {
                    self.save_transcript(&name);
                    self.mode = TuiMode::Main;
                    // If we were in confirm dialog flow, create new after save
                    if self.pending_new_after_save {
                        self.pending_new_after_save = false;
                        self.create_new_transcription();
                    }
                }
                DialogAction::Cancel => {
                    self.pending_new_after_save = false;
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
                    // Update both the audio device manager (for level meter when no transcription)
                    // and the transcription manager's audio capture device
                    let _ = self.audio.select_device(self.dropdown_selection);

                    // Also update transcription manager's device if available
                    if let Some(ref mut trans) = self.transcription {
                        if let Some(device) = self.audio.selected_device() {
                            if let Err(e) = trans.change_device(Some(device.name.clone())) {
                                self.set_status(&format!("Device switch failed: {}", e));
                            } else {
                                self.set_status(&format!("Switched to: {}", device.name));
                            }
                        }
                    }

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
            match (code, modifiers) {
                (KeyCode::Up, _) => self.saved_list.select_prev(),
                (KeyCode::Down, _) => self.saved_list.select_next(),
                (KeyCode::Enter, _) => {
                    // Open selected transcript from DB
                    if let Some(selected) = self.saved_list.selected() {
                        let id = selected.id;
                        if let Some(ref db) = self.database {
                            match db.get_saved_transcript(SavedTranscriptId(id)) {
                                Ok(transcript) => {
                                    // Clear buffer and load transcript content
                                    self.buffer.clear();
                                    self.buffer.append(&transcript.content);
                                    self.set_status(&format!("Loaded: {}", transcript.name));
                                }
                                Err(e) => {
                                    self.set_status(&format!("Load failed: {e}"));
                                }
                            }
                        }
                    }
                    self.mode = TuiMode::Main;
                }
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    // Show confirmation dialog for delete
                    if let Some(selected) = self.saved_list.selected() {
                        self.pending_delete_id = Some(selected.id);
                        self.confirm_dialog.open(&format!("Delete \"{}\"?", selected.name));
                        self.mode = TuiMode::ConfirmDialog;
                    }
                }
                (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                    // Export selected transcript to file
                    self.export_selected_transcript();
                }
                (KeyCode::Esc, _) => {
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
            // Ctrl+N: New transcription
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.handle_new_transcription();
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
            TuiMode::ConfirmDialog => {
                self.render_main(frame, area);
                self.confirm_dialog.render(area, frame);
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
            Span::raw(" Ctrl+T: Record | Ctrl+S: Save | Ctrl+N: New | Ctrl+L: List | Ctrl+C: Copy | Tab: Device | Esc: Exit ")
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
        let content = self.buffer.content().to_string();

        if content.is_empty() {
            self.set_status("Nothing to save");
            return;
        }

        if let Some(ref db) = self.database {
            let new_transcript = NewSavedTranscript::new(name, &content);
            match db.create_saved_transcript(&new_transcript) {
                Ok(_id) => {
                    // Reload the saved list to include the new item
                    if let Ok(transcripts) = db.list_saved_transcripts() {
                        self.saved_list.set_items(
                            transcripts.into_iter().map(SavedTranscript::from).collect()
                        );
                    }

                    self.buffer_saved = true;
                    self.set_status(&format!("Saved: {}", name));
                }
                Err(e) => {
                    self.set_status(&format!("Save failed: {}", e));
                }
            }
        } else {
            self.set_status("No database available");
        }
    }

    /// Exports the selected transcript to a file.
    fn export_selected_transcript(&mut self) {
        let Some(selected) = self.saved_list.selected() else {
            self.set_status("No transcript selected");
            return;
        };

        let Some(ref db) = self.database else {
            self.set_status("No database available");
            return;
        };

        let id = selected.id;
        let transcript = match db.get_saved_transcript(SavedTranscriptId(id)) {
            Ok(t) => t,
            Err(e) => {
                self.set_status(&format!("Load failed: {}", e));
                return;
            }
        };

        // Create safe filename from transcript name
        let safe_name: String = transcript
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let safe_name = safe_name.trim();
        let safe_name = if safe_name.is_empty() { "transcript" } else { safe_name };

        // Export to current directory
        let export_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let filename = format!("{}.txt", safe_name);
        let export_path = export_dir.join(&filename);

        // Write the file
        match std::fs::write(&export_path, &transcript.content) {
            Ok(()) => {
                self.set_status(&format!("Exported: {}", export_path.display()));
            }
            Err(e) => {
                self.set_status(&format!("Export failed: {}", e));
            }
        }
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
        tracing::info!(
            "Toggle recording: current={}, has_transcription={}",
            self.recording,
            self.transcription.is_some()
        );
        self.recording = !self.recording;

        if self.recording {
            // Start recording
            if let Some(ref trans) = self.transcription {
                tracing::info!("Starting recording with transcription manager");
                trans.start_recording();
                self.set_status("Recording started");
            } else {
                tracing::warn!("No transcription manager available for recording");
                self.set_status("Recording (no transcription model)");
            }
        } else {
            // Stop recording
            if let Some(ref trans) = self.transcription {
                tracing::info!("Stopping recording");
                trans.stop_recording();
            }
            self.set_status("Recording stopped");
        }
    }

    /// Handles Ctrl+N to create a new transcription.
    fn handle_new_transcription(&mut self) {
        // If buffer is empty, nothing to do
        if self.buffer.is_empty() {
            self.set_status("Already empty");
            return;
        }

        // If buffer has been saved, just create new without prompting
        if self.buffer_saved {
            self.create_new_transcription();
            return;
        }

        // If confirm_new_transcription is disabled, create new without prompting
        if !self.confirm_new_transcription {
            self.create_new_transcription();
            return;
        }

        // Buffer has unsaved changes, prompt user
        self.confirm_dialog.open("Save current transcription before creating new?");
        self.mode = TuiMode::ConfirmDialog;
    }

    /// Creates a new empty transcription, clearing the buffer.
    fn create_new_transcription(&mut self) {
        // Stop recording if active
        if self.recording {
            self.recording = false;
            if let Some(ref trans) = self.transcription {
                trans.stop_recording();
            }
        }

        // Clear the buffer
        self.buffer.clear();
        self.buffer_saved = true;
        self.set_status("New transcription created");
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
                    // If save dialog is visible, send voice input there
                    if self.save_dialog.is_visible() {
                        self.save_dialog.append_voice(&text);
                    } else {
                        // Otherwise, append transcription to buffer with space separator
                        if !self.buffer.is_empty() {
                            let _ = self.buffer.append(" ");
                        }
                        let flushed = self.buffer.append(&text);
                        self.buffer_saved = false; // Mark as unsaved
                        if flushed {
                            self.set_status("Buffer auto-saved (100k limit)");
                        }
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
        let mut cloned = Self::with_items(self.items().to_vec());
        cloned.select(self.selected_index());
        cloned
    }
}
