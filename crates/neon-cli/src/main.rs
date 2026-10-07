//! `neon-hack`: the command-line entry point.
//!
//! Placeholder of phase R0: the frontends (plain, then TUI) arrive with phase R1 and
//! the game itself with phase R2 (see `docs/ROADMAP.md`).

use clap::Parser;

/// Neon Hack: a cyberpunk text RPG for the terminal.
#[derive(Debug, Parser)]
#[command(name = "neon-hack", version)]
struct Cli {}

fn main() {
    let Cli {} = Cli::parse();
    plain::not_playable_yet();
}

/// Output of the plain frontend. It is the only place allowed to write to the console:
/// a stray `println!` elsewhere would corrupt the full-screen TUI.
mod plain {
    #![allow(clippy::print_stdout)]

    pub(super) fn not_playable_yet() {
        println!(
            "Neon Hack {}: not playable yet, the Rust rewrite is in progress (see docs/ROADMAP.md).",
            env!("CARGO_PKG_VERSION")
        );
    }
}
