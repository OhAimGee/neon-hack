//! What the campaign saves: the mission state of the content language plus what the hub adds.
//!
//! Everything that can be derived from [`State`] is derived (the notoriety is its latest heat
//! reading, a site is pierced when it was compromised, a message is read when it was opened).
//! What is stored is what cannot be derived: the handle, the difficulty, the order in which
//! things were discovered (so that the numbers of a list never move), the hints used, the
//! notoriety the player earned apart from the temporary floors of the story, and the menu
//! being shown, if any.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::content::ids::{ContactId, DecisionId, ItemId, QuestId, ReadableId, SiteId};
use crate::content::money::Credits;
use crate::content::schema::{ContactState, HintBudgets, QuestStatus, ReadableKind, SiteDef, Turn};
use crate::content::state::{HEAT_MAX, State};
use crate::save::{SaveState, hex_u64};

use super::prologue::CONTINUE_PAGES;
use super::tutorial::{Mode, Tutorial};

/// Longest handle, in characters.
pub(super) const HANDLE_MAX_CHARS: usize = 20;

/// The handle used when the player answers nothing.
pub(super) const DEFAULT_HANDLE: &str = "Neon";

/// Value of the `game` field of a campaign save: a save of another game is refused by name
/// before anything else is read.
pub(super) const GAME_TAG: &str = "campaign";

/// Most temporary notoriety modifiers at once (the story only ever has two or three).
const MAX_HEAT_MODS: usize = 16;

/// A typed handle: control characters removed, trimmed, cut to [`HANDLE_MAX_CHARS`] characters,
/// never empty.
pub(super) fn clean_handle(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    let handle: String = cleaned.trim().chars().take(HANDLE_MAX_CHARS).collect();
    if handle.is_empty() {
        DEFAULT_HANDLE.to_owned()
    } else {
        handle
    }
}

/// The difficulty preset, chosen at the start of a campaign and saved with it. Until the
/// tactical run exists it only decides how many hints a quest allows and whether the run can
/// be skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    /// Story: hints to spare, and an intrusion can be skipped.
    Story,
    /// Normal.
    Normal,
    /// Expert.
    Expert,
    /// Hardcore: no hint.
    Hardcore,
}

impl Difficulty {
    /// Every preset, easiest first.
    pub const ALL: [Self; 4] = [Self::Story, Self::Normal, Self::Expert, Self::Hardcore];

    /// The glossary term that names the preset (`term.difficulty_story`...).
    #[must_use]
    pub fn term(self) -> &'static str {
        match self {
            Self::Story => "difficulty_story",
            Self::Normal => "difficulty_normal",
            Self::Expert => "difficulty_expert",
            Self::Hardcore => "difficulty_hardcore",
        }
    }

    /// How many hints one quest may use.
    #[must_use]
    pub fn hint_budget(self, budgets: &HintBudgets) -> u8 {
        match self {
            Self::Story => budgets.story,
            Self::Normal => budgets.normal,
            Self::Expert => budgets.expert,
            Self::Hardcore => budgets.hardcore,
        }
    }

    /// Whether an intrusion can be skipped (the `skip` flag of the commands spec). Nothing
    /// reads it before lot R4, but the preset is already saved.
    #[must_use]
    pub fn allows_skip(self) -> bool {
        self == Self::Story
    }
}

/// The four bands of the notoriety (glossary: Discreet, Watched, Tracked, Hunted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band {
    /// 0 to 29.
    Discreet,
    /// 30 to 49.
    Watched,
    /// 50 to 79.
    Tracked,
    /// 80 and more: the market is closed.
    Hunted,
}

impl Band {
    /// The band a notoriety value is in.
    #[must_use]
    pub fn of(notoriety: u8) -> Self {
        match notoriety {
            0..=29 => Self::Discreet,
            30..=49 => Self::Watched,
            50..=79 => Self::Tracked,
            _ => Self::Hunted,
        }
    }

    /// The glossary term that names the band.
    #[must_use]
    pub fn term(self) -> &'static str {
        match self {
            Self::Discreet => "notoriety_discreet",
            Self::Watched => "notoriety_watched",
            Self::Tracked => "notoriety_tracked",
            Self::Hunted => "notoriety_hunted",
        }
    }
}

/// What the player knows of a site of the net.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteStatus {
    /// Out of reach: its level or its relay is missing.
    Unknown,
    /// Can be hacked.
    Known,
    /// Already compromised once.
    Pierced,
}

