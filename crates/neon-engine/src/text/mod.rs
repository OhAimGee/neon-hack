//! Text is a key plus typed arguments, never a sentence.
//!
//! The engine only emits [`Text`] values. A frontend turns them into strings with
//! [`render()`], in the language and mode it chooses, so the history of a session can be
//! re-rendered when the language changes and tests assert on keys, not on phrases.
//!
//! The templates and catalogs are TOML files embedded in the program (`data/text/<lang>/`):
//! see [`template`] for the syntax (`{name}`, `{count|one|other}`), [`Catalog`] for the
//! keys and their `@sr` and `@ascii` variants, [`render()`] for the modes, and [`check()`]
//! for the rules the content must follow.

use std::borrow::Cow;

mod ascii;
mod catalog;
pub mod check;
mod render;
pub mod template;

pub use ascii::to_ascii;
pub use catalog::{Catalog, Lang, LoadError, LoadErrors, PluralForm};
pub use check::{Issue, check};
pub use render::{RenderMode, render};
pub use template::{Template, TemplateError};

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
