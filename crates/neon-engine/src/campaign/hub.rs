//! The commands that tell: help, status, the journal, contacts, messages, archives, the net,
//! hints, saves and the way out.

use crate::command::{ArgRef, Lookup, NoLists};
use crate::content::Fact;
use crate::content::eval::{ObjectiveState, journal};
use crate::content::ids::{QuestId, ReadableId, SiteId};
use crate::content::schema::{QuestKind, QuestStatus, ReadableKind};
use crate::event::{Event, Table};
use crate::prompt::Input;
use crate::save::{SLOT_COUNT, SaveRequest};
use crate::text::Text;

use super::commands::{COMMANDS, CONTEXT, FRONTEND};
use super::dispatch::{arg_id, invalid};
use super::keys;
use super::play::band_text;
use super::state::{Band, Flow, SiteStatus, clean_handle};
use super::{CampaignGame, count};

/// Trust below which a contact is unknown to the player, then neutral, then friendly.
const TRUST_NEUTRAL: i32 = 20;
const TRUST_FRIENDLY: i32 = 50;
const TRUST_TRUSTED: i32 = 80;

fn table(title: &'static str, columns: &[&'static str], rows: Vec<Vec<Text>>) -> Event {
    Event::Screen(Table {
        title: Text::new(title),
        columns: columns.iter().map(|key| Text::new(*key)).collect(),
        rows,
    })
}

fn dash() -> Text {
    Text::new("campaign.dash")
}

fn status_word(status: QuestStatus) -> Text {
    Text::new(match status {
        QuestStatus::Unavailable => "campaign.quest_status.unavailable",
        QuestStatus::Available => "campaign.quest_status.available",
        QuestStatus::Active => "campaign.quest_status.active",
        QuestStatus::Completed => "campaign.quest_status.completed",
        QuestStatus::Failed => "campaign.quest_status.failed",
    })
}

impl CampaignGame {
    // ---------------------------------------------------------------- the opening

    /// The answer to the handle prompt: the campaign begins.
    pub(super) fn ask_handle(&mut self, input: &Input, events: &mut Vec<Event>) {
        let handle = match input {
            Input::Line(line) => clean_handle(line),
            Input::Cancel => clean_handle(""),
            _ => {
                invalid(events);
                return;
            }
        };
        self.state.flows.pop();
        self.state.handle = handle.clone();
        events.push(Event::narration(
            Text::new("campaign.welcome")
                .with_term("net", "net")
                .with_str("handle", handle),
        ));
        // The first quest was opened when the game was made: tell it, and keep a checkpoint
        // of the very beginning.
        for id in self.state.quest_log.clone() {
            if self.state.missions.status(&id) == QuestStatus::Active {
                events.push(Event::system(
                    Text::new("campaign.out.opened_quest")
                        .with_term("quest", "quest")
                        .with_text("title", keys::quest_title(&id)),
                ));
            }
        }
        self.checkpoint = true;
        events.push(Event::system(
            Text::new("campaign.welcome_hint").with_term("quest", "quest"),
        ));
    }

    // ----------------------------------------------------------------------- help

    pub(super) fn cmd_help(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        match args.first().and_then(ArgRef::as_word) {
            None => events.push(COMMANDS.help(
                Text::new("campaign.help.title"),
                &[
                    Text::new("campaign.help.col_command"),
                    Text::new("campaign.help.col_effect"),
                ],
                CONTEXT,
                FRONTEND,
                |spec| self.availability(spec),
            )),
            Some(name) => self.help_for(name, events),
        }
        self.used("help", events);
    }

    fn help_for(&self, name: &str, events: &mut Vec<Event>) {
        let lookup = COMMANDS.parse(
            name,
            CONTEXT,
            FRONTEND,
            |spec| self.availability(spec),
            &NoLists,
        );
        match lookup {
            // `Missing` is how a command that takes arguments answers a bare name: it exists.
            Lookup::Found { spec, .. } | Lookup::BadArg { spec, .. } => {
                events.push(Event::system(Text::dynamic(spec.help.to_owned())));
                if !spec.args.is_empty() {
                    events.push(Event::system(
                        Text::new("campaign.help.usage")
                            .with_text("usage", Text::dynamic(spec.usage_key())),
                    ));
                }
            }
            Lookup::Locked { reason, .. } => events.push(Event::error(
                Text::new("error.locked").with_text("reason", reason),
            )),
            Lookup::WrongContext { reason, .. } => events.push(Event::error(reason)),
            Lookup::Empty | Lookup::Unknown(_) => invalid(events),
        }
    }

