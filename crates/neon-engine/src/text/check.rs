//! Content checks over the catalogs: the rules a text must follow, and parity between
//! languages. The tests of the crate run them on the embedded catalogs; a future
//! `--check-content` command can run them on a development directory.

use std::collections::BTreeSet;

use thiserror::Error;

use super::catalog::{Catalog, Lang};

/// The variants a key may have besides its base text.
const VARIANT_SUFFIXES: [&str; 2] = ["sr", "ascii"];

/// Symbols a screen reader spells out badly or skips: a text that uses one must have a
/// `key@sr` variant that says the same thing in words.
const SPOKEN_BADLY: [char; 9] = ['→', '←', '↔', '¢', '×', '·', '•', '≤', '≥'];

/// A rule broken by a catalog.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Issue {
    /// A key of the reference language is absent from another one.
    #[error("[{lang:?}] `{key}` is missing")]
    Missing {
        /// The language that lacks the key.
        lang: Lang,
        /// The key.
        key: String,
    },
    /// A key exists in a language but not in the reference one.
    #[error("[{lang:?}] `{key}` has no counterpart in the reference language")]
    Orphan {
        /// The language that has the extra key.
        lang: Lang,
        /// The key.
        key: String,
    },
    /// Two languages (or a variant and its base) use different placeholders.
    #[error("[{lang:?}] `{key}` uses {found:?} where {expected:?} is expected")]
    Placeholders {
        /// The language of the deviating text.
        lang: Lang,
        /// The key.
        key: String,
        /// The placeholders of the model (the reference language, or the base key).
        expected: Vec<String>,
        /// The placeholders actually used.
        found: Vec<String>,
    },
    /// A key ends in `@something` that is not a known variant.
    #[error("[{lang:?}] `{key}`: unknown variant (expected {VARIANT_SUFFIXES:?})")]
    UnknownVariant {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
    },
    /// A variant exists without the base key it varies.
    #[error("[{lang:?}] `{key}` is a variant without a base text")]
    VariantWithoutBase {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
    },
    /// A text with a symbol that screen readers handle badly, and no `@sr` variant.
    #[error("[{lang:?}] `{key}` uses `{symbol}` and has no `@sr` variant")]
    SymbolWithoutSpokenVariant {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
        /// The symbol.
        symbol: char,
    },
    /// A text that renders to nothing.
    #[error("[{lang:?}] `{key}` is empty")]
    Empty {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
    },
    /// A character the game does not use in texts.
    #[error("[{lang:?}] `{key}` contains U+{ch:04X}, which is not allowed")]
    ForbiddenChar {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
        /// The character, as a code point.
        ch: u32,
    },
}

/// Checks every catalog by itself and against the first one, which is the reference.
#[must_use]
pub fn check(catalogs: &[Catalog]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for catalog in catalogs {
        check_one(catalog, &mut issues);
    }
    if let Some((reference, others)) = catalogs.split_first() {
        for other in others {
            check_parity(reference, other, &mut issues);
        }
    }
    issues
}

