//! The glossary: one canonical word per concept and language, and the words that must
//! not be used instead.
//!
//! The canonical terms live in the catalogs, under `term.<id>`, so code cites them with
//! [`Arg::Term`](super::Arg::Term) and the parity between languages is checked like any other
//! text. `data/glossary.toml` adds what a catalog cannot say: what the concept means and which
//! variants are forbidden. [`Glossary::check`] runs over the catalogs in the tests; the
//! rules are written in `docs/spec/glossary.md`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use thiserror::Error;

use super::catalog::{Catalog, Lang};

include!(concat!(env!("OUT_DIR"), "/embedded_glossary.rs"));

/// Key prefix of the canonical terms in a catalog.
const TERM_PREFIX: &str = "term.";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    settings: RawSettings,
    #[serde(default)]
    concept: Vec<RawConcept>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSettings {
    narrative_prefixes: Vec<String>,
    exempt_prefixes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConcept {
    id: String,
    note: String,
    #[serde(default)]
    strict: bool,
    #[serde(default)]
    forbid: BTreeMap<String, Vec<String>>,
}

/// A problem in `glossary.toml` itself.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The file is not valid TOML or does not match the expected shape.
    #[error("glossary: {0}")]
    Toml(#[from] toml::de::Error),
    /// An id is not lowercase letters, digits and `_` (it becomes the key `term.<id>`).
    #[error("concept `{0}`: the id must be lowercase letters, digits and `_`")]
    BadId(String),
    /// Two concepts have the same id.
    #[error("concept `{0}` is declared twice")]
    DuplicateId(String),
    /// A `forbid` table names a language the game does not have.
    #[error("concept `{id}`: `{code}` is not a language of the game")]
    UnknownLanguage {
        /// The concept.
        id: String,
        /// The language code that was written.
        code: String,
    },
    /// A forbidden variant is empty or only spaces.
    #[error("concept `{0}` forbids an empty variant")]
    EmptyVariant(String),
}

/// One concept of the game vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    /// Stable id; the canonical term is the catalog key `term.<id>`.
    pub id: String,
    /// What the concept means, for authors.
    pub note: String,
    /// Whether the forbidden variants are also searched in narration texts.
    pub strict: bool,
    forbidden: Vec<(Lang, String)>,
}

impl Concept {
    /// The variants forbidden in a language, as written in the glossary.
    pub fn forbidden(&self, lang: Lang) -> impl Iterator<Item = &str> {
        self.forbidden
            .iter()
            .filter(move |(candidate, _)| *candidate == lang)
            .map(|(_, variant)| variant.as_str())
    }
}

/// The vocabulary of the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glossary {
    narrative_prefixes: Vec<String>,
    exempt_prefixes: Vec<String>,
    concepts: Vec<Concept>,
}

/// A rule broken between the glossary and the catalogs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Issue {
    /// A concept has no canonical term in a language.
    #[error("[{lang:?}] concept `{id}` has no `term.{id}` text")]
    MissingTerm {
        /// The language.
        lang: Lang,
        /// The concept.
        id: String,
    },
    /// A catalog defines a term that no concept declares.
    #[error("[{lang:?}] `{key}` is a term without a concept in the glossary")]
    UnknownTerm {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
    },
    /// Two concepts have the same canonical word in a language.
    #[error("[{lang:?}] concepts {ids:?} are all called `{word}`")]
    DuplicateTerm {
        /// The language.
        lang: Lang,
        /// The shared word.
        word: String,
        /// The concepts.
        ids: Vec<String>,
    },
    /// A forbidden variant is itself the canonical word of a concept.
    #[error("[{lang:?}] `{variant}`, forbidden for `{concept}`, is the term of `{owner}`")]
    ForbiddenIsCanonical {
        /// The language.
        lang: Lang,
        /// The concept that forbids the variant.
        concept: String,
        /// The variant.
        variant: String,
        /// The concept whose term it is.
        owner: String,
    },
    /// A term (or one of its variants) that is not a plain word: it has a placeholder or a
    /// plural, which [`Arg::Term`](super::Arg::Term) cannot fill, or it is empty.
    #[error("[{lang:?}] `{key}` must be a plain word, without placeholders")]
    TermNotPlain {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
    },
    /// A text uses a forbidden variant.
    #[error("[{lang:?}] `{key}` says `{variant}`: use the term `{concept}`")]
    Forbidden {
        /// The language.
        lang: Lang,
        /// The key.
        key: String,
        /// The concept whose term should be used.
        concept: String,
        /// The forbidden variant found.
        variant: String,
    },
}