    // --------------------------------------------------------------------- status

    pub(super) fn cmd_status(&mut self, events: &mut Vec<Event>) {
        let c = self.content;
        let s = &self.state;
        let missions = &s.missions;
        let notoriety = s.notoriety();
        let line = |key: &'static str| Text::new(key);
        let mut lines = vec![
            line("campaign.status.handle")
                .with_term("handle", "handle")
                .with_str("value", s.handle.clone()),
            line("campaign.status.level")
                .with_term("level", "level")
                .with_int("value", i64::from(missions.tier)),
            line("campaign.status.credits")
                .with_term("credit", "credit")
                .with_int("value", i64::from(s.credits(c).get())),
            line("campaign.status.notoriety")
                .with_term("notoriety", "notoriety")
                .with_int("value", i64::from(notoriety))
                .with_int("max", 100)
                .with_text("band", band_text(Band::of(notoriety))),
            line("campaign.status.reputation")
                .with_term("reputation", "reputation")
                .with_int("value", i64::from(missions.reputation.get())),
            line("campaign.status.chapter")
                .with_term("chapter", "chapter")
                .with_int(
                    "value",
                    i64::from(crate::content::view::chapter(c, missions)),
                ),
            line("campaign.status.difficulty").with_term("difficulty", s.difficulty.term()),
        ];
        let active = s
            .quest_log
            .iter()
            .filter(|id| missions.status(id) == QuestStatus::Active)
            .count();
        lines.push(
            line("campaign.status.quests")
                .with_term("quest", "quest")
                .with_int("active", count(active)),
        );
        if let Some((quest, _)) = self.focus() {
            lines.push(
                line("campaign.status.hints")
                    .with_term("hint", "hint")
                    .with_int("left", i64::from(self.hints_left(&quest)))
                    .with_text("title", keys::quest_title_short(&quest)),
            );
        }
        events.extend(lines.into_iter().map(Event::system));
        self.used("status", events);
    }

    // --------------------------------------------------------------------- quests

    pub(super) fn cmd_quests(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        match arg_id::<QuestId>(args, 0) {
            None => {
                let rows: Vec<Vec<Text>> = self
                    .state
                    .quest_log
                    .iter()
                    .filter_map(|id| {
                        let quest = self.content.quest(id)?;
                        Some(vec![
                            Text::raw(id.as_str()),
                            keys::quest_title(id),
                            status_word(self.state.missions.status(id)),
                            Text::raw(quest.chapter.to_string()),
                        ])
                    })
                    .collect();
                events.push(table(
                    "campaign.quests.title",
                    &[
                        "campaign.col.id",
                        "campaign.col.title",
                        "campaign.col.status",
                        "campaign.col.chapter",
                    ],
                    rows,
                ));
            }
            Some(id) => self.quest_detail(&id, events),
        }
        self.used("quests", events);
    }

    fn quest_detail(&self, id: &QuestId, events: &mut Vec<Event>) {
        let Some(quest) = self.content.quest(id) else {
            return;
        };
        let status = self.state.missions.status(id);
        let kind = if quest.kind == QuestKind::Main {
            "quest"
        } else {
            "contract"
        };
        events.push(Event::system(
            Text::new("campaign.quest.head")
                .with_term("kind", kind)
                .with_text("title", keys::quest_title(id))
                .with_text("status", status_word(status))
                .with_term("chapter", "chapter")
                .with_int("number", i64::from(quest.chapter)),
        ));
        events.push(Event::narration(keys::quest_desc(id)));
        if status == QuestStatus::Available {
            events.push(Event::system(
                Text::new("campaign.quest.offered")
                    .with_text("giver", keys::contact_name(&quest.giver))
                    .with_str("id", id.as_str()),
            ));
            return;
        }
        for line in journal(self.content, &self.state.missions, id) {
            let text = match line.state {
                ObjectiveState::NotApplicable => continue,
                ObjectiveState::Hidden => Text::new("campaign.quest.secret"),
                ObjectiveState::Pending | ObjectiveState::Done => {
                    keys::quest_objective(id, line.index + 1)
                }
            };
            let key = match (line.state, line.optional) {
                (ObjectiveState::Done, false) => "campaign.quest.done",
                (ObjectiveState::Done, true) => "campaign.quest.done_optional",
                (_, false) => "campaign.quest.todo",
                (_, true) => "campaign.quest.todo_optional",
            };
            let mut entry = Text::new(key).with_text("text", text);
            if let Some((have, need)) = line.progress {
                entry = Text::new(if line.optional {
                    "campaign.quest.todo_progress_optional"
                } else {
                    "campaign.quest.todo_progress"
                })
                .with_text("text", keys::quest_objective(id, line.index + 1))
                .with_int("have", i64::from(have))
                .with_int("need", i64::from(need));
            }
            events.push(Event::system(entry));
        }
    }

