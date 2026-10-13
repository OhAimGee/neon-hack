//! The forecast: what the end of the turn will do, worked out by doing it.

use super::action::{Action, Fact};
use super::mission::Mission;
use super::program::Program;
use super::rules::apply;
use super::state::{RunState, Status};

/// What one end of turn does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastTurn {
    /// The turn that ends (from 1).
    pub turn: u32,
    /// Every fact of the end of turn, exactly as ending it would say them.
    pub facts: Vec<Fact>,
    /// The Trace the end of turn adds.
    pub total: u8,
    /// Whether the player could still use a Cloak charge before this turn ends: the forecast
    /// then says that a scan can be cancelled, and says nothing of it otherwise.
    pub cloak_usable: bool,
    /// The state after it.
    pub after: RunState,
}

/// What ending the turn now would do, for one turn or two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forecast {
    /// One entry per turn announced, the first being the turn under way. Shorter than asked
    /// when the run would be burned before.
    pub turns: Vec<ForecastTurn>,
}

impl Forecast {
    /// The Trace the end of the turn under way adds: zero when the run is over.
    #[must_use]
    pub fn total(&self) -> u8 {
        self.turns.first().map_or(0, |turn| turn.total)
    }
}

/// Announces the end of the turn under way and, with `turns` above one, those that follow if
/// the player did nothing more.
///
/// It resolves the end of turn through [`apply`], the function that really ends it, so the
/// announcement and the outcome cannot differ. Chance plays no part in the rules, so there is
/// nothing the forecast leaves out.
#[must_use]
pub fn forecast(mission: &Mission, state: &RunState, turns: u8) -> Forecast {
    let mut announced = Vec::new();
    let mut current = *state;
    for _ in 0..turns {
        if current.status != Status::Running {
            break;
        }
        let cloak_usable = !current.cloak
            && current.can_use(mission, Program::Cloak)
            && current.cycles >= Program::Cloak.spec().cycles;
        let mut facts = Vec::new();
        let Ok(after) = apply(mission, &current, Action::End, &mut facts) else {
            break;
        };
        let total = facts
            .iter()
            .map(|fact| match fact {
                Fact::TraceUp { added } => *added,
                _ => 0,
            })
            .fold(0u8, u8::saturating_add);
        announced.push(ForecastTurn {
            turn: current.turn,
            facts,
            total,
            cloak_usable,
            after,
        });
        current = after;
    }
    Forecast { turns: announced }
}
