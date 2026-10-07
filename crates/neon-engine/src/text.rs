//! Text is a key plus typed arguments, never a sentence.
//!
//! The engine only emits [`Text`] values. A frontend turns them into strings with
//! [`render`], in the language and mode it chooses, so the history of a session can be
//! re-rendered when the language changes and tests assert on keys, not on phrases.
//!
//! Phase R1.2 adds plurals and the real catalogs; this module holds the contract and the
//! minimal `{name}` substitution every later feature builds on.

use std::borrow::Cow;
use std::collections::BTreeSet;

/// A message to show: a catalog key and its named arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    /// Catalog key, for example `demo.scan.found`.
    pub key: Cow<'static, str>,
    /// Named arguments, substituted for `{name}` in the template.
    pub args: Vec<(&'static str, Arg)>,
}

/// An argument of a [`Text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// A number, rendered in decimal.
    Int(i64),
    /// Text that must not be translated (a player name, an id).
    Str(String),
    /// A glossary term, looked up under `term.<name>` so it reads the same everywhere.
    Term(&'static str),
    /// Another text, rendered in the same language (an item name, a contact name).
    Text(Box<Text>),
}

impl Text {
    /// A text with a static key and no argument yet.
    #[must_use]
    pub fn new(key: &'static str) -> Self {
        Self {
            key: Cow::Borrowed(key),
            args: Vec::new(),
        }
    }

    /// A text whose key is built from data (`quest.m05.title`).
    #[must_use]
    pub fn dynamic(key: String) -> Self {
        Self {
            key: Cow::Owned(key),
            args: Vec::new(),
        }
    }

    /// Text shown as is, in every language: names typed by the player, command names.
    /// The catalog defines `ui.raw` as `{value}`.
    #[must_use]
    pub fn raw(value: impl Into<String>) -> Self {
        Self::new("ui.raw").with_str("value", value)
    }

    /// Adds a numeric argument.
    #[must_use]
    pub fn with_int(mut self, name: &'static str, value: i64) -> Self {
        self.args.push((name, Arg::Int(value)));
        self
    }

    /// Adds an untranslated text argument.
    #[must_use]
    pub fn with_str(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.args.push((name, Arg::Str(value.into())));
        self
    }

    /// Adds a glossary term argument.
    #[must_use]
    pub fn with_term(mut self, name: &'static str, term: &'static str) -> Self {
        self.args.push((name, Arg::Term(term)));
        self
    }

    /// Adds a nested text argument.
    #[must_use]
    pub fn with_text(mut self, name: &'static str, text: Text) -> Self {
        self.args.push((name, Arg::Text(Box::new(text))));
        self
    }
}

/// How a text is rendered. The mode selects a variant of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// Everything: accents, symbols, decoration.
    #[default]
    Full,
    /// Screen-reader wording: no symbols to spell out (`key@sr`).
    ScreenReader,
    /// ASCII-friendly: the `key@ascii` variants and ASCII decoration. Accented letters and
    /// text typed by the player are kept until transliteration arrives (phase R1.2).
    Ascii,
}

impl RenderMode {
    /// Suffix of the key variant tried before the base key.
    #[must_use]
    pub fn suffix(self) -> Option<&'static str> {
        match self {
            Self::Full => None,
            Self::ScreenReader => Some("@sr"),
            Self::Ascii => Some("@ascii"),
        }
    }
}

/// A source of templates, one language each.
pub trait Catalog {
    /// The template of a key, if the catalog has one.
    fn lookup(&self, key: &str) -> Option<&str>;
}

/// A catalog backed by a static table. Used by the demo and by tests; the real
/// catalogs are embedded data files (phase R1.2).
#[derive(Debug, Clone, Copy)]
pub struct StaticCatalog {
    entries: &'static [(&'static str, &'static str)],
}

impl StaticCatalog {
    /// Wraps a table of `(key, template)` pairs.
    #[must_use]
    pub const fn new(entries: &'static [(&'static str, &'static str)]) -> Self {
        Self { entries }
    }

    /// The raw `(key, template)` pairs, for parity checks.
    #[must_use]
    pub fn entries(&self) -> &'static [(&'static str, &'static str)] {
        self.entries
    }
}

impl Catalog for StaticCatalog {
    fn lookup(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, template)| *template)
    }
}

