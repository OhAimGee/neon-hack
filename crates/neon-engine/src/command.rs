//! One table for the commands of a game.
//!
//! Dispatch, help and completion all read the same [`Registry`], so a command cannot be
//! runnable but missing from the help, or completed but unknown. A game declares its commands
//! once, with the catalog key of their one-line help, and says at each moment which ones are
//! [`Availability::Open`]: a locked command answers differently from an unknown one, and is
//! never listed or completed (the player must not learn what is still hidden).

use std::collections::BTreeSet;

use crate::event::{Event, Table};
use crate::text::Text;

/// A command, as declared by a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// The official name: lowercase ASCII letters, digits, `-` and `_`.
    pub name: &'static str,
    /// Other names that run the same command, with the same rules.
    pub aliases: &'static [&'static str],
    /// Catalog key of the one-line help.
    pub help: &'static str,
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
pub enum Lookup<'a> {
    /// Nothing typed.
    Empty,
    /// An open command, with the words that follow it.
    Found {
        /// The command.
        spec: &'static CommandSpec,
        /// The remaining words of the line.
        args: Vec<&'a str>,
    },
    /// A known command that is not open yet.
    Locked {
        /// The command.
        spec: &'static CommandSpec,
        /// How it opens.
        reason: Text,
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

    /// Reads a typed line: the first word names the command, whatever its case.
    pub fn parse<'a>(
        &self,
        line: &'a str,
        availability: impl Fn(&CommandSpec) -> Availability,
    ) -> Lookup<'a> {
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else {
            return Lookup::Empty;
        };
        let word = first.to_ascii_lowercase();
        let found = self
            .specs
            .iter()
            .find(|spec| spec.name == word || spec.aliases.contains(&word.as_str()));
        match found {
            None => Lookup::Unknown(word),
            Some(spec) => match availability(spec) {
                Availability::Open => Lookup::Found {
                    spec,
                    args: words.collect(),
                },
                Availability::Locked(reason) => Lookup::Locked { spec, reason },
            },
        }
    }

    /// What can complete the start of a command: open official names first, and aliases
    /// only when no official name fits (so `s` never lists three spellings of one thing).
    pub fn complete(
        &self,
        prefix: &str,
        availability: impl Fn(&CommandSpec) -> Availability,
    ) -> Vec<String> {
        let prefix = prefix.to_ascii_lowercase();
        let open: Vec<&CommandSpec> = self
            .specs
            .iter()
            .filter(|spec| availability(spec) == Availability::Open)
            .collect();
        let names: Vec<String> = open
            .iter()
            .filter(|spec| spec.name.starts_with(&prefix))
            .map(|spec| spec.name.to_owned())
            .collect();
        if !names.is_empty() {
            return names;
        }
        open.iter()
            .flat_map(|spec| spec.aliases.iter())
            .filter(|alias| alias.starts_with(&prefix))
            .map(|alias| (*alias).to_owned())
            .collect()
    }

    /// The help screen: exactly the open commands, one row each.
    pub fn help(
        &self,
        title: Text,
        columns: &[Text; 2],
        availability: impl Fn(&CommandSpec) -> Availability,
    ) -> Event {
        let rows = self
            .specs
            .iter()
            .filter(|spec| availability(spec) == Availability::Open)
            .map(|spec| vec![Text::raw(spec.name), Text::dynamic(spec.help.to_owned())])
            .collect();
        Event::Screen(Table {
            title,
            columns: columns.to_vec(),
            rows,
        })
    }

    /// Problems in the table: bad or repeated names. Games test that this is empty.
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
        }
        issues
    }

    /// The catalog keys of every help line, for the tests that check the catalogs.
    pub fn help_keys(&self) -> impl Iterator<Item = &'static str> {
        self.specs.iter().map(|spec| spec.help)
    }
}

#[cfg(test)]
mod tests;
