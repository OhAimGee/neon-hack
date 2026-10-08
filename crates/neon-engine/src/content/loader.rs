//! The loaded content: typed tables, indices, and the loader with line-numbered errors.
//!
//! The data files are embedded in the program (`data/world/*.toml`, see `build.rs`); the loader
//! itself takes strings, so tests can load a broken copy without touching the disk.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::DeserializeOwned;
use thiserror::Error;
use toml::Spanned;

use crate::content::ids::{ContactId, DecisionId, FlagId, ItemId, QuestId, ReadableId, SiteId};
use crate::content::money::{Credits, Reputation};
use crate::content::schema::{
    CatalogFile, CommandDef, ContactDef, DecisionDef, DecisionsFile, EndingDef, EpilogueDef,
    FlagDef, FlagsFile, ItemDef, QuestDef, QuestsFile, ReadableDef, RewardsFile, Rules, SiteDef,
    Size, SizeDef, TextsFile, TierDef, TopicDef, TopicsFile,
};
use crate::content::validate;

include!(concat!(env!("OUT_DIR"), "/embedded_world.rs"));

/// The data files, in the order errors are reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum File {
    /// `catalog.toml`: sites, items, contacts, readables, commands.
    Catalog,
    /// `flags.toml`: the typed flags.
    Flags,
    /// `rewards.toml`: sizes, tier prices, trust constants.
    Rewards,
    /// `quests.toml`: the quests.
    Quests,
    /// `decisions.toml`: decisions, endings, epilogue.
    Decisions,
    /// `topics.toml`: dialogue topics.
    Topics,
    /// `texts.toml`: the declared text keys.
    Texts,
}

impl File {
    /// Path shown in error messages.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            File::Catalog => "data/world/catalog.toml",
            File::Flags => "data/world/flags.toml",
            File::Rewards => "data/world/rewards.toml",
            File::Quests => "data/world/quests.toml",
            File::Decisions => "data/world/decisions.toml",
            File::Topics => "data/world/topics.toml",
            File::Texts => "data/world/texts.toml",
        }
    }
}

/// The text of the data files. The engine does no I/O: callers hand over strings.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// `catalog.toml`.
    pub catalog: String,
    /// `flags.toml`.
    pub flags: String,
    /// `rewards.toml`.
    pub rewards: String,
    /// `quests.toml`.
    pub quests: String,
    /// `decisions.toml`.
    pub decisions: String,
    /// `topics.toml`.
    pub topics: String,
    /// `texts.toml`.
    pub texts: String,
}

impl Sources {
    /// The shipped data, embedded in the binary.
    #[must_use]
    pub fn embedded() -> Self {
        Self {
            catalog: WORLD_CATALOG.to_owned(),
            flags: WORLD_FLAGS.to_owned(),
            rewards: WORLD_REWARDS.to_owned(),
            quests: WORLD_QUESTS.to_owned(),
            decisions: WORLD_DECISIONS.to_owned(),
            topics: WORLD_TOPICS.to_owned(),
            texts: WORLD_TEXTS.to_owned(),
        }
    }

    /// The text of one file.
    #[must_use]
    pub fn get(&self, file: File) -> &str {
        match file {
            File::Catalog => &self.catalog,
            File::Flags => &self.flags,
            File::Rewards => &self.rewards,
            File::Quests => &self.quests,
            File::Decisions => &self.decisions,
            File::Topics => &self.topics,
            File::Texts => &self.texts,
        }
    }
}

/// One error, with the file and line it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The file the error is in.
    pub file: File,
    /// The line (1-based; 0 for an error that belongs to no line).
    pub line: u32,
    /// What is wrong, in English, for the writers of the content.
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file.name(), self.line, self.message)
    }
}

/// Why the content could not be loaded: every error found, ordered by file then line.
#[derive(Debug, Clone, Error)]
#[error("{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
pub struct LoadError(pub Vec<Diagnostic>);

/// Line (1-based) of a byte offset.
#[must_use]
pub fn line_of(text: &str, at: usize) -> u32 {
    let newlines = text
        .as_bytes()
        .iter()
        .take(at)
        .filter(|b| **b == b'\n')
        .count();
    u32::try_from(newlines).map_or(u32::MAX, |n| n.saturating_add(1))
}

/// A definition that remembers where it was written.
trait Positioned {
    fn set_at(&mut self, at: usize);
}

macro_rules! positioned {
    ($($t:ty),*) => {$(
        impl Positioned for $t {
            fn set_at(&mut self, at: usize) {
                self.at = at;
            }
        }
    )*};
}
positioned!(
    SiteDef,
    ItemDef,
    ContactDef,
    ReadableDef,
    CommandDef,
    FlagDef,
    SizeDef,
    TierDef,
    QuestDef,
    DecisionDef,
    TopicDef,
    EndingDef,
    EpilogueDef
);

