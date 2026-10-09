//! The input line: a one-line editor with history.
//!
//! Pure: it knows nothing of the terminal, the keys or the game, so every behaviour is
//! tested one call at a time. The cursor moves by grapheme cluster, never inside one (a
//! letter and its accent, a flag, a family are one step), and its column is measured in
//! display width. History is the editor's own and bounded; what goes into it is the
//! caller's choice, so that menu answers and typed secrets are never recorded.

use unicode_segmentation::UnicodeSegmentation;
#[cfg(test)]
use unicode_width::UnicodeWidthStr;

/// Longest line kept in the editor, in characters, whatever a paste or a game asks.
pub(crate) const MAX_CHARS: usize = 1024;
/// How many lines of history are kept.
pub(crate) const HISTORY_MAX: usize = 100;

/// A line being typed.
#[derive(Debug, Clone, Default)]
pub(crate) struct LineEditor {
    text: String,
    /// Byte offset of the cursor: always on a grapheme boundary, never past the end.
    cursor: usize,
    /// Longest text accepted, in characters.
    limit: usize,
    /// Oldest first.
    history: Vec<String>,
    /// Which history line is shown while browsing, from the newest (0) back.
    browsing: Option<usize>,
    /// What was typed before browsing started, given back by going down past the newest.
    draft: String,
}

impl LineEditor {
    pub(crate) fn new() -> Self {
        Self {
            limit: MAX_CHARS,
            ..Self::default()
        }
    }

    /// The text of the line.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The text before the cursor.
    pub(crate) fn before_cursor(&self) -> &str {
        self.text.get(..self.cursor).unwrap_or_default()
    }

    /// Display columns between the start of the line and the cursor.
    #[cfg(test)]
    pub(crate) fn cursor_column(&self) -> usize {
        self.text
            .get(..self.cursor)
            .map_or(0, UnicodeWidthStr::width)
    }

    /// Byte offset of the cursor (always at a grapheme boundary).
    #[cfg(test)]
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    /// The history, oldest first.
    #[cfg(test)]
    pub(crate) fn history(&self) -> &[String] {
        &self.history
    }

    /// Limits the line to `max_chars` characters (a prompt that asks for a short answer).
    pub(crate) fn set_limit(&mut self, max_chars: usize) {
        self.limit = max_chars.clamp(1, MAX_CHARS);
        self.truncate_to_limit();
    }

    fn truncate_to_limit(&mut self) {
        if let Some((end, _)) = self.text.char_indices().nth(self.limit) {
            self.text.truncate(end);
            self.cursor = self.snap(self.cursor.min(self.text.len()));
        }
    }

    // ---- Inserting ---------------------------------------------------------------------

    /// Types one character at the cursor.
    pub(crate) fn insert(&mut self, c: char) {
        if c.is_control() || self.text.chars().count() >= self.limit {
            return;
        }
        self.leave_history();
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        // A combining mark typed before another one can fuse with the next cluster.
        self.cursor = self.snap(self.cursor);
    }

    /// Pastes text. It never submits anything: each line break becomes one space (a trailing
    /// one is dropped) and other control characters are dropped, so a pasted block of
    /// commands is one line the player must still confirm.
    pub(crate) fn paste(&mut self, pasted: &str) {
        let normalized = pasted.replace("\r\n", "\n").replace('\r', "\n");
        let mut lines: Vec<&str> = normalized.split('\n').collect();
        if lines.len() > 1 && lines.last() == Some(&"") {
            lines.pop();
        }
        for c in lines.join(" ").chars() {
            self.insert(c);
        }
    }

    // ---- Moving ------------------------------------------------------------------------

    /// Closest grapheme boundary at or after `offset`.
    fn snap(&self, offset: usize) -> usize {
        let offset = offset.min(self.text.len());
        self.text
            .grapheme_indices(true)
            .map(|(start, _)| start)
            .chain(std::iter::once(self.text.len()))
            .find(|start| *start >= offset)
            .unwrap_or(self.text.len())
    }

    fn previous_boundary(&self) -> usize {
        self.text
            .get(..self.cursor)
            .and_then(|before| before.grapheme_indices(true).next_back())
            .map_or(0, |(start, _)| start)
    }

    fn next_boundary(&self) -> usize {
        self.text
            .get(self.cursor..)
            .and_then(|after| after.graphemes(true).next())
            .map_or(self.cursor, |cluster| self.cursor + cluster.len())
    }

    pub(crate) fn left(&mut self) {
        self.cursor = self.previous_boundary();
    }

    pub(crate) fn right(&mut self) {
        self.cursor = self.next_boundary();
    }

