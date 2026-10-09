//! Text measured and cut by what the terminal shows.
//!
//! Everything here counts display columns (`unicode-width`) and moves by grapheme cluster
//! (`unicode-segmentation`), so that an accent written as two code points, a flag or an
//! emoji sequence is never split between two rows and never counted as two characters.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Display width of a text, in terminal columns.
pub(crate) fn width(text: &str) -> usize {
    text.width()
}

/// A text a terminal can show as one row: line breaks and other control characters would
/// move the cursor, so tabs become a space and the rest is dropped.
pub(crate) fn sanitize(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\t' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect()
}

/// Cuts a text into rows of at most `width` columns, at spaces when it can and between
/// graphemes inside a word when it must. The indent of the text is kept on every row.
///
/// A grapheme wider than `width` (a wide character in a one-column area) gets a row of its
/// own and the drawing clips it: the function always makes progress. Empty text is one
/// empty row, so that a blank line stays a blank line.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let text = sanitize(text);
    let body = text.trim_start_matches(' ');
    let widest = body
        .graphemes(true)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let mut indent_columns = (text.len() - body.len()).min(width / 2);
    if width - indent_columns < widest {
        // No room for the indent and the widest character: the text comes first.
        indent_columns = 0;
    }
    let indent = " ".repeat(indent_columns);
    let available = width.saturating_sub(indent_columns).max(1);

    let mut rows: Vec<String> = Vec::new();
    let mut row = String::new();
    let mut used = 0;
    for word in body.split(' ') {
        let word_width = word.width();
        let needed = if used == 0 {
            word_width
        } else {
            used + 1 + word_width
        };
        if needed <= available {
            if used > 0 {
                row.push(' ');
            }
            row.push_str(word);
            used = needed;
            continue;
        }
        if used > 0 {
            rows.push(std::mem::take(&mut row));
            used = 0;
        }
        if word_width <= available {
            row.push_str(word);
            used = word_width;
            continue;
        }
        for grapheme in word.graphemes(true) {
            let grapheme_width = grapheme.width();
            if used > 0 && used + grapheme_width > available {
                rows.push(std::mem::take(&mut row));
                used = 0;
            }
            row.push_str(grapheme);
            used += grapheme_width;
        }
    }
    rows.push(row);
    rows.into_iter()
        .map(|row| {
            if row.is_empty() {
                row
            } else {
                format!("{indent}{row}")
            }
        })
        .collect()
}

/// The start of a text that fits in `width` columns (for art, which must not be re-flowed).
pub(crate) fn head(text: &str, width: usize) -> String {
    let text = sanitize(text);
    let mut kept = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme.width();
        if used + grapheme_width > width {
            break;
        }
        used += grapheme_width;
        kept.push_str(grapheme);
    }
    kept
}

