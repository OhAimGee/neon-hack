//! Helpers shared by the integration tests.

#![allow(dead_code, unreachable_pub)]

use neon_spike_missions::engine::{Output, new_game, refresh};
use neon_spike_missions::ids::{ContactId, QuestId};
use neon_spike_missions::schema::{ContactState, QuestStatus};
use neon_spike_missions::state::QuestRun;
use neon_spike_missions::{Content, Fact, State};

pub fn shipped() -> Content {
    Content::embedded().unwrap_or_else(|e| panic!("shipped content is invalid:\n{e}"))
}

pub fn q(id: &str) -> QuestId {
    QuestId::new(id)
}

/// A game driven by hand: facts are applied as one batch, then the state is refreshed.
pub struct Game {
    pub c: Content,
    pub s: State,
    pub now: u32,
    pub log: Vec<Output>,
}

impl Game {
    pub fn new() -> Self {
        Self::with(shipped())
    }

    pub fn with(c: Content) -> Self {
        let (s, log) = new_game(&c);
        Self { c, s, now: 0, log }
    }

    /// The next turn number.
    pub fn turn(&mut self) -> u32 {
        self.now += 1;
        self.now
    }

    /// Applies a batch of facts, refreshes, and returns what happened.
    pub fn send(&mut self, facts: &[Fact]) -> Vec<Output> {
        for f in facts {
            self.s.apply(&self.c, f);
        }
        let out = refresh(&self.c, &mut self.s);
        self.log.extend(out.clone());
        out
    }

    /// One turn without anything happening.
    pub fn tick(&mut self) -> Vec<Output> {
        let at = self.turn();
        self.send(&[Fact::Tick { at }])
    }

    pub fn status(&self, id: &str) -> QuestStatus {
        self.s.status(&q(id))
    }

    pub fn compromise(&mut self, site: &str) -> Vec<Output> {
        let at = self.turn();
        self.send(&[Fact::SiteCompromised {
            site: site.into(),
            at,
        }])
    }

    pub fn talk(&mut self, contact: &str, times: u32) -> Vec<Output> {
        let facts: Vec<Fact> = (0..times)
            .map(|_| Fact::Talked {
                contact: contact.into(),
                at: self.turn(),
            })
            .collect();
        self.send(&facts)
    }

    pub fn heat(&mut self, heat: u8) -> Vec<Output> {
        let at = self.turn();
        self.send(&[Fact::HeatChanged { heat, at }])
    }

    pub fn extract(&mut self, site: &str, file: &str) -> Vec<Output> {
        self.send(&[Fact::FileExtracted {
            site: site.into(),
            file: file.into(),
        }])
    }

    /// Jumps to the start of chapter 4: M01-M08 done, tier 5, the contacts unlocked.
    pub fn chapter4(c: Content) -> Self {
        let mut g = Self::with(c);
        g.s.tier = 5;
        for id in [
            "m01", "m02", "m03", "m04", "m05", "s06", "m06", "m07", "m08",
        ] {
            g.s.quests.insert(
                q(id),
                QuestRun {
                    status: QuestStatus::Completed,
                    opened_at: 0,
                    done: std::collections::BTreeSet::default(),
                },
            );
        }
        for c in [
            "r4z0r", "phoenix", "miner", "angel", "ghost", "aura", "insider",
        ] {
            g.s.contacts
                .insert(ContactId::new(c), ContactState::Available);
        }
        g.send(&[]);
        g
    }
}
