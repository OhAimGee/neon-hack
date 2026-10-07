//! Catalogs: the texts of one language, loaded from TOML files.
//!
//! Files are tables of strings, flattened to dotted keys: `[ui.tag]` with
//! `alert = "[ALERT]"` is the key `ui.tag.alert`. A key may have variants that a mode
//! prefers over the base text: `key@sr` for screen readers, `key@ascii` for ASCII output.
//! The engine does no I/O: it receives the contents of the files as strings, and the
//! embedded ones are compiled in by the build script (see `build.rs`).

use std::collections::BTreeMap;
use std::fmt;

use thiserror::Error;

use super::template::{Template, TemplateError};

include!(concat!(env!("OUT_DIR"), "/embedded_text.rs"));

/// A language of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {
    /// English.
    En,
    /// French.
    Fr,
}

/// A plural category. Only the two forms every supported language needs are modelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralForm {
    /// The singular form.
    One,
    /// The plural form.
    Other,
}

impl Lang {
    /// Every language, in the order they are listed to the player.
    pub const ALL: [Self; 2] = [Self::En, Self::Fr];

    /// The ISO 639-1 code, which is also the name of the directory under `data/text/`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
        }
    }

    /// The language of a code, as `code` writes it.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|lang| lang.code() == code)
    }

    /// Which plural form a number takes. French has the singular for 0 and 1 ("0 crédit"),
    /// English for 1 only; the sign does not matter.
    #[must_use]
    pub fn plural(self, count: i64) -> PluralForm {
        let magnitude = count.unsigned_abs();
        let singular = match self {
            Self::En => magnitude == 1,
            Self::Fr => magnitude <= 1,
        };
        if singular {
            PluralForm::One
        } else {
            PluralForm::Other
        }
    }
}

/// A problem found while loading a catalog.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LoadError {
    /// The file is not valid TOML.
    #[error("{file}: {message}")]
    Syntax {
        /// File label.
        file: String,
        /// The parser's explanation, with line and column.
        message: String,
    },
    /// A value is not a string (a number, a list, a date).
    #[error("{file}: `{key}` is not a string")]
    NotText {
        /// File label.
        file: String,
        /// The offending key.
        key: String,
    },
    /// A template does not parse.
    #[error("{file}: `{key}`: {source}")]
    Template {
        /// File label.
        file: String,
        /// The offending key.
        key: String,
        /// What is wrong with it.
        source: TemplateError,
    },
    /// A key is defined twice, in one file or in two.
    #[error("{file}: `{key}` is already defined (in {first})")]
    Duplicate {
        /// File of the second definition.
        file: String,
        /// The key.
        key: String,
        /// File of the first definition.
        first: String,
    },
}

/// Every problem of a catalog, so a writer fixes them in one pass.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub struct LoadErrors(pub Vec<LoadError>);

impl fmt::Display for LoadErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

/// The texts of one language: templates by key.
#[derive(Debug, Clone)]
pub struct Catalog {
    lang: Lang,
    entries: BTreeMap<String, Template>,
}

impl Catalog {
    /// Builds a catalog from TOML sources, each given as `(file label, contents)`.
    ///
    /// # Errors
    ///
    /// Every [`LoadError`] found, not only the first.
    pub fn from_sources(lang: Lang, sources: &[(&str, &str)]) -> Result<Self, LoadErrors> {
        let mut entries = BTreeMap::new();
        let mut origins: BTreeMap<String, String> = BTreeMap::new();
        let mut errors = Vec::new();
        for (file, contents) in sources {
            let table = match contents.parse::<toml::Table>() {
                Ok(table) => table,
                Err(error) => {
                    errors.push(LoadError::Syntax {
                        file: (*file).to_owned(),
                        message: error.to_string().trim_end().to_owned(),
                    });
                    continue;
                }
            };
            let mut flat = Vec::new();
            flatten(&table, "", &mut flat);
            for (key, value) in flat {
                // The first definition owns the key even when it is itself broken, so a
                // duplicate is reported in the same pass as that problem.
                if let Some(first) = origins.get(&key) {
                    errors.push(LoadError::Duplicate {
                        file: (*file).to_owned(),
                        first: first.clone(),
                        key,
                    });
                    continue;
                }
                origins.insert(key.clone(), (*file).to_owned());
                let toml::Value::String(source) = value else {
                    errors.push(LoadError::NotText {
                        file: (*file).to_owned(),
                        key,
                    });
                    continue;
                };
                match Template::parse(&source) {
                    Ok(template) => {
                        entries.insert(key, template);
                    }
                    Err(error) => errors.push(LoadError::Template {
                        file: (*file).to_owned(),
                        key,
                        source: error,
                    }),
                }
            }
        }
        if errors.is_empty() {
            Ok(Self { lang, entries })
        } else {
            Err(LoadErrors(errors))
        }
    }