    // --------------------------------------------------------------------- contacts

    pub(super) fn cmd_contacts(&mut self, events: &mut Vec<Event>) {
        let c = self.content;
        let missions = &self.state.missions;
        let rows: Vec<Vec<Text>> = self
            .state
            .contact_book
            .iter()
            .map(|id| {
                let trust = missions.trust(c, id);
                let offers = c
                    .quests
                    .iter()
                    .filter(|q| q.giver == *id && missions.status(&q.id) == QuestStatus::Available)
                    .count();
                vec![
                    Text::raw(id.as_str()),
                    keys::contact_name(id),
                    Text::new(match trust {
                        t if t < TRUST_NEUTRAL => "campaign.relation.unknown",
                        t if t < TRUST_FRIENDLY => "campaign.relation.neutral",
                        t if t < TRUST_TRUSTED => "campaign.relation.friendly",
                        _ => "campaign.relation.trusted",
                    }),
                    super::resolver::contact_state(missions.contact(id)),
                    if offers == 0 {
                        dash()
                    } else {
                        Text::new("campaign.contacts.offers")
                            .with_term("contract", "contract")
                            .with_int("n", count(offers))
                    },
                ]
            })
            .collect();
        events.push(table(
            "campaign.contacts.title",
            &[
                "campaign.col.id",
                "campaign.col.name",
                "campaign.col.relation",
                "campaign.col.state",
                "campaign.col.offers",
            ],
            rows,
        ));
    }

    // ------------------------------------------------------------------- messages

    pub(super) fn cmd_messages(&mut self, events: &mut Vec<Event>) {
        let rows: Vec<Vec<Text>> = self
            .state
            .inbox
            .iter()
            .map(|id| {
                let read = self.state.missions.opened.contains(id);
                vec![
                    Text::raw(id.as_str()),
                    keys::mail_subject(id),
                    Text::new(if read {
                        "campaign.message.read"
                    } else {
                        "campaign.message.unread"
                    }),
                ]
            })
            .collect();
        if rows.is_empty() {
            events.push(Event::system(
                Text::new("campaign.messages.empty").with_term("inbox", "inbox"),
            ));
            return;
        }
        events.push(table(
            "campaign.messages.title",
            &[
                "campaign.col.id",
                "campaign.col.subject",
                "campaign.col.status",
            ],
            rows,
        ));
    }

    pub(super) fn cmd_read(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        let Some(id) = arg_id::<ReadableId>(args, 0) else {
            return;
        };
        events.push(Event::system(keys::mail_subject(&id)));
        events.push(Event::narration(keys::mail_body(&id)));
        self.mark_read(&id, events);
    }

    /// The first time a readable is read, the world hears of it.
    fn mark_read(&mut self, id: &ReadableId, events: &mut Vec<Event>) {
        if self.state.missions.opened.contains(id) {
            return;
        }
        let (at, mut facts) = self.tick();
        facts.push(Fact::Read { id: id.clone() });
        self.apply(facts, at, events);
    }

    // -------------------------------------------------------------------- archives

