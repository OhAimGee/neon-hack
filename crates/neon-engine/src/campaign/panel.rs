//! What the side panel of a full-screen frontend lists: the quests under way, the net as far
//! as it is known, what the player holds, and what waits to be read.
//!
//! Every row is something a command already says (`quests`, `net`, `status`, `messages`,
//! `archives`), in the same words: the panel only keeps it in view.

use crate::content::eval::{ObjectiveState, journal};
use crate::content::schema::QuestStatus;
use crate::game::{PanelRow, PanelSection, Tone};
use crate::text::Text;

use super::hub::site_status_word;
use super::state::SiteStatus;
use super::{CampaignGame, count, keys};

impl CampaignGame {
    /// The sections of the panel, empty of rows never: a list with nothing in it says so.
    /// Before the handle is known there is nothing to list.
    pub(super) fn panel_sections(&self) -> Vec<PanelSection> {
        if self.state.handle.is_empty() {
            return Vec::new();
        }
        let mut sections = vec![
            self.panel_quests(),
            self.panel_net(),
            self.panel_inventory(),
        ];
        sections.extend(self.panel_unread());
        for section in &mut sections {
            keys::gloss(&mut section.title);
            for row in &mut section.rows {
                keys::gloss(&mut row.text);
            }
        }
        sections
    }

    /// The active quests: the short title of each, then what is left to do.
    fn panel_quests(&self) -> PanelSection {
        let mut rows = Vec::new();
        for id in &self.state.quest_log {
            if self.state.missions.status(id) != QuestStatus::Active {
                continue;
            }
            rows.push(PanelRow::new(keys::quest_title_short(id), Tone::Strong));
            for line in journal(self.content, &self.state.missions, id) {
                if line.state != ObjectiveState::Pending || line.optional {
                    continue;
                }
                let objective = keys::quest_objective(id, line.index + 1);
                let text = match line.progress {
                    Some((have, need)) => Text::new("campaign.panel.objective_progress")
                        .with_text("text", objective)
                        .with_int("have", i64::from(have))
                        .with_int("need", i64::from(need)),
                    None => Text::new("campaign.view.objective").with_text("text", objective),
                };
                rows.push(PanelRow::new(text, Tone::Plain));
            }
        }
        if rows.is_empty() {
            rows.push(PanelRow::new(
                Text::new("campaign.panel.no_quest"),
                Tone::Muted,
            ));
        }
        PanelSection {
            title: Text::new("campaign.panel.quests"),
            rows,
        }
    }

    /// The sites the player knows, with where each stands. The unknown ones are not named.
    fn panel_net(&self) -> PanelSection {
        let mut rows: Vec<PanelRow> = self
            .content
            .sites
            .iter()
            .filter_map(|site| {
                let status = self.state.site_status(site);
                let tone = match status {
                    SiteStatus::Unknown => return None,
                    SiteStatus::Known => Tone::Plain,
                    SiteStatus::Pierced => Tone::Good,
                };
                let text = Text::new("campaign.panel.site")
                    .with_str("name", site.id.as_str())
                    .with_text("status", site_status_word(status));
                Some(PanelRow::new(text, tone))
            })
            .collect();
        if rows.is_empty() {
            rows.push(PanelRow::new(
                Text::new("campaign.panel.no_site"),
                Tone::Muted,
            ));
        }
        PanelSection {
            title: Text::new("campaign.panel.net"),
            rows,
        }
    }

    /// The credits in hand, and the items bought.
    fn panel_inventory(&self) -> PanelSection {
        let mut rows = vec![PanelRow::new(
            Text::new("campaign.status.credits")
                .with_term("credit", "credit")
                .with_int("value", i64::from(self.credits())),
            Tone::Plain,
        )];
        rows.extend(
            self.state
                .missions
                .owned
                .iter()
                .map(|item| PanelRow::new(keys::item_name(item), Tone::Plain)),
        );
        if rows.len() == 1 {
            rows.push(PanelRow::new(
                Text::new("campaign.panel.no_item"),
                Tone::Muted,
            ));
        }
        PanelSection {
            title: Text::new("campaign.panel.inventory"),
            rows,
        }
    }

    /// What waits to be read: messages not opened, and archives not read or not decrypted.
    /// Absent when nothing waits.
    fn panel_unread(&self) -> Option<PanelSection> {
        let opened = &self.state.missions.opened;
        let messages = self
            .state
            .inbox
            .iter()
            .filter(|id| !opened.contains(*id))
            .count();
        // A fragment is opened by reading it, a document by decrypting it: both wait in the
        // archives until then.
        let archives = self
            .state
            .archive
            .iter()
            .filter(|id| !opened.contains(*id))
            .count();
        let mut rows = Vec::new();
        if messages > 0 {
            rows.push(PanelRow::new(
                Text::new("campaign.panel.messages").with_int("n", count(messages)),
                Tone::Warning,
            ));
        }
        if archives > 0 {
            rows.push(PanelRow::new(
                Text::new("campaign.panel.archives").with_int("n", count(archives)),
                Tone::Warning,
            ));
        }
        (!rows.is_empty()).then(|| PanelSection {
            title: Text::new("campaign.panel.unread"),
            rows,
        })
    }
}
