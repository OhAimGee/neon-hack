//! The lists the commands name things from, and the reasons a listed thing cannot be used.
//!
//! The registry turns a typed word into a row of the list the screen showed; the rows are
//! built here from the live state, in a stable order: the order things were met for quests,
//! contacts, messages and archives (so a number never moves), the order of the data for sites,
//! items, services and slots (all of them listed, the unusable ones with their reason).

use crate::command::{ArgKind, CommandSpec, Listing, Resolver, Row};
use crate::content::Content;
use crate::content::ids::ItemId;
use crate::content::money::Credits;
use crate::content::schema::{ContactState, QuestStatus, ReadableKind};
use crate::save::SLOT_COUNT;
use crate::text::Text;

use super::keys;
use super::state::{Band, CampaignState, SiteStatus};

/// The name of an unknown site in a list: it can be picked by number, and says nothing else.
pub(super) const UNKNOWN_NAME: &str = "???";

/// The live lists of a game.
pub(super) struct Lists<'a> {
    pub(super) c: &'a Content,
    pub(super) s: &'a CampaignState,
}

impl Resolver for Lists<'_> {
    fn list(&self, kind: ArgKind, command: &CommandSpec) -> Listing {
        let (see, rows) = match kind {
            ArgKind::Quest => ("quests", self.quests(command.name)),
            ArgKind::Contact => ("contacts", self.contacts(command.name)),
            ArgKind::Message => ("messages", self.messages()),
            ArgKind::Document => ("archives", self.documents(command.name)),
            ArgKind::Site => ("net", self.sites()),
            ArgKind::Item => ("shop", self.items()),
            ArgKind::Service => ("laylow", self.services()),
            ArgKind::Slot => ("help save", Self::slots()),
            ArgKind::Number
            | ArgKind::Node
            | ArgKind::Program
            | ArgKind::Word(_)
            | ArgKind::Path => ("help", Vec::new()),
        };
        Listing { see, rows }
    }
}

impl Lists<'_> {
    fn quests(&self, command: &str) -> Vec<Row> {
        self.s
            .quest_log
            .iter()
            .map(|id| {
                let names = [id.as_str()];
                let status = self.s.missions.status(id);
                match (command, status) {
                    ("accept", QuestStatus::Active) => Row::closed(
                        id.as_str(),
                        &names,
                        Text::new("campaign.reason.quest_active"),
                    ),
                    ("accept", QuestStatus::Completed) => {
                        Row::closed(id.as_str(), &names, Text::new("campaign.reason.quest_done"))
                    }
                    ("accept", QuestStatus::Failed | QuestStatus::Unavailable) => {
                        Row::closed(id.as_str(), &names, Text::new("campaign.reason.quest_gone"))
                    }
                    _ => Row::open(id.as_str(), &names),
                }
            })
            .collect()
    }

    fn contacts(&self, command: &str) -> Vec<Row> {
        self.s
            .contact_book
            .iter()
            .map(|id| {
                let names = [id.as_str()];
                let state = self.s.missions.contact(id);
                let refusal = (command == "link")
                    .then(|| self.s.link_refusal(self.c, id))
                    .flatten();
                if !state.reachable() {
                    Row::closed(id.as_str(), &names, contact_unreachable(state))
                } else if let Some(reason) = refusal {
                    Row::closed(id.as_str(), &names, reason)
                } else {
                    Row::open(id.as_str(), &names)
                }
            })
            .collect()
    }

    fn messages(&self) -> Vec<Row> {
        self.s
            .inbox
            .iter()
            .map(|id| Row::open(id.as_str(), &[id.as_str()]))
            .collect()
    }

    fn documents(&self, command: &str) -> Vec<Row> {
        self.s
            .archive
            .iter()
            .map(|id| {
                let names = [id.as_str()];
                let Some(def) = self.c.readable(id) else {
                    return Row::open(id.as_str(), &names);
                };
                let opened = self.s.missions.opened.contains(id);
                let encrypted = def.kind == ReadableKind::Document && !opened;
                match command {
                    "decrypt" if def.kind != ReadableKind::Document => Row::closed(
                        id.as_str(),
                        &names,
                        Text::new("campaign.reason.not_encrypted"),
                    ),
                    "decrypt" if opened => {
                        Row::closed(id.as_str(), &names, Text::new("campaign.reason.decrypted"))
                    }
                    "decrypt" => match def
                        .requires_item
                        .as_ref()
                        .filter(|item| !self.s.missions.owned.contains(*item))
                    {
                        Some(item) => Row::closed(
                            id.as_str(),
                            &names,
                            Text::new("campaign.reason.needs_item")
                                .with_text("item", keys::item_name(item)),
                        ),
                        None => Row::open(id.as_str(), &names),
                    },
                    "archives" if encrypted => {
                        Row::closed(id.as_str(), &names, Text::new("campaign.reason.encrypted"))
                    }
                    _ => Row::open(id.as_str(), &names),
                }
            })
            .collect()
    }

    fn sites(&self) -> Vec<Row> {
        self.c
            .sites
            .iter()
            .map(|site| match self.s.site_status(site) {
                SiteStatus::Unknown => Row::closed(
                    site.id.as_str(),
                    &[UNKNOWN_NAME],
                    Text::new("campaign.reason.site_unknown"),
                ),
                SiteStatus::Known | SiteStatus::Pierced => {
                    Row::open(site.id.as_str(), &[site.id.as_str()])
                }
            })
            .collect()
    }

    fn items(&self) -> Vec<Row> {
        self.c
            .items
            .iter()
            .map(|item| {
                let names = [item.id.as_str()];
                match self.s.item_refusal(self.c, &item.id) {
                    None => Row::open(item.id.as_str(), &names),
                    Some(reason) => Row::closed(item.id.as_str(), &names, reason),
                }
            })
            .collect()
    }

    fn services(&self) -> Vec<Row> {
        self.c
            .services
            .iter()
            .map(|service| {
                let names = [service.id.as_str()];
                match self.s.service_refusal(self.c, &service.id) {
                    None => Row::open(service.id.as_str(), &names),
                    Some(reason) => Row::closed(service.id.as_str(), &names, reason),
                }
            })
            .collect()
    }

    fn slots() -> Vec<Row> {
        (1..=SLOT_COUNT)
            .map(|slot| {
                let name = slot.to_string();
                Row::open(name.clone(), &[name.as_str()])
            })
            .collect()
    }
}

