//! One table for the commands of a game.
//!
//! Dispatch, help and completion all read the same [`Registry`], so a command cannot be
//! runnable but missing from the help, or completed but unknown. A game declares its commands
//! once, with the catalog key of their one-line help, the [`Context`] they belong to, who
//! handles them and the typed arguments they take, and says at each moment which ones are
//! [`Availability::Open`]. Three different answers exist for a command that cannot run:
//! unknown (no such command), locked (not yet) and wrong context (not here); the last two are
//! never listed or completed, so the player must not learn what is still hidden, but is never
//! left with "unknown command" when the name exists.
//!
//! The arguments are read once, by [`Registry::parse`], with a [`Resolver`] the game provides:
//! a command never sees an ambiguous or unknown word (docs/spec/commands.md, section 3).

use std::collections::BTreeSet;

use crate::event::{Event, Table};
use crate::text::{Catalog, Text};

mod resolve;

pub use resolve::{
    ArgError, ArgRef, Candidate, Listing, NoLists, Resolution, Resolver, Row, fold, resolve,
};

/// Where a command can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// Everywhere.
    Anywhere,
    /// At the hub (the hideout and the world), never during a run.
    Hub,
    /// During a run (an intrusion), never at the hub.
    Run,
}

impl Context {
    /// Whether a command declared for `self` can be used when the game is in `current`.
    #[must_use]
    pub fn serves(self, current: Self) -> bool {
        self == Self::Anywhere || self == current
    }
}

/// Which frontends can handle a frontend command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The plain frontend and the full-screen one.
    AnyFrontend,
    /// The full-screen frontend only (`panel`, `plain`).
    TuiOnly,
}

/// Who runs a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandledBy {
    /// The game: it changes the state.
    Engine,
    /// The frontend: it acts on the display or replaces the game, before the engine sees the
    /// line. The command is still in the table, so it is listed, completed and checked.
    Frontend(Scope),
}

/// What the running frontend can do. The table is filtered by it: a command scoped
/// [`Scope::TuiOnly`] does not exist for a plain frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// The frontend is the full-screen one.
    pub tui: bool,
}

impl Capabilities {
    /// The plain frontend.
    pub const PLAIN: Self = Self { tui: false };
    /// The full-screen frontend.
    pub const TUI: Self = Self { tui: true };
}

/// What an argument designates (docs/spec/commands.md, section 3). Except the first three
/// below, a kind names a list of the game: a number or a name of that list is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind {
    /// A plain decimal number, not a row of a list.
    Number,
    /// A quest of the journal.
    Quest,
    /// A contact.
    Contact,
    /// A message of the inbox.
    Message,
    /// A document of the archives.
    Document,
    /// A site of the net.
    Site,
    /// A node of the run graph.
    Node,
    /// A program of the deck.
    Program,
    /// An item (equipment, something to buy).
    Item,
    /// A save slot.
    Slot,
    /// A service that lowers notoriety.
    Service,
    /// One of a fixed set of words (`brief`, `normal`, `full`), resolved by the engine.
    Word(&'static [&'static str]),
    /// The rest of the line, verbatim: spaces kept, no case or accent folding, no number, no
    /// completion by the engine. It must be the last argument.
    Path,
}

impl ArgKind {
    /// The glossary term that names this kind in a message (`term.<id>`).
    #[must_use]
    pub fn term(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Quest => "quest",
            Self::Contact => "contact",
            Self::Message => "message",
            Self::Document => "document",
            Self::Site => "site",
            Self::Node => "node",
            Self::Program => "program",
            Self::Item => "item",
            Self::Slot => "slot",
            Self::Service => "service",
            Self::Word(_) => "word",
            Self::Path => "path",
        }
    }
}

/// One argument of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgSpec {
    /// What it designates.
    pub kind: ArgKind,
    /// The command works without it. Optional arguments come after the required ones.
    pub optional: bool,
}

impl ArgSpec {
    /// A required argument.
    #[must_use]
    pub const fn required(kind: ArgKind) -> Self {
        Self {
            kind,
            optional: false,
        }
    }

    /// An optional argument.
    #[must_use]
    pub const fn optional(kind: ArgKind) -> Self {
        Self {
            kind,
            optional: true,
        }
    }
}

/// A command, as declared by a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// The official name: lowercase ASCII letters, digits, `-` and `_`.
    pub name: &'static str,
    /// Other names that run the same command, with the same rules.
    pub aliases: &'static [&'static str],
    /// Catalog key of the one-line help. The key of its usage is this one plus `.usage`.
    pub help: &'static str,
    /// Where it can be used.
    pub context: Context,
    /// Who runs it.
    pub handled_by: HandledBy,
    /// The arguments it reads, in order.
    pub args: &'static [ArgSpec],
}

