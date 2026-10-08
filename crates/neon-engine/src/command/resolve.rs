//! Resolving a typed word into a thing of a list (docs/spec/commands.md, section 3).
//!
//! A game implements [`Resolver`]: for each kind of argument it says which things are listed
//! on screen right now, in the order of the list, and which of them can be used. One function
//! of the engine, [`resolve`], then reads a typed word the same way for every command:
//! a number, an exact name, a unique prefix, otherwise ambiguous or unknown. Case and accents
//! never matter.
//!
//! Numbering belongs to the game: row `n` of the listing is the thing the screen shows as
//! `n`, and a thing that cannot be used right now stays listed with its reason, so a number
//! never moves while its list exists (rule 5 of the spec).

use crate::text::{Text, to_ascii};

use super::{ArgKind, CommandSpec};

/// One listed thing, as the player sees it in a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Stable identifier of the thing in the game (never shown, never typed).
    pub id: String,
    /// The words that name it: the official one first, then aliases. Each is one word of
    /// input (no whitespace); the display name may have several words, the input has one.
    pub names: Vec<String>,
    /// `Err` carries the reason, said in words, why the thing cannot be used by the command
    /// now. The row stays listed and keeps its number.
    pub available: Result<(), Text>,
}

impl Row {
    /// A usable thing.
    #[must_use]
    pub fn open(id: impl Into<String>, names: &[&str]) -> Self {
        Self {
            id: id.into(),
            names: names.iter().map(|name| (*name).to_owned()).collect(),
            available: Ok(()),
        }
    }

    /// A thing that is listed but cannot be used now.
    #[must_use]
    pub fn closed(id: impl Into<String>, names: &[&str], reason: Text) -> Self {
        Self {
            available: Err(reason),
            ..Self::open(id, names)
        }
    }

    /// What to call it in a message: its official name, or its id when it has no name.
    #[must_use]
    pub fn display(&self) -> &str {
        self.names.first().map_or(self.id.as_str(), String::as_str)
    }
}

/// The things of one kind that a command can name right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The command that shows this list, for the "See `map`." of an unknown answer.
    pub see: &'static str,
    /// The rows, in the order (and with the numbers: row 1 is number 1) of the list.
    pub rows: Vec<Row>,
}

/// What a game knows about the things its commands can name.
pub trait Resolver {
    /// The things of `kind` as listed for `command`. The same thing may be open for one
    /// command and closed for another (`equip` and `buy` do not refuse the same items), but
    /// it keeps its number. Only called for the listed kinds: numbers, words and paths are
    /// read by the engine.
    fn list(&self, kind: ArgKind, command: &CommandSpec) -> Listing;
}

/// The resolver of a game that has no list: every list is empty.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLists;

impl Resolver for NoLists {
    fn list(&self, _kind: ArgKind, _command: &CommandSpec) -> Listing {
        Listing {
            see: "help",
            rows: Vec::new(),
        }
    }
}

/// How one typed word resolved against a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The number (1-based) of the one row it designates.
    Found(usize),
    /// Several rows fit; their numbers, in list order.
    Ambiguous(Vec<usize>),
    /// Nothing fits.
    Unknown,
}

/// A thing among the several a word fits, for the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Its number in the list, when the kind has numbers (a fixed word has none).
    pub number: Option<usize>,
    /// The name to type.
    pub name: String,
}

/// An argument that was read and resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgRef {
    /// A plain number (`ArgKind::Number`).
    Number(u32),
    /// One of the fixed words of an `ArgKind::Word`, as declared (not as typed).
    Word(&'static str),
    /// The rest of the line, verbatim (`ArgKind::Path`).
    Path(String),
    /// A thing of a list.
    Listed {
        /// Its kind.
        kind: ArgKind,
        /// Its number in the list (1-based).
        number: usize,
        /// Its identifier in the game.
        id: String,
    },
}

impl ArgRef {
    /// The identifier, for a thing of a list.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Listed { id, .. } => Some(id),
            _ => None,
        }
    }

    /// The number, for a plain number.
    #[must_use]
    pub fn as_number(&self) -> Option<u32> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// The fixed word, for a word.
    #[must_use]
    pub fn as_word(&self) -> Option<&'static str> {
        match self {
            Self::Word(word) => Some(word),
            _ => None,
        }
    }

    /// The text, for a path.
    #[must_use]
    pub fn as_path(&self) -> Option<&str> {
        match self {
            Self::Path(path) => Some(path),
            _ => None,
        }
    }
}