/// A temporary change of the notoriety told by the story (`heat_floor`): it counts until the
/// quest `until` opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct HeatMod {
    pub(super) delta: i8,
    pub(super) until: QuestId,
}

/// What the game waits for beyond the command prompt. A stack, so that a menu can open another
/// one (the decision menu of a conversation) and be saved in the middle of either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum Flow {
    /// The handle is asked.
    AskHandle,
    /// A page of the prologue is shown and waits for "continue" (pages `1..=CONTINUE_PAGES`;
    /// the last page is shown with the tutorial offer).
    Prologue { page: u8 },
    /// ECHO-7 offers the guided tutorial: the last question of the opening.
    OfferTutorial,
    /// The menu of an open conversation.
    Talk { contact: ContactId },
    /// The options of a decision, opened from a conversation.
    Decision { decision: DecisionId },
    /// An intrusion waits for confirmation.
    ConfirmHack { site: SiteId },
    /// A purchase waits for confirmation (the checkpoint was made before it).
    ConfirmBuy { item: ItemId },
    /// `quit` waits for confirmation.
    ConfirmQuit,
}

/// The saved state of a campaign.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignState {
    /// Always `campaign`: tells this game's saves from another game's.
    pub(super) game: String,
    /// Reserved for the tactical run (loot variants); the hub does not use it.
    #[serde(with = "hex_u64")]
    pub(super) seed: u64,
    pub(super) handle: String,
    pub(super) difficulty: Difficulty,
    /// The mission state; its clock is the game clock.
    pub(super) missions: State,
    /// The notoriety the player earned (intrusions, cover services), before the temporary
    /// modifiers of the story. The notoriety shown is [`State::heat`].
    pub(super) base_notoriety: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) heat_mods: Vec<HeatMod>,
    /// Credits spent on cover services. They are not payments of the story (a `Paid` fact would
    /// count toward the `pay` objectives of quests), so the mission ledger does not know them.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub(super) service_spend: u32,
    /// The quests the player has met, in the order they were met: the journal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) quest_log: Vec<QuestId>,
    /// The contacts the player has met, in the order they were met.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) contact_book: Vec<ContactId>,
    /// The messages received, in the order they arrived.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) inbox: Vec<ReadableId>,
    /// The fragments and documents obtained, in the order they were obtained.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) archive: Vec<ReadableId>,
    /// Hints used: `<quest>.<objective number>` to how many times.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) hints: BTreeMap<String, u8>,
    /// The guided tutorial. Absent from a save that predates it (and from a game that declined
    /// it), which then reads as "off": an old game never starts a tutorial it never offered.
    #[serde(default, skip_serializing_if = "Tutorial::is_off")]
    pub(super) tutorial: Tutorial,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) flows: Vec<Flow>,
}

impl CampaignState {
    /// The handle of the player (empty until it is asked).
    #[must_use]
    pub fn handle(&self) -> &str {
        &self.handle
    }

    /// The difficulty preset the campaign was started with.
    #[must_use]
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    /// The mission state of the content language: statuses, flags, ledger.
    #[must_use]
    pub fn missions(&self) -> &State {
        &self.missions
    }

    /// The state of a new campaign, waiting for the handle.
    pub(super) fn new(c: &Content, difficulty: Difficulty, seed: u64) -> Self {
        let (missions, _) = crate::content::new_game(c);
        let mut state = Self {
            game: GAME_TAG.to_owned(),
            seed,
            handle: String::new(),
            difficulty,
            missions,
            base_notoriety: 0,
            heat_mods: Vec::new(),
            service_spend: 0,
            quest_log: Vec::new(),
            contact_book: Vec::new(),
            inbox: Vec::new(),
            archive: Vec::new(),
            hints: BTreeMap::new(),
            tutorial: Tutorial::default(),
            flows: vec![Flow::AskHandle],
        };
        state.sync_lists(c);
        state
    }

    /// The game clock: how many turns have passed.
    pub(super) fn clock(&self) -> Turn {
        self.missions.clock
    }

    /// The turn the next change happens at.
    pub(super) fn next_turn(&self) -> Turn {
        self.missions.clock.saturating_add(1)
    }

    /// The notoriety shown: the latest heat reading.
    pub(super) fn notoriety(&self) -> u8 {
        self.missions.heat()
    }

    /// Credits in hand: what the mission ledger holds, less what cover services cost.
    pub(super) fn credits(&self, c: &Content) -> Credits {
        self.missions
            .credits(c)
            .saturating_sub(Credits::new(self.service_spend))
    }

