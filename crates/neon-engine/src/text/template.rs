//! The template language of the catalogs: `{name}` and `{count|singular|plural}`.
//!
//! A template is parsed once, when a catalog is loaded, so a malformed one is a load error
//! with the file and the key, never a surprise in the middle of a game.
//!
//! * `{name}` is replaced by the argument `name`.
//! * `{count|one|other}` picks a form with the plural rule of the language, from the
//!   numeric argument `count`. A form may hold placeholders, `count` included:
//!   `{n|a node|{n} nodes}`, or `{n} {n|node|nodes}`.
//! * `{name^}` is `{name}` with its first letter in capitals (Unicode, whatever the language):
//!   a sentence that opens with a glossary term, which the catalogs hold in lowercase.
//! * Braces are reserved for placeholders; `|` is only special inside a plural.

use std::collections::BTreeSet;

use thiserror::Error;

/// A parsed template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub(super) parts: Vec<Part>,
}

/// One piece of a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Part {
    Literal(String),
    Placeholder(String),
    /// `{name^}`: the argument with its first letter in capitals.
    Capitalized(String),
    Plural {
        selector: String,
        one: Template,
        other: Template,
    },
}

/// Why a template could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TemplateError {
    /// A `{` was never closed.
    #[error("a `{{` is never closed")]
    Unclosed,
    /// A `}` closes nothing.
    #[error("a `}}` closes nothing")]
    StrayClose,
    /// A placeholder has no name: `{}` or `{|a|b}`.
    #[error("a placeholder has no name")]
    EmptyName,
    /// A placeholder name has a character other than `a-z`, `0-9` and `_`.
    #[error("`{0}` is not allowed in a placeholder name (use a-z, 0-9 and _)")]
    BadName(char),
    /// A plural has another number of forms than two.
    #[error("a plural needs exactly two forms, `{{name|one|other}}`")]
    PluralForms,
}

/// What ended a run of parts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum End {
    Eof,
    Close,
    Pipe,
}

impl Template {
    /// Parses a template.
    ///
    /// # Errors
    ///
    /// [`TemplateError`] when a brace is unbalanced, a name is empty or malformed, or a
    /// plural does not have two forms.
    pub fn parse(source: &str) -> Result<Self, TemplateError> {
        let mut chars = source.chars().peekable();
        let (template, end) = parse_sequence(&mut chars, false)?;
        match end {
            End::Eof => Ok(template),
            End::Close => Err(TemplateError::StrayClose),
            // Only a plural form stops at a `|`, and this run is not one.
            End::Pipe => Err(TemplateError::PluralForms),
        }
    }

    /// The names a caller must provide: every placeholder and plural count, nested ones
    /// included. Translations of one text must use the same set.
    #[must_use]
    pub fn placeholders(&self) -> BTreeSet<&str> {
        let mut names = BTreeSet::new();
        self.collect_placeholders(&mut names);
        names
    }

    fn collect_placeholders<'a>(&'a self, names: &mut BTreeSet<&'a str>) {
        for part in &self.parts {
            match part {
                Part::Literal(_) => {}
                Part::Placeholder(name) | Part::Capitalized(name) => {
                    names.insert(name);
                }
                Part::Plural {
                    selector,
                    one,
                    other,
                } => {
                    names.insert(selector);
                    one.collect_placeholders(names);
                    other.collect_placeholders(names);
                }
            }
        }
    }

    /// Every fixed piece of text of the template, for the content checks.
    pub(super) fn literals(&self) -> Vec<&str> {
        let mut found = Vec::new();
        self.collect_literals(&mut found);
        found
    }

    fn collect_literals<'a>(&'a self, found: &mut Vec<&'a str>) {
        for part in &self.parts {
            match part {
                Part::Literal(text) => found.push(text),
                Part::Placeholder(_) | Part::Capitalized(_) => {}
                Part::Plural { one, other, .. } => {
                    one.collect_literals(found);
                    other.collect_literals(found);
                }
            }
        }
    }

    /// True when rendering can produce no text at all, whatever the arguments.
    pub(super) fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

/// Parses parts until the end of the text, a `}` or (inside a plural form) a `|`.
fn parse_sequence(chars: &mut Chars<'_>, in_form: bool) -> Result<(Template, End), TemplateError> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let flush = |literal: &mut String, parts: &mut Vec<Part>| {
        if !literal.is_empty() {
            parts.push(Part::Literal(std::mem::take(literal)));
        }
    };
    let end = loop {
        match chars.next() {
            None if in_form => return Err(TemplateError::Unclosed),
            None => break End::Eof,
            Some('}') => break End::Close,
            Some('|') if in_form => break End::Pipe,
            Some('{') => {
                flush(&mut literal, &mut parts);
                parts.push(parse_placeholder(chars)?);
            }
            Some(other) => literal.push(other),
        }
    };
    flush(&mut literal, &mut parts);
    Ok((Template { parts }, end))
}