/// Why a contact cannot be talked to, by its state.
pub(super) fn contact_unreachable(state: ContactState) -> Text {
    Text::new("campaign.reason.contact").with_text("state", contact_state(state))
}

/// The word for a state of a contact.
pub(super) fn contact_state(state: ContactState) -> Text {
    Text::new(match state {
        ContactState::Available => "campaign.contact_state.available",
        ContactState::Busy => "campaign.contact_state.busy",
        ContactState::Offline => "campaign.contact_state.offline",
        ContactState::Compromised => "campaign.contact_state.compromised",
        ContactState::Hostile => "campaign.contact_state.hostile",
        ContactState::Silenced => "campaign.contact_state.silenced",
        ContactState::Dead => "campaign.contact_state.dead",
    })
}

/// The credits missing to pay `price` with `have` in hand, as a reason.
fn missing_credits(price: Credits, have: Credits) -> Text {
    Text::new("campaign.reason.credits")
        .with_int("missing", i64::from(price.saturating_sub(have).get()))
}

impl CampaignState {
    /// Why an item cannot be bought now, or `None` when it can.
    pub(super) fn item_refusal(&self, c: &Content, id: &ItemId) -> Option<Text> {
        let item = c.item(id)?;
        if self.missions.owned.contains(id) {
            return Some(Text::new("campaign.reason.owned"));
        }
        if !self.missions.unlocked(&item.unlock) {
            return Some(match item.unlock.tier {
                Some(tier) if self.missions.tier < tier => {
                    Text::new("campaign.reason.level").with_int("number", i64::from(tier))
                }
                _ => Text::new("campaign.reason.item_locked"),
            });
        }
        if Band::of(self.notoriety()) == Band::Hunted {
            return Some(Text::new("campaign.reason.market_closed"));
        }
        let have = self.credits(c);
        (have < item.price).then(|| missing_credits(item.price, have))
    }

    /// Why a cover service cannot be bought now, or `None` when it can.
    pub(super) fn service_refusal(
        &self,
        c: &Content,
        id: &crate::content::ids::ServiceId,
    ) -> Option<Text> {
        let service = c.services.iter().find(|service| service.id == *id)?;
        if self.base_notoriety == 0 {
            return Some(Text::new("campaign.reason.nothing_to_hide"));
        }
        let have = self.credits(c);
        (have < service.price).then(|| missing_credits(service.price, have))
    }
}
