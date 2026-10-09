//! The prologue: what a new campaign says between the handle and the first command.
//!
//! Three short pages, in the order of the first chapter: the place (a studio of sector 7, a
//! deck that boots by itself), the world (Nexus Corp, the free clinics, the underground), and
//! the first contact (ECHO-7 on the encrypted channel 7, who then offers the guided tutorial).
//! The first two wait for "continue"; the third is followed by the offer, which is the last
//! question of the opening. Backing out (Escape, or typing `skip`) jumps to the third page:
//! the contact with ECHO-7 is what the first quest starts from, so it is never skipped.
//!
//! The opening is a menu of the flow stack, so a game saved in the middle of it comes back to
//! the page it was on (see `Flow::Prologue`).

use crate::content::schema::QuestStatus;
use crate::event::Event;
use crate::prompt::Input;
use crate::text::Text;

use super::CampaignGame;
use super::dispatch::invalid;
use super::keys;
use super::state::Flow;

/// How many pages wait for "continue". The page after them is shown with the tutorial offer.
pub(super) const CONTINUE_PAGES: u8 = 2;

/// The page shown with the offer of the tutorial.
const LAST_PAGE: u8 = CONTINUE_PAGES + 1;

fn narration(key: &'static str) -> Event {
    Event::narration(Text::new(key))
}

impl CampaignGame {
    /// The opening begins: the first page, waiting for "continue".
    pub(super) fn begin_prologue(&mut self, events: &mut Vec<Event>) {
        self.state.flows.push(Flow::Prologue { page: 1 });
        events.extend(self.prologue_page(1));
    }

    /// What a page says. Page numbers outside the prologue say nothing.
    pub(super) fn prologue_page(&self, page: u8) -> Vec<Event> {
        let mut events = vec![Event::Break];
        match page {
            1 => {
                events.push(narration("prologue.p1.1"));
                events.push(narration("prologue.p1.2"));
                events.push(Event::system(Text::new("prologue.skip_hint")));
            }
            2 => {
                events.push(narration("prologue.p2.1"));
                events.push(narration("prologue.p2.2"));
            }
            LAST_PAGE => {
                events.push(Event::system(Text::new("prologue.p3.channel")));
                for key in ["prologue.p3.1", "prologue.p3.2", "prologue.p3.3"] {
                    events.push(Event::say(
                        keys::echo7(),
                        Text::new(key).with_str("name", self.state.handle.clone()),
                    ));
                }
            }
            _ => return Vec::new(),
        }
        events
    }

    /// The answer to a page: "continue" turns it, backing out jumps to the contact.
    pub(super) fn prologue_step(&mut self, page: u8, input: &Input, events: &mut Vec<Event>) {
        let skipped = match input {
            Input::Continue => false,
            Input::Cancel => true,
            Input::Line(line) if line.trim().eq_ignore_ascii_case("skip") => true,
            _ => {
                invalid(events);
                return;
            }
        };
        self.state.flows.pop();
        if !skipped && page < CONTINUE_PAGES {
            self.state.flows.push(Flow::Prologue { page: page + 1 });
            events.extend(self.prologue_page(page + 1));
        } else {
            self.state.flows.push(Flow::OfferTutorial);
            events.extend(self.prologue_page(LAST_PAGE));
        }
    }

    /// For a game taken up again in the middle of the opening: the page it was on.
    pub(super) fn prologue_resume(&self) -> Option<Vec<Event>> {
        match self.state.flows.last()? {
            Flow::Prologue { page } => Some(self.prologue_page(*page)),
            Flow::OfferTutorial => Some(self.prologue_page(LAST_PAGE)),
            _ => None,
        }
    }

    /// The opening is over: the first quest is told, a checkpoint keeps the very beginning,
    /// and a player who declined the tutorial is told where to start.
    pub(super) fn finish_opening(&mut self, guided: bool, events: &mut Vec<Event>) {
        if !guided {
            events.push(Event::system(Text::new("tutorial.skipped")));
        }
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
        if !guided {
            events.push(Event::system(
                Text::new("campaign.welcome_hint").with_term("quest", "quest"),
            ));
        }
    }
}