/// The part of a one-row text to show in `width` columns so that the cursor stays in sight,
/// and the column of the cursor in what is shown. A text that fits is shown from its start;
/// a longer one loses its first graphemes until the cursor, which needs a cell of its own,
/// is in the last column at most.
pub(crate) fn window(text: &str, cursor_column: usize, width: usize) -> (String, usize) {
    let text = sanitize(text);
    let width = width.max(1);
    if cursor_column < width {
        return (head(&text, width), cursor_column);
    }
    let to_drop = cursor_column + 1 - width;
    let mut dropped = 0;
    let mut start = 0;
    for grapheme in text.graphemes(true) {
        if dropped >= to_drop {
            break;
        }
        dropped += grapheme.width();
        start += grapheme.len();
    }
    let rest = text.get(start..).unwrap_or_default();
    (head(rest, width), cursor_column.saturating_sub(dropped))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn wrapping_cuts_at_spaces_and_inside_words_only_when_it_must() {
        assert_eq!(wrap("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap("", 10), [""]);
        assert_eq!(wrap("short", 100), ["short"]);
        assert_eq!(
            wrap("ab cd", 0),
            ["a", "b", "c", "d"],
            "a zero width is one"
        );
    }

    #[test]
    fn wide_characters_take_two_columns() {
        assert_eq!(wrap("日本語日本語", 6), ["日本語", "日本語"]);
        assert_eq!(wrap("日本語", 5), ["日本", "語"]);
        // A character wider than the area still makes progress: one per row.
        assert_eq!(wrap("日本", 1), ["日", "本"]);
    }

    #[test]
    fn a_grapheme_is_never_split() {
        let accented = "e\u{301}";
        let text = accented.repeat(5);
        assert_eq!(
            wrap(&text, 2),
            [accented.repeat(2), accented.repeat(2), accented.to_owned()]
        );
        // A flag is two regional indicators, a family is a sequence joined by ZWJ.
        let flag = "\u{1F1EB}\u{1F1F7}";
        assert_eq!(wrap(&flag.repeat(3), 4), [flag.repeat(2), flag.to_owned()]);
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        assert_eq!(wrap(&family.repeat(2), 2), [family, family]);
    }

    #[test]
    fn the_indent_is_kept_on_every_row_and_blank_rows_stay_blank() {
        assert_eq!(wrap("  ab cd ef", 6), ["  ab", "  cd", "  ef"]);
        assert_eq!(wrap("  ", 6), [""]);
        // The indent never takes more than half the width, so there is always room to write.
        assert_eq!(wrap("          x", 4), ["  x"]);
    }

    #[test]
    fn control_characters_are_never_sent_to_the_terminal() {
        assert_eq!(wrap("a\u{1b}[31mb\tc\nd", 40), ["a[31mb cd"]);
        assert_eq!(sanitize("x\u{7}y"), "xy");
    }

    #[test]
    fn the_input_window_keeps_the_cursor_in_sight() {
        assert_eq!(window("> abc", 5, 10), ("> abc".to_owned(), 5));
        // At the end of a line as wide as the area, the cursor needs the next cell.
        assert_eq!(window("> abcdefgh", 10, 10), (" abcdefgh".to_owned(), 9));
        assert_eq!(window("> abcdefghij", 12, 6), ("fghij".to_owned(), 5));
        // Cursor in the middle of a long line: it is shown, with what follows it.
        assert_eq!(window("> abcdefghij", 4, 6), ("> abcd".to_owned(), 4));
        // A wide character can make the drop overshoot by one column.
        let (shown, column) = window("日本語日本語", 12, 5);
        assert!(column < 5 && width(&shown) <= 5, "{shown:?} {column}");
    }

    #[test]
    fn head_keeps_whole_graphemes_that_fit() {
        assert_eq!(head("abcdef", 3), "abc");
        assert_eq!(head("日本語", 5), "日本");
        let accented = "e\u{301}";
        assert_eq!(head(&accented.repeat(4), 2), accented.repeat(2));
        assert_eq!(head("ab", 0), "");
    }

    /// Texts with spaces, accents, wide characters, combining marks, flags and controls.
    fn any_text() -> impl Strategy<Value = String> {
        let piece = prop_oneof![
            Just(" ".to_owned()),
            Just("  ".to_owned()),
            Just("a".to_owned()),
            Just("word".to_owned()),
            Just("é".to_owned()),
            Just("e\u{301}".to_owned()),
            Just("日".to_owned()),
            Just("\u{1F1EB}\u{1F1F7}".to_owned()),
            Just("\u{1F468}\u{200D}\u{1F469}".to_owned()),
            Just("█".to_owned()),
            Just("\t".to_owned()),
            Just("\n".to_owned()),
            Just("\u{1b}".to_owned()),
        ];
        proptest::collection::vec(piece, 0..40).prop_map(|pieces| pieces.concat())
    }

    fn visible_graphemes(text: &str) -> Vec<String> {
        sanitize(text)
            .graphemes(true)
            .filter(|g| *g != " ")
            .map(str::to_owned)
            .collect()
    }

    proptest! {
        #[test]
        fn wrapped_rows_fit_lose_nothing_and_split_no_grapheme(text in any_text(), width in 0usize..30) {
            let rows = wrap(&text, width);
            prop_assert!(!rows.is_empty());
            let widest_grapheme = sanitize(&text)
                .graphemes(true)
                .map(UnicodeWidthStr::width)
                .max()
                .unwrap_or(0);
            let limit = width.max(1).max(widest_grapheme);
            for row in &rows {
                prop_assert!(row.width() <= limit, "{row:?} is wider than {limit}");
            }
            // The same graphemes in the same order: nothing lost, nothing cut in two.
            let kept: Vec<String> = rows.iter().flat_map(|row| visible_graphemes(row)).collect();
            prop_assert_eq!(kept, visible_graphemes(&text));
        }

        #[test]
        fn the_window_always_fits_and_holds_the_cursor(
            text in any_text(),
            cursor in 0usize..60,
            width in 1usize..30,
        ) {
            let cursor = cursor.min(sanitize(&text).width());
            let (shown, column) = window(&text, cursor, width);
            prop_assert!(shown.width() <= width);
            prop_assert!(column < width);
        }

        #[test]
        fn head_fits_and_is_the_start_of_the_text(text in any_text(), width in 0usize..30) {
            let start = head(&text, width);
            prop_assert!(start.width() <= width);
            prop_assert!(sanitize(&text).starts_with(&start));
        }
    }
}
