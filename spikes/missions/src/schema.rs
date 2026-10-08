//! The data schema: what the TOML files may say.
//!
//! Every struct denies unknown fields. Arrays of tables (`[[quest]]`, `[[site]]`...) keep
//! the file order, which is the display order and the canonical evaluation order.
//! Ids are only checked against the catalogs by [`crate::validate`], never here.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::ids::{
    ChoiceId, CommandId, ContactId, DecisionId, EndingId, FileId, FlagId, ItemId, LineId, QuestId,
    ReadableId, SiteId, TopicId,
};

/// Game time: the number of player turns. The engine has no clock.
pub type Turn = u32;

/// Symbolic reward size, resolved by `rewards.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Size {
    S,
    M,
    L,
    XL,
}

impl Size {
    /// The four sizes, in increasing order.
    pub const ALL: [Size; 4] = [Size::S, Size::M, Size::L, Size::XL];
}

/// How a quest is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestKind {
    /// Opens by itself as soon as it is available.
    Main,
    /// Offered by a contact; the player must accept it.
    Side,
    /// Like `Side`, but opened by a decision (S07).
    Branch,
}

/// Life cycle of a quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestStatus {
    /// Prerequisites or availability condition not met.
    Unavailable,
    /// Offered by its giver, waiting for the player to accept.
    Available,
    /// In progress.
    Active,
    /// Done (terminal).
    Completed,
    /// Lost (terminal, only for failable quests).
    Failed,
}

/// The seven states of a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactState {
    Available,
    Busy,
    Offline,
    Compromised,
    Hostile,
    Silenced,
    Dead,
}

impl ContactState {
    /// Whether the player can talk to a contact in this state.
    pub fn reachable(self) -> bool {
        matches!(self, Self::Available | Self::Compromised | Self::Silenced)
    }

    /// Whether the contact can hand out quests in this state.
    pub fn can_give(self) -> bool {
        matches!(self, Self::Available | Self::Compromised)
    }
}

/// State left on a site (`BACKDOOR`, `UPLOAD_VIRUS`, `ANALYZE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteMark {
    Backdoor,
    Virus,
    Analyzed,
}

/// What a readable is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadableKind {
    Fragment,
    Mail,
    Scene,
    /// Encrypted: "opening" it means decrypting it.
    Document,
}

/// Type of a narrative flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagKind {
    Bool,
    Enum,
    Counter,
    Bitset,
}

/// The value of a flag, in data and in the state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    Bool(bool),
    Int(u32),
    Str(String),
    Bits(BTreeSet<String>),
}

/// Availability gate of a quest, site or item: a tier, a completed quest, or both.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    pub tier: Option<u8>,
    pub quest: Option<QuestId>,
}

// ---------------------------------------------------------------- conditions

/// A condition. Shared by quests (availability, failure), objectives (`when`), decisions
/// (`requires`), dialogue topics, effects (`when`) and endings. It is a closed tree of
/// side-effect-free tests: it cannot loop and cannot write.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cond {
    All {
        of: Vec<Cond>,
    },
    Any {
        of: Vec<Cond>,
    },
    Not {
        of: Box<Cond>,
    },
    /// Exactly one of `is` (bool / enum / counter equality), `at_least` (counter), `has` (bit).
    Flag {
        name: FlagId,
        is: Option<FlagValue>,
        at_least: Option<u32>,
        has: Option<String>,
    },
    Quest {
        id: QuestId,
        is: QuestStatus,
    },
    /// The contact is in one of the listed states.
    Contact {
        id: ContactId,
        is: Vec<ContactState>,
    },
    Trust {
        contact: ContactId,
        at_least: i32,
    },
    /// A fragment/mail/scene was read, or a document decrypted.
    Opened {
        id: ReadableId,
    },
    /// An objective (or an alternative of an `any_of`) of a quest was achieved.
    Objective {
        quest: QuestId,
        id: String,
    },
}

// ------------------------------------------------------------------- effects

/// A symbolic payment: credits and/or reputation, scaled by the tier of the quest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub credits: Option<Size>,
    pub reputation: Option<Size>,
    /// Negative reputation instead of positive.
    #[serde(default)]
    pub penalty: bool,
}

