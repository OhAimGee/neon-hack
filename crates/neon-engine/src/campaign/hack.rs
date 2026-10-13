//! `hack`: an intrusion on a site, played turn by turn, and what its end settles.
//!
//! The intrusion is a tactical run ([`crate::intrusion`]) played inside the campaign: `hack`
//! asks, then opens it, and from then on the context is the run's until it ends. The save keeps
//! the start of the turn under way (`state.run`), refreshed after every command, so that a game
//! taken up again begins the turn again (`docs/spec/run-screen.md` § 9).
//!
//! Its end gives the hub the same facts the automatic intrusion gave. A won run compromises the
//! site, takes the files of the nodes whose loot was taken, leaves the marks the quests in
//! progress ask for on the site, and costs a quarter of the final Trace in notoriety (at
//! least one). The first breach of a site pays what the content says (`first_breach`), once:
//! an intrusion on a pierced site pays nothing, which is the anti-farm rule (invariant I1). It
//! still counts as a new breach for the quests that ask for one after they opened, and it
//! still costs notoriety. A burned run takes nothing and costs the notoriety of the whole
//! Trace; in Hardcore it loses the game.

use crate::command::ArgRef;
use crate::content::Fact;
use crate::content::eval::applicable;
use crate::content::ids::SiteId;
use crate::content::runs::site_tier;
use crate::content::schema::{Goal, Objective, QuestStatus, SiteMark};
use crate::event::{Effect, Event, Severity};
use crate::intrusion::{Intrusion, Question, Reply};
use crate::prompt::Input;
use crate::run::Status;
use crate::save::SaveRequest;
use crate::text::Text;