    /// Spends credits on a cover service. The caller checked that they are in hand.
    pub(super) fn spend(&mut self, price: Credits) {
        self.service_spend = self.service_spend.saturating_add(price.get());
    }

    /// The notoriety the player should be at: what was earned plus the modifiers of the
    /// story that are still running, kept between 0 and 100.
    pub(super) fn target_notoriety(&self) -> u8 {
        let sum: i32 = self
            .heat_mods
            .iter()
            .map(|modifier| i32::from(modifier.delta))
            .sum();
        let value = i32::from(self.base_notoriety)
            .saturating_add(sum)
            .clamp(0, i32::from(HEAT_MAX));
        u8::try_from(value).unwrap_or(HEAT_MAX)
    }

    /// What the player knows of a site, derived from the mission state.
    pub(super) fn site_status(&self, site: &SiteDef) -> SiteStatus {
        let pierced = |id: &SiteId| self.missions.compromised.contains_key(id);
        if pierced(&site.id) {
            SiteStatus::Pierced
        } else if self.missions.unlocked(&site.unlock) && site.relay.as_ref().is_none_or(pierced) {
            SiteStatus::Known
        } else {
            SiteStatus::Unknown
        }
    }

    /// Hints used on one quest, all objectives together.
    pub(super) fn hints_used(&self, quest: &QuestId) -> u32 {
        let prefix = format!("{quest}.");
        self.hints
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .map(|(_, used)| u32::from(*used))
            .sum()
    }

    /// Appends what the player has newly met to the lists, in file order among the new ones,
    /// and drops the temporary modifiers whose quest has opened. Idempotent.
    pub(super) fn sync_lists(&mut self, c: &Content) {
        for quest in &c.quests {
            if self.missions.status(&quest.id) != QuestStatus::Unavailable
                && !self.quest_log.contains(&quest.id)
            {
                self.quest_log.push(quest.id.clone());
            }
        }
        for contact in &c.contacts {
            if self.missions.contact(&contact.id) != ContactState::Offline
                && !self.contact_book.contains(&contact.id)
            {
                self.contact_book.push(contact.id.clone());
            }
        }
        for readable in &c.readables {
            if !self.missions.obtained(c, &readable.id) {
                continue;
            }
            let list = match readable.kind {
                ReadableKind::Mail => &mut self.inbox,
                ReadableKind::Fragment | ReadableKind::Document => &mut self.archive,
                ReadableKind::Scene => continue,
            };
            if !list.contains(&readable.id) {
                list.push(readable.id.clone());
            }
        }
        let missions = &self.missions;
        self.heat_mods.retain(|modifier| {
            matches!(
                missions.status(&modifier.until),
                QuestStatus::Unavailable | QuestStatus::Available
            )
        });
    }
}

/// The content the save is checked against: the embedded one, loaded once. A save is read
/// before any game exists, so [`SaveState::validate`] cannot be given the content.
pub(super) fn embedded_content() -> Result<&'static Content, String> {
    use std::sync::OnceLock;
    static CONTENT: OnceLock<Result<Content, String>> = OnceLock::new();
    CONTENT
        .get_or_init(|| Content::embedded().map_err(|error| error.to_string()))
        .as_ref()
        .map_err(Clone::clone)
}

impl SaveState for CampaignState {
    fn validate(&self) -> Result<(), String> {
        let c = embedded_content()?;
        self.validate_against(c)
    }
}

impl CampaignState {
    /// Rejects a state the campaign could never have reached.
    pub(super) fn validate_against(&self, c: &Content) -> Result<(), String> {
        if self.game != GAME_TAG {
            return Err(format!(
                "this is not a campaign save (game `{}`)",
                self.game
            ));
        }
        self.missions.validate(c)?;
        if self.missions.overdrawn(c) {
            return Err("the purchases and payments cost more than the credits earned".to_owned());
        }
        if u64::from(self.service_spend) > u64::from(self.missions.credits(c).get()) {
            return Err("the cover services cost more than the credits in hand".to_owned());
        }
        if self.base_notoriety > HEAT_MAX {
            return Err(format!(
                "notoriety {} is above {HEAT_MAX}",
                self.base_notoriety
            ));
        }
        self.validate_mods(c)?;
        self.validate_lists(c)?;
        self.validate_hints(c)?;
        self.validate_flows(c)?;
        self.validate_tutorial()
    }