fn names(template: &super::template::Template) -> Vec<String> {
    template
        .placeholders()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn check_one(catalog: &Catalog, issues: &mut Vec<Issue>) {
    let lang = catalog.lang();
    for (key, template) in catalog.iter() {
        if template.is_empty() {
            issues.push(Issue::Empty {
                lang,
                key: key.to_owned(),
            });
        }
        let forbidden = template
            .literals()
            .into_iter()
            .flat_map(str::chars)
            .find(|c| is_forbidden(*c));
        if let Some(ch) = forbidden {
            issues.push(Issue::ForbiddenChar {
                lang,
                key: key.to_owned(),
                ch: u32::from(ch),
            });
        }
        let Some((base, variant)) = key.split_once('@') else {
            let symbol = template
                .literals()
                .into_iter()
                .flat_map(str::chars)
                .find(|c| SPOKEN_BADLY.contains(c));
            if let Some(symbol) = symbol
                && catalog.get(&format!("{key}@sr")).is_none()
            {
                issues.push(Issue::SymbolWithoutSpokenVariant {
                    lang,
                    key: key.to_owned(),
                    symbol,
                });
            }
            continue;
        };
        if !VARIANT_SUFFIXES.contains(&variant) {
            issues.push(Issue::UnknownVariant {
                lang,
                key: key.to_owned(),
            });
        }
        match catalog.get(base) {
            None => issues.push(Issue::VariantWithoutBase {
                lang,
                key: key.to_owned(),
            }),
            Some(base_template) if base_template.placeholders() != template.placeholders() => {
                issues.push(Issue::Placeholders {
                    lang,
                    key: key.to_owned(),
                    expected: names(base_template),
                    found: names(template),
                });
            }
            Some(_) => {}
        }
    }
}

fn check_parity(reference: &Catalog, other: &Catalog, issues: &mut Vec<Issue>) {
    let lang = other.lang();
    let reference_keys: BTreeSet<&str> = reference.iter().map(|(key, _)| key).collect();
    let other_keys: BTreeSet<&str> = other.iter().map(|(key, _)| key).collect();
    for key in reference_keys.difference(&other_keys) {
        issues.push(Issue::Missing {
            lang,
            key: (*key).to_owned(),
        });
    }
    for key in other_keys.difference(&reference_keys) {
        issues.push(Issue::Orphan {
            lang,
            key: (*key).to_owned(),
        });
    }
    for (key, template) in reference.iter() {
        let Some(translated) = other.get(key) else {
            continue;
        };
        if template.placeholders() != translated.placeholders() {
            issues.push(Issue::Placeholders {
                lang,
                key: key.to_owned(),
                expected: names(template),
                found: names(translated),
            });
        }
    }
}

/// Control characters (a text is one line), non-breaking and narrow spaces, the ellipsis
/// character, and emoji: the terminals the game targets draw them unreliably.
fn is_forbidden(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{00A0}' | '\u{202F}' | '\u{2026}' | '\u{200D}' | '\u{FE0F}'
                | '\u{2600}'..='\u{27BF}'
                | '\u{1F000}'..='\u{1FAFF}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(lang: Lang, source: &str) -> Catalog {
        Catalog::from_sources(lang, &[("test.toml", source)]).unwrap()
    }

    #[test]
    fn matching_catalogs_have_no_issue() {
        let en = catalog(
            Lang::En,
            "hello = \"Hi {name}\"\n\"hello@sr\" = \"Hi, {name}\"\nn = \"{n|one|{n} many}\"\n",
        );
        let fr = catalog(
            Lang::Fr,
            "hello = \"Salut {name}\"\n\"hello@sr\" = \"Salut, {name}\"\nn = \"{n|un|{n} nombreux}\"\n",
        );
        assert_eq!(check(&[en, fr]), []);
    }

    #[test]
    fn each_rule_catches_its_break() {
        let en = catalog(
            Lang::En,
            r#"
            only_en = "x"
            hello = "Hi {name}"
            "hello@sr" = "Hi {nom}"
            "ghost@ascii" = "boo"
            "hello@weird" = "Hi {name}"
            "#,
        );
        let fr = catalog(
            Lang::Fr,
            "hello = \"Salut {name} {more}\"\nonly_fr = \"y\"\n",
        );
        let issues = check(&[en, fr]);
        let has = |issue: &Issue| issues.contains(issue);
        let key = |text: &str| text.to_owned();
        assert!(
            has(&Issue::Missing {
                lang: Lang::Fr,
                key: key("only_en")
            }),
            "{issues:#?}"
        );
        assert!(has(&Issue::Orphan {
            lang: Lang::Fr,
            key: key("only_fr")
        }));
        assert!(has(&Issue::Placeholders {
            lang: Lang::En,
            key: key("hello@sr"),
            expected: vec![key("name")],
            found: vec![key("nom")],
        }));
        assert!(has(&Issue::Placeholders {
            lang: Lang::Fr,
            key: key("hello"),
            expected: vec![key("name")],
            found: vec![key("more"), key("name")],
        }));
        assert!(has(&Issue::VariantWithoutBase {
            lang: Lang::En,
            key: key("ghost@ascii")
        }));
        assert!(has(&Issue::UnknownVariant {
            lang: Lang::En,
            key: key("hello@weird")
        }));
    }

    #[test]
    fn a_symbol_screen_readers_handle_badly_needs_its_spoken_variant() {
        let en = catalog(
            Lang::En,
            r#"
            bad = "{a} → {b}"
            good = "{a} → {b}"
            "good@sr" = "{a} to {b}"
            quotes = "« fine »"
            "#,
        );
        assert_eq!(
            check(&[en]),
            [Issue::SymbolWithoutSpokenVariant {
                lang: Lang::En,
                key: "bad".to_owned(),
                symbol: '→',
            }]
        );
    }

    #[test]
    fn empty_texts_and_forbidden_characters_are_caught() {
        let en = catalog(
            Lang::En,
            "empty = \"\"\ndots = \"wait…\"\nnbsp = \"a\u{00A0}b\"\nsmile = \"ok \u{1F600}\"\nlines = \"\"\"a\nb\"\"\"\nplain = \"fine « ok »\"\n",
        );
        let issues = check(&[en]);
        let key = |text: &str| text.to_owned();
        assert!(issues.contains(&Issue::Empty {
            lang: Lang::En,
            key: key("empty")
        }));
        for (name, ch) in [
            ("dots", 0x2026),
            ("nbsp", 0xA0),
            ("smile", 0x1F600),
            ("lines", 0x0A),
        ] {
            assert!(
                issues.contains(&Issue::ForbiddenChar {
                    lang: Lang::En,
                    key: key(name),
                    ch
                }),
                "{name}: {issues:#?}"
            );
        }
        assert_eq!(issues.len(), 5, "`plain` is fine: {issues:#?}");
    }
}