/// What can change when a quest ends, a decision is made or a topic is chosen.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    /// Sets a flag. A bitset flag gets the named bit; the other types take the value.
    SetFlag {
        flag: FlagId,
        value: FlagValue,
    },
    SetContactState {
        contact: ContactId,
        state: ContactState,
    },
    /// Adds to the bonus trust of a contact.
    Trust {
        contact: ContactId,
        delta: i32,
    },
    Grant(Grant),
    /// Raises the player's tier (never lowers it).
    GrantTier {
        tier: u8,
    },
    /// Delivers a mail, plays a scene, reveals a fragment or a document.
    Unlock {
        readable: ReadableId,
    },
    /// Forces the heat to a value (betrayal of Phoenix).
    HeatForce {
        value: u8,
    },
    /// Adds a floor to the heat until the given quest opens.
    HeatFloor {
        delta: i8,
        until: QuestId,
    },
    /// Stores the first ending whose condition holds in the `ending` flag.
    SettleEnding,
}

/// A list of effects guarded by one condition (always true when absent).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub when: Option<Cond>,
    pub then: Vec<Effect>,
}

// ---------------------------------------------------------------- objectives

/// The kinds of objective, as written in the data (`kind = "..."`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveKind {
    Talk,
    Buy,
    Compromise,
    Extract,
    Open,
    Link,
    Reputation,
    Use,
    SiteState,
    Pay,
    Choice,
    HeatEndBelow,
    HeatPeakBelow,
    AnyOf,
}

/// An objective as written: a flat table whose useful fields depend on `kind`. Converted to a
/// typed [`Goal`] with explicit errors for missing or useless fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawObjective {
    pub kind: ObjectiveKind,
    pub id: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub secret: bool,
    pub when: Option<Cond>,
    pub site: Option<SiteId>,
    pub file: Option<FileId>,
    pub contact: Option<ContactId>,
    pub item: Option<ItemId>,
    pub readable: Option<ReadableId>,
    pub decision: Option<DecisionId>,
    pub command: Option<CommandId>,
    pub group: Option<String>,
    pub mark: Option<SiteMark>,
    pub count: Option<u32>,
    pub all: Option<bool>,
    pub level: Option<u8>,
    pub min: Option<u32>,
    pub below: Option<u32>,
    pub amount: Option<Size>,
    #[serde(default)]
    pub of: Vec<Spanned<RawObjective>>,
}

/// What an `extract` objective asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractWhat {
    File(FileId),
    All,
    Count(u32),
}

/// The typed meaning of an objective.
#[derive(Debug, Clone, PartialEq)]
pub enum Goal {
    /// `n` conversations with a contact since the quest opened (event). `MEET` is `count = 1`.
    Talk {
        contact: ContactId,
        count: u32,
    },
    /// The item is owned (stock).
    Buy {
        item: ItemId,
    },
    /// `site`, or `count` different sites, compromised since the quest opened (event).
    Compromise {
        site: Option<SiteId>,
        count: u32,
    },
    /// Files extracted (stock): a named one, all of a site, or `n` (of a site or anywhere).
    Extract {
        site: Option<SiteId>,
        what: ExtractWhat,
    },
    /// A readable was read or a document decrypted (stock).
    Open {
        readable: ReadableId,
    },
    /// Neural link level with a contact (stock).
    Link {
        contact: ContactId,
        level: u8,
    },
    Reputation {
        min: u32,
    },
    /// A command (or any command of a group) was used at least once (stock).
    Use {
        command: Option<CommandId>,
        group: Option<String>,
    },
    /// A backdoor/virus/analysis is on the site (stock).
    SiteState {
        site: SiteId,
        mark: SiteMark,
    },
    /// A payment of the given size since the quest opened (event).
    Pay {
        amount: Size,
    },
    /// The decision was made (stock).
    Choice {
        decision: DecisionId,
    },
    /// Heat below the bound at the instant the quest concludes (condition).
    HeatEndBelow {
        max: u32,
    },
    /// Heat peak since the quest opened below the bound (condition, can be lost).
    HeatPeakBelow {
        max: u32,
    },
    /// One of the alternatives is enough.
    AnyOf(Vec<Objective>),
    /// Shape error found while converting; reported by the validator.
    Invalid(String),
}

/// A typed objective.
#[derive(Debug, Clone, PartialEq)]
pub struct Objective {
    pub id: Option<String>,
    /// Bonus: does not block the conclusion.
    pub optional: bool,
    /// Shown as `???` until achieved.
    pub secret: bool,
    /// Applicable only when this holds; a non-applicable objective is ignored.
    pub when: Option<Cond>,
    pub goal: Goal,
    /// Byte offset of the table in `quests.toml`, for line numbers.
    pub at: usize,
}

