//! The solver against an independent brute force, and against its own promises.

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::search::{Settings, search};
use super::*;
use crate::run::tests::{Dice, every_action, payroll, random_mission, random_play};
use crate::run::{Family, Ice, LootKind, MissionBuilder, NodeId, Program, Status, Supply, apply};

/// Plays a plan from a state and returns the state it ends in.
fn replay(mission: &Mission, start: &RunState, plan: &Plan) -> RunState {
    let mut state = *start;
    for action in &plan.actions {
        state = apply(mission, &state, *action, &mut ())
            .unwrap_or_else(|e| panic!("{action:?} refused in {state:?}: {e}"));
    }
    state
}

/// The lowest final Trace by exhaustive search, sharing only `apply` with the solver: no
/// action generator, no canonical state, no bound, no dominance. States are whole `RunState`
/// values, layer by layer (one layer per turn).
fn brute_force(mission: &Mission) -> Option<u8> {
    let universe = every_action(mission);
    let mut layer: BTreeSet<RunState> = BTreeSet::from([mission.initial_state()]);
    let mut best: Option<u8> = None;
    let mut turns = 0u32;
    while !layer.is_empty() {
        let mut next = BTreeSet::new();
        let mut seen = BTreeSet::new();
        for state in &layer {
            seen.clear();
            walk(mission, &universe, state, &mut seen, &mut next, &mut best);
        }
        layer = next;
        turns += 1;
        // The Trace at the start of a turn never goes down and every end of turn adds the
        // ambient Trace, so no later layer can end under `turns * ambient`.
        if let Some(cost) = best
            && turns * u32::from(mission.gain(mission.ambient())) >= u32::from(cost)
        {
            break;
        }
    }
    best
}

fn walk(
    mission: &Mission,
    universe: &[Action],
    state: &RunState,
    seen: &mut BTreeSet<RunState>,
    next: &mut BTreeSet<RunState>,
    best: &mut Option<u8>,
) {
    for action in universe {
        let Ok(child) = apply(mission, state, *action, &mut ()) else {
            continue;
        };
        match (action, child.status()) {
            (_, Status::Burned) => {}
            (_, Status::Won) => {
                *best = Some(best.map_or(child.trace(), |b| b.min(child.trace())));
            }
            (Action::End, _) => {
                next.insert(child);
            }
            _ => {
                if seen.insert(child) {
                    walk(mission, universe, &child, seen, next, best);
                }
            }
        }
    }
}

/// A mission small enough for the brute force: 2 to 4 nodes, weak ICE, a short deck.
fn small_mission(seed: u64) -> Mission {
    let mut dice = Dice::new(seed, 21);
    let count = 2 + dice.below(3);
    let mut builder = MissionBuilder::new();
    let mut nodes = Vec::new();
    for index in 0..count {
        let ice = (index > 0).then(|| Ice {
            family: Family::ALL[dice.below(4)],
            strength: 1 + dice.byte(3),
        });
        let loot = (index == count - 1).then_some(LootKind::Objective);
        nodes.push(builder.node(ice, loot));
    }
    for pair in nodes.windows(2) {
        builder.link(pair[0], pair[1]);
    }
    if dice.below(2) == 0 {
        let length = 1 + dice.below(3);
        let route: Vec<NodeId> = (0..length)
            .map(|_| nodes[dice.below(nodes.len())])
            .collect();
        builder.patrol(&route, None);
    }
    builder.program(Program::BruteForce);
    for program in [
        Program::Exploit,
        Program::Decrypt,
        Program::Virus,
        Program::Cloak,
        Program::Spoof,
        Program::Ghost,
        Program::Overclock,
    ] {
        if dice.below(3) == 0 {
            builder.program(program);
        }
    }
    builder
        .cycles_per_turn(2 + dice.byte(2))
        .gain_quarters([2, 4, 5][dice.below(3)])
        .ambient(1 + dice.byte(2))
        .trace_cap(14 + dice.byte(20))
        .intel_known(true);
    builder.build().expect("small missions are valid")
}

