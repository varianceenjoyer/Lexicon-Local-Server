//! Transcription Buffer
//!
//! Text buffer for live transcription with auto-flush capability.

/// Text buffer for live transcription with auto-flush.
pub struct TranscriptionBuffer {
    /// Current text content.
    text: String,
    /// Maximum buffer size before auto-flush.
    max_chars: usize,
    /// Count of auto-flushes that have occurred.
    flush_count: usize,
    /// Scroll offset for display (line offset).
    scroll_offset: usize,
}

impl TranscriptionBuffer {
    /// Creates a new buffer with the specified maximum size.
    #[must_use]
    pub fn new(max_chars: usize) -> Self {
        Self {
            text: String::new(),
            max_chars,
            flush_count: 0,
            scroll_offset: 0,
        }
    }

    /// Appends text to the buffer.
    ///
    /// Returns `true` if auto-flush occurred.
    pub fn append(&mut self, text: &str) -> bool {
        self.text.push_str(text);

        if self.text.chars().count() >= self.max_chars {
            self.flush_count += 1;
            // In a full implementation, we'd flush to database here
            // For now, just truncate the beginning
            let chars: Vec<char> = self.text.chars().collect();
            let keep_from = chars.len() / 2;
            self.text = chars[keep_from..].iter().collect();
            return true;
        }

        false
    }

    /// Returns the buffer content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.text
    }

    /// Returns the character count.
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    /// Returns the usage as (used, total).
    #[must_use]
    pub fn usage(&self) -> (usize, usize) {
        (self.char_count(), self.max_chars)
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.text.clear();
        self.scroll_offset = 0;
    }

    /// Returns the number of auto-flushes that have occurred.
    #[must_use]
    pub fn flush_count(&self) -> usize {
        self.flush_count
    }

    /// Returns the current scroll offset.
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scrolls up by the specified number of lines.
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scrolls down by the specified number of lines.
    pub fn scroll_down(&mut self, lines: usize, max_lines: usize) {
        let total_lines = self.line_count();
        if total_lines > max_lines {
            self.scroll_offset = (self.scroll_offset + lines).min(total_lines - max_lines);
        }
    }

    /// Returns the number of lines in the buffer.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.text.lines().count().max(1)
    }

    /// Returns lines for display, accounting for scroll offset.
    pub fn visible_lines(&self, max_lines: usize) -> Vec<&str> {
        self.text
            .lines()
            .skip(self.scroll_offset)
            .take(max_lines)
            .collect()
    }

    /// Returns whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = TranscriptionBuffer::new(100_000);
        assert!(buffer.is_empty());
        assert_eq!(buffer.char_count(), 0);
        assert_eq!(buffer.usage(), (0, 100_000));
    }

    #[test]
    fn test_buffer_append() {
        let mut buffer = TranscriptionBuffer::new(100);
        let flushed = buffer.append("Hello, world!");

        assert!(!flushed);
        assert_eq!(buffer.char_count(), 13);
        assert_eq!(buffer.content(), "Hello, world!");
    }

    #[test]
    fn test_buffer_auto_flush() {
        let mut buffer = TranscriptionBuffer::new(10);

        // Fill buffer beyond capacity
        let flushed = buffer.append("Hello, world!"); // 13 chars, exceeds 10

        assert!(flushed);
        assert_eq!(buffer.flush_count(), 1);
        // Buffer should have been truncated
        assert!(buffer.char_count() < 13);
    }

    #[test]
    fn test_buffer_clear() {
        let mut buffer = TranscriptionBuffer::new(100);
        buffer.append("Some text");
        buffer.clear();

        assert!(buffer.is_empty());
        assert_eq!(buffer.char_count(), 0);
    }

    #[test]
    fn test_buffer_scroll() {
        let mut buffer = TranscriptionBuffer::new(100);
        buffer.append("Line 1\nLine 2\nLine 3\nLine 4\nLine 5");

        assert_eq!(buffer.line_count(), 5);
        assert_eq!(buffer.scroll_offset(), 0);

        buffer.scroll_down(2, 3);
        assert_eq!(buffer.scroll_offset(), 2);

        buffer.scroll_up(1);
        assert_eq!(buffer.scroll_offset(), 1);
    }

    #[test]
    fn test_visible_lines() {
        let mut buffer = TranscriptionBuffer::new(100);
        buffer.append("Line 1\nLine 2\nLine 3\nLine 4\nLine 5");

        let lines = buffer.visible_lines(3);
        assert_eq!(lines, vec!["Line 1", "Line 2", "Line 3"]);

        buffer.scroll_down(2, 3);
        let lines = buffer.visible_lines(3);
        assert_eq!(lines, vec!["Line 3", "Line 4", "Line 5"]);
    }
}
