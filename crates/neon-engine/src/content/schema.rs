//! The data schema: what the TOML files may say.
//!
//! Every struct denies unknown fields. Arrays of tables (`[[quest]]`, `[[site]]`...) keep
//! the file order, which is the display order and the canonical evaluation order.
//! Ids are only checked against the catalogs by [`crate::content::validate`], never here.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::content::ids::{
    ChoiceId, CommandId, ContactId, DecisionId, EndingId, FileId, FlagId, ItemId, LineId, QuestId,
    ReadableId, ServiceId, SiteId, TopicId,
};
use crate::content::money::{Credits, Reputation};

/// Highest tier of the campaign. Tiers go from 1 to this value.
pub const MAX_TIER: u8 = 6;

/// Game time: the number of player turns. The engine has no clock.
pub type Turn = u32;

/// Symbolic reward size, resolved by `rewards.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Size {
    /// Small.
    S,
    /// Medium.
    M,
    /// Large.
    L,
    /// Extra large.
    XL,
}

impl Size {
    /// The four sizes, in increasing order.
    pub const ALL: [Self; 4] = [Self::S, Self::M, Self::L, Self::XL];
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
    /// Can be talked to and can give quests.
    Available,
    /// Cannot be reached for now.
    Busy,
    /// Not unlocked yet, or gone silent.
    Offline,
    /// Still reachable and can still give quests, but burnt.
    Compromised,
    /// Refuses to talk.
    Hostile,
    /// Can be talked to but gives nothing.
    Silenced,
    /// Gone for good.
    Dead,
}

impl ContactState {
    /// Whether the player can talk to a contact in this state.
    #[must_use]
    pub fn reachable(self) -> bool {
        matches!(self, Self::Available | Self::Compromised | Self::Silenced)
    }

    /// Whether the contact can hand out quests in this state.
    #[must_use]
    pub fn can_give(self) -> bool {
        matches!(self, Self::Available | Self::Compromised)
    }
}

/// State left on a site (`BACKDOOR`, `UPLOAD_VIRUS`, `ANALYZE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteMark {
    /// A backdoor was left on the site.
    Backdoor,
    /// A virus was uploaded.
    Virus,
    /// The site was analyzed.
    Analyzed,
}

/// What a readable is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadableKind {
    /// A story fragment (`f01`...).
    Fragment,
    /// A mail.
    Mail,
    /// A cutscene or an ending screen.
    Scene,
    /// Encrypted: "opening" it means decrypting it.
    Document,
}

/// Type of a narrative flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagKind {
    /// True or false (false by default).
    Bool,
    /// One of the declared values (the first is the default).
    Enum,
    /// A number from 0 to `max`.
    Counter,
    /// A set of the declared bit names.
    Bitset,
}

/// The value of a flag, in data and in the state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    /// A boolean flag.
    Bool(bool),
    /// A counter.
    Int(u32),
    /// An enum value, or the name of the bit a `set_flag` adds to a bitset.
    Str(String),
    /// The bits of a bitset.
    Bits(BTreeSet<String>),
}

/// Availability gate of a quest, site or item: a tier, a completed quest, or both.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    /// The tier the player must have reached.
    pub tier: Option<u8>,
    /// The quest that must be completed.
    pub quest: Option<QuestId>,
}

// ---------------------------------------------------------------- conditions

