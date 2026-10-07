//! Pure game engine of Neon Hack.
//!
//! Rules, state, events and the random number generator live here. Nothing in this
//! crate may read the clock, the environment, the disk or the console: that is enforced
//! by `clippy.toml`, so the engine stays deterministic, cloneable and testable without a
//! terminal. See `docs/design/architecture-rust.md`.

pub mod rng;

/// Version of the engine crate, for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
