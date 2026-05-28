//! Single-line text buffer with cursor support.
//!
//! Stored as `Vec<char>` so the cursor can be expressed as a character
//! index in `[0, len()]`, independent of UTF-8 byte boundaries. The
//! buffer is single-line on purpose: questa's form fields and the
//! note/contact prompts never need multi-line input, and avoiding
//! `\n` keeps the rendering trivial.
//!
//! The vocabulary mirrors readline / Emacs: `home`/`end`, `backspace`,
//! `delete` (forward), `delete_word_back` (Ctrl-W), `clear` (Ctrl-U).

/// A semantic edit, decoupled from the keyboard. The terminal layer maps
/// key chords (Left, Ctrl-W, Backspace, …) to a `TextAction`; the
/// application layer applies it to whichever buffer is currently
/// focused. This split keeps `app.rs` and the tests free of crossterm
/// types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAction {
    Left,
    Right,
    Home,
    End,
    WordLeft,
    WordRight,
    Backspace,
    Delete,
    DeleteWordBack,
    Clear,
    Insert(char),
}

#[derive(Debug, Clone, Default)]
pub struct TextBuf {
    chars: Vec<char>,
    cursor: usize,
}

impl TextBuf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_string(&self) -> String {
        self.chars.iter().collect()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, c: char) {
        self.chars.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Remove the character immediately to the left of the cursor.
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// Remove the character at the cursor (forward delete).
    pub fn delete(&mut self) {
        if self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.chars.len();
    }

    /// Delete from the cursor back to the start of the previous word.
    /// Skips trailing whitespace, then deletes the run of non-whitespace.
    /// Matches readline's Ctrl-W behaviour.
    pub fn delete_word_back(&mut self) {
        while self.cursor > 0 && self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
        while self.cursor > 0 && !self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// Move the cursor one word to the left. Skips trailing whitespace
    /// then the run of non-whitespace before it.
    pub fn word_left(&mut self) {
        while self.cursor > 0 && self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
        }
        while self.cursor > 0 && !self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
        }
    }

    /// Move the cursor one word to the right.
    pub fn word_right(&mut self) {
        let n = self.chars.len();
        while self.cursor < n && !self.chars[self.cursor].is_whitespace() {
            self.cursor += 1;
        }
        while self.cursor < n && self.chars[self.cursor].is_whitespace() {
            self.cursor += 1;
        }
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    pub fn apply(&mut self, action: TextAction) {
        match action {
            TextAction::Left => self.left(),
            TextAction::Right => self.right(),
            TextAction::Home => self.home(),
            TextAction::End => self.end(),
            TextAction::WordLeft => self.word_left(),
            TextAction::WordRight => self.word_right(),
            TextAction::Backspace => self.backspace(),
            TextAction::Delete => self.delete(),
            TextAction::DeleteWordBack => self.delete_word_back(),
            TextAction::Clear => self.clear(),
            TextAction::Insert(c) => self.insert(c),
        }
    }
}

impl From<&str> for TextBuf {
    fn from(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let cursor = chars.len();
        Self { chars, cursor }
    }
}

impl From<String> for TextBuf {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> TextBuf {
        TextBuf::from(s)
    }

    #[test]
    fn from_str_places_cursor_at_end() {
        let b = buf("hello");
        assert_eq!(b.cursor(), 5);
        assert_eq!(b.as_string(), "hello");
    }

    #[test]
    fn insert_pushes_at_cursor() {
        let mut b = buf("hllo");
        b.cursor = 1;
        b.insert('e');
        assert_eq!(b.as_string(), "hello");
        assert_eq!(b.cursor(), 2);
    }

    #[test]
    fn backspace_and_delete_remove_adjacent_chars() {
        let mut b = buf("hello");
        b.cursor = 3;
        b.backspace();
        assert_eq!(b.as_string(), "helo");
        assert_eq!(b.cursor(), 2);
        b.delete();
        assert_eq!(b.as_string(), "heo");
        assert_eq!(b.cursor(), 2);
    }

    #[test]
    fn home_and_end_jump_extremes() {
        let mut b = buf("abc");
        b.cursor = 1;
        b.home();
        assert_eq!(b.cursor(), 0);
        b.end();
        assert_eq!(b.cursor(), 3);
    }

    #[test]
    fn left_right_clamp() {
        let mut b = buf("ab");
        b.home();
        b.left();
        assert_eq!(b.cursor(), 0);
        b.right();
        b.right();
        b.right();
        assert_eq!(b.cursor(), 2);
    }

    #[test]
    fn delete_word_back_strips_trailing_ws_then_word() {
        let mut b = buf("hello world  ");
        b.delete_word_back();
        assert_eq!(b.as_string(), "hello ");
        assert_eq!(b.cursor(), 6);
        b.delete_word_back();
        assert_eq!(b.as_string(), "");
    }

    #[test]
    fn word_left_and_right_jump_over_words() {
        let mut b = buf("hello world  foo");
        b.home();
        b.word_right();
        assert_eq!(b.cursor(), 6, "after first word + skipped space");
        b.word_right();
        assert_eq!(b.cursor(), 13);
        b.word_left();
        assert_eq!(b.cursor(), 6);
    }

    #[test]
    fn unicode_is_one_character() {
        let mut b = buf("héllo");
        assert_eq!(b.len(), 5);
        b.cursor = 1;
        b.delete();
        assert_eq!(b.as_string(), "hllo");
    }

    #[test]
    fn clear_resets() {
        let mut b = buf("abc");
        b.clear();
        assert!(b.is_empty());
        assert_eq!(b.cursor(), 0);
    }
}