/// A condition. Shared by quests (availability, failure), objectives (`when`), decisions
/// (`requires`), dialogue topics, effects (`when`) and endings. It is a closed tree of
/// side-effect-free tests: it cannot loop and cannot write.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cond {
    /// Every sub-condition holds.
    All {
        /// The sub-conditions.
        of: Vec<Cond>,
    },
    /// At least one sub-condition holds.
    Any {
        /// The sub-conditions.
        of: Vec<Cond>,
    },
    /// The sub-condition does not hold.
    Not {
        /// The negated condition.
        of: Box<Cond>,
    },
    /// Exactly one of `is` (bool / enum / counter equality), `at_least` (counter), `has` (bit).
    Flag {
        /// The flag read.
        name: FlagId,
        /// Equality with a value.
        is: Option<FlagValue>,
        /// Counter at least this high.
        at_least: Option<u32>,
        /// The bitset holds this bit.
        has: Option<String>,
    },
    /// The quest has the status.
    Quest {
        /// The quest.
        id: QuestId,
        /// The expected status.
        is: QuestStatus,
    },
    /// The contact is in one of the listed states.
    Contact {
        /// The contact.
        id: ContactId,
        /// The accepted states.
        is: Vec<ContactState>,
    },
    /// The trust in a contact reaches a value.
    Trust {
        /// The contact.
        contact: ContactId,
        /// The least trust.
        at_least: i32,
    },
    /// A fragment/mail/scene was read, or a document decrypted.
    Opened {
        /// The readable.
        id: ReadableId,
    },
    /// An objective (or an alternative of an `any_of`) of a quest was achieved.
    Objective {
        /// The quest.
        quest: QuestId,
        /// The `id` of the objective or of the alternative.
        id: String,
    },
}

// ------------------------------------------------------------------- effects

/// A symbolic payment: credits and/or reputation, scaled by the tier of the quest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    /// Credits paid, as a size.
    pub credits: Option<Size>,
    /// Reputation paid, as a size.
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
        /// The flag written.
        flag: FlagId,
        /// The new value.
        value: FlagValue,
    },
    /// Unlocks or closes a contact.
    SetContactState {
        /// The contact.
        contact: ContactId,
        /// The new state.
        state: ContactState,
    },
    /// Adds to the bonus trust of a contact.
    Trust {
        /// The contact.
        contact: ContactId,
        /// Points gained (or lost when negative).
        delta: i32,
    },
    /// Pays credits and/or reputation.
    Grant(Grant),
    /// Raises the player's tier (never lowers it).
    GrantTier {
        /// The tier reached.
        tier: u8,
    },
    /// Delivers a mail, plays a scene, reveals a fragment or a document.
    Unlock {
        /// The readable handed out.
        readable: ReadableId,
    },
    /// Forces the heat to a value (betrayal of Phoenix).
    HeatForce {
        /// The heat, 0 to 100.
        value: u8,
    },
    /// Adds a floor to the heat until the given quest opens.
    HeatFloor {
        /// The floor added.
        delta: i8,
        /// The quest whose opening lifts the floor.
        until: QuestId,
    },
    /// Stores the first ending whose condition holds in the `ending` flag.
    SettleEnding,
}

/// A list of effects guarded by one condition (always true when absent).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    /// The guard (always true when absent).
    pub when: Option<Cond>,
    /// The effects, in order.
    pub then: Vec<Effect>,
}

// ---------------------------------------------------------------- objectives

/// The kinds of objective, as written in the data (`kind = "..."`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveKind {
    /// Talk to a contact.
    Talk,
    /// Own an item.
    Buy,
    /// Breach a site, or several.
    Compromise,
    /// Extract files.
    Extract,
    /// Read or decrypt a readable.
    Open,
    /// Reach a neural link level.
    Link,
    /// Reach a reputation.
    Reputation,
    /// Use a command.
    Use,
    /// Leave a mark on a site.
    SiteState,
    /// Make a payment.
    Pay,
    /// Make a decision.
    Choice,
    /// Heat below a bound when the quest concludes.
    HeatEndBelow,
    /// Heat peak below a bound since the quest opened.
    HeatPeakBelow,
    /// One of several alternatives.
    AnyOf,
}

