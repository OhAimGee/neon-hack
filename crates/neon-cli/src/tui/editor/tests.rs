use proptest::prelude::*;

use super::*;

fn typed(text: &str) -> LineEditor {
    let mut editor = LineEditor::new();
    for c in text.chars() {
        editor.insert(c);
    }
    editor
}

#[test]
fn typing_inserts_at_the_cursor() {
    let mut editor = typed("hck 1");
    editor.home();
    editor.right();
    editor.insert('a');
    assert_eq!(editor.text(), "hack 1");
    assert_eq!(editor.cursor(), 2);
    editor.end();
    editor.insert('0');
    assert_eq!(editor.text(), "hack 10");
}

#[test]
fn the_cursor_moves_and_stops_at_both_ends() {
    let mut editor = typed("ab");
    editor.right();
    assert_eq!(editor.cursor(), 2);
    editor.left();
    editor.left();
    editor.left();
    assert_eq!(editor.cursor(), 0);
    editor.end();
    assert_eq!(editor.cursor(), 2);
    editor.home();
    assert_eq!(editor.cursor(), 0);
}

#[test]
fn a_cluster_of_several_characters_is_one_step() {
    // e + combining acute, a flag (two regional indicators), a wide character.
    let mut editor = typed("e\u{301}\u{1F1EB}\u{1F1F7}日");
    assert_eq!(editor.cursor_column(), 1 + 2 + 2);
    editor.left();
    assert_eq!(editor.cursor_column(), 3, "past the wide character");
    editor.left();
    assert_eq!(editor.cursor_column(), 1, "past the whole flag");
    editor.left();
    assert_eq!(editor.cursor_column(), 0, "past the letter and its accent");
    editor.right();
    editor.right();
    editor.backspace();
    assert_eq!(editor.text(), "e\u{301}日", "the flag goes whole");
    editor.delete();
    assert_eq!(editor.text(), "e\u{301}");
    editor.backspace();
    assert_eq!(editor.text(), "");
}

#[test]
fn a_mark_typed_before_a_letter_does_not_leave_the_cursor_inside_a_cluster() {
    let mut editor = typed("a");
    editor.home();
    editor.insert('e');
    editor.insert('\u{301}');
    assert!(editor.text().is_char_boundary(editor.cursor()));
    let boundaries: Vec<usize> = editor
        .text()
        .grapheme_indices(true)
        .map(|(i, _)| i)
        .collect();
    assert!(boundaries.contains(&editor.cursor()) || editor.cursor() == editor.text().len());
}

#[test]
fn control_keys_edit_the_line() {
    let mut editor = typed("hack corp gateway");
    editor.kill_word();
    assert_eq!(editor.text(), "hack corp ");
    editor.kill_word();
    assert_eq!(editor.text(), "hack ");
    editor.home();
    editor.right();
    editor.kill_to_end();
    assert_eq!(editor.text(), "h");
    editor.insert('x');
    editor.kill_to_start();
    assert_eq!(editor.text(), "");
    editor.kill_to_start();
    editor.kill_word();
    editor.backspace();
    editor.delete();
    assert_eq!(editor.text(), "", "nothing to remove is not an error");
}

#[test]
fn words_are_skipped_by_the_word_keys() {
    let mut editor = typed("  one  two ");
    editor.home();
    editor.word_right();
    assert_eq!(editor.cursor(), 5, "end of the first word");
    editor.word_right();
    assert_eq!(editor.cursor(), 10);
    editor.word_left();
    assert_eq!(editor.cursor(), 7, "start of the second word");
    editor.word_left();
    assert_eq!(editor.cursor(), 2);
}

#[test]
fn control_characters_are_never_typed() {
    let mut editor = LineEditor::new();
    for c in ['\u{1b}', '\n', '\t', '\u{7f}', 'a'] {
        editor.insert(c);
    }
    assert_eq!(editor.text(), "a");
}

#[test]
fn a_paste_is_one_line_and_never_submits() {
    let mut editor = LineEditor::new();
    editor.paste("scan\nhack 1\r\nquit\n");
    assert_eq!(editor.text(), "scan hack 1 quit");
    editor.clear();
    editor.paste("one\rtwo\u{1b}[31m\t\n");
    assert_eq!(editor.text(), "one two[31m");
    editor.clear();
    editor.paste("\n\n");
    assert_eq!(
        editor.text(),
        " ",
        "a paste of line breaks is still harmless"
    );
}

