//! `link`: strengthening the neural link with a companion, one level at a time.
//!
//! The content language asks for a link level with a contact (`link` objectives, 1 to 3) and
//! has a fact for it ([`Fact::LinkChanged`]), but the tactical run that would raise the link
//! (the companion slot of the deck) is lot R4. Until then the player raises it by hand: one
//! level per command, free, for the contacts the quests ask a link with. The tactical run
//! will give the same facts, so the quests do not change (decision R-17).

use crate::command::ArgRef;
use crate::content::ids::ContactId;
use crate::content::schema::{Goal, Objective};
use crate::content::state::MAX_LINK_LEVEL;
use crate::content::{Content, Fact};
use crate::event::Event;
use crate::text::Text;

use super::CampaignGame;
use super::dispatch::arg_id;
use super::keys;
use super::state::CampaignState;

/// Whether some quest asks for a link with this contact: the contacts that can be linked are
/// the companions the story needs, and the data is the one place that says which.
pub(super) fn is_companion(c: &Content, contact: &ContactId) -> bool {
    let asks = |objective: &Objective| matches!(&objective.goal, Goal::Link { contact: with, .. } if with == contact);
    c.quests.iter().any(|quest| {
        quest
            .objective
            .iter()
            .any(|objective| asks(objective) || objective.children().iter().any(asks))
    })
}

impl CampaignState {
    /// The neural link level with a contact, 0 when there is none.
    pub(super) fn link_level(&self, contact: &ContactId) -> u8 {
        self.missions.links.get(contact).copied().unwrap_or(0)
    }

    /// Why a contact cannot be linked now, or `None` when it can.
    pub(super) fn link_refusal(&self, c: &Content, contact: &ContactId) -> Option<Text> {
        if !is_companion(c, contact) {
            return Some(Text::new("campaign.reason.no_link"));
        }
        if self.link_level(contact) >= MAX_LINK_LEVEL {
            return Some(Text::new("campaign.reason.link_max"));
        }
        None
    }
}

impl CampaignGame {
    /// `link <contact>`: one more level of the neural link.
    pub(super) fn cmd_link(&mut self, args: &[ArgRef], events: &mut Vec<Event>) {
        let Some(contact) = arg_id::<ContactId>(args, 0) else {
            return;
        };
        if let Some(reason) = self.state.link_refusal(self.content, &contact) {
            events.push(Event::error(reason));
            return;
        }
        let level = self.state.link_level(&contact).saturating_add(1);
        events.push(Event::system(
            Text::new("campaign.link.done")
                .with_text("name", keys::contact_name(&contact))
                .with_int("number", i64::from(level)),
        ));
        let (at, mut facts) = self.tick();
        facts.push(Fact::LinkChanged { contact, level });
        self.apply(facts, at, events);
    }
}