/// An objective as written: a flat table whose useful fields depend on `kind`. Converted to a
/// typed [`Goal`] with explicit errors for missing or useless fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawObjective {
    /// The kind of objective.
    pub kind: ObjectiveKind,
    /// Name other conditions can refer to.
    pub id: Option<String>,
    /// Bonus objective.
    #[serde(default)]
    pub optional: bool,
    /// Shown as `???` until achieved.
    #[serde(default)]
    pub secret: bool,
    /// Applicability guard.
    pub when: Option<Cond>,
    /// A site (`compromise`, `extract`, `site_state`).
    pub site: Option<SiteId>,
    /// A file of the site (`extract`).
    pub file: Option<FileId>,
    /// A contact (`talk`, `link`).
    pub contact: Option<ContactId>,
    /// An item (`buy`).
    pub item: Option<ItemId>,
    /// A readable (`open`).
    pub readable: Option<ReadableId>,
    /// A decision (`choice`).
    pub decision: Option<DecisionId>,
    /// A command (`use`).
    pub command: Option<CommandId>,
    /// A command group (`use`).
    pub group: Option<String>,
    /// A mark (`site_state`).
    pub mark: Option<SiteMark>,
    /// A number of times, sites or files.
    pub count: Option<u32>,
    /// Every file of the site (`extract`).
    pub all: Option<bool>,
    /// A neural link level (`link`).
    pub level: Option<u8>,
    /// The reputation to reach (`reputation`).
    pub min: Option<Reputation>,
    /// The heat bound (`heat_end_below`, `heat_peak_below`).
    pub below: Option<u32>,
    /// The size of the payment (`pay`).
    pub amount: Option<Size>,
    /// The alternatives (`any_of`).
    #[serde(default)]
    pub of: Vec<Spanned<RawObjective>>,
}

/// What an `extract` objective asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractWhat {
    /// A named file.
    File(FileId),
    /// Every file of the site.
    All,
    /// This many files.
    Count(u32),
}

/// The typed meaning of an objective.
#[derive(Debug, Clone, PartialEq)]
pub enum Goal {
    /// `n` conversations with a contact since the quest opened (event). `MEET` is `count = 1`.
    Talk {
        /// The contact.
        contact: ContactId,
        /// How many conversations.
        count: u32,
    },
    /// The item is owned (stock).
    Buy {
        /// The item.
        item: ItemId,
    },
    /// `site`, or `count` different sites, compromised since the quest opened (event).
    Compromise {
        /// The site, when a named one is asked.
        site: Option<SiteId>,
        /// How many different sites when no site is named.
        count: u32,
    },
    /// Files extracted (stock): a named one, all of a site, or `n` (of a site or anywhere).
    Extract {
        /// The site, or any site when absent (with a count).
        site: Option<SiteId>,
        /// What is asked.
        what: ExtractWhat,
    },
    /// A readable was read or a document decrypted (stock).
    Open {
        /// The readable.
        readable: ReadableId,
    },
    /// Neural link level with a contact (stock).
    Link {
        /// The contact.
        contact: ContactId,
        /// The level, 1 to 3.
        level: u8,
    },
    /// The reputation reaches a value (stock).
    Reputation {
        /// The least reputation.
        min: Reputation,
    },
    /// A command (or any command of a group) was used at least once (stock).
    Use {
        /// The command.
        command: Option<CommandId>,
        /// A group of commands, any of which counts.
        group: Option<String>,
    },
    /// A backdoor/virus/analysis is on the site (stock).
    SiteState {
        /// The site.
        site: SiteId,
        /// The mark.
        mark: SiteMark,
    },
    /// A payment of the given size since the quest opened (event).
    Pay {
        /// The size, resolved at the tier of the quest.
        amount: Size,
    },
    /// The decision was made (stock).
    Choice {
        /// The decision.
        decision: DecisionId,
    },
    /// Heat below the bound at the instant the quest concludes (condition).
    HeatEndBelow {
        /// The bound (exclusive).
        max: u32,
    },
    /// Heat peak since the quest opened below the bound (condition, can be lost).
    HeatPeakBelow {
        /// The bound (exclusive).
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
    /// Name other conditions can refer to.
    pub id: Option<String>,
    /// Bonus: does not block the conclusion.
    pub optional: bool,
    /// Shown as `???` until achieved.
    pub secret: bool,
    /// Applicable only when this holds; a non-applicable objective is ignored.
    pub when: Option<Cond>,
    /// What is asked.
    pub goal: Goal,
    /// Byte offset of the table in `quests.toml`, for line numbers.
    pub at: usize,
}

impl Goal {
    /// Event objectives count only what happens after the quest opened.
    #[must_use]
    pub fn is_event(&self) -> bool {
        match self {
            Goal::Talk { .. } | Goal::Compromise { .. } | Goal::Pay { .. } => true,
            Goal::AnyOf(of) => of.iter().any(|o| o.goal.is_event()),
            _ => false,
        }
    }

