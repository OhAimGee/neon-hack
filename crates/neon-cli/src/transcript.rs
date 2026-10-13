//! What a session said and was told, as the plain text of the lines.
//!
//! Both frontends keep one. A line is recorded as the player saw it when it was said (in the
//! language, the verbosity and the character set of that moment), without any colour, so the
//! transcript of a session is the same text whichever frontend played it. `export` writes it
//! down. It keeps the most recent [`CAPACITY`] lines: a session is never that long, and a
//! bound is what keeps a game left open for days from growing without end.

use std::collections::VecDeque;

/// Lines kept; the oldest go first.
const CAPACITY: usize = 50_000;

#[derive(Debug, Default)]
pub(crate) struct Transcript {
    lines: VecDeque<String>,
}

impl Transcript {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Adds lines at the end.
    pub(crate) fn record<I>(&mut self, lines: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        for line in lines {
            if self.lines.len() >= CAPACITY {
                self.lines.pop_front();
            }
            self.lines.push_back(line.into());
        }
    }

    /// How many lines are kept.
    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    /// The whole text: one line each, `\n` after every one of them.
    pub(crate) fn text(&self) -> String {
        let mut text = String::new();
        for line in &self.lines {
            text.push_str(line);
            text.push('\n');
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_come_out_in_order_with_a_newline_after_each() {
        let mut transcript = Transcript::new();
        transcript.record(["> status", "Handle: Neon"]);
        transcript.record(vec![String::new()]);
        assert_eq!(transcript.text(), "> status\nHandle: Neon\n\n");
        assert_eq!(transcript.len(), 3);
    }

    #[test]
    fn the_oldest_lines_go_first_when_it_is_full() {
        let mut transcript = Transcript::new();
        transcript.record((0..CAPACITY + 5).map(|n| n.to_string()));
        assert_eq!(transcript.len(), CAPACITY);
        assert!(transcript.text().starts_with("5\n"));
        assert!(transcript.text().ends_with(&format!("{}\n", CAPACITY + 4)));
    }

    #[test]
    fn an_empty_transcript_is_empty_text() {
        assert_eq!(Transcript::new().text(), "");
    }
}