    pub(super) fn cmd_archives(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        if let Some(id) = arg_id::<ReadableId>(args, 0) {
            self.read_archive(&id, events);
            return;
        }
        let rows: Vec<Vec<Text>> = self
            .state
            .archive
            .iter()
            .filter_map(|id| {
                let def = self.content.readable(id)?;
                let opened = self.state.missions.opened.contains(id);
                let (title, state) = if def.kind == ReadableKind::Document {
                    (
                        keys::document_title(id),
                        if opened {
                            "campaign.archive.decrypted"
                        } else {
                            "campaign.archive.encrypted"
                        },
                    )
                } else {
                    (
                        keys::fragment_title(id),
                        if opened {
                            "campaign.archive.read"
                        } else {
                            "campaign.archive.unread"
                        },
                    )
                };
                Some(vec![Text::raw(id.as_str()), title, Text::new(state)])
            })
            .collect();
        if rows.is_empty() {
            events.push(Event::system(
                Text::new("campaign.archives.empty").with_term("archives", "archives"),
            ));
            return;
        }
        events.push(table(
            "campaign.archives.title",
            &[
                "campaign.col.id",
                "campaign.col.title",
                "campaign.col.status",
            ],
            rows,
        ));
    }

    fn read_archive(&mut self, id: &ReadableId, events: &mut Vec<Event>) {
        let Some(def) = self.content.readable(id) else {
            return;
        };
        if def.kind == ReadableKind::Document {
            events.push(Event::system(keys::document_title(id)));
            if let Some(yields) = &def.yields {
                events.push(Event::system(
                    Text::new("campaign.archives.yields")
                        .with_term("fragment", "fragment")
                        .with_text("title", keys::fragment_title(yields)),
                ));
            }
            return;
        }
        events.push(Event::system(keys::fragment_title(id)));
        events.push(Event::narration(keys::fragment_body(id)));
        self.mark_read(id, events);
    }

    pub(super) fn cmd_decrypt(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        let Some(id) = arg_id::<ReadableId>(args, 0) else {
            return;
        };
        let (at, mut facts) = self.tick();
        facts.push(Fact::Decrypted { id: id.clone() });
        events.push(Event::system(
            Text::new("campaign.decrypt.done")
                .with_term("document", "document")
                .with_text("title", keys::document_title(&id)),
        ));
        self.apply(facts, at, events);
        if let Some(yields) = self
            .content
            .readable(&id)
            .and_then(|def| def.yields.clone())
        {
            events.push(Event::system(
                Text::new("campaign.decrypt.yield")
                    .with_term("fragment", "fragment")
                    .with_term("archives", "archives")
                    .with_text("title", keys::fragment_title(&yields)),
            ));
        }
    }

    // ------------------------------------------------------------------------- net

    pub(super) fn cmd_net(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        match arg_id::<SiteId>(args, 0) {
            None => {
                let rows: Vec<Vec<Text>> = self
                    .content
                    .sites
                    .iter()
                    .map(|site| {
                        let status = self.state.site_status(site);
                        if status == SiteStatus::Unknown {
                            return vec![
                                Text::raw(super::resolver::UNKNOWN_NAME),
                                site_status_word(status),
                                dash(),
                            ];
                        }
                        vec![
                            Text::raw(site.id.as_str()),
                            site_status_word(status),
                            Text::raw(site.unlock.tier.unwrap_or(1).to_string()),
                        ]
                    })
                    .collect();
                events.push(table(
                    "campaign.net.title",
                    &[
                        "campaign.col.name",
                        "campaign.col.status",
                        "campaign.col.level",
                    ],
                    rows,
                ));
            }
            Some(id) => self.site_detail(&id, events),
        }
        // The old `scan` is the map of the net now: the `use` objectives still ask for it.
        self.used("scan", events);
    }

    fn site_detail(&self, id: &SiteId, events: &mut Vec<Event>) {
        let Some(site) = self.content.site(id) else {
            return;
        };
        let status = self.state.site_status(site);
        events.push(Event::system(
            Text::new("campaign.site.head")
                .with_term("site", "site")
                .with_str("name", id.as_str())
                .with_text("status", site_status_word(status))
                .with_term("level", "level")
                .with_int("number", i64::from(site.unlock.tier.unwrap_or(1))),
        ));
        events.push(Event::narration(Text::dynamic(format!(
            "node.{}.desc",
            id.as_str().replace('-', "_")
        ))));
        if let Some(relay) = &site.relay {
            events.push(Event::system(
                Text::new("campaign.site.relay").with_str("relay", relay.as_str()),
            ));
        }
        let total = site.file.len();
        if total > 0 {
            let taken = site
                .file
                .iter()
                .filter(|file| {
                    self.state
                        .missions
                        .extracted
                        .contains(&(id.clone(), file.id.clone()))
                })
                .count();
            events.push(Event::system(
                Text::new("campaign.site.loot")
                    .with_term("loot", "loot")
                    .with_int("taken", count(taken))
                    .with_int("total", count(total)),
            ));
        }
    }