impl Goal {
    /// Event objectives count only what happens after the quest opened.
    pub fn is_event(&self) -> bool {
        match self {
            Goal::Talk { .. } | Goal::Compromise { .. } | Goal::Pay { .. } => true,
            Goal::AnyOf(of) => of.iter().any(|o| o.goal.is_event()),
            _ => false,
        }
    }

    /// Condition objectives are not latched: they must hold when the quest concludes.
    pub fn is_condition(&self) -> bool {
        matches!(self, Goal::HeatEndBelow { .. } | Goal::HeatPeakBelow { .. })
    }
}

impl Objective {
    /// The alternatives of an `any_of`, or nothing.
    pub fn children(&self) -> &[Objective] {
        match &self.goal {
            Goal::AnyOf(of) => of,
            _ => &[],
        }
    }
}

impl<'de> Deserialize<'de> for Objective {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = Spanned::<RawObjective>::deserialize(d)?;
        Ok(convert(raw.get_ref(), raw.span().start))
    }
}

fn convert(raw: &RawObjective, at: usize) -> Objective {
    Objective {
        id: raw.id.clone(),
        optional: raw.optional,
        secret: raw.secret,
        when: raw.when.clone(),
        goal: build_goal(raw).unwrap_or_else(Goal::Invalid),
        at,
    }
}

fn build_goal(raw: &RawObjective) -> Result<Goal, String> {
    let provided: Vec<(&str, bool)> = vec![
        ("site", raw.site.is_some()),
        ("file", raw.file.is_some()),
        ("contact", raw.contact.is_some()),
        ("item", raw.item.is_some()),
        ("readable", raw.readable.is_some()),
        ("decision", raw.decision.is_some()),
        ("command", raw.command.is_some()),
        ("group", raw.group.is_some()),
        ("mark", raw.mark.is_some()),
        ("count", raw.count.is_some()),
        ("all", raw.all.is_some()),
        ("level", raw.level.is_some()),
        ("min", raw.min.is_some()),
        ("below", raw.below.is_some()),
        ("amount", raw.amount.is_some()),
        ("of", !raw.of.is_empty()),
    ];
    let (name, allowed): (&str, &[&str]) = match raw.kind {
        ObjectiveKind::Talk => ("talk", &["contact", "count"]),
        ObjectiveKind::Buy => ("buy", &["item"]),
        ObjectiveKind::Compromise => ("compromise", &["site", "count"]),
        ObjectiveKind::Extract => ("extract", &["site", "file", "all", "count"]),
        ObjectiveKind::Open => ("open", &["readable"]),
        ObjectiveKind::Link => ("link", &["contact", "level"]),
        ObjectiveKind::Reputation => ("reputation", &["min"]),
        ObjectiveKind::Use => ("use", &["command", "group"]),
        ObjectiveKind::SiteState => ("site_state", &["site", "mark"]),
        ObjectiveKind::Pay => ("pay", &["amount"]),
        ObjectiveKind::Choice => ("choice", &["decision"]),
        ObjectiveKind::HeatEndBelow => ("heat_end_below", &["below"]),
        ObjectiveKind::HeatPeakBelow => ("heat_peak_below", &["below"]),
        ObjectiveKind::AnyOf => ("any_of", &["of"]),
    };
    if let Some((field, _)) = provided
        .iter()
        .find(|(f, set)| *set && !allowed.contains(f))
    {
        return Err(format!(
            "field `{field}` is not used by objective kind `{name}`"
        ));
    }
    let need = |what: &str| format!("objective kind `{name}` needs `{what}`");
    let count = raw.count.unwrap_or(1);
    Ok(match raw.kind {
        ObjectiveKind::Talk => Goal::Talk {
            contact: raw.contact.clone().ok_or_else(|| need("contact"))?,
            count,
        },
        ObjectiveKind::Buy => Goal::Buy {
            item: raw.item.clone().ok_or_else(|| need("item"))?,
        },
        ObjectiveKind::Compromise => match (&raw.site, raw.count) {
            (Some(_), Some(_)) => {
                return Err("`compromise` takes `site` or `count`, not both".into());
            }
            (None, None) => return Err(need("site` or `count")),
            (site, _) => Goal::Compromise {
                site: site.clone(),
                count,
            },
        },
        ObjectiveKind::Extract => {
            let what = match (&raw.file, raw.all, raw.count) {
                (Some(f), None, None) => ExtractWhat::File(f.clone()),
                (None, Some(true), None) => ExtractWhat::All,
                (None, None, c) => ExtractWhat::Count(c.unwrap_or(1)),
                _ => return Err("`extract` takes one of `file`, `all = true`, `count`".into()),
            };
            if matches!(what, ExtractWhat::File(_) | ExtractWhat::All) && raw.site.is_none() {
                return Err(need("site"));
            }
            Goal::Extract {
                site: raw.site.clone(),
                what,
            }
        }
        ObjectiveKind::Open => Goal::Open {
            readable: raw.readable.clone().ok_or_else(|| need("readable"))?,
        },
        ObjectiveKind::Link => Goal::Link {
            contact: raw.contact.clone().ok_or_else(|| need("contact"))?,
            level: raw.level.ok_or_else(|| need("level"))?,
        },
        ObjectiveKind::Reputation => Goal::Reputation {
            min: raw.min.ok_or_else(|| need("min"))?,
        },
        ObjectiveKind::Use => match (&raw.command, &raw.group) {
            (None, None) | (Some(_), Some(_)) => {
                return Err("`use` takes `command` or `group`".into());
            }
            (command, group) => Goal::Use {
                command: command.clone(),
                group: group.clone(),
            },
        },
        ObjectiveKind::SiteState => Goal::SiteState {
            site: raw.site.clone().ok_or_else(|| need("site"))?,
            mark: raw.mark.ok_or_else(|| need("mark"))?,
        },
        ObjectiveKind::Pay => Goal::Pay {
            amount: raw.amount.ok_or_else(|| need("amount"))?,
        },
        ObjectiveKind::Choice => Goal::Choice {
            decision: raw.decision.clone().ok_or_else(|| need("decision"))?,
        },
        ObjectiveKind::HeatEndBelow => Goal::HeatEndBelow {
            max: raw.below.ok_or_else(|| need("below"))?,
        },
        ObjectiveKind::HeatPeakBelow => Goal::HeatPeakBelow {
            max: raw.below.ok_or_else(|| need("below"))?,
        },
        ObjectiveKind::AnyOf => Goal::AnyOf(
            raw.of
                .iter()
                .map(|s| convert(s.get_ref(), s.span().start))
                .collect(),
        ),
    })
}

