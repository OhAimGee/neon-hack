//! `neon-sim`: balancing harness.
//!
//! The harness plays the real engine with automatic players and reports what it sees
//! (`docs/spec/neon-sim.md`). It never changes the game data: it produces reports, and a person
//! sets the numbers.
//!
//! - [`brain`] decides what to type next from the public state of the game, and [`driver`]
//!   types it through the frontend contract, handing every step to an observer. The engine's
//!   own tests use both.
//! - [`record`] and [`campaign`] play batches of games and keep what each one cost.
//! - [`sites`] measures every site with the solver.
//! - [`report`] writes all of it as canonical JSON, [`targets`] holds the thresholds as data and
//!   checks reports against them.

pub mod brain;
pub mod campaign;
pub mod driver;
pub mod record;
pub mod report;
pub mod sites;
pub mod targets;