    /// Condition objectives are not latched: they must hold when the quest concludes.
    #[must_use]
    pub fn is_condition(&self) -> bool {
        matches!(self, Goal::HeatEndBelow { .. } | Goal::HeatPeakBelow { .. })
    }
}

impl Objective {
    /// The alternatives of an `any_of`, or nothing.
    #[must_use]
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
    let provided = [
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
    /// Id of the file in its site.
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
    /// Id of the site.
    pub id: SiteId,
    /// Gate of the site.
    #[serde(default)]
    pub unlock: Unlock,
    /// The site that must have been compromised before this one can be reached.
    pub relay: Option<SiteId>,
    /// Paid once, the first time the site is compromised.
    pub first_breach: Option<Size>,
    /// The loot files.
    #[serde(default)]
    pub file: Vec<FileDef>,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// A buyable or quest item.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDef {
    /// Id of the item.
    pub id: ItemId,
    /// Name in the original game (`Stealth Module v2.0`), for traceability.
    pub legacy_name: String,
    /// Price in credits.
    pub price: Credits,
    /// Gate of the item.
    #[serde(default)]
    pub unlock: Unlock,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// A contact.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactDef {
    /// Id of the contact.
    pub id: ContactId,
    /// State at the start of the game (`offline` until a quest unlocks the contact).
    pub state: ContactState,
    /// Trust at the start of the game.
    #[serde(default)]
    pub trust: i32,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// A fragment, mail, scene or encrypted document.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadableDef {
    /// Id of the readable.
    pub id: ReadableId,
    /// What it is.
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
    /// Gate of the readable.
    #[serde(default)]
    pub unlock: Unlock,
    /// Number of paragraphs (scenes only): `cutscene.<id>.pNN` keys.
    #[serde(default)]
    pub paragraphs: u8,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// A command the player can use; `group` lets `use` objectives accept "any cover method".
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandDef {
    /// Id of the command.
    pub id: CommandId,
    /// The group `use` objectives can name.
    pub group: Option<String>,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// A typed flag declaration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagDef {
    /// Id of the flag.
    pub id: FlagId,
    /// Its type.
    pub kind: FlagKind,
    /// Enum values, or bit names of a bitset.
    #[serde(default)]
    pub values: Vec<String>,
    /// Upper bound of a counter.
    pub max: Option<u32>,
    /// Value at the start of the game (the type's natural default when absent).
    pub default: Option<FlagValue>,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// Credits percentage and reputation of a reward size.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeDef {
    /// The size.
    pub id: Size,
    /// Credits = this percentage of `R(P)`.
    pub credits_percent: u32,
    /// Reputation paid.
    pub reputation: Reputation,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// `R(P)`: price of the flagship item of a tier.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TierDef {
    /// The tier, 1 to [`MAX_TIER`].
    pub id: u8,
    /// `R(P)` in credits.
    pub price: Credits,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// Trust constants.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// Credits at the start of the game.
    pub start_credits: Credits,
    /// Trust gained per conversation.
    pub per_talk: i32,
    /// Trust gained by the giver of a completed quest.
    pub quest_completed: i32,
}

/// A service that lowers the notoriety (`laylow`): what it costs and how much it cools.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDef {
    /// Id of the service; its texts are `service.<id>.name` and `service.<id>.desc`.
    pub id: ServiceId,
    /// Price in credits.
    pub price: Credits,
    /// Notoriety points removed.
    pub cooling: u8,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// What an automatic intrusion costs in notoriety, by the tier of the site.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoResolveRules {
    /// Notoriety added by one intrusion; entry `n - 1` is for a site of tier `n`, so there are
    /// exactly [`MAX_TIER`] entries.
    pub nominal_heat: Vec<u8>,
}

/// How many hints a quest may use, by difficulty.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HintBudgets {
    /// Story difficulty.
    pub story: u8,
    /// Normal difficulty.
    pub normal: u8,
    /// Expert difficulty.
    pub expert: u8,
    /// Hardcore difficulty.
    pub hardcore: u8,
}

/// When a command of the game opens (docs/spec/commands.md, section 6). A command without a
/// rule is open from the start.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnlockDef {
    /// The command, by its official name in the command table of the game.
    pub command: String,
    /// The command is open while this holds.
    pub when: Cond,
    /// Text key of the reason given while it is closed (`unlock.shop`).
    pub reason: String,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

// -------------------------------------------------------------------- quests

/// A quest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDef {
    /// Id of the quest.
    pub id: QuestId,
    /// Code of the original enum (`NEXUS_DATA_BREACH`), for traceability.
    pub code: String,
    /// How it is offered.
    pub kind: QuestKind,
    /// Chapter of the story, 1 to 6.
    pub chapter: u8,
    /// The contact who offers it.
    pub giver: ContactId,
    /// Reference tier `P` of the rewards and payments (not a gate).
    pub tier: u8,
    /// Gate of the quest.
    #[serde(default)]
    pub unlock: Unlock,
    /// Quests that must be completed first.
    #[serde(default)]
    pub prereq: Vec<QuestId>,
    /// Offered only while this holds (re-checked until the quest is accepted).
    pub available_if: Option<Cond>,
    /// The quest fails as soon as this holds (failable quests only).
    pub fail_if: Option<Cond>,
    /// Whether the quest can fail (never a main quest).
    #[serde(default)]
    pub failable: bool,
    /// The objectives, at most [`crate::content::validate::MAX_OBJECTIVES`].
    #[serde(default)]
    pub objective: Vec<Objective>,
    /// Paid when the quest completes.
    pub reward: Option<Grant>,
    /// Effects fired once, when the quest opens (a scene, a chapter change...).
    #[serde(default)]
    pub on_open: Vec<Block>,
    /// Effects fired once, when the quest completes.
    #[serde(default)]
    pub on_complete: Vec<Block>,
    /// Effects fired once, when the quest fails.
    #[serde(default)]
    pub on_fail: Vec<Block>,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// One option of a decision.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceDef {
    /// Id of the choice.
    pub id: ChoiceId,
    /// The option is offered only while this holds.
    pub requires: Option<Cond>,
    /// What the choice does.
    #[serde(default)]
    pub then: Vec<Block>,
}

