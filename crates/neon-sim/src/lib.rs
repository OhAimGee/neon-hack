//! `neon-sim`: balancing harness.
//!
//! The harness plays the real engine with automatic players and reports what it sees
//! (`docs/spec/neon-sim.md`). This library holds what the harness and the engine's own tests
//! share: the [`brain`], which decides what to type next from the public state of the game, and
//! the [`driver`], which types it through the frontend contract and hands every step to an
//! observer.

pub mod brain;
pub mod driver;
