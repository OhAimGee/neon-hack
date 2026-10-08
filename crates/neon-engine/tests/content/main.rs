//! The campaign content language, played and attacked from the outside.
//!
//! One test binary for the whole language: `common` is the hand-driven game, `optimist` the
//! optimistic player (a test tool, not part of the engine), and each other module a family of
//! checks. The shipped data is `data/world/*.toml`; no test reads the disk, they all go
//! through the embedded copy.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "this crate is test code: a failed expectation is the failure to report, and clippy only \
              exempts the `#[test]` functions themselves, not the helpers they share"
)]

mod common;
mod language;
mod optimist;
mod persist;
mod props;
mod regressions;
mod shipped;
mod validation;
mod view;
mod walk;
