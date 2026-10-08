//! The campaign content language, played and attacked from the outside.
//!
//! One test binary for the whole language: `common` is the hand-driven game, `optimist` the
//! optimistic player (a test tool, not part of the engine), and each other module a family of
//! checks. The shipped data is `data/world/*.toml`; no test reads the disk, they all go
//! through the embedded copy.

mod common;
mod language;
mod optimist;
mod props;
mod regressions;
mod shipped;
mod validation;
mod walk;
