//! The campaign content and the language that drives it: quests, objectives, flags,
//! decisions, dialogue topics, endings.
//!
//! The content is declarative data (`data/world/*.toml`, embedded in the program by
//! `build.rs`) that holds no story text, only ids; the text of an id comes from a derived key
//! ([`texts`]). It is loaded and validated once ([`Content`]), then read by pure functions of
//! `(Content, State)`:
//!
//! 1. the game records what happened as [`Fact`]s with [`State::apply`], which only writes in
//!    a *ledger* of sets, maxima and a Pareto frontier (idempotent and commutative);
//! 2. [`refresh`] reads the ledger, moves the quest statuses to a fixed point, pays the
//!    rewards and tells what happened as [`Outcome`]s, which the campaign game turns into
//!    [`Event`](crate::Event)s.
//!
//! There is no clock (the time is a turn counter), no I/O, no callback, no loop in the data.
//! The language reference, in French, is `docs/spec/missions-language.md`; the decisions it
//! rests on are in `docs/spec/missions.md`.
//!
//! The state is saved with [`persist`]: an ordered, text-keyed table that
//! [`State::validate`] checks against the content when a save is loaded.

#![warn(missing_docs)]

pub mod engine;
pub mod eval;
pub mod ids;
pub mod loader;
pub mod money;
pub mod persist;
pub mod schema;
pub mod state;
pub mod texts;
pub mod validate;

pub use engine::{Outcome, new_game, refresh};
pub use loader::{Content, Diagnostic, File, LoadError, Sources};
pub use money::{Credits, Reputation};
pub use state::{Fact, State};