impl Glossary {
    /// Reads a glossary file.
    ///
    /// # Errors
    ///
    /// [`ParseError`] when the file is malformed, repeats an id, or names an unknown
    /// language.
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        let raw: Raw = toml::from_str(source)?;
        let mut seen = BTreeSet::new();
        let mut concepts = Vec::with_capacity(raw.concept.len());
        for concept in raw.concept {
            let well_formed = !concept.id.is_empty()
                && concept
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
            if !well_formed {
                return Err(ParseError::BadId(concept.id));
            }
            if !seen.insert(concept.id.clone()) {
                return Err(ParseError::DuplicateId(concept.id));
            }
            let mut forbidden = Vec::new();
            for (code, variants) in concept.forbid {
                let Some(lang) = Lang::from_code(&code) else {
                    return Err(ParseError::UnknownLanguage {
                        id: concept.id,
                        code,
                    });
                };
                for variant in variants {
                    if variant.trim().is_empty() {
                        return Err(ParseError::EmptyVariant(concept.id));
                    }
                    forbidden.push((lang, variant));
                }
            }
            concepts.push(Concept {
                id: concept.id,
                note: concept.note,
                strict: concept.strict,
                forbidden,
            });
        }
        Ok(Self {
            narrative_prefixes: raw.settings.narrative_prefixes,
            exempt_prefixes: raw.settings.exempt_prefixes,
            concepts,
        })
    }

    /// The glossary embedded in the program (`data/glossary.toml`).
    ///
    /// # Errors
    ///
    /// [`ParseError`]; the tests make this impossible in a build that passes them.
    pub fn embedded() -> Result<Self, ParseError> {
        Self::parse(EMBEDDED_GLOSSARY)
    }

    /// Every concept, in file order.
    #[must_use]
    pub fn concepts(&self) -> &[Concept] {
        &self.concepts
    }

    /// Checks the catalogs against the glossary: each concept has a canonical term in
    /// every language, no term is unknown or shared, and no forbidden variant is used.
    #[must_use]
    pub fn check(&self, catalogs: &[Catalog]) -> Vec<Issue> {
        let mut issues = Vec::new();
        for catalog in catalogs {
            self.check_terms(catalog, &mut issues);
            self.check_variants(catalog, &mut issues);
        }
        issues
    }

    fn check_terms(&self, catalog: &Catalog, issues: &mut Vec<Issue>) {
        let lang = catalog.lang();
        let known: BTreeSet<&str> = self.concepts.iter().map(|c| c.id.as_str()).collect();
        let mut by_word: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for concept in &self.concepts {
            match term_text(catalog, &concept.id) {
                Some(word) if !word.is_empty() => by_word
                    .entry(word.to_lowercase())
                    .or_default()
                    .push(concept.id.clone()),
                // Present but not a word: reported below with the other malformed terms.
                Some(_) => {}
                None => issues.push(Issue::MissingTerm {
                    lang,
                    id: concept.id.clone(),
                }),
            }
        }
        for (key, template) in catalog.iter() {
            let Some(rest) = key.strip_prefix(TERM_PREFIX) else {
                continue;
            };
            let id = rest.split_once('@').map_or(rest, |(base, _)| base);
            if !known.contains(id) {
                issues.push(Issue::UnknownTerm {
                    lang,
                    key: key.to_owned(),
                });
            }
            if !template.placeholders().is_empty() || template.literals().concat().trim().is_empty()
            {
                issues.push(Issue::TermNotPlain {
                    lang,
                    key: key.to_owned(),
                });
            }
        }
        for concept in &self.concepts {
            for variant in concept.forbidden(lang) {
                if let Some(owner) = by_word.get(&variant.to_lowercase()) {
                    issues.push(Issue::ForbiddenIsCanonical {
                        lang,
                        concept: concept.id.clone(),
                        variant: variant.to_owned(),
                        owner: owner.join(", "),
                    });
                }
            }
        }
        for (word, ids) in by_word {
            if ids.len() > 1 {
                issues.push(Issue::DuplicateTerm { lang, word, ids });
            }
        }
    }

    fn check_variants(&self, catalog: &Catalog, issues: &mut Vec<Issue>) {
        let lang = catalog.lang();
        for (key, template) in catalog.iter() {
            // The canonical word itself is what the variants are measured against; its
            // `@sr` and `@ascii` variants are texts like any other and are searched.
            let canonical = key.starts_with(TERM_PREFIX) && !key.contains('@');
            if canonical || self.is_exempt(key) {
                continue;
            }
            let narrative = self.is_narrative(key);
            let literals: Vec<String> = template
                .literals()
                .into_iter()
                .map(str::to_lowercase)
                .collect();
            for concept in &self.concepts {
                if narrative && !concept.strict {
                    continue;
                }
                for variant in concept.forbidden(lang) {
                    let needle = variant.to_lowercase();
                    if literals.iter().any(|text| contains_word(text, &needle)) {
                        issues.push(Issue::Forbidden {
                            lang,
                            key: key.to_owned(),
                            concept: concept.id.clone(),
                            variant: variant.to_owned(),
                        });
                    }
                }
            }
        }
    }

    fn is_exempt(&self, key: &str) -> bool {
        self.exempt_prefixes
            .iter()
            .any(|prefix| key.starts_with(prefix.as_str()))
    }

    fn is_narrative(&self, key: &str) -> bool {
        self.narrative_prefixes
            .iter()
            .any(|prefix| key.starts_with(prefix.as_str()))
    }
}

/// The canonical word of a concept in a catalog.
fn term_text(catalog: &Catalog, id: &str) -> Option<String> {
    catalog
        .get(&format!("{TERM_PREFIX}{id}"))
        .map(|template| template.literals().concat())
}

/// Whether `needle` appears in `haystack` as whole words (both already lowercase): the
/// characters around it, if any, are not letters or digits.
fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(start, found)| {
        let before = haystack
            .get(..start)
            .and_then(|head| head.chars().next_back());
        let after = haystack
            .get(start + found.len()..)
            .and_then(|tail| tail.chars().next());
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

#[cfg(test)]
mod tests;
