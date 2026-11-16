//! A tiny text buffer with a cursor, used by every editable field.
//!
//! Indices are in characters so multi-byte input behaves. Rendering splits the
//! buffer into three runs (before / selection / after) so the caret and the
//! selection highlight land exactly where the text does.

#[derive(Clone, Debug, Default)]
pub struct TextBuffer {
    text: String,
    /// Caret position in characters.
    cursor: usize,
    /// Selection anchor in characters; equal to `cursor` when there is none.
    anchor: usize,
}

impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.chars().count();
        Self {
            text,
            cursor,
            anchor: cursor,
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn cursor_chars(&self) -> usize {
        self.text.chars().count()
    }

    /// Caret position in characters.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.cursor_chars();
        self.anchor = self.cursor;
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.anchor = 0;
    }

    pub fn take(&mut self) -> String {
        let text = std::mem::take(&mut self.text);
        self.cursor = 0;
        self.anchor = 0;
        text
    }

    fn byte_index(&self, char_index: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_index)
            .map(|(index, _)| index)
            .unwrap_or(self.text.len())
    }

    pub fn selection(&self) -> (usize, usize) {
        if self.cursor < self.anchor {
            (self.cursor, self.anchor)
        } else {
            (self.anchor, self.cursor)
        }
    }

    pub fn has_selection(&self) -> bool {
        self.cursor != self.anchor
    }

    pub fn delete_selection(&mut self) -> bool {
        if !self.has_selection() {
            return false;
        }
        let (start, end) = self.selection();
        let start_byte = self.byte_index(start);
        let end_byte = self.byte_index(end);
        self.text.replace_range(start_byte..end_byte, "");
        self.cursor = start;
        self.anchor = start;
        true
    }

    pub fn insert(&mut self, ch: char) {
        self.delete_selection();
        let index = self.byte_index(self.cursor);
        self.text.insert(index, ch);
        self.cursor += 1;
        self.anchor = self.cursor;
    }

    pub fn insert_str(&mut self, value: &str) {
        self.delete_selection();
        let index = self.byte_index(self.cursor);
        self.text.insert_str(index, value);
        self.cursor += value.chars().count();
        self.anchor = self.cursor;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let index = self.byte_index(self.cursor - 1);
        self.text.remove(index);
        self.cursor -= 1;
        self.anchor = self.cursor;
    }

    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor >= self.cursor_chars() {
            return;
        }
        let index = self.byte_index(self.cursor);
        self.text.remove(index);
    }

    pub fn move_left(&mut self, extend: bool) {
        self.cursor = self.cursor.saturating_sub(1);
        if !extend {
            self.anchor = self.cursor;
        }
    }

    pub fn move_right(&mut self, extend: bool) {
        self.cursor = (self.cursor + 1).min(self.cursor_chars());
        if !extend {
            self.anchor = self.cursor;
        }
    }

    pub fn home(&mut self, extend: bool) {
        self.cursor = 0;
        if !extend {
            self.anchor = 0;
        }
    }

    pub fn end(&mut self, extend: bool) {
        self.cursor = self.cursor_chars();
        if !extend {
            self.anchor = self.cursor;
        }
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.cursor_chars();
    }

    /// Put the caret at a character index, clamped to the text. `extend`
    /// keeps the old anchor, which turns a mouse drag into a selection.
    pub fn set_cursor(&mut self, index: usize, extend: bool) {
        self.cursor = index.min(self.cursor_chars());
        if !extend {
            self.anchor = self.cursor;
        }
    }

    pub fn move_word_left(&mut self, extend: bool) {
        let chars: Vec<char> = self.text.chars().collect();
        let mut index = self.cursor;
        while index > 0 && chars[index - 1].is_whitespace() {
            index -= 1;
        }
        while index > 0 && !chars[index - 1].is_whitespace() {
            index -= 1;
        }
        self.cursor = index;
        if !extend {
            self.anchor = index;
        }
    }

    pub fn move_word_right(&mut self, extend: bool) {
        let chars: Vec<char> = self.text.chars().collect();
        let mut index = self.cursor;
        while index < chars.len() && !chars[index].is_whitespace() {
            index += 1;
        }
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        self.cursor = index;
        if !extend {
            self.anchor = index;
        }
    }

    pub fn selected_text(&self) -> String {
        let (start, end) = self.selection();
        self.text.chars().skip(start).take(end - start).collect()
    }

    /// Multi-line helper: insert a newline at the caret.
    pub fn newline(&mut self) {
        self.insert('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_cursor_clamps_and_can_extend_a_selection() {
        let mut buffer = TextBuffer::new("hello");
        buffer.set_cursor(2, false);
        assert_eq!(buffer.cursor(), 2);
        assert!(!buffer.has_selection());
        buffer.set_cursor(4, true);
        assert_eq!(buffer.selection(), (2, 4));
        buffer.set_cursor(99, false);
        assert_eq!(buffer.cursor(), 5);
        assert!(!buffer.has_selection());
    }
}