impl CommandSpec {
    /// Catalog key of the usage, which every command with arguments must have
    /// (`help.breach` has `help.breach.usage`).
    #[must_use]
    pub fn usage_key(&self) -> String {
        format!("{}.usage", self.help)
    }

    /// Why this command does not exist here, if it does not: the frontend cannot do it, or
    /// the game is in another context.
    fn refusal(&self, context: Context, capabilities: Capabilities) -> Option<Text> {
        if self.handled_by == HandledBy::Frontend(Scope::TuiOnly) && !capabilities.tui {
            return Some(Text::new("error.wrong_context.tui"));
        }
        if self.context.serves(context) {
            return None;
        }
        Some(match self.context {
            Context::Run => Text::new("error.wrong_context.run").with_term("run", "run"),
            Context::Hub | Context::Anywhere => {
                Text::new("error.wrong_context.hub").with_term("run", "run")
            }
        })
    }
}

/// Whether a command can be used right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Listed, completed, and runnable.
    Open,
    /// Known, but not yet: the text tells the player how it opens.
    Locked(Text),
}

/// What a typed line means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// Nothing typed.
    Empty,
    /// An open command of this context, with its arguments read and resolved.
    Found {
        /// The command.
        spec: &'static CommandSpec,
        /// One per argument present, in order. An optional argument that was not typed is
        /// absent; surplus words after the declared arguments are ignored.
        args: Vec<ArgRef>,
    },
    /// A known command that is not open yet.
    Locked {
        /// The command.
        spec: &'static CommandSpec,
        /// How it opens.
        reason: Text,
    },
    /// A known command of another context, or one this frontend cannot do.
    WrongContext {
        /// The command.
        spec: &'static CommandSpec,
        /// Why not here (`error.wrong_context.*`).
        reason: Text,
    },
    /// An open command whose argument is missing, unknown, ambiguous or refused.
    BadArg {
        /// The command.
        spec: &'static CommandSpec,
        /// What is wrong; [`ArgError::text`] says it.
        error: ArgError,
    },
    /// Not a command of this game; the first word, lowercased, for the message.
    Unknown(String),
}