#[test]
fn the_line_is_bounded_whatever_is_pasted() {
    let mut editor = LineEditor::new();
    editor.paste(&"x".repeat(5000));
    assert_eq!(editor.text().chars().count(), MAX_CHARS);
    let mut short = LineEditor::new();
    short.set_limit(4);
    short.paste("handle");
    assert_eq!(short.text(), "hand");
    short.set_limit(2);
    assert_eq!(short.text(), "ha");
    assert!(short.cursor() <= short.text().len());
}

#[test]
fn history_goes_back_and_forth_and_keeps_the_draft() {
    let mut editor = LineEditor::new();
    for line in ["scan", "net", "hack 1"] {
        editor.remember(line);
    }
    editor.paste("sta");
    editor.history_previous();
    assert_eq!(editor.text(), "hack 1");
    editor.history_previous();
    editor.history_previous();
    assert_eq!(editor.text(), "scan");
    editor.history_previous();
    assert_eq!(editor.text(), "scan", "the oldest line stays shown");
    editor.history_next();
    assert_eq!(editor.text(), "net");
    editor.history_next();
    editor.history_next();
    assert_eq!(editor.text(), "sta", "the draft comes back");
    editor.history_next();
    assert_eq!(editor.text(), "sta");
    assert_eq!(
        editor.cursor(),
        3,
        "the cursor is at the end of a recalled line"
    );
}

#[test]
fn editing_a_recalled_line_makes_it_the_current_line() {
    let mut editor = LineEditor::new();
    editor.remember("hack 1");
    editor.history_previous();
    editor.insert('0');
    assert_eq!(editor.text(), "hack 10");
    editor.history_next();
    assert_eq!(editor.text(), "hack 10", "no longer browsing");
    editor.history_previous();
    assert_eq!(editor.text(), "hack 1");
}

#[test]
fn history_skips_empty_lines_and_consecutive_duplicates() {
    let mut editor = LineEditor::new();
    for line in ["scan", "scan", "  ", "", " scan ", "net", "scan"] {
        editor.remember(line);
    }
    assert_eq!(editor.history(), ["scan", "net", "scan"]);
    editor.history_previous();
    assert_eq!(editor.text(), "scan");
    // An empty history has nothing to recall.
    let mut empty = LineEditor::new();
    empty.history_previous();
    empty.history_next();
    assert_eq!(empty.text(), "");
}

#[test]
fn history_is_bounded_and_forgets_the_oldest() {
    let mut editor = LineEditor::new();
    for number in 0..(HISTORY_MAX + 20) {
        editor.remember(&format!("line {number}"));
    }
    assert_eq!(editor.history().len(), HISTORY_MAX);
    assert_eq!(
        editor.history().first().map(String::as_str),
        Some("line 20")
    );
    assert_eq!(
        editor.history().last().map(String::as_str),
        Some(format!("line {}", HISTORY_MAX + 19).as_str())
    );
}

#[test]
fn taking_the_line_starts_a_new_one() {
    let mut editor = typed("scan");
    editor.remember("old");
    editor.history_previous();
    assert_eq!(editor.take(), "old");
    assert_eq!(editor.text(), "");
    assert_eq!(editor.cursor(), 0);
    editor.history_next();
    assert_eq!(editor.text(), "", "browsing stopped with the line");
}

#[test]
fn setting_the_line_puts_the_cursor_at_its_end() {
    let mut editor = typed("sc");
    editor.set("scan ");
    assert_eq!(editor.text(), "scan ");
    assert_eq!(editor.cursor(), 5);
    editor.set("a\nb");
    assert_eq!(editor.text(), "ab");
}

/// One thing the player can do to the line.
#[derive(Debug, Clone)]
enum Op {
    Insert(char),
    Paste(String),
    Left,
    Right,
    Home,
    End,
    WordLeft,
    WordRight,
    Backspace,
    Delete,
    KillStart,
    KillEnd,
    KillWord,
    Previous,
    Next,
    Remember(String),
    Take,
    Set(String),
    Limit(usize),
}