/// Why an argument could not be used. [`ArgError::text`] says it in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgError {
    /// A required argument is absent.
    Missing {
        /// The usage of the command (`help.<command>.usage`).
        usage: Text,
    },
    /// The word names nothing of the list.
    Unknown {
        /// What was expected.
        kind: ArgKind,
        /// The word as typed.
        word: String,
        /// The command that lists the things, to look there.
        see: String,
    },
    /// The word fits several things.
    Ambiguous {
        /// What was expected.
        kind: ArgKind,
        /// The word as typed.
        word: String,
        /// The things it fits.
        candidates: Vec<Candidate>,
    },
    /// The thing exists but the command refuses it now.
    Unavailable {
        /// The name of the thing.
        name: String,
        /// Why, in words.
        reason: Text,
    },
}

impl ArgError {
    /// The error as a text (`error.arg.*`), to send as an `Error` event.
    #[must_use]
    pub fn text(&self) -> Text {
        match self {
            Self::Missing { usage } => {
                Text::new("error.arg.missing").with_text("usage", usage.clone())
            }
            Self::Unknown { kind, word, see } => Text::new("error.arg.unknown")
                .with_term("kind", kind.term())
                .with_str("value", word.as_str())
                .with_str("see", see.as_str()),
            Self::Ambiguous {
                kind,
                word,
                candidates,
            } => Text::new("error.arg.ambiguous")
                .with_term("kind", kind.term())
                .with_str("value", word.as_str())
                .with_str("candidates", list_candidates(candidates)),
            Self::Unavailable { name, reason } => Text::new("error.arg.unavailable")
                .with_str("name", name.as_str())
                .with_text("reason", reason.clone()),
        }
    }
}

/// `2. spoof, 3. stealth`: the names are input, so they are not translated.
fn list_candidates(candidates: &[Candidate]) -> String {
    let parts: Vec<String> = candidates
        .iter()
        .map(|candidate| match candidate.number {
            Some(number) => format!("{number}. {}", candidate.name),
            None => candidate.name.clone(),
        })
        .collect();
    parts.join(", ")
}

/// The text without case or accents, for comparing what a player types with a name.
///
/// A letter the game cannot transliterate stays as it is (lowercased) instead of becoming a
/// `?`, so two different foreign words are never equal.
#[must_use]
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c.to_ascii_lowercase());
            continue;
        }
        let plain = to_ascii(c.encode_utf8(&mut [0; 4]));
        if plain == "?" {
            out.extend(c.to_lowercase());
        } else {
            out.push_str(&plain.to_ascii_lowercase());
        }
    }
    out
}

/// Reads one typed word against a list: a number, an exact name, a unique prefix, otherwise
/// ambiguous or unknown (section 3 of the spec). Case and accents are ignored; a row
/// matches through any of its names, and counts once.
#[must_use]
pub fn resolve(word: &str, rows: &[Row]) -> Resolution {
    if !word.is_empty() && word.bytes().all(|b| b.is_ascii_digit()) {
        return match word.parse::<usize>() {
            Ok(number) if (1..=rows.len()).contains(&number) => Resolution::Found(number),
            _ => Resolution::Unknown,
        };
    }
    let names: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.names.iter().map(|name| fold(name)).collect())
        .collect();
    match pick(&fold(word), &names) {
        Pick::One(index) => Resolution::Found(index + 1),
        Pick::Many(indexes) => {
            Resolution::Ambiguous(indexes.into_iter().map(|index| index + 1).collect())
        }
        Pick::Nothing => Resolution::Unknown,
    }
}

/// Reads one typed word against the fixed words of an `ArgKind::Word`: the same steps
/// without numbers. `Ok` is the index in `set`.
pub(super) fn resolve_fixed(word: &str, set: &[&str]) -> Result<usize, Vec<usize>> {
    let names: Vec<Vec<String>> = set.iter().map(|name| vec![fold(name)]).collect();
    match pick(&fold(word), &names) {
        Pick::One(index) => Ok(index),
        Pick::Many(indexes) => Err(indexes),
        Pick::Nothing => Err(Vec::new()),
    }
}

enum Pick {
    One(usize),
    Many(Vec<usize>),
    Nothing,
}

/// Exact name first, then prefix; `names[i]` are the folded names of candidate `i`.
fn pick(word: &str, names: &[Vec<String>]) -> Pick {
    if word.is_empty() {
        return Pick::Nothing;
    }
    let matching = |fits: &dyn Fn(&str) -> bool| -> Vec<usize> {
        names
            .iter()
            .enumerate()
            .filter(|(_, own)| own.iter().any(|name| fits(name)))
            .map(|(index, _)| index)
            .collect()
    };
    let mut found = matching(&|name| name == word);
    if found.is_empty() {
        found = matching(&|name| name.starts_with(word));
    }
    match found.as_slice() {
        [] => Pick::Nothing,
        [one] => Pick::One(*one),
        _ => Pick::Many(found),
    }
}