/// A problem in a command table, found by [`Registry::issues`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    /// A name is empty or has a character outside `a-z`, `0-9`, `-` and `_`.
    BadName(&'static str),
    /// A name or alias is used twice, by one command or by two.
    Duplicate(&'static str),
    /// The command has an `ArgKind::Word` with no word.
    EmptyWordSet(&'static str),
    /// The command has a required argument after an optional one.
    RequiredAfterOptional(&'static str),
    /// The command has an `ArgKind::Path` that is not its last argument.
    PathNotLast(&'static str),
}

/// A text missing from a catalog, found by [`Registry::catalog_issues`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogIssue {
    /// The one-line help of a command has no text.
    MissingHelp(String),
    /// A command with arguments has no usage text.
    MissingUsage(String),
}

/// The commands of a game.
#[derive(Debug, Clone, Copy)]
pub struct Registry {
    specs: &'static [CommandSpec],
}

impl Registry {
    /// Wraps a table. The order of the table is the order of the help and of completions.
    #[must_use]
    pub const fn new(specs: &'static [CommandSpec]) -> Self {
        Self { specs }
    }

    /// Every declared command.
    #[must_use]
    pub fn specs(&self) -> &'static [CommandSpec] {
        self.specs
    }

    fn find(&self, word: &str) -> Option<&'static CommandSpec> {
        self.specs
            .iter()
            .find(|spec| spec.name == word || spec.aliases.contains(&word))
    }

    /// Whether the command is listed, completed and runnable here and now.
    fn is_listed(
        spec: &CommandSpec,
        context: Context,
        capabilities: Capabilities,
        availability: &impl Fn(&CommandSpec) -> Availability,
    ) -> bool {
        spec.refusal(context, capabilities).is_none() && availability(spec) == Availability::Open
    }

    /// Reads a typed line: the first word names the command, whatever its case, and the
    /// next words are its arguments, resolved with `resolver`.
    ///
    /// The answers are checked in this order: unknown name, wrong context (or a command the
    /// frontend cannot do), locked, then the arguments from left to right.
    pub fn parse(
        &self,
        line: &str,
        context: Context,
        capabilities: Capabilities,
        availability: impl Fn(&CommandSpec) -> Availability,
        resolver: &impl Resolver,
    ) -> Lookup {
        let line = line.trim_start();
        if line.is_empty() {
            return Lookup::Empty;
        }
        let (first, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let word = first.to_ascii_lowercase();
        let Some(spec) = self.find(&word) else {
            return Lookup::Unknown(word);
        };
        if let Some(reason) = spec.refusal(context, capabilities) {
            return Lookup::WrongContext { spec, reason };
        }
        if let Availability::Locked(reason) = availability(spec) {
            return Lookup::Locked { spec, reason };
        }
        match read_args(spec, rest, resolver) {
            Ok(args) => Lookup::Found { spec, args },
            Err(error) => Lookup::BadArg { spec, error },
        }
    }

    /// What can complete the start of a command: open official names first, and aliases
    /// only when no official name fits (so `s` never lists three spellings of one thing).
    /// A command of another context, or that the frontend cannot do, is not proposed.
    pub fn complete(
        &self,
        prefix: &str,
        context: Context,
        capabilities: Capabilities,
        availability: impl Fn(&CommandSpec) -> Availability,
    ) -> Vec<String> {
        let prefix = prefix.to_ascii_lowercase();
        let listed: Vec<&CommandSpec> = self
            .specs
            .iter()
            .filter(|spec| Self::is_listed(spec, context, capabilities, &availability))
            .collect();
        let names: Vec<String> = listed
            .iter()
            .filter(|spec| spec.name.starts_with(&prefix))
            .map(|spec| spec.name.to_owned())
            .collect();
        if !names.is_empty() {
            return names;
        }
        listed
            .iter()
            .flat_map(|spec| spec.aliases.iter())
            .filter(|alias| alias.starts_with(&prefix))
            .map(|alias| (*alias).to_owned())
            .collect()
    }

    /// What can complete the word being typed after a command: `line` is everything typed so
    /// far (`"breach 3 br"`, or `"breach 3 "` for a new word). Only things that can be used
    /// now are proposed, never a locked or unknown one, official names first and aliases only
    /// when no official name fits. A path gets nothing (the frontend completes files), nor a
    /// number, nor an argument past the last one, nor a command that is not listed here.
    pub fn complete_args(
        &self,
        line: &str,
        context: Context,
        capabilities: Capabilities,
        availability: impl Fn(&CommandSpec) -> Availability,
        resolver: &impl Resolver,
    ) -> Vec<String> {
        let Some((first, rest)) = line.trim_start().split_once(char::is_whitespace) else {
            return Vec::new();
        };
        let Some(spec) = self.find(&first.to_ascii_lowercase()) else {
            return Vec::new();
        };
        if !Self::is_listed(spec, context, capabilities, &availability) {
            return Vec::new();
        }
        let mut done: Vec<&str> = rest.split_whitespace().collect();
        let partial = if rest.ends_with(char::is_whitespace) {
            ""
        } else {
            done.pop().unwrap_or("")
        };
        let Some(arg) = spec.args.get(done.len()) else {
            return Vec::new();
        };
        let partial = fold(partial);
        match arg.kind {
            ArgKind::Number | ArgKind::Path => Vec::new(),
            ArgKind::Word(set) => set
                .iter()
                .filter(|word| fold(word).starts_with(&partial))
                .map(|word| (*word).to_owned())
                .collect(),
            kind => {
                let listing = resolver.list(kind, spec);
                let open: Vec<&Row> = listing
                    .rows
                    .iter()
                    .filter(|row| row.available.is_ok())
                    .collect();
                let fits = |name: &&String| fold(name).starts_with(&partial);
                let official: Vec<String> = open
                    .iter()
                    .filter_map(|row| row.names.first())
                    .filter(fits)
                    .cloned()
                    .collect();
                if !official.is_empty() {
                    return official;
                }
                open.iter()
                    .flat_map(|row| row.names.iter().skip(1))
                    .filter(fits)
                    .cloned()
                    .collect()
            }
        }
    }

    /// The help screen: exactly the commands listed here and now, one row each.
    pub fn help(
        &self,
        title: Text,
        columns: &[Text; 2],
        context: Context,
        capabilities: Capabilities,
        availability: impl Fn(&CommandSpec) -> Availability,
    ) -> Event {
        let rows = self
            .specs
            .iter()
            .filter(|spec| Self::is_listed(spec, context, capabilities, &availability))
            .map(|spec| vec![Text::raw(spec.name), Text::dynamic(spec.help.to_owned())])
            .collect();
        Event::Screen(Table {
            title,
            columns: columns.to_vec(),
            rows,
        })
    }

    /// Problems in the table: bad or repeated names, arguments in the wrong order, a word
    /// set with no word. Games test that this is empty.
    #[must_use]
    pub fn issues(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        let mut seen = BTreeSet::new();
        for spec in self.specs {
            for name in std::iter::once(&spec.name).chain(spec.aliases) {
                let well_formed = !name.is_empty()
                    && name.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'
                    });
                if !well_formed {
                    issues.push(Issue::BadName(name));
                }
                if !seen.insert(*name) {
                    issues.push(Issue::Duplicate(name));
                }
            }
            check_args(spec, &mut issues);
        }
        issues
    }

    /// The texts a catalog must have for this table: the help of every command and the usage
    /// of every command with arguments. Separate from [`Registry::issues`] because it needs
    /// the catalogs; a game checks it for each language.
    #[must_use]
    pub fn catalog_issues(&self, catalog: &Catalog) -> Vec<CatalogIssue> {
        let mut issues = Vec::new();
        for spec in self.specs {
            if catalog.get(spec.help).is_none() {
                issues.push(CatalogIssue::MissingHelp(spec.help.to_owned()));
            }
            let usage = spec.usage_key();
            if !spec.args.is_empty() && catalog.get(&usage).is_none() {
                issues.push(CatalogIssue::MissingUsage(usage));
            }
        }
        issues
    }

    /// The catalog keys of every help line, for the tests that check the catalogs.
    pub fn help_keys(&self) -> impl Iterator<Item = &'static str> {
        self.specs.iter().map(|spec| spec.help)
    }
}

