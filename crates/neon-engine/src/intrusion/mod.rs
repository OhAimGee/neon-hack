//! The tactical intrusion, with the words: the commands that play it, the lines it says, and the
//! view an interface draws a panel from.
//!
//! [`crate::run`] is the rules and says nothing; this module turns its facts into the lines of
//! `docs/spec/run-screen.md` (one fact per line, in a fixed order, with the importance of § 7),
//! reads the commands of `docs/spec/commands.md` § 2.3 against the lists of the screen, and
//! gives [`RunView`], whose every field is also said in words: the plain interface is the
//! reference and the panel only repeats it.
//!
//! * [`Intrusion`] is an intrusion on a site: the run, and everything that is said of it.
//! * [`IntrusionGame`] plays one on its own, as a [`Game`](crate::Game): how the interfaces
//!   play an intrusion until the campaign drives [`Intrusion`] itself (lot R4.6).

mod commands;
mod game;
mod gloss;
mod lists;
mod screens;
mod session;
mod speak;
mod view;

#[cfg(test)]
mod tests;

pub use game::{IntrusionError, IntrusionGame};
pub use session::{Intrusion, StartError};
pub use view::{
    ActiveUtility, Charges, Cycles, DefenseView, LootState, NodeState, NodeView, PatrolView,
    ProgramView, RunView,
};

/// The names of the commands of an intrusion, in table order.
pub fn command_names() -> impl Iterator<Item = &'static str> {
    commands::SPECS.iter().map(|spec| spec.name)
}