fn unspan<T: Positioned>(v: Vec<Spanned<T>>) -> Vec<T> {
    v.into_iter()
        .map(|s| {
            let at = s.span().start;
            let mut inner = s.into_inner();
            inner.set_at(at);
            inner
        })
        .collect()
}

fn parse<T: DeserializeOwned>(file: File, text: &str, diags: &mut Vec<Diagnostic>) -> Option<T> {
    match toml::from_str::<T>(text) {
        Ok(v) => Some(v),
        Err(e) => {
            let line = e.span().map_or(1, |s| line_of(text, s.start));
            diags.push(Diagnostic {
                file,
                line,
                message: e.message().to_owned(),
            });
            None
        }
    }
}

type Index = BTreeMap<String, usize>;

fn index<T>(v: &[T], key: impl Fn(&T) -> &str) -> Index {
    let mut ix = Index::new();
    for (i, item) in v.iter().enumerate() {
        ix.entry(key(item).to_owned()).or_insert(i);
    }
    ix
}

/// All the content of the game, validated.
#[derive(Debug, Clone)]
pub struct Content {
    /// The sites of the macro map, in file order.
    pub sites: Vec<SiteDef>,
    /// The items.
    pub items: Vec<ItemDef>,
    /// The contacts.
    pub contacts: Vec<ContactDef>,
    /// The fragments, mails, scenes and documents.
    pub readables: Vec<ReadableDef>,
    /// The commands the objectives can name.
    pub commands: Vec<CommandDef>,
    /// The typed flags.
    pub flags: Vec<FlagDef>,
    /// The reward sizes.
    pub sizes: Vec<SizeDef>,
    /// The `R(P)` of each tier.
    pub tiers: Vec<TierDef>,
    /// Trust constants and starting credits.
    pub rules: Rules,
    /// The quests, in file order (the order they are evaluated in).
    pub quests: Vec<QuestDef>,
    /// The decisions.
    pub decisions: Vec<DecisionDef>,
    /// The dialogue topics.
    pub topics: Vec<TopicDef>,
    /// The endings, first match wins.
    pub endings: Vec<EndingDef>,
    /// The lines of the epilogue montage.
    pub epilogue: Vec<EpilogueDef>,
    /// The text keys declared in `texts.toml`.
    pub texts: BTreeSet<String>,
    pub(crate) sources: Sources,
    site_ix: Index,
    item_ix: Index,
    contact_ix: Index,
    readable_ix: Index,
    command_ix: Index,
    flag_ix: Index,
    quest_ix: Index,
    decision_ix: Index,
}

impl Content {
    /// The shipped campaign content, embedded in the program.
    ///
    /// # Errors
    ///
    /// A [`LoadError`] when the embedded data is invalid; the tests make sure it is not.
    pub fn embedded() -> Result<Self, LoadError> {
        Self::from_sources(&Sources::embedded())
    }

    /// Parses and validates the data files. All the errors found are returned, ordered by
    /// file then line; a syntax or schema error stops the file it is in (the parser cannot go
    /// on), and semantic validation only runs when every file parsed.
    ///
    /// # Errors
    ///
    /// A [`LoadError`] with every problem found.
    pub fn from_sources(src: &Sources) -> Result<Self, LoadError> {
        Self::load(src, true)
    }

    /// Like [`Content::from_sources`] but without comparing `texts.toml` to the derived keys:
    /// the tool that regenerates that file needs the structure before the file exists.
    ///
    /// # Errors
    ///
    /// A [`LoadError`] with every problem found.
    pub fn from_sources_without_texts(src: &Sources) -> Result<Self, LoadError> {
        Self::load(src, false)
    }

