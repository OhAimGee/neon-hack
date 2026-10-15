//! The whole campaign, played to the epilogue through the commands the game offers.
//!
//! `neon_sim::brain` decides what to type (the optimistic player of the content tests,
//! extended to commands) and `neon_sim::driver` types it; `player` observes the real
//! `CampaignGame` through the frontend contract and checks everything it says, `walk` holds
//! the tests: every ending, every decision, both languages, saves, determinism. The disk is
//! never read: everything is embedded.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "this crate is test code: a failed expectation is the failure to report, and clippy only \
              exempts the `#[test]` functions themselves, not the helpers they share"
)]

mod player;
mod walk;
