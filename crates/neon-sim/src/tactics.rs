//! How a bot plays an intrusion.
//!
//! The optimist follows the plan of the solver. The other bots play by hand, one action at a
//! time, from the state the game shows: a heuristic of their own, so that they can get burned.
//! Chance plays no part in the rules of an intrusion, so a plan of the solver never burns; only a
//! player who does not search can, and what the harness measures is how often.
//!
//! Both go for the nearest loot, breach what stands on the way, take everything and jack out.
//! They differ in what they spend and what they look at:
//!
//! - [`Style::Reckless`] breaches with the program that is the strongest against the ICE,
//!   whatever its noise, never uses a utility and ends the turn without a look at the forecast.
//! - [`Style::Prudent`] breaches with the program that makes the least noise for its progress,
//!   reads the forecast before it ends a turn, cloaks when a scan is coming and spoofs the
//!   noise it made when it made a lot.

use neon_engine::run::{
    Action, Mission, NodeId, Program, RunState, Supply, Utility, forecast, legal_actions,
};

/// What the prudent player calls a lot of noise in one turn: it spoofs above this.
const NOISY_TURN: u8 = 6;

/// A way to play an intrusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// The strongest program, no cloak, no forecast.
    Reckless,
    /// The quietest program, the forecast read, the cloak and the spoof used.
    Prudent,
}

/// The action the style plays in a state of a run: always one the rules accept, `End` when
/// nothing else is worth doing.
#[must_use]
pub fn choose(style: Style, mission: &Mission, state: &RunState) -> Action {
    let legal = legal_actions(mission, state);
    let can = |action: Action| legal.contains(&action);
    if can(Action::Loot) {
        return Action::Loot;
    }
    let Some(target) = nearest_loot(mission, state) else {
        // Everything is taken.
        return if can(Action::Jackout) {
            Action::Jackout
        } else {
            Action::End
        };
    };
    let here = state.position();
    let Some(hop) = next_hop(mission, here, target) else {
        return end_turn(style, mission, state, &legal);
    };
    if state.is_open(mission, hop) {
        return if can(Action::Move(hop)) {
            Action::Move(hop)
        } else {
            end_turn(style, mission, state, &legal)
        };
    }
    match breach_with(style, mission, hop, &legal) {
        Some(action) => action,
        None => end_turn(style, mission, state, &legal),
    }
}

/// The node with loot still to take that is the nearest, the lowest number first.
fn nearest_loot(mission: &Mission, state: &RunState) -> Option<NodeId> {
    mission
        .nodes()
        .filter(|node| mission.loot(*node).is_some() && !state.is_looted(*node))
        .min_by_key(|node| (mission.distance(state.position(), *node), node.index()))
}

/// The linked node that gets closer to the target, the lowest number first.
fn next_hop(mission: &Mission, here: NodeId, target: NodeId) -> Option<NodeId> {
    let now = mission.distance(here, target);
    mission
        .neighbours(here)
        .filter(|node| mission.distance(*node, target) < now)
        .min_by_key(|node| (mission.distance(*node, target), node.index()))
}

/// The breach of `node` the style prefers among those the rules accept now.
fn breach_with(style: Style, mission: &Mission, node: NodeId, legal: &[Action]) -> Option<Action> {
    let ice = mission.ice(node)?;
    legal
        .iter()
        .filter_map(|action| match action {
            Action::Breach { node: at, program } if *at == node => Some(*program),
            _ => None,
        })
        .max_by(|a, b| match style {
            Style::Reckless => strength(*a, ice.family).cmp(&strength(*b, ice.family)),
            Style::Prudent => {
                quietness(*a, ice.family, mission).cmp(&quietness(*b, ice.family, mission))
            }
        })
        .map(|program| Action::Breach { node, program })
}

/// How hard a program hits an ICE of a family.
fn strength(program: Program, family: neon_engine::run::Family) -> u8 {
    program.power_against(family)
}

/// A score that grows as the noise per point of progress falls; an unlimited program wins a
/// tie, the consumable being worth keeping.
fn quietness(program: Program, family: neon_engine::run::Family, mission: &Mission) -> (u32, bool) {
    let spec = program.spec();
    let power = u32::from(program.power_against(family)).max(1);
    // Noise per progress in thousandths, the lower the better.
    let per_point = u32::from(spec.noise) * 1000 / power;
    let unlimited = mission.supply(program) == Some(Supply::Unlimited);
    (u32::MAX - per_point, unlimited)
}

