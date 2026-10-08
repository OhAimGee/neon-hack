//! Spike P1: a small declarative language for missions.
//!
//! The content (`data/*.toml`) declares quests, objectives, flags, decisions, topics and
//! endings; this crate loads and validates it ([`Content`]), evaluates it without any engine
//! ([`State`], [`Fact`], [`refresh`]) and proves there is no dead end by playing it
//! ([`optimist`]). See `README.md` (in French) for the language reference.

#![forbid(unsafe_code)]

pub mod content;
pub mod engine;
pub mod eval;
pub mod ids;
pub mod optimist;
pub mod schema;
pub mod state;
pub mod texts;
pub mod validate;

pub use content::{Content, Diagnostic, File, LoadError, Sources};
pub use engine::{Output, new_game, refresh};
pub use state::{Fact, State};