    /// The catalog compiled into the program for a language.
    ///
    /// # Errors
    ///
    /// [`LoadErrors`] if an embedded file is malformed; the tests of this crate make that
    /// impossible in a build that passes them.
    pub fn embedded(lang: Lang) -> Result<Self, LoadErrors> {
        let sources: Vec<(&str, &str)> = EMBEDDED_TEXT
            .iter()
            .filter(|(code, _, _)| *code == lang.code())
            .map(|(_, file, contents)| (*file, *contents))
            .collect();
        Self::from_sources(lang, &sources)
    }

    /// The language of this catalog.
    #[must_use]
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// The template of a key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Template> {
        self.entries.get(key)
    }

    /// Every key with its template, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Template)> {
        self.entries
            .iter()
            .map(|(key, template)| (key.as_str(), template))
    }

    /// How many texts the catalog holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the catalog holds no text.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Walks nested tables, collecting `(dotted key, value)` for every leaf.
fn flatten(table: &toml::Table, prefix: &str, out: &mut Vec<(String, toml::Value)>) {
    for (name, value) in table {
        let key = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        match value {
            toml::Value::Table(inner) => flatten(inner, &key, out),
            leaf => out.push((key, leaf.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(source: &str) -> Result<Catalog, LoadErrors> {
        Catalog::from_sources(Lang::En, &[("test.toml", source)])
    }

    #[test]
    fn nested_tables_flatten_to_dotted_keys_and_variants_are_plain_keys() {
        let catalog = load(
            r#"
            [ui]
            say = "{speaker}: {text}"
            "say@sr" = "{speaker} says {text}"
            [ui.tag]
            alert = "[ALERT]"
            "#,
        )
        .unwrap();
        let keys: Vec<&str> = catalog.iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["ui.say", "ui.say@sr", "ui.tag.alert"]);
        assert_eq!(catalog.len(), 3);
        assert!(catalog.get("ui.tag.alert").is_some());
        assert!(catalog.get("ui.tag").is_none());
    }

    #[test]
    fn every_problem_is_reported_in_one_pass() {
        let errors = Catalog::from_sources(
            Lang::En,
            &[
                ("a.toml", "good = \"ok\"\nbroken = \"{name\"\ncount = 3\n"),
                (
                    "b.toml",
                    "broken = \"fine\"\ngood = \"again\"\nlist = [\"x\"]\n",
                ),
                ("c.toml", "not toml at all ==="),
            ],
        )
        .unwrap_err()
        .0;
        assert_eq!(errors.len(), 6, "{errors:#?}");
        assert!(errors.contains(&LoadError::Template {
            file: "a.toml".into(),
            key: "broken".into(),
            source: TemplateError::Unclosed,
        }));
        assert!(errors.contains(&LoadError::NotText {
            file: "a.toml".into(),
            key: "count".into(),
        }));
        assert!(errors.contains(&LoadError::Duplicate {
            file: "b.toml".into(),
            key: "good".into(),
            first: "a.toml".into(),
        }));
        assert!(errors.contains(&LoadError::NotText {
            file: "b.toml".into(),
            key: "list".into(),
        }));
        assert!(
            errors.contains(&LoadError::Duplicate {
                file: "b.toml".into(),
                key: "broken".into(),
                first: "a.toml".into(),
            }),
            "a duplicate of a broken text is reported in the same pass"
        );
        assert!(matches!(&errors[5], LoadError::Syntax { file, .. } if file == "c.toml"));
        let shown = LoadErrors(errors).to_string();
        assert!(
            shown.contains("a.toml: `broken`: a `{` is never closed"),
            "{shown}"
        );
    }

    #[test]
    fn plural_rules_follow_each_language() {
        use PluralForm::{One, Other};
        for (count, english, french) in [
            (0, Other, One),
            (1, One, One),
            (-1, One, One),
            (2, Other, Other),
            (100, Other, Other),
            (i64::MIN, Other, Other),
        ] {
            assert_eq!(Lang::En.plural(count), english, "en {count}");
            assert_eq!(Lang::Fr.plural(count), french, "fr {count}");
        }
    }

    #[test]
    fn every_embedded_file_belongs_to_a_known_language_and_every_language_has_files() {
        for (code, file, _) in EMBEDDED_TEXT {
            assert!(
                Lang::from_code(code).is_some(),
                "{code}/{file}: unknown language"
            );
        }
        for lang in Lang::ALL {
            let files: Vec<&str> = EMBEDDED_TEXT
                .iter()
                .filter(|(code, _, _)| *code == lang.code())
                .map(|(_, file, _)| *file)
                .collect();
            assert!(files.contains(&"ui.toml"), "{lang:?}: {files:?}");
        }
    }

    #[test]
    fn language_codes_round_trip() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Lang::from_code("de"), None);
    }
}
