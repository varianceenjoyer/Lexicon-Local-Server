//! Main Diagnostic View
//!
//! The primary TUI view with device selection, level meter, and transcription display.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::widgets::{DeviceDropdown, LevelMeter};
use crate::tui::{AudioDeviceManager, TranscriptionBuffer};

/// Renders the main diagnostic view.
pub struct MainView;

impl MainView {
    /// Renders the main view to the frame.
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        audio: &AudioDeviceManager,
        buffer: &TranscriptionBuffer,
        recording: bool,
        dropdown_open: bool,
        dropdown_hover: usize,
    ) {
        // Layout: Header (3) + Toolbar (1) + Content (rest) + Footer (3)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header with device selector
                Constraint::Length(1),  // Toolbar (meter + char count + REC)
                Constraint::Min(5),     // Transcription area
                Constraint::Length(3),  // Footer with buttons/help
            ])
            .split(area);

        // Render each section
        Self::render_header(frame, chunks[0], audio, dropdown_open, dropdown_hover);
        Self::render_toolbar(frame, chunks[1], audio, buffer, recording);
        Self::render_transcription(frame, chunks[2], buffer);
        Self::render_footer(frame, chunks[3]);
    }

    /// Renders the header with device selection.
    fn render_header(
        frame: &mut Frame,
        area: Rect,
        audio: &AudioDeviceManager,
        dropdown_open: bool,
        dropdown_hover: usize,
    ) {
        let device_name = audio
            .selected_device()
            .map(|d| d.name.as_str())
            .unwrap_or("No device");

        let block = Block::default()
            .title(" Lexicon Diagnostics ")
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Right-aligned device selector
        if inner.width > 20 {
            let selector_width = device_name.len() as u16 + 6; // "[name ▼]"
            let selector_x = inner.x + inner.width.saturating_sub(selector_width);

            if dropdown_open {
                // Render expanded dropdown
                let dropdown_height = (audio.device_count() + 2).min(8) as u16;
                let dropdown_area = Rect::new(
                    selector_x.saturating_sub(10),
                    inner.y,
                    selector_width + 15,
                    dropdown_height,
                );

                let dropdown = DeviceDropdown::new(audio.devices(), dropdown_hover)
                    .expanded(true);
                frame.render_widget(dropdown, dropdown_area);
            } else {
                // Render collapsed dropdown
                let dropdown_area = Rect::new(selector_x, inner.y, selector_width, 1);
                let dropdown = DeviceDropdown::new(audio.devices(), audio.selected_index())
                    .expanded(false);
                frame.render_widget(dropdown, dropdown_area);
            }
        }
    }

    /// Renders the toolbar with level meter and status.
    fn render_toolbar(
        frame: &mut Frame,
        area: Rect,
        audio: &AudioDeviceManager,
        buffer: &TranscriptionBuffer,
        recording: bool,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25),  // Level meter
                Constraint::Length(22),  // Char count
                Constraint::Min(15),     // Recording indicator
            ])
            .split(area);

        // Level meter
        let level = audio.get_level();
        let meter = LevelMeter::new(level).width(20).label("│");
        frame.render_widget(meter, chunks[0]);

        // Char count
        let (used, total) = buffer.usage();
        let count_text = format!("│ {} / {} ", used, total);
        let count = Paragraph::new(count_text)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(count, chunks[1]);

        // Recording indicator
        let rec_span = if recording {
            Span::styled(
                "│ ● REC (Ctrl+T) ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                "│ ○ Ready (Ctrl+T) ",
                Style::default().fg(Color::DarkGray),
            )
        };
        let rec = Paragraph::new(rec_span);
        frame.render_widget(rec, chunks[2]);
    }

    /// Renders the transcription text area.
    fn render_transcription(frame: &mut Frame, area: Rect, buffer: &TranscriptionBuffer) {
        let block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = if buffer.is_empty() {
            Span::styled(
                "Press Ctrl+T to start recording...",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )
        } else {
            Span::raw(buffer.content())
        };

        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((buffer.scroll_offset() as u16, 0));

        frame.render_widget(paragraph, inner);
    }

    /// Renders the footer with help text.
    fn render_footer(frame: &mut Frame, area: Rect) {
        let help_spans = vec![
            Span::styled(" [Copy]", Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled("[Save]", Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled("[View Saved]", Style::default().fg(Color::Cyan)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::raw("Ctrl+T: Record "),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::raw(" Ctrl+S: Save "),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::raw(" Ctrl+L: Saved "),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::raw(" Esc: Exit"),
        ];

        let paragraph = Paragraph::new(Line::from(help_spans))
            .block(
                Block::default()
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                    .border_style(Style::default().fg(Color::Cyan)),
            );

        frame.render_widget(paragraph, area);
    }
}
