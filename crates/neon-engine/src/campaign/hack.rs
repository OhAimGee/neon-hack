//! `hack`: an intrusion resolved by rule, instantly (`AutoResolve`).
//!
//! The tactical run of lot R4 replaces this without changing the campaign: both hand the
//! content the same facts. The rule is nominal. The intrusion compromises the site and takes
//! the files the player may take (all of them for now), and it costs notoriety by the level of
//! the site (`[auto_resolve]` in `rewards.toml`). The first breach of a site pays what the
//! content says (`first_breach`), once: an intrusion on a pierced site pays nothing, which is
//! the anti-farm rule (invariant I1). It still counts as a new breach for the quests that ask
//! for one after they opened, and it still costs notoriety.

use crate::command::ArgRef;
use crate::content::Fact;
use crate::content::ids::SiteId;
use crate::event::Event;
use crate::prompt::Input;
use crate::text::Text;

use super::dispatch::{arg_id, invalid};
use super::state::{Flow, SiteStatus};
use super::{CampaignGame, count};

impl CampaignGame {
    /// `hack <site>`: asks first, the intrusion cannot be taken back.
    pub(super) fn cmd_hack(&mut self, args: &[ArgRef]) {
        let Some(site) = arg_id::<SiteId>(args, 0) else {
            return;
        };
        self.state.flows.push(Flow::ConfirmHack { site });
    }

    pub(super) fn confirm_hack(&mut self, site: &SiteId, input: &Input, events: &mut Vec<Event>) {
        match input {
            Input::Confirm(true) => {
                self.state.flows.pop();
                self.auto_resolve(site, events);
            }
            Input::Confirm(false) | Input::Cancel => {
                self.state.flows.pop();
                events.push(Event::system(Text::new("campaign.hack.cancelled")));
            }
            _ => invalid(events),
        }
    }

    /// Resolves an intrusion on a site by the nominal rule.
    fn auto_resolve(&mut self, site: &SiteId, events: &mut Vec<Event>) {
        let c = self.content;
        let Some(def) = c.site(site) else {
            return;
        };
        let status = self.state.site_status(def);
        if status == SiteStatus::Unknown {
            events.push(Event::error(Text::new("campaign.reason.site_unknown")));
            return;
        }
        let (at, mut facts) = self.tick();
        facts.push(Fact::SiteCompromised {
            site: site.clone(),
            at,
        });
        let mut taken = 0;
        for file in &def.file {
            let key = (site.clone(), file.id.clone());
            if !self.state.missions.extracted.contains(&key) {
                facts.push(Fact::FileExtracted {
                    site: site.clone(),
                    file: file.id.clone(),
                });
                taken += 1;
            }
        }
        // The intrusion leaves a trace in the world, whatever it takes.
        let tier = usize::from(def.unlock.tier.unwrap_or(1)).max(1);
        let nominal = c
            .auto_resolve
            .nominal_heat
            .get(tier - 1)
            .copied()
            .unwrap_or(0);
        self.state.base_notoriety = self.state.base_notoriety.saturating_add(nominal).min(100);
        let result = match (taken, status) {
            (0, SiteStatus::Pierced) => "campaign.hack.again",
            (0, _) => "campaign.hack.empty",
            _ => "campaign.hack.done",
        };
        events.push(Event::system(
            Text::new(result)
                .with_str("name", site.as_str())
                .with_int("n", count(taken)),
        ));
        self.apply(facts, at, events);
    }
}
