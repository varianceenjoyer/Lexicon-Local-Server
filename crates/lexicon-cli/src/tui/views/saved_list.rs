//! Saved Transcriptions List View
//!
//! Displays a scrollable list of saved transcriptions.

#![allow(dead_code)] // Some methods reserved for future use

use chrono::{DateTime, Utc};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use lexicon_core::storage::SavedTranscript as DbSavedTranscript;

/// Summary of a saved transcript.
#[derive(Debug, Clone)]
pub struct SavedTranscript {
    /// Database ID.
    pub id: i64,
    /// User-provided name.
    pub name: String,
    /// Preview text (first ~50 chars).
    pub preview: String,
    /// Total character count.
    pub char_count: usize,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// View for browsing saved transcriptions.
pub struct SavedTranscriptionsList {
    /// List of saved transcriptions.
    items: Vec<SavedTranscript>,
    /// List widget state.
    state: ListState,
    /// Scroll offset for virtual scrolling.
    scroll_offset: usize,
}

impl SavedTranscriptionsList {
    /// Creates a new empty list.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            state: ListState::default(),
            scroll_offset: 0,
        }
    }

    /// Creates a list with the given items.
    pub fn with_items(items: Vec<SavedTranscript>) -> Self {
        let mut list = Self {
            items,
            state: ListState::default(),
            scroll_offset: 0,
        };
        if !list.items.is_empty() {
            list.state.select(Some(0));
        }
        list
    }

    /// Sets the items in the list.
    pub fn set_items(&mut self, items: Vec<SavedTranscript>) {
        self.items = items;
        if self.items.is_empty() {
            self.state.select(None);
        } else {
            // Keep selection valid
            let selected = self.state.selected().unwrap_or(0);
            self.state.select(Some(selected.min(self.items.len() - 1)));
        }
    }

    /// Returns the items.
    pub fn items(&self) -> &[SavedTranscript] {
        &self.items
    }

    /// Returns whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Selects the next item.
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Selects the previous item.
    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.items.len().saturating_sub(1),
        };
        self.state.select(Some(i));
    }

    /// Returns the selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Returns the selected transcript ID.
    pub fn selected_id(&self) -> Option<i64> {
        self.state.selected().and_then(|i| self.items.get(i).map(|t| t.id))
    }

    /// Returns the selected transcript.
    pub fn selected(&self) -> Option<&SavedTranscript> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    /// Removes the selected item and returns its ID.
    pub fn remove_selected(&mut self) -> Option<i64> {
        if let Some(i) = self.state.selected() {
            if i < self.items.len() {
                let transcript = self.items.remove(i);
                // Adjust selection
                if self.items.is_empty() {
                    self.state.select(None);
                } else if i >= self.items.len() {
                    self.state.select(Some(self.items.len() - 1));
                }
                return Some(transcript.id);
            }
        }
        None
    }

    /// Renders the list in the given area.
    pub fn render(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let block = Block::default()
            .title(" Saved Transcriptions ")
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let _inner = block.inner(area);

        if self.items.is_empty() {
            // Show empty state
            let text = Paragraph::new("No saved transcriptions.\n\nPress Esc to return.")
                .style(Style::default().fg(Color::Gray))
                .block(block);
            frame.render_widget(text, area);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = self.items
            .iter()
            .enumerate()
            .map(|(i, transcript)| {
                let is_selected = self.state.selected() == Some(i);

                // Format: Name | Chars | Date
                let date_str = transcript.created_at.format("%b %d, %H:%M").to_string();
                let preview = if transcript.preview.len() > 40 {
                    format!("{}...", &transcript.preview[..37])
                } else {
                    transcript.preview.clone()
                };

                let line1 = Line::from(vec![
                    Span::styled(
                        format!("{:<30}", transcript.name),
                        if is_selected {
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        },
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        format!("{:>8} chars", transcript.char_count),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::raw(" | "),
                    Span::styled(date_str, Style::default().fg(Color::DarkGray)),
                ]);

                let line2 = Line::from(vec![
                    Span::styled(
                        format!("  \"{}\"", preview),
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                    ),
                ]);

                ListItem::new(vec![line1, line2])
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.state);

        // Render help at bottom
        let help_y = area.y + area.height.saturating_sub(1);
        if help_y < frame.area().height {
            let help_area = Rect::new(area.x + 1, help_y, area.width.saturating_sub(2), 1);
            let help = Paragraph::new(" [Enter: Open] [Ctrl+E: Export] [Ctrl+D: Delete] [Esc: Back] ")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(help, help_area);
        }
    }
}

impl Default for SavedTranscriptionsList {
    fn default() -> Self {
        Self::new()
    }
}

impl From<DbSavedTranscript> for SavedTranscript {
    fn from(db: DbSavedTranscript) -> Self {
        // Parse the ISO 8601 timestamp from the database
        let created_at = chrono::DateTime::parse_from_rfc3339(&db.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        // Generate preview before moving name
        let preview = db.preview();
        let char_count = db.char_count as usize;

        Self {
            id: db.id.0,
            name: db.name,
            preview,
            char_count,
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_transcript(id: i64, name: &str) -> SavedTranscript {
        SavedTranscript {
            id,
            name: name.to_string(),
            preview: "This is a test preview...".to_string(),
            char_count: 1000,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_list_creation() {
        let list = SavedTranscriptionsList::new();
        assert!(list.is_empty());
        assert_eq!(list.selected_index(), None);
    }

    #[test]
    fn test_list_with_items() {
        let items = vec![
            make_test_transcript(1, "Test 1"),
            make_test_transcript(2, "Test 2"),
        ];
        let list = SavedTranscriptionsList::with_items(items);
        assert_eq!(list.len(), 2);
        assert_eq!(list.selected_index(), Some(0));
    }

    #[test]
    fn test_list_navigation() {
        let items = vec![
            make_test_transcript(1, "Test 1"),
            make_test_transcript(2, "Test 2"),
            make_test_transcript(3, "Test 3"),
        ];
        let mut list = SavedTranscriptionsList::with_items(items);

        assert_eq!(list.selected_index(), Some(0));

        list.select_next();
        assert_eq!(list.selected_index(), Some(1));

        list.select_next();
        assert_eq!(list.selected_index(), Some(2));

        list.select_next(); // Wrap around
        assert_eq!(list.selected_index(), Some(0));

        list.select_prev(); // Wrap around back
        assert_eq!(list.selected_index(), Some(2));
    }

    #[test]
    fn test_remove_selected() {
        let items = vec![
            make_test_transcript(1, "Test 1"),
            make_test_transcript(2, "Test 2"),
        ];
        let mut list = SavedTranscriptionsList::with_items(items);

        let removed = list.remove_selected();
        assert_eq!(removed, Some(1));
        assert_eq!(list.len(), 1);
        assert_eq!(list.selected_id(), Some(2));
    }
}