fn optimum_cost(mission: &Mission) -> Option<u8> {
    match optimum(mission, 0).outcome {
        Outcome::Solved { plan, optimal } => {
            assert!(optimal);
            Some(plan.trace)
        }
        Outcome::Unsolvable => None,
        Outcome::Unknown { .. } => panic!("no limit was set"),
    }
}

#[test]
fn the_payroll_run_has_a_plan_that_plays_and_beats_the_walkthrough() {
    let mission = payroll();
    let report = optimum(&mission, 0);
    let Outcome::Solved { plan, optimal } = report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(optimal);
    assert!(plan.trace < 23, "the walkthrough of the spec ends at 23");
    assert_eq!(plan.actions.last(), Some(&Action::Jackout));
    let end = replay(&mission, &mission.initial_state(), &plan);
    assert_eq!((end.status(), end.trace()), (Status::Won, plan.trace));
    assert_eq!(optimum_cost(&mission), brute_force(&mission));
}

#[test]
fn a_mission_that_cannot_be_won_is_proven_so() {
    // One breach makes more noise than the cap allows.
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, None);
    let b = builder.node(
        Some(Ice {
            family: Family::Network,
            strength: 2,
        }),
        Some(LootKind::Objective),
    );
    builder.link(a, b).program(Program::BruteForce).trace_cap(5);
    let mission = builder.build().unwrap();
    assert_eq!(prove_solvable(&mission, 0), Proof::Unsolvable);
    assert_eq!(optimum_cost(&mission), None);
    assert_eq!(brute_force(&mission), None);
    assert_eq!(
        auto_resolve(&mission, &mission.initial_state()),
        Resolution::Nominal
    );

    // And a deck with nothing to breach with cannot open the ICE at all.
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, None);
    let b = builder.node(
        Some(Ice {
            family: Family::Network,
            strength: 1,
        }),
        Some(LootKind::Objective),
    );
    builder.link(a, b).program(Program::Cloak);
    let mission = builder.build().unwrap();
    assert_eq!(prove_solvable(&mission, 0), Proof::Unsolvable);
}

#[test]
fn the_minimal_deck_leaves_out_what_runs_out() {
    let mission = payroll();
    let minimal = mission.unlimited_only();
    assert_eq!(minimal.supply(Program::BruteForce), Some(Supply::Unlimited));
    assert_eq!(minimal.supply(Program::Exploit), None);
    assert_eq!(minimal.supply(Program::Cloak), None);
    // It is harder: the optimum with fewer programs is never lower.
    let (full, minimal) = (optimum_cost(&mission), optimum_cost(&minimal));
    assert!(minimal >= full, "{minimal:?} < {full:?}");
}

#[test]
fn a_limit_gives_a_proven_lower_bound_and_the_best_plan_met() {
    let mission = payroll();
    let best = optimum_cost(&mission).unwrap();
    let mut saw_unknown = false;
    for limit in [1, 3, 10, 30, 100] {
        let report = solve(
            &mission,
            &mission.initial_state(),
            Goal::Optimal,
            limit,
            u8::MAX,
        );
        match report.outcome {
            Outcome::Unknown {
                best: plan,
                lower_bound,
            } => {
                saw_unknown = true;
                assert!(lower_bound <= best, "limit {limit}: {lower_bound} > {best}");
                assert!(report.stats.expanded <= limit);
                if let Some(plan) = plan {
                    assert!(plan.trace >= best);
                    let end = replay(&mission, &mission.initial_state(), &plan);
                    assert_eq!(end.status(), Status::Won);
                }
            }
            Outcome::Solved { plan, optimal } => {
                assert!(optimal);
                assert_eq!(plan.trace, best);
            }
            Outcome::Unsolvable => panic!("the payroll run can be won"),
        }
    }
    assert!(saw_unknown, "a limit of one expansion cannot finish");
}