use super::dispatch::{arg_id, invalid};
use super::state::{Difficulty, Flow, RunSave, SiteStatus};
use super::tutorial::Completion;
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
                self.start_run(site, events);
            }
            Input::Confirm(false) | Input::Cancel => {
                self.state.flows.pop();
                events.push(Event::system(Text::new("campaign.hack.cancelled")));
            }
            _ => invalid(events),
        }
    }

    /// Opens the intrusion on a site, after a checkpoint: it is the way back in Hardcore.
    fn start_run(&mut self, site: &SiteId, events: &mut Vec<Event>) {
        let Some(def) = self.content.site(site) else {
            return;
        };
        if self.state.site_status(def) == SiteStatus::Unknown {
            events.push(Event::error(Text::new("campaign.reason.site_unknown")));
            return;
        }
        let intrusion = match Intrusion::new(self.content, site, self.state.difficulty) {
            Ok(intrusion) => intrusion,
            Err(error) => {
                events.push(Event::error(
                    Text::new("campaign.hack.unavailable").with_str("reason", error.to_string()),
                ));
                return;
            }
        };
        let (turn_start, undo) = intrusion.saved();
        self.state.run = Some(RunSave {
            site: site.clone(),
            turn_start,
            undo,
        });
        let mut opening = intrusion.opening();
        Intrusion::gloss(&mut opening);
        events.extend(opening);
        self.intrusion = Some(intrusion);
        self.checkpoint = true;
    }

    /// A command of the intrusion under way.
    pub(super) fn cmd_run(&mut self, name: &str, args: &[ArgRef], events: &mut Vec<Event>) {
        let Some(intrusion) = self.intrusion.as_mut() else {
            return;
        };
        let mut said = Vec::new();
        let reply = intrusion.execute(name, args, &mut said);
        Intrusion::gloss(&mut said);
        events.extend(said);
        if let Reply::Ask(question) = reply {
            self.question = Some(question);
        }
        self.after_run(events);
    }

    /// The answer to what the intrusion asked.
    pub(super) fn answer_question(
        &mut self,
        question: Question,
        input: &Input,
        events: &mut Vec<Event>,
    ) -> Option<SaveRequest> {
        match input {
            Input::Confirm(yes) => {
                self.question = None;
                if question == Question::Quit {
                    if *yes {
                        self.over = true;
                        events.push(Event::system(
                            Text::new("campaign.quit.bye")
                                .with_str("handle", self.state.handle.clone()),
                        ));
                        return Some(SaveRequest::Autosave);
                    }
                    return None;
                }
                if let Some(intrusion) = self.intrusion.as_mut() {
                    let mut said = Vec::new();
                    intrusion.answer(question, *yes, &mut said);
                    Intrusion::gloss(&mut said);
                    events.extend(said);
                }
                self.after_run(events);
            }
            Input::Cancel => self.question = None,
            _ => invalid(events),
        }
        None
    }

    /// After a command of the intrusion: the save follows the start of the turn, or the run
    /// is over and the hub settles it.
    fn after_run(&mut self, events: &mut Vec<Event>) {
        let Some(intrusion) = &self.intrusion else {
            return;
        };
        if intrusion.is_over() {
            self.finish_run(events);
        } else if let Some(run) = self.state.run.as_mut() {
            let (turn_start, undo) = intrusion.saved();
            run.turn_start = turn_start;
            run.undo = undo;
        }
    }

    /// The end of the intrusion: what it says, then what it changes in the world.
    fn finish_run(&mut self, events: &mut Vec<Event>) {
        let Some(intrusion) = self.intrusion.take() else {
            return;
        };
        self.state.run = None;
        self.question = None;
        let c = self.content;
        let site = intrusion.site().clone();
        let state = *intrusion.state();
        let burned = state.status() == Status::Burned;
        let mut closing = intrusion.closing();
        Intrusion::gloss(&mut closing);
        let Some(def) = c.site(&site) else {
            events.extend(closing);
            return;
        };
        let status = self.state.site_status(def);
        let (at, mut facts) = self.tick();
        // Notoriety follows the Trace the intrusion ended on: a quarter of it, never nothing.
        let trace = state.trace().min(intrusion.run().mission().trace_cap());
        let heat = (trace / 4).max(1);
        self.state.base_notoriety = self.state.base_notoriety.saturating_add(heat).min(100);
        if burned {
            events.extend(closing);
            if self.state.difficulty == Difficulty::Hardcore {
                self.state.lost = true;
                let mut lost = [Event::alert(
                    Severity::Danger,
                    Text::new("campaign.hack.lost")
                        .with_int("max", i64::from(intrusion.run().mission().trace_cap()))
                        .with_str("handle", self.state.handle.clone()),
                )];
                Intrusion::gloss(&mut lost);
                events.extend(lost);
            } else {
                events.push(Event::system(
                    Text::new("campaign.hack.retry").with_str("name", site.as_str()),
                ));
            }
            self.apply(facts, at, events);
            return;
        }
        facts.push(Fact::SiteCompromised {
            site: site.clone(),
            at,
        });
        let mut taken = 0;
        if let Some(run) = c.run(&site) {
            for node in intrusion.looted() {
                for file in run.files_at(node) {
                    let key = (site.clone(), file.clone());
                    if !self.state.missions.extracted.contains(&key) {
                        facts.push(Fact::FileExtracted {
                            site: site.clone(),
                            file: file.clone(),
                        });
                        taken += 1;
                    }
                }
            }
        }
        for mark in self.wanted_marks(&site) {
            facts.push(Fact::SiteMarked {
                site: site.clone(),
                mark,
            });
        }
        let result = match (taken, status) {
            (0, SiteStatus::Pierced) => "campaign.hack.again",
            (0, _) => "campaign.hack.empty",
            _ => "campaign.hack.done",
        };
        // The same tier that gives the deck says how well the site is defended.
        events.push(Event::Effect(Effect::Hack {
            site: site.as_str().to_owned(),
            security: site_tier(def),
            taken: u32::try_from(taken).unwrap_or(u32::MAX),
        }));
        events.extend(closing);
        events.push(Event::system(
            Text::new(result)
                .with_str("name", site.as_str())
                .with_int("n", count(taken)),
        ));
        self.apply(facts, at, events);
        self.state.tutorial.observe("hack", Completion::Outcome);
    }

    /// The marks the quests in progress still ask for on a site.
    fn wanted_marks(&self, site: &SiteId) -> Vec<SiteMark> {
        let (c, missions) = (self.content, &self.state.missions);
        let mut marks = Vec::new();
        let mut want = |objective: &Objective| {
            if let Goal::SiteState { site: at, mark } = &objective.goal
                && at == site
                && applicable(c, missions, objective)
                && !missions.marks.contains(&(site.clone(), *mark))
                && !marks.contains(mark)
            {
                marks.push(*mark);
            }
        };
        for quest in c
            .quests
            .iter()
            .filter(|quest| missions.status(&quest.id) == QuestStatus::Active)
        {
            for objective in &quest.objective {
                want(objective);
                objective.children().iter().for_each(&mut want);
            }
        }
        marks
    }
}
