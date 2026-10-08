//! Spike P1: can an exact deterministic solver live in `neon-engine` at run time?
//!
//! See `README.md` (French) for the question, the method and the numbers.
#![forbid(unsafe_code)]

pub mod analysis;
pub mod brute;
pub mod fallback;
pub mod heuristic;
pub mod missions;
pub mod model;
pub mod rng;
pub mod solver;
pub mod table;
pub mod testing;
