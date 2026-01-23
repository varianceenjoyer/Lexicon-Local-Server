//! Confirmation Dialog
//!
//! Dialog for yes/no/cancel confirmations.

#![allow(dead_code)] // Some methods reserved for future use

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear};

/// Actions that can result from confirmation dialog interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    /// No action taken.
    None,
    /// User selected Yes.
    Yes,
    /// User selected No.
    No,
    /// User cancelled the dialog.
    Cancel,
}

/// Confirmation dialog with Yes/No/Cancel options.
pub struct ConfirmDialog {
    /// Dialog title/message.
    message: String,
    /// Currently selected option (0=Yes, 1=No).
    selected: usize,
    /// Whether the dialog is visible.
    visible: bool,
}

impl ConfirmDialog {
    /// Creates a new confirmation dialog (not visible by default).
    #[must_use]
    pub fn new() -> Self {
        Self {
            message: String::new(),
            selected: 0,
            visible: false,
        }
    }

    /// Returns whether the dialog is visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Opens the dialog with a message.
    pub fn open(&mut self, message: &str) {
        self.message = message.to_string();
        self.selected = 0; // Default to Yes
        self.visible = true;
    }

    /// Closes the dialog.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Handles a key event.
    ///
    /// Returns the resulting action.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> ConfirmAction {
        match (code, modifiers) {
            // Enter: Confirm selected option
            (KeyCode::Enter, _) => {
                self.visible = false;
                if self.selected == 0 {
                    ConfirmAction::Yes
                } else {
                    ConfirmAction::No
                }
            }
            // Escape or Ctrl+N: Cancel
            (KeyCode::Esc, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.visible = false;
                ConfirmAction::Cancel
            }
            // Y key: Yes
            (KeyCode::Char('y'), KeyModifiers::NONE) | (KeyCode::Char('Y'), KeyModifiers::SHIFT) => {
                self.visible = false;
                ConfirmAction::Yes
            }
            // N key (without Ctrl): No
            (KeyCode::Char('n'), KeyModifiers::NONE) | (KeyCode::Char('N'), KeyModifiers::SHIFT) => {
                self.visible = false;
                ConfirmAction::No
            }
            // Left/Right: Toggle selection
            (KeyCode::Left, _) | (KeyCode::Right, _) | (KeyCode::Tab, _) => {
                self.selected = 1 - self.selected;
                ConfirmAction::None
            }
            _ => ConfirmAction::None,
        }
    }

    /// Renders the dialog to a frame.
    pub fn render(&self, area: Rect, frame: &mut ratatui::Frame) {
        if !self.visible {
            return;
        }
        self.render_to_buffer(area, frame.buffer_mut());
    }

    /// Renders the dialog to a buffer.
    pub fn render_to_buffer(&self, area: Rect, buf: &mut Buffer) {
        // Calculate dialog size and position (centered)
        let dialog_width = 50u16.min(area.width.saturating_sub(4));
        let dialog_height = 6u16;
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Clear background
        Clear.render(dialog_area, buf);

        // Dialog box
        let block = Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        // Message
        if inner.height > 0 && inner.width > 0 {
            // Truncate message if needed
            let display_msg = if self.message.len() > inner.width as usize {
                format!("{}...", &self.message[..inner.width as usize - 3])
            } else {
                self.message.clone()
            };

            buf.set_string(
                inner.x,
                inner.y,
                &display_msg,
                Style::default().fg(Color::White),
            );

            // Buttons
            if inner.height > 2 {
                let button_y = inner.y + 2;
                let yes_style = if self.selected == 0 {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let no_style = if self.selected == 1 {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                };

                buf.set_string(inner.x + 8, button_y, " [ Yes ] ", yes_style);
                buf.set_string(inner.x + 20, button_y, " [ No ] ", no_style);
            }

            // Help text
            if inner.height > 3 {
                let help = "Y/N: Select | Enter: Confirm | Esc: Cancel";
                let help_y = inner.y + 4;
                buf.set_string(
                    inner.x,
                    help_y,
                    help,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }
    }
}

impl Default for ConfirmDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_dialog_creation() {
        let dialog = ConfirmDialog::new();
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_confirm_dialog_open() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Save changes?");
        assert!(dialog.is_visible());
        assert_eq!(dialog.selected, 0);
    }

    #[test]
    fn test_confirm_dialog_yes() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Test?");

        let action = dialog.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(action, ConfirmAction::Yes);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_confirm_dialog_no() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Test?");

        let action = dialog.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(action, ConfirmAction::No);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_confirm_dialog_cancel_esc() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Test?");

        let action = dialog.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(action, ConfirmAction::Cancel);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_confirm_dialog_cancel_ctrl_n() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Test?");

        let action = dialog.handle_key(KeyCode::Char('n'), KeyModifiers::CONTROL);
        assert_eq!(action, ConfirmAction::Cancel);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_confirm_dialog_toggle() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Test?");
        assert_eq!(dialog.selected, 0);

        dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(dialog.selected, 1);

        dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(dialog.selected, 0);
    }

    #[test]
    fn test_confirm_dialog_enter_yes() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Test?");

        let action = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(action, ConfirmAction::Yes);
    }

    #[test]
    fn test_confirm_dialog_enter_no() {
        let mut dialog = ConfirmDialog::new();
        dialog.open("Test?");
        dialog.handle_key(KeyCode::Tab, KeyModifiers::NONE); // Select No

        let action = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(action, ConfirmAction::No);
    }
}
