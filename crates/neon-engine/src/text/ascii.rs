//! Transliteration to 7-bit ASCII, for `--ascii`.
//!
//! One function, applied to the final text of a line (see [`super::render()`]), so the layout
//! of a frontend is computed on what is really displayed. It covers what the game writes
//! (French and English texts) and what a player may type (Latin letters with accents);
//! anything else becomes `?`, one for each character.

/// The text with every character outside ASCII replaced by an ASCII equivalent.
///
/// Accented letters lose their accent (`é` → `e`), typographic punctuation becomes its
/// plain form (`«` → `"`, `…` → `...`), arrows and signs are spelled (`→` → `->`,
/// `¢` → `cr`), combining accents are dropped, and an unknown character is `?`.
#[must_use]
pub fn to_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else if is_combining_mark(c) {
            // The base letter is already there: "e" + U+0301 reads "e".
        } else {
            out.push_str(equivalent(c).unwrap_or("?"));
        }
    }
    out
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}')
}

fn equivalent(c: char) -> Option<&'static str> {
    Some(match c {
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' => "A",
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'Æ' => "AE",
        'æ' => "ae",
        'Ç' | 'Ć' | 'Č' => "C",
        'ç' | 'ć' | 'č' => "c",
        'Ď' | 'Đ' | 'Ð' => "D",
        'ď' | 'đ' | 'ð' => "d",
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ė' | 'Ę' | 'Ě' => "E",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => "e",
        'Ğ' => "G",
        'ğ' => "g",
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' | 'İ' => "I",
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ı' => "i",
        'Ł' => "L",
        'ł' => "l",
        'Ñ' | 'Ń' | 'Ň' => "N",
        'ñ' | 'ń' | 'ň' => "n",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' | 'Ő' => "O",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'Œ' => "OE",
        'œ' => "oe",
        'Ř' => "R",
        'ř' => "r",
        'Ś' | 'Š' | 'Ş' => "S",
        'ś' | 'š' | 'ş' => "s",
        'ß' => "ss",
        'Ť' | 'Ţ' => "T",
        'ť' | 'ţ' => "t",
        'Þ' => "TH",
        'þ' => "th",
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' | 'Ů' | 'Ű' => "U",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
        'Ý' | 'Ÿ' => "Y",
        'ý' | 'ÿ' => "y",
        'Ź' | 'Ż' | 'Ž' => "Z",
        'ź' | 'ż' | 'ž' => "z",
        '«' | '»' | '“' | '”' | '„' => "\"",
        '‘' | '’' | '‚' | '‹' | '›' => "'",
        '–' | '—' | '―' | '‐' | '‑' | '−' => "-",
        '…' => "...",
        '\u{00A0}' | '\u{2007}' | '\u{2009}' | '\u{202F}' => " ",
        '·' | '•' | '∙' => "*",
        '×' => "x",
        '÷' => "/",
        '°' => "deg",
        '€' => "EUR",
        '¢' => "cr",
        '→' | '⟶' | '➜' => "->",
        '←' => "<-",
        '↔' => "<->",
        '≤' => "<=",
        '≥' => ">=",
        '≠' => "!=",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn ascii_text_is_returned_unchanged() {
        let text = "Neon Hack 2087 - [ALERT] {x} ~ \t|";
        assert_eq!(to_ascii(text), text);
    }

    #[test]
    fn french_and_typography_become_plain() {
        assert_eq!(
            to_ascii("À l'écran : CANAL CHIFFRÉ #7"),
            "A l'ecran : CANAL CHIFFRE #7"
        );
        assert_eq!(to_ascii("Graver « Zoë » ?"), "Graver \" Zoe \" ?");
        assert_eq!(
            to_ascii("cœur, Œuvre, ça, Ça, œ"),
            "coeur, OEuvre, ca, Ca, oe"
        );
        assert_eq!(
            to_ascii("de 4 → 14, 3 × 2… 5 ¢"),
            "de 4 -> 14, 3 x 2... 5 cr"
        );
        assert_eq!(to_ascii("it’s “quoted” – done"), "it's \"quoted\" - done");
        assert_eq!(to_ascii("a\u{00A0}b\u{202F}c"), "a b c");
    }

    #[test]
    fn combining_accents_are_dropped_and_unknown_characters_are_marked() {
        assert_eq!(to_ascii("e\u{0301}te\u{0301}"), "ete");
        assert_eq!(to_ascii("ネオ"), "??");
        assert_eq!(to_ascii("ok \u{1F600}"), "ok ?");
        assert_eq!(to_ascii(""), "");
    }

    proptest! {
        #[test]
        fn the_result_is_always_ascii(text in "\\PC{0,40}") {
            prop_assert!(to_ascii(&text).is_ascii());
        }

        #[test]
        fn ascii_input_is_a_fixed_point(text in "[ -~]{0,40}") {
            prop_assert_eq!(to_ascii(&text), text);
        }
    }
}
