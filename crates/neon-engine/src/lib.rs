//! Pure game engine of Neon Hack.
//!
//! Rules, state, events and the random number generator live here. Nothing in this
//! crate may read the clock, the environment, the disk or the console: that is enforced
//! by `clippy.toml`, so the engine stays deterministic, cloneable and testable without a
//! terminal. See `docs/design/architecture-rust.md`.
//!
//! The contract with the frontends is [`Game`]: the engine receives an [`Input`] and
//! answers a [`Step`] made of [`Event`]s and the next [`Prompt`]. [`demo`] is a small
//! complete game that exercises it.

pub mod demo;
pub mod event;
pub mod game;
pub mod ids;
pub mod prompt;
pub mod rng;
pub mod save;
pub mod text;

pub use event::Event;
pub use game::{Game, View};
pub use prompt::{Input, Prompt, Step};

/// Version of the engine crate, for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
