//! Playing facts: giving them to the content language, and telling what it answers.
//!
//! [`CampaignGame::apply`] is the one door through which the world changes. It records the
//! facts, lets the language settle ([`refresh`](crate::content::refresh)), turns each
//! [`Outcome`] into events with derived text keys (roles and importance per the event contract:
//! rewards and alerts are essential), and plays the few outcomes that need the game to answer:
//! a forced notoriety, a floor on it, a scene to watch.

use crate::content::engine::montage;
use crate::content::schema::{FlagValue, QuestKind, ReadableKind, Turn};
use crate::content::view::ending;
use crate::content::{Fact, Outcome};
use crate::event::{Event, Gauge, Severity};
use crate::ids::ContactId;
use crate::text::Text;

use super::CampaignGame;
use super::keys;
use super::resolver::contact_state;
use super::state::{Band, HeatMod};

/// Rounds of facts a single command may cause (a forced notoriety, a scene to mark seen...).
/// The content settles in far fewer; this only guards against a loop.
const MAX_ROUNDS: usize = 8;

fn is_main(c: &crate::content::Content, id: &crate::content::ids::QuestId) -> bool {
    c.quest(id)
        .is_some_and(|quest| quest.kind == QuestKind::Main)
}

/// The band of the notoriety, as a text.
pub(super) fn band_text(band: Band) -> Text {
    Text::new("campaign.band").with_term("band", band.term())
}

impl CampaignGame {
    /// Records `facts` (the turn of this change is `at`), settles the content, and tells the
    /// player what happened. The lists of the hub are brought up to date.
    pub(super) fn apply(&mut self, facts: Vec<Fact>, at: Turn, events: &mut Vec<Event>) {
        let mut facts = facts;
        let mut at = at;
        for _ in 0..MAX_ROUNDS {
            // The notoriety the player should be at is a fact of this round, so that quests
            // that read it (`heat_end_below`) see it when they settle.
            let target = self.state.target_notoriety();
            let before = self.state.notoriety();
            if target != before {
                facts.push(Fact::HeatChanged { heat: target, at });
            }
            for fact in &facts {
                self.state.missions.apply(self.content, fact);
            }
            let outcomes = self.state.missions.refresh(self.content);
            events.extend(self.heat_events(before));
            let follow_up = self.announce(&outcomes, events);
            // After the telling: a contact is new to the player until the book lists it.
            self.state.sync_lists(self.content);
            let settled = self.state.target_notoriety() == self.state.notoriety();
            if follow_up.is_empty() && settled {
                return;
            }
            facts = follow_up;
            at = self.state.next_turn();
        }
    }

    /// The events of a notoriety that moved: the gauge, and an alert when the band changed.
    fn heat_events(&self, from: u8) -> Vec<Event> {
        let to = self.state.notoriety();
        if from == to {
            return Vec::new();
        }
        let (old, new) = (Band::of(from), Band::of(to));
        let mut events = vec![Event::Changed {
            gauge: Gauge::Notoriety,
            from: i32::from(from),
            to: i32::from(to),
            band: band_text(new),
        }];
        if old != new {
            let (severity, key) = match (new, new > old) {
                (Band::Hunted, _) => (Severity::Danger, "campaign.alert.band_up"),
                (_, true) => (Severity::Warning, "campaign.alert.band_up"),
                (_, false) => (Severity::Notice, "campaign.alert.band_down"),
            };
            events.push(Event::alert(
                severity,
                Text::dynamic(key.to_owned())
                    .with_term("notoriety", "notoriety")
                    .with_text("band", band_text(new)),
            ));
        }
        events
    }

