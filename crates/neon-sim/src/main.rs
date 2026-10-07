//! `neon-sim`: balancing harness.
//!
//! Placeholder of phase R0. The bots, metrics and acceptance thresholds are specified in
//! phase P1 and built in phase R6 (see `docs/ROADMAP.md`).

fn main() {
    report::banner();
}

/// Console output of the harness.
mod report {
    #![allow(clippy::print_stdout)]

    pub(super) fn banner() {
        println!(
            "neon-sim: balancing harness, not implemented yet (engine {}).",
            neon_engine::VERSION
        );
    }
}