    // ------------------------------------------------------------------------ hint

    /// The quest and objective a hint would be about: the first active quest, its first
    /// objective still to do (a required one before an optional one).
    pub(super) fn focus(&self) -> Option<(QuestId, usize)> {
        let missions = &self.state.missions;
        let active = self
            .state
            .quest_log
            .iter()
            .filter(|id| missions.status(id) == QuestStatus::Active);
        for id in active {
            let lines = journal(self.content, missions, id);
            let todo = |optional: bool| {
                lines
                    .iter()
                    .find(|line| {
                        matches!(line.state, ObjectiveState::Pending | ObjectiveState::Hidden)
                            && line.optional == optional
                    })
                    .map(|line| line.index + 1)
            };
            if let Some(number) = todo(false).or_else(|| todo(true)) {
                return Some((id.clone(), number));
            }
        }
        None
    }

    /// Hints a quest may still use.
    pub(super) fn hints_left(&self, quest: &QuestId) -> u8 {
        let budget = u32::from(self.state.difficulty.hint_budget(&self.content.hints));
        let left = budget.saturating_sub(self.state.hints_used(quest));
        u8::try_from(left).unwrap_or(u8::MAX)
    }

    pub(super) fn cmd_hint(&mut self, events: &mut Vec<Event>) {
        let Some((quest, number)) = self.focus() else {
            events.push(Event::system(Text::new("campaign.hint.nothing")));
            return;
        };
        if self.hints_left(&quest) == 0 {
            events.push(Event::error(
                Text::new("campaign.hint.none_left")
                    .with_text("title", keys::quest_title_short(&quest)),
            ));
            return;
        }
        let key = format!("{quest}.{number}");
        let given = self.state.hints.get(&key).copied().unwrap_or(0);
        // The first hint is a lead; asking again on the same objective gives the solution of
        // main quests (contracts have only the lead).
        let level = if given >= 1 { 2 } else { 1 };
        let mut text_key = keys::quest_hint_key(&quest, number, level);
        if level == 2 && self.content.texts.get(&text_key).is_none() {
            text_key = keys::quest_hint_key(&quest, number, 1);
        }
        self.state.hints.insert(key, given.saturating_add(1));
        events.push(Event::say(keys::echo7(), Text::dynamic(text_key)));
        events.push(Event::system(
            Text::new("campaign.hint.left")
                .with_term("hint", "hint")
                .with_int("left", i64::from(self.hints_left(&quest))),
        ));
    }

    // ------------------------------------------------------------------ save, quit

    pub(super) fn cmd_save(args: &[ArgRef], events: &mut Vec<Event>) -> Option<SaveRequest> {
        let slot = match args.first() {
            None => Some(1),
            Some(arg) => arg.id().and_then(|id| id.parse::<u8>().ok()),
        };
        let slot = slot.filter(|slot| (1..=SLOT_COUNT).contains(slot));
        if slot.is_none() {
            events.push(Event::error(
                Text::new("campaign.save.bad_slot").with_int("max", i64::from(SLOT_COUNT)),
            ));
        }
        slot.map(SaveRequest::Slot)
    }

    pub(super) fn cmd_quit(&mut self) {
        self.state.flows.push(Flow::ConfirmQuit);
    }

    /// Leaving saves the game at the command line, so the next launch picks up from there.
    pub(super) fn confirm_quit(
        &mut self,
        input: &Input,
        events: &mut Vec<Event>,
    ) -> Option<SaveRequest> {
        match input {
            Input::Confirm(true) => {
                self.state.flows.pop();
                self.over = true;
                events.push(Event::system(
                    Text::new("campaign.quit.bye").with_str("handle", self.state.handle.clone()),
                ));
                Some(SaveRequest::Autosave)
            }
            Input::Confirm(false) | Input::Cancel => {
                self.state.flows.pop();
                None
            }
            _ => {
                invalid(events);
                None
            }
        }
    }
}

fn site_status_word(status: SiteStatus) -> Text {
    Text::new(match status {
        SiteStatus::Unknown => "campaign.site_status.unknown",
        SiteStatus::Known => "campaign.site_status.known",
        SiteStatus::Pierced => "campaign.site_status.pierced",
    })
}