fn any_char() -> impl Strategy<Value = char> {
    prop_oneof![
        Just('a'),
        Just(' '),
        Just('é'),
        Just('\u{301}'),
        Just('日'),
        Just('\u{1F1EB}'),
        Just('\u{1F1F7}'),
        Just('\u{200D}'),
        Just('\u{1F468}'),
        Just('\n'),
        Just('\u{1b}'),
        any::<char>(),
    ]
}

fn any_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(any_char(), 0..30).prop_map(|chars| chars.into_iter().collect())
}

fn any_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        any_char().prop_map(Op::Insert),
        any_text().prop_map(Op::Paste),
        Just(Op::Left),
        Just(Op::Right),
        Just(Op::Home),
        Just(Op::End),
        Just(Op::WordLeft),
        Just(Op::WordRight),
        Just(Op::Backspace),
        Just(Op::Delete),
        Just(Op::KillStart),
        Just(Op::KillEnd),
        Just(Op::KillWord),
        Just(Op::Previous),
        Just(Op::Next),
        any_text().prop_map(Op::Remember),
        Just(Op::Take),
        any_text().prop_map(Op::Set),
        (0usize..40).prop_map(Op::Limit),
    ]
}

fn apply(editor: &mut LineEditor, op: Op) {
    match op {
        Op::Insert(c) => editor.insert(c),
        Op::Paste(text) => editor.paste(&text),
        Op::Left => editor.left(),
        Op::Right => editor.right(),
        Op::Home => editor.home(),
        Op::End => editor.end(),
        Op::WordLeft => editor.word_left(),
        Op::WordRight => editor.word_right(),
        Op::Backspace => editor.backspace(),
        Op::Delete => editor.delete(),
        Op::KillStart => editor.kill_to_start(),
        Op::KillEnd => editor.kill_to_end(),
        Op::KillWord => editor.kill_word(),
        Op::Previous => editor.history_previous(),
        Op::Next => editor.history_next(),
        Op::Remember(line) => editor.remember(&line),
        Op::Take => drop(editor.take()),
        Op::Set(text) => editor.set(&text),
        Op::Limit(max) => editor.set_limit(max),
    }
}

proptest! {
    #[test]
    fn whatever_is_done_the_cursor_stays_on_a_grapheme_boundary_inside_the_line(
        ops in proptest::collection::vec(any_op(), 0..60),
    ) {
        let mut editor = LineEditor::new();
        for op in ops {
            apply(&mut editor, op);
            let text = editor.text();
            let cursor = editor.cursor();
            prop_assert!(cursor <= text.len(), "cursor {cursor} past {text:?}");
            prop_assert!(text.is_char_boundary(cursor));
            let on_boundary = cursor == text.len()
                || text.grapheme_indices(true).any(|(start, _)| start == cursor);
            prop_assert!(on_boundary, "cursor {cursor} is inside a cluster of {text:?}");
            prop_assert!(editor.cursor_column() <= text.width());
            prop_assert!(!text.chars().any(char::is_control), "control character in {text:?}");
            prop_assert!(text.chars().count() <= MAX_CHARS);
            // History is bounded and never holds an empty line or two equal lines in a row.
            let history = editor.history();
            prop_assert!(history.len() <= HISTORY_MAX);
            prop_assert!(history.iter().all(|line| !line.trim().is_empty()));
            prop_assert!(history.windows(2).all(|pair| pair[0] != pair[1]));
        }
    }

    #[test]
    fn moving_left_to_the_start_then_right_to_the_end_visits_every_cluster(text in any_text()) {
        let mut editor = LineEditor::new();
        editor.paste(&text);
        let clusters = editor.text().graphemes(true).count();
        let mut steps = 0;
        while editor.cursor() > 0 {
            let before = editor.cursor();
            editor.left();
            prop_assert!(editor.cursor() < before);
            steps += 1;
        }
        prop_assert_eq!(steps, clusters);
        let mut steps = 0;
        while editor.cursor() < editor.text().len() {
            let before = editor.cursor();
            editor.right();
            prop_assert!(editor.cursor() > before);
            steps += 1;
        }
        prop_assert_eq!(steps, clusters);
    }
}