#[test]
fn the_automatic_chain_gives_the_optimum_of_a_small_run_and_a_plan_otherwise() {
    let mission = payroll();
    let resolution = auto_resolve(&mission, &mission.initial_state());
    let Resolution::Optimal(plan) = &resolution else {
        panic!("{resolution:?}");
    };
    assert_eq!(Some(plan.trace), optimum_cost(&mission));
    assert_eq!(resolution.trace(), Some(plan.trace));
    assert_eq!(Resolution::Nominal.trace(), None);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(120))]

    /// The optimum is the brute force's, the plan plays, dominance changes nothing.
    #[test]
    fn the_optimum_agrees_with_the_brute_force(seed in any::<u64>()) {
        let mission = small_mission(seed);
        let expected = brute_force(&mission);
        let report = optimum(&mission, 0);
        match (&report.outcome, expected) {
            (Outcome::Solved { plan, optimal }, Some(cost)) => {
                prop_assert!(*optimal);
                prop_assert_eq!(plan.trace, cost);
                let end = replay(&mission, &mission.initial_state(), plan);
                prop_assert_eq!((end.status(), end.trace()), (Status::Won, cost));
            }
            (Outcome::Unsolvable, None) => {}
            (other, expected) => prop_assert!(false, "{other:?} against {expected:?}"),
        }
        let without = search(&mission, &mission.initial_state(), Settings {
            goal: Goal::Optimal, max_expanded: 0, cap: u8::MAX, dominance: false,
        });
        prop_assert_eq!(
            without.plan().map(|p| p.trace),
            report.plan().map(|p| p.trace)
        );
        prop_assert!(report.stats.expanded <= without.stats.expanded);
    }

    /// An early-exit plan plays, and is never better than the optimum.
    #[test]
    fn a_feasible_plan_plays_and_is_no_better_than_the_optimum(seed in any::<u64>()) {
        let mission = small_mission(seed);
        let best = optimum_cost(&mission);
        match prove_solvable(&mission, 0) {
            Proof::Solvable(plan) => {
                let end = replay(&mission, &mission.initial_state(), &plan);
                prop_assert_eq!((end.status(), end.trace()), (Status::Won, plan.trace));
                prop_assert!(Some(plan.trace) >= best);
            }
            Proof::Unsolvable => prop_assert_eq!(best, None),
            Proof::Unknown => prop_assert!(false, "no limit was set"),
        }
    }

    /// From any state met in play, a plan found plays from there.
    #[test]
    fn a_plan_from_the_middle_of_a_run_plays(seed in any::<u64>()) {
        let mission = random_mission(seed);
        let mut states = Vec::new();
        random_play(&mission, seed, 25, |state, _| states.push(*state));
        for state in states.iter().step_by(6).filter(|s| s.is_running()) {
            let report = solve(&mission, state, Goal::Feasible, 1500, u8::MAX);
            if let Outcome::Solved { plan, .. } = report.outcome {
                let end = replay(&mission, state, &plan);
                prop_assert_eq!((end.status(), end.trace()), (Status::Won, plan.trace));
            }
        }
    }

    /// The automatic chain never claims more than it has: its plan plays, and an
    /// `Optimal` one is not beaten by the anytime search.
    #[test]
    fn the_automatic_chain_gives_plans_that_play(seed in any::<u64>()) {
        let mission = random_mission(seed);
        let start = mission.initial_state();
        match auto_resolve(&mission, &start) {
            Resolution::Optimal(plan) | Resolution::Best(plan) => {
                let end = replay(&mission, &start, &plan);
                prop_assert_eq!((end.status(), end.trace()), (Status::Won, plan.trace));
            }
            Resolution::Nominal => {}
        }
    }
}

#[test]
fn the_dominance_keeps_a_node_part_way_open_when_a_virus_can_still_use_it() {
    // A seed the property test found: an open node is not better than a node open part way when
    // the Virus that finishes it also puts down the patrol standing there. The optimum with
    // dominance (the default) has to be the brute force's.
    let mission = small_mission(12_663_343_630_951_363_990);
    let expected = brute_force(&mission);
    assert_eq!(expected, Some(11));
    let report = optimum(&mission, 0);
    let Outcome::Solved { plan, optimal } = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(*optimal);
    assert_eq!(Some(plan.trace), expected);
}