// ------------------------------------------------------------------ catalogs

/// A loot file of a site.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileDef {
    pub id: FileId,
    /// Original file name (`neural_maps.bin`), for traceability.
    pub file_name: String,
    /// Extracting it hands out this readable (a fragment or an encrypted document).
    pub yields: Option<ReadableId>,
}

/// A macro-map site.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteDef {
    pub id: SiteId,
    #[serde(default)]
    pub unlock: Unlock,
    /// The site that must have been compromised before this one can be reached.
    pub relay: Option<SiteId>,
    /// Paid once, the first time the site is compromised.
    pub first_breach: Option<Size>,
    #[serde(default)]
    pub file: Vec<FileDef>,
    #[serde(skip)]
    pub at: usize,
}

/// A buyable or quest item.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDef {
    pub id: ItemId,
    /// Name in the original game (`Stealth Module v2.0`), for traceability.
    pub legacy_name: String,
    pub price: u32,
    #[serde(default)]
    pub unlock: Unlock,
    #[serde(skip)]
    pub at: usize,
}

/// A contact.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactDef {
    pub id: ContactId,
    /// State at the start of the game (`offline` until a quest unlocks the contact).
    pub state: ContactState,
    #[serde(default)]
    pub trust: i32,
    #[serde(skip)]
    pub at: usize,
}

/// A fragment, mail, scene or encrypted document.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadableDef {
    pub id: ReadableId,
    pub kind: ReadableKind,
    /// Code in the bible (`DOC_LEDGER`), for traceability.
    pub legacy_code: Option<String>,
    /// Item needed to decrypt (documents only).
    pub requires_item: Option<ItemId>,
    /// Decrypting it hands out this readable (documents only).
    pub yields: Option<ReadableId>,
    /// Known from the start of the game (once its `unlock` holds).
    #[serde(default)]
    pub start: bool,
    #[serde(default)]
    pub unlock: Unlock,
    /// Number of paragraphs (scenes only): `cutscene.<id>.pNN` keys.
    #[serde(default)]
    pub paragraphs: u8,
    #[serde(skip)]
    pub at: usize,
}

/// A command the player can use; `group` lets `use` objectives accept "any cover method".
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandDef {
    pub id: CommandId,
    pub group: Option<String>,
    #[serde(skip)]
    pub at: usize,
}

/// A typed flag declaration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagDef {
    pub id: FlagId,
    pub kind: FlagKind,
    /// Enum values, or bit names of a bitset.
    #[serde(default)]
    pub values: Vec<String>,
    /// Upper bound of a counter.
    pub max: Option<u32>,
    pub default: Option<FlagValue>,
    #[serde(skip)]
    pub at: usize,
}