/// Renders a text with a catalog. A missing key shows as `<missing:key>` so the problem
/// is visible, and tests can assert that it never happens.
#[must_use]
pub fn render(text: &Text, catalog: &dyn Catalog, mode: RenderMode) -> String {
    let template = mode
        .suffix()
        .and_then(|suffix| catalog.lookup(&format!("{}{suffix}", text.key)))
        .or_else(|| catalog.lookup(&text.key));
    let Some(template) = template else {
        return format!("<missing:{}>", text.key);
    };
    let mut out = String::new();
    substitute(template, &text.args, catalog, mode, &mut out);
    out
}

fn substitute(
    template: &str,
    args: &[(&'static str, Arg)],
    catalog: &dyn Catalog,
    mode: RenderMode,
    out: &mut String,
) {
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once('{') {
        out.push_str(before);
        if let Some((name, remainder)) = after_open.split_once('}') {
            if let Some((_, arg)) = args.iter().find(|(arg_name, _)| *arg_name == name) {
                push_arg(arg, catalog, mode, out);
            } else {
                out.push_str("<?");
                out.push_str(name);
                out.push('>');
            }
            rest = remainder;
        } else {
            out.push('{');
            rest = after_open;
        }
    }
    out.push_str(rest);
}

fn push_arg(arg: &Arg, catalog: &dyn Catalog, mode: RenderMode, out: &mut String) {
    match arg {
        Arg::Int(value) => out.push_str(&value.to_string()),
        Arg::Str(value) => out.push_str(value),
        Arg::Term(term) => {
            let key = format!("term.{term}");
            out.push_str(&render(&Text::dynamic(key), catalog, mode));
        }
        Arg::Text(text) => out.push_str(&render(text, catalog, mode)),
    }
}

/// The names of the `{placeholders}` of a template. Translations must use the same set.
#[must_use]
pub fn placeholders(template: &str) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    let mut rest = template;
    while let Some((_, after_open)) = rest.split_once('{') {
        match after_open.split_once('}') {
            Some((name, remainder)) => {
                names.insert(name);
                rest = remainder;
            }
            None => break,
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: StaticCatalog = StaticCatalog::new(&[
        ("ui.raw", "{value}"),
        ("hello", "Hello {name}, you have {n} credits."),
        ("hello@sr", "Hello {name}. You have {n} credits."),
        ("hello@ascii", "Hello {name} - {n} credits."),
        ("plain", "No placeholder."),
        ("outer", "Bought {item}."),
        ("item.proxy", "a proxy"),
        ("term.trace", "Trace"),
        ("with.term", "Watch the {gauge}."),
        ("broken", "Unclosed { brace"),
        ("unknown.arg", "Value: {nothing}."),
    ]);

    fn full(text: &Text) -> String {
        render(text, &CATALOG, RenderMode::Full)
    }

    #[test]
    fn substitutes_typed_arguments() {
        let text = Text::new("hello")
            .with_str("name", "Case")
            .with_int("n", 50);
        assert_eq!(full(&text), "Hello Case, you have 50 credits.");
    }

    #[test]
    fn nested_texts_and_terms_are_rendered_too() {
        let nested = Text::new("outer").with_text("item", Text::new("item.proxy"));
        assert_eq!(full(&nested), "Bought a proxy.");
        let term = Text::new("with.term").with_term("gauge", "trace");
        assert_eq!(full(&term), "Watch the Trace.");
    }

    #[test]
    fn raw_text_is_shown_unchanged() {
        assert_eq!(full(&Text::raw("ECHO-7")), "ECHO-7");
    }

    #[test]
    fn mode_picks_its_variant_and_falls_back_to_the_base_key() {
        let text = Text::new("hello").with_str("name", "Case").with_int("n", 5);
        assert_eq!(
            render(&text, &CATALOG, RenderMode::ScreenReader),
            "Hello Case. You have 5 credits."
        );
        assert_eq!(
            render(&text, &CATALOG, RenderMode::Ascii),
            "Hello Case - 5 credits."
        );
        let plain = Text::new("plain");
        assert_eq!(
            render(&plain, &CATALOG, RenderMode::ScreenReader),
            "No placeholder."
        );
        assert_eq!(
            render(&plain, &CATALOG, RenderMode::Ascii),
            "No placeholder."
        );
    }

    #[test]
    fn problems_are_visible_not_silent() {
        assert_eq!(full(&Text::new("nope")), "<missing:nope>");
        assert_eq!(full(&Text::new("unknown.arg")), "Value: <?nothing>.");
        assert_eq!(full(&Text::new("broken")), "Unclosed { brace");
    }

    #[test]
    fn placeholders_lists_the_names_once() {
        let names = placeholders("{a} then {b} then {a}, but not { c");
        assert_eq!(names.into_iter().collect::<Vec<_>>(), ["a", "b"]);
        assert!(placeholders("no braces").is_empty());
    }
}
