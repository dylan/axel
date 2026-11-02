//! A single-line text editor driven by [`KeyEvent`]s. Pure data: no rendering.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextInput {
    chars: Vec<char>,
    /// Cursor position in characters, `0..=chars.len()`.
    cursor: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_text(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let cursor = chars.len();
        Self { chars, cursor }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_text(&mut self, text: &str) {
        self.chars = text.chars().collect();
        self.cursor = self.chars.len();
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    pub fn insert_char(&mut self, ch: char) {
        if ch == '\n' || ch == '\r' {
            return;
        }
        self.chars.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Insert pasted text; newlines are collapsed to spaces.
    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            match ch {
                '\n' | '\r' | '\t' => self.insert_char(' '),
                c if c.is_control() => {}
                c => self.insert_char(c),
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

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

    /// Ctrl+U: delete everything before the cursor.
    pub fn kill_to_start(&mut self) {
        self.chars.drain(..self.cursor);
        self.cursor = 0;
    }

    /// Ctrl+K: delete everything after the cursor.
    pub fn kill_to_end(&mut self) {
        self.chars.truncate(self.cursor);
    }

    /// Ctrl+W: delete the word before the cursor (plus trailing whitespace).
    pub fn delete_word(&mut self) {
        let mut start = self.cursor;
        while start > 0 && self.chars[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !self.chars[start - 1].is_whitespace() {
            start -= 1;
        }
        self.chars.drain(start..self.cursor);
        self.cursor = start;
    }

    /// Returns `true` when the key was consumed as an editing command.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Char(c) if ctrl => match c {
                'u' => self.kill_to_start(),
                'k' => self.kill_to_end(),
                'w' => self.delete_word(),
                'a' => self.home(),
                'e' => self.end(),
                'b' => self.left(),
                'f' => self.right(),
                'h' => self.backspace(),
                'd' => self.delete(),
                _ => return false,
            },
            KeyCode::Char(_) if alt => return false,
            KeyCode::Char(c) => self.insert_char(c),
            KeyCode::Backspace => {
                if alt { self.delete_word() } else { self.backspace() }
            }
            KeyCode::Delete => self.delete(),
            KeyCode::Left => {
                if alt { self.word_left() } else { self.left() }
            }
            KeyCode::Right => {
                if alt { self.word_right() } else { self.right() }
            }
            KeyCode::Home => self.home(),
            KeyCode::End => self.end(),
            _ => return false,
        }
        true
    }

    fn word_left(&mut self) {
        while self.cursor > 0 && self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
        }
        while self.cursor > 0 && !self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
        }
    }

    fn word_right(&mut self) {
        let n = self.chars.len();
        while self.cursor < n && !self.chars[self.cursor].is_whitespace() {
            self.cursor += 1;
        }
        while self.cursor < n && self.chars[self.cursor].is_whitespace() {
            self.cursor += 1;
        }
    }

    /// Visible window of the text for a field `width` columns wide, and the
    /// cursor column relative to that window. Keeps the cursor in view.
    pub fn view(&self, width: usize) -> (String, u16) {
        if width == 0 {
            return (String::new(), 0);
        }
        let inner = width.saturating_sub(1).max(1); // leave one cell for the cursor at the end
        let start = self.cursor.saturating_sub(inner);
        let shown: String = self.chars.iter().skip(start).take(width).collect();
        let col = (self.cursor - start).min(width.saturating_sub(1)) as u16;
        (shown, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn typing_and_backspace() {
        let mut t = TextInput::new();
        for c in "hello".chars() {
            assert!(t.handle_key(key(KeyCode::Char(c))));
        }
        assert_eq!(t.text(), "hello");
        t.handle_key(key(KeyCode::Backspace));
        assert_eq!(t.text(), "hell");
        assert_eq!(t.cursor(), 4);
    }

    #[test]
    fn cursor_movement_and_insert_in_middle() {
        let mut t = TextInput::with_text("ac");
        t.handle_key(key(KeyCode::Left));
        t.handle_key(key(KeyCode::Char('b')));
        assert_eq!(t.text(), "abc");
        t.handle_key(key(KeyCode::Home));
        assert_eq!(t.cursor(), 0);
        t.handle_key(key(KeyCode::End));
        assert_eq!(t.cursor(), 3);
        t.handle_key(key(KeyCode::Right)); // no-op at end
        assert_eq!(t.cursor(), 3);
    }

    #[test]
    fn ctrl_u_and_ctrl_w() {
        let mut t = TextInput::with_text("fix the login bug");
        t.handle_key(ctrl('w'));
        assert_eq!(t.text(), "fix the login ");
        t.handle_key(ctrl('w'));
        assert_eq!(t.text(), "fix the ");
        t.handle_key(ctrl('u'));
        assert_eq!(t.text(), "");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn ctrl_u_only_kills_before_cursor() {
        let mut t = TextInput::with_text("abcdef");
        t.left();
        t.left();
        t.handle_key(ctrl('u'));
        assert_eq!(t.text(), "ef");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn paste_collapses_newlines() {
        let mut t = TextInput::new();
        t.insert_str("line one\nline two\t!");
        assert_eq!(t.text(), "line one line two !");
    }

    #[test]
    fn view_scrolls_to_cursor() {
        let t = TextInput::with_text("abcdefghij");
        let (shown, col) = t.view(5);
        assert_eq!(shown, "ghij");
        assert_eq!(col, 4);
        let (shown, col) = t.view(20);
        assert_eq!(shown, "abcdefghij");
        assert_eq!(col, 10);
    }
}
