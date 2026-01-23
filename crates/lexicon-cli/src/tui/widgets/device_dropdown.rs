//! Device Dropdown Widget
//!
//! A dropdown selector for audio input devices.
//!
//! This widget is currently scaffolded for future use. The device dropdown
//! functionality is currently implemented inline in app.rs.

#![allow(dead_code)] // Scaffolded for future use

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Widget};

use crate::tui::audio::AudioDeviceInfo;

/// A dropdown widget for selecting audio devices.
pub struct DeviceDropdown<'a> {
    /// Available devices.
    devices: &'a [AudioDeviceInfo],
    /// Currently selected index.
    selected: usize,
    /// Whether the dropdown is expanded.
    expanded: bool,
    /// Maximum visible items when expanded.
    max_visible: usize,
}

impl<'a> DeviceDropdown<'a> {
    /// Creates a new device dropdown.
    #[must_use]
    pub fn new(devices: &'a [AudioDeviceInfo], selected: usize) -> Self {
        Self {
            devices,
            selected,
            expanded: false,
            max_visible: 5,
        }
    }

    /// Sets whether the dropdown is expanded.
    #[must_use]
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Sets the maximum number of visible items when expanded.
    #[must_use]
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Renders the collapsed state (just the selected device).
    fn render_collapsed(&self, area: Rect, buf: &mut Buffer) {
        let device_name = self
            .devices
            .get(self.selected)
            .map(|d| d.name.as_str())
            .unwrap_or("No device");

        let display = format!("[{} ▼]", device_name);
        let style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        buf.set_string(area.x, area.y, &display, style);
    }

    /// Renders the expanded state (list of devices).
    fn render_expanded(&self, area: Rect, buf: &mut Buffer) {
        // Clear the area
        Clear.render(area, buf);

        let items: Vec<ListItem> = self
            .devices
            .iter()
            .enumerate()
            .map(|(i, device)| {
                let marker = if device.is_default { "● " } else { "  " };
                let text = format!("{}{}", marker, device.name);
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Select Device ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(self.selected));

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut state);
    }
}

impl Widget for DeviceDropdown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.expanded {
            self.render_expanded(area, buf);
        } else {
            self.render_collapsed(area, buf);
        }
    }
}

/// State for the device dropdown.
#[derive(Debug, Default)]
pub struct DeviceDropdownState {
    /// Current selection in dropdown (may differ from actual selected device).
    pub hover_index: usize,
    /// Whether the dropdown is currently open.
    pub is_open: bool,
}

impl DeviceDropdownState {
    /// Creates a new dropdown state.
    #[must_use]
    pub fn new(selected: usize) -> Self {
        Self {
            hover_index: selected,
            is_open: false,
        }
    }

    /// Opens the dropdown.
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Closes the dropdown.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Toggles the dropdown open/closed state.
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Moves selection up.
    pub fn up(&mut self, device_count: usize) {
        if device_count > 0 {
            if self.hover_index > 0 {
                self.hover_index -= 1;
            } else {
                self.hover_index = device_count - 1;
            }
        }
    }

    /// Moves selection down.
    pub fn down(&mut self, device_count: usize) {
        if device_count > 0 {
            if self.hover_index < device_count - 1 {
                self.hover_index += 1;
            } else {
                self.hover_index = 0;
            }
        }
    }

    /// Selects the current hover item.
    ///
    /// Returns the selected index.
    pub fn select(&mut self) -> usize {
        self.is_open = false;
        self.hover_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_devices() -> Vec<AudioDeviceInfo> {
        vec![
            AudioDeviceInfo {
                name: "Device 1".to_string(),
                id: "device1".to_string(),
                is_default: true,
                sample_rate: 48000,
                channels: 2,
            },
            AudioDeviceInfo {
                name: "Device 2".to_string(),
                id: "device2".to_string(),
                is_default: false,
                sample_rate: 44100,
                channels: 1,
            },
        ]
    }

    #[test]
    fn test_dropdown_creation() {
        let devices = make_devices();
        let dropdown = DeviceDropdown::new(&devices, 0);
        assert_eq!(dropdown.selected, 0);
        assert!(!dropdown.expanded);
    }

    #[test]
    fn test_dropdown_state_navigation() {
        let mut state = DeviceDropdownState::new(0);
        let device_count = 3;

        state.down(device_count);
        assert_eq!(state.hover_index, 1);

        state.down(device_count);
        assert_eq!(state.hover_index, 2);

        state.down(device_count);
        assert_eq!(state.hover_index, 0); // Wrap around

        state.up(device_count);
        assert_eq!(state.hover_index, 2); // Wrap around
    }

    #[test]
    fn test_dropdown_state_toggle() {
        let mut state = DeviceDropdownState::new(0);
        assert!(!state.is_open);

        state.toggle();
        assert!(state.is_open);

        state.toggle();
        assert!(!state.is_open);
    }

    #[test]
    fn test_dropdown_state_select() {
        let mut state = DeviceDropdownState::new(0);
        state.open();
        state.down(3);
        state.down(3);

        let selected = state.select();
        assert_eq!(selected, 2);
        assert!(!state.is_open);
    }
}