    /// Tells what the content answered, and returns the facts the game owes in return.
    fn announce(&mut self, outcomes: &[Outcome], events: &mut Vec<Event>) -> Vec<Fact> {
        let c = self.content;
        let mut follow_up = Vec::new();
        for outcome in outcomes {
            match outcome {
                // A main quest is offered and opened in the same breath: it is told when it opens.
                Outcome::QuestOffered(id) if !is_main(c, id) => {
                    let giver = c.quest(id).map(|quest| keys::contact_name(&quest.giver));
                    if let Some(giver) = giver {
                        events.push(Event::system(
                            Text::new("campaign.out.offered")
                                .with_term("contract", "contract")
                                .with_text("giver", giver)
                                .with_text("title", keys::quest_title(id))
                                .with_str("id", id.as_str()),
                        ));
                    }
                }
                Outcome::QuestOpened(id) => {
                    let main = c
                        .quest(id)
                        .is_some_and(|quest| quest.kind == QuestKind::Main);
                    if main {
                        self.checkpoint = true;
                    }
                    let key = if main {
                        "campaign.out.opened_quest"
                    } else {
                        "campaign.out.opened_contract"
                    };
                    events.push(Event::system(
                        Text::dynamic(key.to_owned())
                            .with_term(
                                if main { "quest" } else { "contract" },
                                if main { "quest" } else { "contract" },
                            )
                            .with_text("title", keys::quest_title(id)),
                    ));
                }
                Outcome::QuestCompleted(id) => {
                    let (term, key) = if is_main(c, id) {
                        ("quest", "campaign.out.completed_quest")
                    } else {
                        ("contract", "campaign.out.completed_contract")
                    };
                    events.push(Event::system(
                        Text::new(key)
                            .with_term(term, term)
                            .with_text("title", keys::quest_title(id)),
                    ));
                    if let Some(quest) = c.quest(id) {
                        events.push(Event::say(
                            quest.giver.clone(),
                            Text::dynamic(format!(
                                "quest.{}.debrief",
                                id.as_str().replace('-', "_")
                            )),
                        ));
                    }
                }
                Outcome::QuestFailed(id) => {
                    let (term, key) = if is_main(c, id) {
                        ("quest", "campaign.out.failed_quest")
                    } else {
                        ("contract", "campaign.out.failed_contract")
                    };
                    events.push(Event::alert(
                        Severity::Warning,
                        Text::new(key)
                            .with_term(term, term)
                            .with_text("title", keys::quest_title(id)),
                    ));
                }
                Outcome::QuestWithdrawn(id) => events.push(Event::system(
                    Text::new("campaign.out.withdrawn")
                        .with_term("contract", "contract")
                        .with_text("title", keys::quest_title(id)),
                )),
                Outcome::QuestRestarted(id) => events.push(Event::system(
                    Text::new("campaign.out.restarted")
                        .with_term("quest", "quest")
                        .with_text("title", keys::quest_title(id)),
                )),
                Outcome::ObjectiveDone { quest, key } if key.1 == 0 => {
                    events.push(Event::system(
                        Text::new("campaign.out.objective")
                            .with_term("objective", "objective")
                            .with_text(
                                "text",
                                keys::quest_objective(quest, usize::from(key.0) + 1),
                            ),
                    ));
                }
                Outcome::Reward {
                    credits,
                    reputation,
                    ..
                } => self.announce_reward(credits.get(), reputation.get(), events),
                Outcome::TierGranted(tier) => events.push(Event::reward(
                    Text::new("campaign.out.level")
                        .with_term("level", "level")
                        .with_int("level_number", i64::from(*tier)),
                )),
                Outcome::Unlocked(id) => self.announce_unlock(id, events, &mut follow_up),
                Outcome::ContactChanged { contact, state } => {
                    events.push(self.contact_event(contact, *state));
                }
                Outcome::FlagChanged { flag, value } if flag.as_str() == "chapter" => {
                    if let FlagValue::Int(chapter) = value {
                        events.push(Event::narration(
                            Text::new("campaign.out.chapter")
                                .with_term("chapter", "chapter")
                                .with_int("chapter_number", i64::from(*chapter)),
                        ));
                    }
                }
                Outcome::HeatForced(value) => self.state.base_notoriety = *value,
                Outcome::HeatFloor { delta, until } => {
                    let opened = !matches!(
                        self.state.missions.status(until),
                        crate::content::schema::QuestStatus::Unavailable
                            | crate::content::schema::QuestStatus::Available
                    );
                    if !opened {
                        self.state.heat_mods.push(HeatMod {
                            delta: *delta,
                            until: until.clone(),
                        });
                    }
                }
                Outcome::DecisionRejected { .. } => {
                    events.push(Event::error(Text::new("campaign.out.decision_rejected")))
                }
                Outcome::BudgetExceeded => events.push(Event::alert(
                    Severity::Danger,
                    Text::new("campaign.out.budget"),
                )),
                // Told elsewhere, or nothing to tell: a trust change shows in `contacts`, the
                // ending shows in the scene that plays it, and only the first alternative of
                // an objective is announced.
                Outcome::QuestOffered(_)
                | Outcome::ObjectiveDone { .. }
                | Outcome::FlagChanged { .. }
                | Outcome::TrustChanged { .. }
                | Outcome::EndingSettled(_) => {}
            }
        }
        follow_up
    }