    pub(crate) fn home(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn end(&mut self) {
        self.cursor = self.text.len();
    }

    /// Start of the word before the cursor (spaces first, then the word).
    fn word_start(&self) -> usize {
        let before = self.text.get(..self.cursor).unwrap_or_default();
        let mut start = self.cursor;
        let mut in_word = false;
        for (index, cluster) in before.grapheme_indices(true).rev() {
            let is_space = cluster.chars().all(char::is_whitespace);
            if in_word && is_space {
                break;
            }
            in_word |= !is_space;
            start = index;
        }
        start
    }

    /// End of the word after the cursor (spaces first, then the word).
    fn word_end(&self) -> usize {
        let after = self.text.get(self.cursor..).unwrap_or_default();
        let mut end = self.cursor;
        let mut in_word = false;
        for (index, cluster) in after.grapheme_indices(true) {
            let is_space = cluster.chars().all(char::is_whitespace);
            if in_word && is_space {
                break;
            }
            in_word |= !is_space;
            end = self.cursor + index + cluster.len();
        }
        end
    }

    pub(crate) fn word_left(&mut self) {
        self.cursor = self.word_start();
    }

    pub(crate) fn word_right(&mut self) {
        self.cursor = self.word_end();
    }

    // ---- Deleting ----------------------------------------------------------------------

    fn remove(&mut self, from: usize, to: usize) {
        if from < to && to <= self.text.len() {
            self.leave_history();
            self.text.replace_range(from..to, "");
            self.cursor = self.snap(from);
        }
    }

    /// Backspace: the grapheme before the cursor.
    pub(crate) fn backspace(&mut self) {
        self.remove(self.previous_boundary(), self.cursor);
    }

    /// Delete: the grapheme under the cursor.
    pub(crate) fn delete(&mut self) {
        self.remove(self.cursor, self.next_boundary());
    }

    /// Ctrl-U: everything before the cursor.
    pub(crate) fn kill_to_start(&mut self) {
        self.remove(0, self.cursor);
    }

    /// Ctrl-K: everything from the cursor on.
    pub(crate) fn kill_to_end(&mut self) {
        self.remove(self.cursor, self.text.len());
    }

    /// Ctrl-W: the word before the cursor.
    pub(crate) fn kill_word(&mut self) {
        self.remove(self.word_start(), self.cursor);
    }

    // ---- Whole line --------------------------------------------------------------------

    /// Replaces the line, the cursor at its end.
    #[cfg(test)]
    pub(crate) fn set(&mut self, text: &str) {
        self.set_at(text, usize::MAX);
    }

    /// Replaces the line (a completion), the cursor at byte `cursor` or the nearest grapheme
    /// boundary after it.
    pub(crate) fn set_at(&mut self, text: &str, cursor: usize) {
        self.leave_history();
        self.text = text.chars().filter(|c| !c.is_control()).collect();
        self.truncate_to_limit();
        self.cursor = self.snap(cursor);
    }

    /// Gives the line and starts a new one.
    pub(crate) fn take(&mut self) -> String {
        self.cursor = 0;
        self.browsing = None;
        self.draft.clear();
        std::mem::take(&mut self.text)
    }

    /// Throws the line away.
    pub(crate) fn clear(&mut self) {
        drop(self.take());
    }

    // ---- History -----------------------------------------------------------------------

    /// Remembers a line: empty lines and a repeat of the newest line are not kept, and the
    /// oldest line goes when the history is full.
    pub(crate) fn remember(&mut self, line: &str) {
        let line: String = line.chars().filter(|c| !c.is_control()).collect();
        let line = line.trim();
        if line.is_empty() || self.history.last().is_some_and(|last| last == line) {
            return;
        }
        if self.history.len() >= HISTORY_MAX {
            self.history.remove(0);
        }
        self.history.push(line.to_owned());
    }

    /// Typing while browsing makes the shown line the line being edited.
    fn leave_history(&mut self) {
        self.browsing = None;
    }

    /// Up: the previous line of the history. The line being typed is kept as a draft.
    pub(crate) fn history_previous(&mut self) {
        let next = match self.browsing {
            None if self.history.is_empty() => return,
            None => {
                self.draft = self.text.clone();
                0
            }
            Some(depth) => (depth + 1).min(self.history.len().saturating_sub(1)),
        };
        self.show_history(next);
    }

    /// Down: the next line of the history, then the draft.
    pub(crate) fn history_next(&mut self) {
        match self.browsing {
            None => {}
            Some(0) => {
                self.browsing = None;
                let draft = std::mem::take(&mut self.draft);
                self.text = draft;
                self.truncate_to_limit();
                self.cursor = self.text.len();
            }
            Some(depth) => self.show_history(depth - 1),
        }
    }

    fn show_history(&mut self, depth: usize) {
        let index = self.history.len().checked_sub(depth + 1);
        if let Some(line) = index.and_then(|index| self.history.get(index)) {
            self.text.clone_from(line);
            self.truncate_to_limit();
            self.cursor = self.text.len();
            self.browsing = Some(depth);
        }
    }
}

#[cfg(test)]
mod tests;
