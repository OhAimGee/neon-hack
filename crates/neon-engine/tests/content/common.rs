//! Helpers shared by the tests: ids from text, the shipped content, a game driven by hand.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::str::FromStr;

use neon_engine::content::engine::{Outcome, new_game, refresh};
use neon_engine::content::ids::{ContactId, QuestId};
use neon_engine::content::schema::{ContactState, QuestStatus};
use neon_engine::content::state::QuestRun;
use neon_engine::content::{Content, Fact, State};

/// An id of any namespace from its text (the tests only write valid ones).
pub(crate) fn id<T: FromStr>(text: &str) -> T
where
    T::Err: Debug,
{
    text.parse()
        .unwrap_or_else(|e| panic!("`{text}` is not a valid id: {e:?}"))
}

pub(crate) fn shipped() -> Content {
    Content::embedded().unwrap_or_else(|e| panic!("shipped content is invalid:\n{e}"))
}

pub(crate) fn q(text: &str) -> QuestId {
    id(text)
}

/// A game driven by hand: facts are applied as one batch, then the state is refreshed.
pub(crate) struct Game {
    pub(crate) c: Content,
    pub(crate) s: State,
    pub(crate) now: u32,
    pub(crate) log: Vec<Outcome>,
}

impl Game {
    pub(crate) fn new() -> Self {
        Self::with(shipped())
    }

    pub(crate) fn with(c: Content) -> Self {
        let (s, log) = new_game(&c);
        Self { c, s, now: 0, log }
    }

    /// The next turn number.
    pub(crate) fn turn(&mut self) -> u32 {
        self.now += 1;
        self.now
    }

    /// Applies a batch of facts, refreshes, and returns what happened.
    pub(crate) fn send(&mut self, facts: &[Fact]) -> Vec<Outcome> {
        for f in facts {
            self.s.apply(&self.c, f);
        }
        let out = refresh(&self.c, &mut self.s);
        self.log.extend(out.clone());
        out
    }

    /// One turn without anything happening.
    pub(crate) fn tick(&mut self) -> Vec<Outcome> {
        let at = self.turn();
        self.send(&[Fact::Tick { at }])
    }

    pub(crate) fn status(&self, quest: &str) -> QuestStatus {
        self.s.status(&q(quest))
    }

    pub(crate) fn compromise(&mut self, site: &str) -> Vec<Outcome> {
        let at = self.turn();
        self.send(&[Fact::SiteCompromised { site: id(site), at }])
    }

    pub(crate) fn talk(&mut self, contact: &str, times: u32) -> Vec<Outcome> {
        let facts: Vec<Fact> = (0..times)
            .map(|_| Fact::Talked {
                contact: id(contact),
                at: self.turn(),
            })
            .collect();
        self.send(&facts)
    }

    pub(crate) fn heat(&mut self, heat: u8) -> Vec<Outcome> {
        let at = self.turn();
        self.send(&[Fact::HeatChanged { heat, at }])
    }

    pub(crate) fn extract(&mut self, site: &str, file: &str) -> Vec<Outcome> {
        self.send(&[Fact::FileExtracted {
            site: id(site),
            file: id(file),
        }])
    }

    /// Jumps to the start of chapter 4: M01-M08 done, tier 5, the contacts unlocked.
    pub(crate) fn chapter4(c: Content) -> Self {
        let mut g = Self::with(c);
        g.s.tier = 5;
        for done in [
            "m01", "m02", "m03", "m04", "m05", "s06", "m06", "m07", "m08",
        ] {
            g.s.quests.insert(
                q(done),
                QuestRun {
                    status: QuestStatus::Completed,
                    opened_at: 0,
                    done: BTreeSet::default(),
                },
            );
        }
        for contact in [
            "r4z0r", "phoenix", "miner", "angel", "ghost", "aura", "insider",
        ] {
            g.s.contacts
                .insert(id::<ContactId>(contact), ContactState::Available);
        }
        g.send(&[]);
        g
    }
}