/// What to do when the turn is done: the prudent player looks first at what the end of the turn
/// would bring.
fn end_turn(style: Style, mission: &Mission, state: &RunState, legal: &[Action]) -> Action {
    if style == Style::Prudent {
        let can = |utility: Utility| legal.contains(&Action::Use(utility));
        let coming = forecast(mission, state, 1).total();
        let ambient = mission.gain(mission.ambient());
        if can(Utility::Cloak) && !state.cloak_active() && coming > ambient {
            return Action::Use(Utility::Cloak);
        }
        let noise_this_turn = state.trace().saturating_sub(state.turn_floor());
        if can(Utility::Spoof) && noise_this_turn >= NOISY_TURN {
            return Action::Use(Utility::Spoof);
        }
    }
    Action::End
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "test code: a failed expectation is the failure to report"
    )]

    use neon_engine::run::solve::{Goal, Outcome, solve};
    use neon_engine::run::{Family, Ice, LootKind, MissionBuilder, Status, apply};

    use super::*;

    /// A run of two nodes: a gateway and a vault behind an ICE.
    fn small(ice: Ice, programs: &[Program]) -> Mission {
        let mut builder = MissionBuilder::new();
        let gate = builder.node(None, None);
        let vault = builder.node(Some(ice), Some(LootKind::Objective));
        builder.link(gate, vault);
        builder.programs(programs);
        builder.cycles_per_turn(3);
        builder.build().unwrap()
    }

    fn play(style: Style, mission: &Mission) -> RunState {
        let mut state = mission.initial_state();
        for _ in 0..200 {
            if state.status() != Status::Running {
                break;
            }
            let action = choose(style, mission, &state);
            state = apply(mission, &state, action, &mut ()).unwrap();
        }
        state
    }

    #[test]
    fn the_reckless_player_picks_the_strongest_program_and_the_prudent_one_the_quietest() {
        let ice = Ice {
            family: Family::Network,
            strength: 4,
        };
        let mission = small(ice, &[Program::BruteForce, Program::Backdoor]);
        let state = mission.initial_state();
        let vault = NodeId::new(1).unwrap();
        // Brute force: power 2 against a network for 5 noise. Backdoor: 1 for 1.
        assert_eq!(
            choose(Style::Reckless, &mission, &state),
            Action::Breach {
                node: vault,
                program: Program::BruteForce
            }
        );
        assert_eq!(
            choose(Style::Prudent, &mission, &state),
            Action::Breach {
                node: vault,
                program: Program::Backdoor
            }
        );
    }

    #[test]
    fn both_styles_finish_an_easy_run_by_taking_the_loot_and_leaving() {
        let ice = Ice {
            family: Family::Network,
            strength: 2,
        };
        let mission = small(ice, &[Program::BruteForce, Program::Backdoor]);
        for style in [Style::Reckless, Style::Prudent] {
            let end = play(style, &mission);
            assert_eq!(end.status(), Status::Won, "{style:?}");
        }
    }

    #[test]
    fn a_run_the_player_cannot_afford_ends_burned_and_never_loops() {
        // Strength 6 behind a cap of 20: the noise of the breaches burns the run.
        let ice = Ice {
            family: Family::Human,
            strength: 6,
        };
        let mut builder = MissionBuilder::new();
        let gate = builder.node(None, None);
        let vault = builder.node(Some(ice), Some(LootKind::Objective));
        builder.link(gate, vault);
        builder.programs(&[Program::BruteForce]);
        builder.cycles_per_turn(3);
        builder.trace_cap(20);
        let mission = builder.build().unwrap();
        for style in [Style::Reckless, Style::Prudent] {
            assert_eq!(play(style, &mission).status(), Status::Burned, "{style:?}");
        }
        // The solver agrees that this run cannot be won.
        let report = solve(
            &mission,
            &mission.initial_state(),
            Goal::Feasible,
            100_000,
            u8::MAX,
        );
        assert!(matches!(report.outcome, Outcome::Unsolvable));
    }

    #[test]
    fn the_shipped_sites_can_be_played_to_an_end_by_both_styles_at_every_difficulty() {
        let content = neon_engine::content::Content::embedded().unwrap();
        for site in &content.sites {
            for difficulty in neon_engine::campaign::Difficulty::ALL {
                let Some(mission) = content.mission(&site.id, difficulty.pace()) else {
                    continue;
                };
                for style in [Style::Reckless, Style::Prudent] {
                    let end = play(style, &mission);
                    assert_ne!(end.status(), Status::Running, "{} {style:?}", site.id);
                    if std::env::var("NEON_SIM_SHOW").is_ok() {
                        eprintln!(
                            "{:20} {:8} {:8?} {:?} trace {} turn {}",
                            site.id,
                            format!("{difficulty:?}"),
                            style,
                            end.status(),
                            end.trace(),
                            end.turn()
                        );
                    }
                }
            }
        }
    }
}