fn check_args(spec: &CommandSpec, issues: &mut Vec<Issue>) {
    if spec
        .args
        .iter()
        .any(|arg| matches!(arg.kind, ArgKind::Word([])))
    {
        issues.push(Issue::EmptyWordSet(spec.name));
    }
    let mut optional_seen = false;
    for arg in spec.args {
        if optional_seen && !arg.optional {
            issues.push(Issue::RequiredAfterOptional(spec.name));
            break;
        }
        optional_seen |= arg.optional;
    }
    let last = spec.args.len().saturating_sub(1);
    if spec
        .args
        .iter()
        .enumerate()
        .any(|(index, arg)| arg.kind == ArgKind::Path && index != last)
    {
        issues.push(Issue::PathNotLast(spec.name));
    }
}

/// Reads the words after a command name, one per declared argument.
fn read_args(
    spec: &'static CommandSpec,
    line: &str,
    resolver: &impl Resolver,
) -> Result<Vec<ArgRef>, ArgError> {
    let mut rest = line;
    let mut args = Vec::new();
    for arg in spec.args {
        rest = rest.trim_start();
        if rest.is_empty() {
            if arg.optional {
                break;
            }
            return Err(ArgError::Missing {
                usage: Text::dynamic(spec.usage_key()),
            });
        }
        if arg.kind == ArgKind::Path {
            args.push(ArgRef::Path(rest.trim_end().to_owned()));
            break;
        }
        let (word, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        rest = tail;
        args.push(read_word(spec, arg.kind, word, resolver)?);
    }
    Ok(args)
}

/// Resolves one typed word for one argument.
fn read_word(
    spec: &CommandSpec,
    kind: ArgKind,
    word: &str,
    resolver: &impl Resolver,
) -> Result<ArgRef, ArgError> {
    let unknown = |see: &str| ArgError::Unknown {
        kind,
        word: word.to_owned(),
        see: see.to_owned(),
    };
    match kind {
        ArgKind::Number => word
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| word.parse::<u32>().ok())
            .flatten()
            .map(ArgRef::Number)
            .ok_or_else(|| unknown(&format!("help {}", spec.name))),
        ArgKind::Word(set) => match resolve::resolve_fixed(word, set) {
            Ok(index) => set
                .get(index)
                .copied()
                .map(ArgRef::Word)
                .ok_or_else(|| unknown(&format!("help {}", spec.name))),
            Err(indexes) if indexes.is_empty() => Err(unknown(&format!("help {}", spec.name))),
            Err(indexes) => Err(ArgError::Ambiguous {
                kind,
                word: word.to_owned(),
                candidates: indexes
                    .into_iter()
                    .filter_map(|index| set.get(index))
                    .map(|fixed| Candidate {
                        number: None,
                        name: (*fixed).to_owned(),
                    })
                    .collect(),
            }),
        },
        // `read_args` takes the whole rest of the line for a path before it gets here.
        ArgKind::Path => Ok(ArgRef::Path(word.to_owned())),
        listed => {
            let listing = resolver.list(listed, spec);
            match resolve(word, &listing.rows) {
                Resolution::Unknown => Err(unknown(listing.see)),
                Resolution::Ambiguous(numbers) => Err(ArgError::Ambiguous {
                    kind,
                    word: word.to_owned(),
                    candidates: numbers
                        .into_iter()
                        .filter_map(|number| {
                            let row = listing.rows.get(number.checked_sub(1)?)?;
                            Some(Candidate {
                                number: Some(number),
                                name: row.display().to_owned(),
                            })
                        })
                        .collect(),
                }),
                Resolution::Found(number) => {
                    let Some(row) = number.checked_sub(1).and_then(|i| listing.rows.get(i)) else {
                        return Err(unknown(listing.see));
                    };
                    match &row.available {
                        Ok(()) => Ok(ArgRef::Listed {
                            kind,
                            number,
                            id: row.id.clone(),
                        }),
                        Err(reason) => Err(ArgError::Unavailable {
                            name: row.display().to_owned(),
                            reason: reason.clone(),
                        }),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