    fn load(src: &Sources, check_texts: bool) -> Result<Self, LoadError> {
        let mut diags = Vec::new();
        let catalog = parse::<CatalogFile>(File::Catalog, &src.catalog, &mut diags);
        let flags = parse::<FlagsFile>(File::Flags, &src.flags, &mut diags);
        let rewards = parse::<RewardsFile>(File::Rewards, &src.rewards, &mut diags);
        let quests = parse::<QuestsFile>(File::Quests, &src.quests, &mut diags);
        let decisions = parse::<DecisionsFile>(File::Decisions, &src.decisions, &mut diags);
        let topics = parse::<TopicsFile>(File::Topics, &src.topics, &mut diags);
        let texts = parse::<TextsFile>(File::Texts, &src.texts, &mut diags);
        let (
            Some(catalog),
            Some(flags),
            Some(rewards),
            Some(quests),
            Some(decisions),
            Some(topics),
            Some(texts),
        ) = (catalog, flags, rewards, quests, decisions, topics, texts)
        else {
            return Err(LoadError(diags));
        };
        let sites = unspan(catalog.site);
        let items = unspan(catalog.item);
        let contacts = unspan(catalog.contact);
        let readables = unspan(catalog.readable);
        let commands = unspan(catalog.command);
        let flags = unspan(flags.flag);
        let quests = unspan(quests.quest);
        let decisions_defs = unspan(decisions.decision);
        let content = Self {
            site_ix: index(&sites, |x| x.id.as_str()),
            item_ix: index(&items, |x| x.id.as_str()),
            contact_ix: index(&contacts, |x| x.id.as_str()),
            readable_ix: index(&readables, |x| x.id.as_str()),
            command_ix: index(&commands, |x| x.id.as_str()),
            flag_ix: index(&flags, |x| x.id.as_str()),
            quest_ix: index(&quests, |x| x.id.as_str()),
            decision_ix: index(&decisions_defs, |x| x.id.as_str()),
            sites,
            items,
            contacts,
            readables,
            commands,
            flags,
            sizes: unspan(rewards.size),
            tiers: unspan(rewards.tier),
            rules: rewards.rules,
            quests,
            decisions: decisions_defs,
            topics: unspan(topics.topic),
            endings: unspan(decisions.ending),
            epilogue: unspan(decisions.epilogue),
            texts: texts.keys.into_iter().collect(),
            sources: src.clone(),
        };
        let mut errors = validate::validate(&content, check_texts);
        if errors.is_empty() {
            Ok(content)
        } else {
            errors.sort_by_key(|d| (d.file, d.line));
            Err(LoadError(errors))
        }
    }

    /// Builds a diagnostic for a byte offset of a file.
    #[must_use]
    pub fn diag(&self, file: File, at: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            file,
            line: line_of(self.sources.get(file), at),
            message: message.into(),
        }
    }

    /// The site with this id.
    #[must_use]
    pub fn site(&self, id: &SiteId) -> Option<&SiteDef> {
        self.site_ix
            .get(id.as_str())
            .and_then(|i| self.sites.get(*i))
    }

    /// The item with this id.
    #[must_use]
    pub fn item(&self, id: &ItemId) -> Option<&ItemDef> {
        self.item_ix
            .get(id.as_str())
            .and_then(|i| self.items.get(*i))
    }

    /// The contact with this id.
    #[must_use]
    pub fn contact(&self, id: &ContactId) -> Option<&ContactDef> {
        self.contact_ix
            .get(id.as_str())
            .and_then(|i| self.contacts.get(*i))
    }

    /// The readable with this id.
    #[must_use]
    pub fn readable(&self, id: &ReadableId) -> Option<&ReadableDef> {
        self.readable_ix
            .get(id.as_str())
            .and_then(|i| self.readables.get(*i))
    }

    /// Whether a command with this id exists.
    #[must_use]
    pub fn has_command(&self, id: &str) -> bool {
        self.command_ix.contains_key(id)
    }

    /// The flag with this id.
    #[must_use]
    pub fn flag(&self, id: &FlagId) -> Option<&FlagDef> {
        self.flag_ix
            .get(id.as_str())
            .and_then(|i| self.flags.get(*i))
    }

    /// The quest with this id.
    #[must_use]
    pub fn quest(&self, id: &QuestId) -> Option<&QuestDef> {
        self.quest_ix
            .get(id.as_str())
            .and_then(|i| self.quests.get(*i))
    }

    /// Position of a quest in file order.
    #[must_use]
    pub fn quest_index(&self, id: &QuestId) -> Option<usize> {
        self.quest_ix.get(id.as_str()).copied()
    }

    /// The decision with this id.
    #[must_use]
    pub fn decision(&self, id: &DecisionId) -> Option<&DecisionDef> {
        self.decision_ix
            .get(id.as_str())
            .and_then(|i| self.decisions.get(*i))
    }

    /// The flag with this name, when the name is not a typed id yet (`ending`).
    #[must_use]
    pub fn flag_named(&self, name: &str) -> Option<&FlagDef> {
        self.flag_ix.get(name).and_then(|i| self.flags.get(*i))
    }

    /// `R(P)` for a tier.
    #[must_use]
    pub fn tier_price(&self, tier: u8) -> Credits {
        self.tiers
            .iter()
            .find(|t| t.id == tier)
            .map_or(Credits::ZERO, |t| t.price)
    }

    /// Resolves a symbolic size at a tier: `(credits, reputation)`.
    #[must_use]
    pub fn resolve(&self, size: Size, tier: u8) -> (Credits, Reputation) {
        let Some(def) = self.sizes.iter().find(|s| s.id == size) else {
            return (Credits::ZERO, Reputation::ZERO);
        };
        (
            self.tier_price(tier).percent(def.credits_percent),
            def.reputation,
        )
    }
}
