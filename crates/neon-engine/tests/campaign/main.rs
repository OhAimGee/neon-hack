//! The whole campaign, played to the epilogue through the commands the game offers.
//!
//! `brain` decides what to type (the optimistic player of the content tests, extended to
//! commands), `player` drives the real `CampaignGame` through the frontend contract and
//! checks everything it says, `walk` holds the tests: every ending, every decision, both
//! languages, saves, determinism. The disk is never read: everything is embedded.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "this crate is test code: a failed expectation is the failure to report, and clippy only \
              exempts the `#[test]` functions themselves, not the helpers they share"
)]

mod brain;
mod player;
mod walk;