/// A decision. Its `flag` (an enum whose values are the choice ids) records the choice.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionDef {
    /// Id of the decision.
    pub id: DecisionId,
    /// The enum flag that records the choice.
    pub flag: FlagId,
    /// The quest that carries the `choice` objective (and the tier for payments).
    pub quest: QuestId,
    /// The options.
    pub choice: Vec<ChoiceDef>,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// A dialogue topic: same conditions and effects as quests.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopicDef {
    /// Id of the topic, unique for its contact.
    pub id: TopicId,
    /// The contact who talks about it.
    pub contact: ContactId,
    /// The topic is offered only while this holds.
    pub when: Option<Cond>,
    /// What choosing the topic does.
    #[serde(default)]
    pub then: Vec<Block>,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// An ending: the first one (in file order) whose condition holds is the ending.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndingDef {
    /// Id of the ending.
    pub id: EndingId,
    /// The condition of the ending.
    pub when: Cond,
    /// Number of paragraphs: `ending.<id>.pNN` keys.
    #[serde(default)]
    pub paragraphs: u8,
    /// Byte offset of the entry in its file, for line numbers.
    #[serde(skip)]
    pub at: usize,
}

/// A line of the epilogue montage.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpilogueDef {
    /// Id of the line.
    pub id: LineId,
    /// The contact the line is about.
    pub contact: ContactId,
    /// The line is shown while this holds.
    pub when: Cond,
    /// Byte offset of the entry in its file, for line numbers.
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
    #[serde(default)]
    pub service: Vec<Spanned<ServiceDef>>,
    #[serde(default)]
    pub auto_resolve: AutoResolveRules,
    #[serde(default)]
    pub hints: HintBudgets,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnlocksFile {
    #[serde(default)]
    pub unlock: Vec<Spanned<UnlockDef>>,
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