    fn validate_mods(&self, c: &Content) -> Result<(), String> {
        if self.heat_mods.len() > MAX_HEAT_MODS {
            return Err(format!("more than {MAX_HEAT_MODS} notoriety modifiers"));
        }
        for modifier in &self.heat_mods {
            if c.quest(&modifier.until).is_none() {
                return Err(format!("unknown quest `{}`", modifier.until));
            }
            if i32::from(modifier.delta).abs() > i32::from(HEAT_MAX) {
                return Err(format!(
                    "notoriety modifier {} is out of range",
                    modifier.delta
                ));
            }
        }
        Ok(())
    }

    fn validate_lists(&self, c: &Content) -> Result<(), String> {
        unique(&self.quest_log, "quest log", |id| c.quest(id).is_some())?;
        unique(&self.contact_book, "contact book", |id| {
            c.contact(id).is_some()
        })?;
        unique(&self.inbox, "inbox", |id| {
            c.readable(id)
                .is_some_and(|def| def.kind == ReadableKind::Mail)
        })?;
        unique(&self.archive, "archives", |id| {
            c.readable(id).is_some_and(|def| {
                matches!(def.kind, ReadableKind::Fragment | ReadableKind::Document)
            })
        })
    }

    fn validate_hints(&self, c: &Content) -> Result<(), String> {
        let budget = u32::from(self.difficulty.hint_budget(&c.hints));
        let mut per_quest: BTreeMap<&str, u32> = BTreeMap::new();
        for (key, used) in &self.hints {
            let bad = || format!("`{key}` is not a hint key");
            let (quest, number) = key.rsplit_once('.').ok_or_else(bad)?;
            let id: QuestId = quest.parse().map_err(|_| bad())?;
            let number: usize = number.parse().map_err(|_| bad())?;
            let quest_def = c.quest(&id).ok_or_else(bad)?;
            if number == 0 || number > quest_def.objective.len() || *used == 0 {
                return Err(bad());
            }
            *per_quest.entry(quest).or_insert(0) += u32::from(*used);
        }
        match per_quest.iter().find(|(_, used)| **used > budget) {
            Some((quest, used)) => Err(format!(
                "{used} hints used on `{quest}` where {budget} are allowed"
            )),
            None => Ok(()),
        }
    }

    fn validate_flows(&self, c: &Content) -> Result<(), String> {
        let handle_ok = !self.handle.is_empty() && clean_handle(&self.handle) == self.handle;
        match self.flows.as_slice() {
            // Before the handle exists, nothing else has started.
            [Flow::AskHandle] => {
                return if self.handle.is_empty() {
                    Ok(())
                } else {
                    Err("the handle is asked twice".to_owned())
                };
            }
            _ if !handle_ok => return Err("the handle is not a clean handle".to_owned()),
            [] | [Flow::ConfirmQuit] => {}
            [Flow::Prologue { page }] if (1..=CONTINUE_PAGES).contains(page) => {}
            [Flow::OfferTutorial] => {}
            [Flow::ConfirmHack { site }] if c.site(site).is_some() => {}
            [Flow::ConfirmBuy { item }] if c.item(item).is_some() => {}
            [Flow::Talk { contact }] if c.contact(contact).is_some() => {}
            [Flow::Talk { contact }, Flow::Decision { decision }]
                if c.contact(contact).is_some() && c.decision(decision).is_some() => {}
            other => {
                return Err(format!(
                    "{} stacked menus do not make a state of the game",
                    other.len()
                ));
            }
        }
        Ok(())
    }
}

impl CampaignState {
    /// The tutorial is consistent with the rest of the state: nothing of it exists before the
    /// opening is over (the offer is the question that starts it), and its steps are known.
    fn validate_tutorial(&self) -> Result<(), String> {
        let opening = self.flows.iter().any(|flow| {
            matches!(
                flow,
                Flow::AskHandle | Flow::Prologue { .. } | Flow::OfferTutorial
            )
        });
        if opening && self.tutorial.mode() != Mode::Off {
            return Err("the tutorial has started before the opening is over".to_owned());
        }
        self.tutorial.validate()
    }
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde calls `skip_serializing_if` with a reference"
)]
fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A list of ids with no repeat, each accepted by `known`.
fn unique<T: Ord + Clone + std::fmt::Display>(
    ids: &[T],
    what: &str,
    known: impl Fn(&T) -> bool,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !known(id) {
            return Err(format!("{what}: `{id}` is not valid here"));
        }
        if !seen.insert(id.clone()) {
            return Err(format!("{what}: `{id}` is listed twice"));
        }
    }
    Ok(())
}
