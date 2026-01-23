//! Audio Level Meter Widget
//!
//! Visual representation of audio input level using block characters.

#![allow(dead_code)] // Some methods reserved for future use

use ratatui::prelude::*;
use ratatui::widgets::Widget;

/// Audio level meter visualization.
pub struct LevelMeter {
    /// Current level (0.0 - 1.0).
    level: f32,
    /// Meter width in characters.
    width: u16,
    /// Label to show before the meter.
    label: Option<String>,
}

impl LevelMeter {
    /// Creates a new level meter with the given level.
    #[must_use]
    pub fn new(level: f32) -> Self {
        Self {
            level: level.clamp(0.0, 1.0),
            width: 20,
            label: None,
        }
    }

    /// Sets the meter width in characters.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Sets a label to display before the meter.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns the color for a position in the meter.
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

impl Widget for LevelMeter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let mut x_offset = area.x;

        // Render label if present
        if let Some(label) = &self.label {
            let label_len = label.len() as u16;
            if label_len < area.width {
                buf.set_string(
                    area.x,
                    area.y,
                    label,
                    Style::default().fg(Color::White),
                );
                x_offset = area.x + label_len + 1;
            }
        }

        let meter_width = self.width.min(area.width.saturating_sub(x_offset - area.x));
        let filled_full = (self.level * meter_width as f32) as u16;

        // Block characters for partial fill
        let blocks = ['░', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

        for x in 0..meter_width {
            let abs_x = x_offset + x;
            if abs_x >= area.x + area.width {
                break;
            }

            let (ch, style) = if x < filled_full {
                let color = Self::color_for_position(x, meter_width);
                ('█', Style::default().fg(color))
            } else if x == filled_full {
                // Partial fill for the current position
                let partial = ((self.level * meter_width as f32).fract() * 8.0) as usize;
                if partial > 0 {
                    let color = Self::color_for_position(x, meter_width);
                    (blocks[partial], Style::default().fg(color))
                } else {
                    ('░', Style::default().fg(Color::DarkGray))
                }
            } else {
                ('░', Style::default().fg(Color::DarkGray))
            };

            if let Some(cell) = buf.cell_mut(Position::new(abs_x, area.y)) {
                cell.set_char(ch).set_style(style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_meter_creation() {
        let meter = LevelMeter::new(0.5);
        assert_eq!(meter.level, 0.5);
        assert_eq!(meter.width, 20);
    }

    #[test]
    fn test_level_meter_clamping() {
        let meter = LevelMeter::new(1.5);
        assert_eq!(meter.level, 1.0);

        let meter = LevelMeter::new(-0.5);
        assert_eq!(meter.level, 0.0);
    }

    #[test]
    fn test_level_meter_builder() {
        let meter = LevelMeter::new(0.5)
            .width(30)
            .label("Audio:");

        assert_eq!(meter.width, 30);
        assert_eq!(meter.label, Some("Audio:".to_string()));
    }

    #[test]
    fn test_color_for_position() {
        assert_eq!(LevelMeter::color_for_position(5, 20), Color::Green);
        assert_eq!(LevelMeter::color_for_position(15, 20), Color::Yellow);
        assert_eq!(LevelMeter::color_for_position(19, 20), Color::Red);
    }
}