    fn announce_reward(&self, credits: u32, reputation: i32, events: &mut Vec<Event>) {
        if credits > 0 {
            events.push(Event::reward(
                Text::new("campaign.out.credits").with_int("amount", i64::from(credits)),
            ));
        }
        match reputation {
            0 => {}
            gain if gain > 0 => events.push(Event::reward(
                Text::new("campaign.out.reputation")
                    .with_term("reputation", "reputation")
                    .with_int("points", i64::from(gain)),
            )),
            loss => events.push(Event::alert(
                Severity::Notice,
                Text::new("campaign.out.reputation_lost")
                    .with_term("reputation", "reputation")
                    .with_int("points", i64::from(loss.unsigned_abs())),
            )),
        }
    }

    fn contact_event(
        &self,
        contact: &ContactId,
        state: crate::content::schema::ContactState,
    ) -> Event {
        use crate::content::schema::ContactState;
        // A contact leaving `offline` is a new contact for the player; any other change is news.
        let met = state != ContactState::Offline && !self.state.contact_book.contains(contact);
        if met {
            Event::system(
                Text::new("campaign.out.new_contact")
                    .with_term("contact", "contact")
                    .with_text("name", keys::contact_name(contact)),
            )
        } else {
            Event::system(
                Text::new("campaign.out.contact_state")
                    .with_text("name", keys::contact_name(contact))
                    .with_text("state", contact_state(state)),
            )
        }
    }

    /// A readable handed out by the content: a message arrives, a fragment or a document
    /// joins the archives, a scene plays (and is marked seen).
    fn announce_unlock(
        &self,
        id: &crate::content::ids::ReadableId,
        events: &mut Vec<Event>,
        follow_up: &mut Vec<Fact>,
    ) {
        let Some(def) = self.content.readable(id) else {
            return;
        };
        match def.kind {
            ReadableKind::Mail => events.push(Event::system(
                Text::new("campaign.out.new_message")
                    .with_term("message", "message")
                    .with_text("subject", keys::mail_subject(id)),
            )),
            ReadableKind::Fragment => events.push(Event::system(
                Text::new("campaign.out.new_fragment")
                    .with_term("fragment", "fragment")
                    .with_term("archives", "archives")
                    .with_text("title", keys::fragment_title(id)),
            )),
            ReadableKind::Document => events.push(Event::system(
                Text::new("campaign.out.new_document")
                    .with_term("document", "document")
                    .with_term("archives", "archives")
                    .with_text("title", keys::document_title(id)),
            )),
            ReadableKind::Scene => {
                events.push(Event::Break);
                for number in 1..=def.paragraphs {
                    events.push(Event::narration(keys::cutscene_paragraph(id, number)));
                }
                events.extend(self.finale_events(id.as_str()));
                events.push(Event::Break);
                follow_up.push(Fact::Read { id: id.clone() });
            }
        }
    }

    /// The two scenes that close the campaign show the ending reached and the epilogue.
    fn finale_events(&self, scene: &str) -> Vec<Event> {
        let (c, s) = (self.content, &self.state.missions);
        match scene {
            "ending-screen" => ending(c, s).map_or_else(Vec::new, |end| {
                let mut events = vec![Event::narration(keys::ending_title(&end.id))];
                events.extend(
                    (1..=end.paragraphs)
                        .map(|n| Event::narration(keys::ending_paragraph(&end.id, n))),
                );
                events
            }),
            "epilogue-montage" => montage(c, s)
                .into_iter()
                .map(|line| Event::narration(keys::epilogue_line(&line.id)))
                .collect(),
            _ => Vec::new(),
        }
    }
}