/// Parses what follows a `{`: a name, then `}` or a plural's two forms.
fn parse_placeholder(chars: &mut Chars<'_>) -> Result<Part, TemplateError> {
    let mut name = String::new();
    let mut capitalize = false;
    let separator = loop {
        match chars.next() {
            None => return Err(TemplateError::Unclosed),
            Some(c @ ('}' | '|')) => break c,
            // The marker closes a plain placeholder: `{name^}`.
            Some('^') if !name.is_empty() && !capitalize && chars.peek() == Some(&'}') => {
                capitalize = true;
            }
            Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' => name.push(c),
            Some(c) => return Err(TemplateError::BadName(c)),
        }
    };
    if name.is_empty() {
        return Err(TemplateError::EmptyName);
    }
    if separator == '}' {
        return Ok(if capitalize {
            Part::Capitalized(name)
        } else {
            Part::Placeholder(name)
        });
    }
    if capitalize {
        // A plural picks a form; capitalizing is for a plain argument.
        return Err(TemplateError::BadName('^'));
    }
    let (one, end) = parse_sequence(chars, true)?;
    if end != End::Pipe {
        return Err(TemplateError::PluralForms);
    }
    let (other, end) = parse_sequence(chars, true)?;
    if end != End::Close {
        return Err(TemplateError::PluralForms);
    }
    Ok(Part::Plural {
        selector: name,
        one,
        other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(source: &str) -> Vec<String> {
        Template::parse(source)
            .unwrap()
            .placeholders()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn plain_text_and_placeholders_parse() {
        assert_eq!(Template::parse("").unwrap().parts, []);
        assert!(Template::parse("").unwrap().is_empty());
        assert_eq!(
            Template::parse("a {b} c").unwrap().parts,
            [
                Part::Literal("a ".into()),
                Part::Placeholder("b".into()),
                Part::Literal(" c".into()),
            ]
        );
        assert_eq!(names("{a} then {b} then {a}"), ["a", "b"]);
        assert!(names("no braces | a pipe is only text here").is_empty());
    }

    #[test]
    fn the_capital_marker_is_a_placeholder_that_capitalizes() {
        assert_eq!(
            Template::parse("{a^} then {b}").unwrap().parts,
            [
                Part::Capitalized("a".into()),
                Part::Literal(" then ".into()),
                Part::Placeholder("b".into()),
            ]
        );
        // Same names as without the marker: translations may use either.
        assert_eq!(names("{a^} and {a}"), ["a"]);
        assert_eq!(names("{n|{a^} has one|{a^} has {n}}"), ["a", "n"]);
    }

    #[test]
    fn plurals_hold_placeholders_and_count_toward_the_set() {
        assert_eq!(names("{n|a node|{n} nodes}"), ["n"]);
        assert_eq!(names("{n} {n|node|nodes}"), ["n"]);
        assert_eq!(
            names("{n|{site} has a port|{site} has {n} ports}"),
            ["n", "site"]
        );
        assert_eq!(names("{a|{b|x|y}|z}"), ["a", "b"]);
        let template = Template::parse("{n||s}").unwrap();
        assert_eq!(template.placeholders().len(), 1);
        assert_eq!(template.literals(), ["s"]);
    }

    #[test]
    fn malformed_templates_are_rejected_with_a_reason() {
        let cases = [
            ("Unclosed { brace", TemplateError::BadName(' ')),
            ("Unclosed {name", TemplateError::Unclosed),
            ("{n|one|other", TemplateError::Unclosed),
            ("{n|one", TemplateError::Unclosed),
            ("stray } brace", TemplateError::StrayClose),
            ("{}", TemplateError::EmptyName),
            ("{|a|b}", TemplateError::EmptyName),
            ("{Name}", TemplateError::BadName('N')),
            ("{a.b}", TemplateError::BadName('.')),
            ("{a{b}}", TemplateError::BadName('{')),
            ("{a^b}", TemplateError::BadName('^')),
            ("{^a}", TemplateError::BadName('^')),
            ("{a^^}", TemplateError::BadName('^')),
            ("{n^|a|b}", TemplateError::BadName('^')),
            ("{n|only}", TemplateError::PluralForms),
            ("{n|a|b|c}", TemplateError::PluralForms),
        ];
        for (source, expected) in cases {
            assert_eq!(Template::parse(source), Err(expected), "`{source}`");
        }
    }
}