/// Credits percentage and reputation of a reward size.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeDef {
    pub id: Size,
    /// Credits = this percentage of `R(P)`.
    pub credits_percent: u32,
    pub reputation: i32,
    #[serde(skip)]
    pub at: usize,
}

/// `R(P)`: price of the flagship item of a tier.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TierDef {
    pub id: u8,
    pub price: u32,
    #[serde(skip)]
    pub at: usize,
}

/// Trust constants.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub start_credits: u32,
    pub per_talk: i32,
    pub quest_completed: i32,
}

// -------------------------------------------------------------------- quests

/// A quest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDef {
    pub id: QuestId,
    /// Code of the original enum (`NEXUS_DATA_BREACH`), for traceability.
    pub code: String,
    pub kind: QuestKind,
    pub chapter: u8,
    pub giver: ContactId,
    /// Reference tier `P` of the rewards and payments (not a gate).
    pub tier: u8,
    #[serde(default)]
    pub unlock: Unlock,
    #[serde(default)]
    pub prereq: Vec<QuestId>,
    /// Offered only while this holds (re-checked until the quest is accepted).
    pub available_if: Option<Cond>,
    /// The quest fails as soon as this holds (failable quests only).
    pub fail_if: Option<Cond>,
    #[serde(default)]
    pub failable: bool,
    #[serde(default)]
    pub objective: Vec<Objective>,
    pub reward: Option<Grant>,
    /// Effects fired once, when the quest opens (a scene, a chapter change...).
    #[serde(default)]
    pub on_open: Vec<Block>,
    #[serde(default)]
    pub on_complete: Vec<Block>,
    #[serde(default)]
    pub on_fail: Vec<Block>,
    #[serde(skip)]
    pub at: usize,
}

/// One option of a decision.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceDef {
    pub id: ChoiceId,
    /// The option is offered only while this holds.
    pub requires: Option<Cond>,
    #[serde(default)]
    pub then: Vec<Block>,
}

/// A decision. Its `flag` (an enum whose values are the choice ids) records the choice.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionDef {
    pub id: DecisionId,
    pub flag: FlagId,
    /// The quest that carries the `choice` objective (and the tier for payments).
    pub quest: QuestId,
    pub choice: Vec<ChoiceDef>,
    #[serde(skip)]
    pub at: usize,
}

/// A dialogue topic: same conditions and effects as quests.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopicDef {
    pub id: TopicId,
    pub contact: ContactId,
    pub when: Option<Cond>,
    #[serde(default)]
    pub then: Vec<Block>,
    #[serde(skip)]
    pub at: usize,
}

/// An ending: the first one (in file order) whose condition holds is the ending.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndingDef {
    pub id: EndingId,
    pub when: Cond,
    /// Number of paragraphs: `ending.<id>.pNN` keys.
    #[serde(default)]
    pub paragraphs: u8,
    #[serde(skip)]
    pub at: usize,
}

/// A line of the epilogue montage.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpilogueDef {
    pub id: LineId,
    pub contact: ContactId,
    pub when: Cond,
    #[serde(skip)]
    pub at: usize,
}

// --------------------------------------------------------------- file roots

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogFile {
    #[serde(default)]
    pub site: Vec<Spanned<SiteDef>>,
    #[serde(default)]
    pub item: Vec<Spanned<ItemDef>>,
    #[serde(default)]
    pub contact: Vec<Spanned<ContactDef>>,
    #[serde(default)]
    pub readable: Vec<Spanned<ReadableDef>>,
    #[serde(default)]
    pub command: Vec<Spanned<CommandDef>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FlagsFile {
    #[serde(default)]
    pub flag: Vec<Spanned<FlagDef>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RewardsFile {
    #[serde(default)]
    pub size: Vec<Spanned<SizeDef>>,
    #[serde(default)]
    pub tier: Vec<Spanned<TierDef>>,
    pub rules: Rules,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QuestsFile {
    #[serde(default)]
    pub quest: Vec<Spanned<QuestDef>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionsFile {
    #[serde(default)]
    pub decision: Vec<Spanned<DecisionDef>>,
    #[serde(default)]
    pub ending: Vec<Spanned<EndingDef>>,
    #[serde(default)]
    pub epilogue: Vec<Spanned<EpilogueDef>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TopicsFile {
    #[serde(default)]
    pub topic: Vec<Spanned<TopicDef>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextsFile {
    #[serde(default)]
    pub keys: Vec<String>,
}
