//! Save Dialog
//!
//! Dialog for saving/renaming transcriptions with text or voice input.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear};

/// Actions that can result from dialog interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAction {
    /// No action taken.
    None,
    /// Confirm with the given name.
    Confirm(String),
    /// Cancel the dialog.
    Cancel,
}

/// Save/rename dialog with text input.
pub struct SaveDialog {
    /// Current input text.
    input: String,
    /// Cursor position in the input.
    cursor: usize,
    /// Selection start position (for Ctrl+A select all).
    selection_start: Option<usize>,
    /// Whether the dialog is visible.
    visible: bool,
}

impl SaveDialog {
    /// Creates a new empty save dialog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            selection_start: None,
            visible: false,
        }
    }

    /// Creates a save dialog with a default name.
    #[must_use]
    pub fn with_default(name: &str) -> Self {
        Self {
            input: name.to_string(),
            cursor: name.len(),
            selection_start: None,
            visible: true,
        }
    }

    /// Returns whether the dialog is visible.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Opens the dialog with a default name.
    pub fn open_with_name(&mut self, name: &str) {
        self.input = name.to_string();
        self.cursor = name.len();
        self.selection_start = None;
        self.visible = true;
    }

    /// Closes the dialog.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Handles a key event.
    ///
    /// Returns the resulting action.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> DialogAction {
        match (code, modifiers) {
            (KeyCode::Enter, _) => {
                self.visible = false;
                DialogAction::Confirm(self.input.clone())
            }
            (KeyCode::Esc, _) => {
                self.visible = false;
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

                    // Convert to char indices
                    let chars: Vec<char> = self.input.chars().collect();
                    let new_input: String = chars[..from]
                        .iter()
                        .chain(chars[to..].iter())
                        .collect();
                    self.input = new_input;
                    self.cursor = from;
                }
                DialogAction::None
            }
            (KeyCode::Left, KeyModifiers::CONTROL) => {
                // Move word left
                self.cursor = self.find_word_boundary_left();
                self.selection_start = None;
                DialogAction::None
            }
            (KeyCode::Right, KeyModifiers::CONTROL) => {
                // Move word right
                self.cursor = self.find_word_boundary_right();
                self.selection_start = None;
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
                let char_count = self.input.chars().count();
                if self.cursor < char_count {
                    self.cursor += 1;
                }
                self.selection_start = None;
                DialogAction::None
            }
            (KeyCode::Home, _) => {
                self.cursor = 0;
                self.selection_start = None;
                DialogAction::None
            }
            (KeyCode::End, _) => {
                self.cursor = self.input.chars().count();
                self.selection_start = None;
                DialogAction::None
            }
            (KeyCode::Backspace, _) => {
                if let Some(start) = self.selection_start.take() {
                    // Delete selection
                    let (from, to) = if start < self.cursor {
                        (start, self.cursor)
                    } else {
                        (self.cursor, start)
                    };
                    let chars: Vec<char> = self.input.chars().collect();
                    let new_input: String = chars[..from]
                        .iter()
                        .chain(chars[to..].iter())
                        .collect();
                    self.input = new_input;
                    self.cursor = from;
                } else if self.cursor > 0 {
                    // Delete char before cursor
                    let chars: Vec<char> = self.input.chars().collect();
                    let new_input: String = chars[..self.cursor - 1]
                        .iter()
                        .chain(chars[self.cursor..].iter())
                        .collect();
                    self.input = new_input;
                    self.cursor -= 1;
                }
                DialogAction::None
            }
            (KeyCode::Delete, _) => {
                if let Some(start) = self.selection_start.take() {
                    // Delete selection
                    let (from, to) = if start < self.cursor {
                        (start, self.cursor)
                    } else {
                        (self.cursor, start)
                    };
                    let chars: Vec<char> = self.input.chars().collect();
                    let new_input: String = chars[..from]
                        .iter()
                        .chain(chars[to..].iter())
                        .collect();
                    self.input = new_input;
                    self.cursor = from;
                } else {
                    let char_count = self.input.chars().count();
                    if self.cursor < char_count {
                        // Delete char at cursor
                        let chars: Vec<char> = self.input.chars().collect();
                        let new_input: String = chars[..self.cursor]
                            .iter()
                            .chain(chars[self.cursor + 1..].iter())
                            .collect();
                        self.input = new_input;
                    }
                }
                DialogAction::None
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                // Clear selection and insert char
                self.selection_start = None;
                let chars: Vec<char> = self.input.chars().collect();
                let mut new_input: String = chars[..self.cursor].iter().collect();
                new_input.push(c);
                new_input.extend(chars[self.cursor..].iter());
                self.input = new_input;
                self.cursor += 1;
                DialogAction::None
            }
            _ => DialogAction::None,
        }
    }

    /// Appends text from voice input.
    pub fn append_voice(&mut self, text: &str) {
        self.input.push_str(text);
        self.cursor = self.input.chars().count();
        self.selection_start = None;
    }

    /// Returns the current input value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.input
    }

    /// Finds the word boundary to the left of the cursor.
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

    /// Finds the word boundary to the right of the cursor.
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
        let dialog_height = 5u16;
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Clear background
        Clear.render(dialog_area, buf);

        // Dialog box
        let block = Block::default()
            .title(" Save Transcript ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::DarkGray));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        // Prompt label
        if inner.height > 0 && inner.width > 6 {
            buf.set_string(
                inner.x,
                inner.y,
                "Name:",
                Style::default().fg(Color::White),
            );

            // Input text
            let input_x = inner.x + 6;
            let input_width = inner.width.saturating_sub(7);
            let display_input = if self.input.len() > input_width as usize {
                let start = self.input.len() - input_width as usize;
                &self.input[start..]
            } else {
                &self.input
            };

            buf.set_string(
                input_x,
                inner.y,
                display_input,
                Style::default().fg(Color::Yellow),
            );

            // Show cursor position indicator
            let cursor_x = input_x + self.cursor.min(input_width as usize) as u16;
            if cursor_x < inner.x + inner.width {
                if let Some(cell) = buf.cell_mut(Position::new(cursor_x, inner.y)) {
                    cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
                }
            }

            // Help text
            if inner.height > 2 {
                let help = "Enter: Save | Esc: Cancel | Ctrl+A: Select All";
                let help_y = inner.y + 2;
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

impl Default for SaveDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_creation() {
        let dialog = SaveDialog::new();
        assert!(dialog.value().is_empty());
        assert_eq!(dialog.cursor, 0);
    }

    #[test]
    fn test_dialog_with_default() {
        let dialog = SaveDialog::with_default("My Transcript");
        assert_eq!(dialog.value(), "My Transcript");
        assert_eq!(dialog.cursor, 13);
    }

    #[test]
    fn test_dialog_typing() {
        let mut dialog = SaveDialog::new();

        dialog.handle_key(KeyCode::Char('H'), KeyModifiers::SHIFT);
        dialog.handle_key(KeyCode::Char('i'), KeyModifiers::NONE);

        assert_eq!(dialog.value(), "Hi");
        assert_eq!(dialog.cursor, 2);
    }

    #[test]
    fn test_dialog_backspace() {
        let mut dialog = SaveDialog::with_default("Hello");

        dialog.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(dialog.value(), "Hell");
        assert_eq!(dialog.cursor, 4);
    }

    #[test]
    fn test_dialog_navigation() {
        let mut dialog = SaveDialog::with_default("Hello World");

        // Move to start
        dialog.handle_key(KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(dialog.cursor, 0);

        // Move to end
        dialog.handle_key(KeyCode::End, KeyModifiers::NONE);
        assert_eq!(dialog.cursor, 11);

        // Move left
        dialog.handle_key(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(dialog.cursor, 10);

        // Move right
        dialog.handle_key(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(dialog.cursor, 11);
    }

    #[test]
    fn test_dialog_select_all_and_delete() {
        let mut dialog = SaveDialog::with_default("Hello");

        // Select all
        dialog.handle_key(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(dialog.selection_start, Some(0));
        assert_eq!(dialog.cursor, 5);

        // Cut (delete selection)
        dialog.handle_key(KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(dialog.value().is_empty());
    }

    #[test]
    fn test_dialog_enter_confirm() {
        let mut dialog = SaveDialog::with_default("Test Name");

        let action = dialog.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(action, DialogAction::Confirm("Test Name".to_string()));
    }

    #[test]
    fn test_dialog_esc_cancel() {
        let mut dialog = SaveDialog::new();

        let action = dialog.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(action, DialogAction::Cancel);
    }

    #[test]
    fn test_dialog_voice_append() {
        let mut dialog = SaveDialog::with_default("Voice ");

        dialog.append_voice("memo");
        assert_eq!(dialog.value(), "Voice memo");
        assert_eq!(dialog.cursor, 10);
    }
}
