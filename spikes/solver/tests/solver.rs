//! Property-style and cross-check tests of the spike (deterministic, fast in debug).

use neon_spike_solver::analysis::{bracket, preview_each_turn};
use neon_spike_solver::brute::brute_force;
use neon_spike_solver::fallback::beam_plan;
use neon_spike_solver::heuristic::{Heuristic, INFEASIBLE};
use neon_spike_solver::missions;
use neon_spike_solver::model::{
    Action, Difficulty, Event, Mission, Program, State, Status, forecast, legal_actions, resolve,
    step, step_with,
};
use neon_spike_solver::rng::Pcg32;
use neon_spike_solver::solver::{Config, Goal, Outcome, solvable_within, solve, solve_from};
use neon_spike_solver::table::TableKind;
use neon_spike_solver::testing::{random_prefix, random_state};

fn configs() -> Vec<(&'static str, Config)> {
    let b = Config::baseline();
    vec![
        ("baseline", b),
        (
            "btree",
            Config {
                tables: TableKind::BTree,
                ..b
            },
        ),
        (
            "fine+h",
            Config {
                heuristic: true,
                ..b
            },
        ),
        (
            "fine+dom",
            Config {
                dominance: true,
                ..b
            },
        ),
        (
            "fine+h+dom",
            Config {
                heuristic: true,
                dominance: true,
                ..b
            },
        ),
        (
            "macro",
            Config {
                macro_turns: true,
                ..b
            },
        ),
        (
            "macro+h",
            Config {
                macro_turns: true,
                heuristic: true,
                ..b
            },
        ),
        ("tuned (fine+h+dom+close)", Config::tuned()),
        (
            "fine+h+dom+close",
            Config {
                heuristic: true,
                dominance: true,
                close_first: true,
                ..b
            },
        ),
        (
            "macro+h+dom",
            Config {
                dominance: true,
                ..Config::tuned()
            },
        ),
        (
            "macro+btree",
            Config {
                macro_turns: true,
                tables: TableKind::BTree,
                ..b
            },
        ),
    ]
}

fn plan_cost(m: &Mission, cfg: &Config) -> (u8, u16) {
    let r = solve(m, cfg);
    let Outcome::Solved { plan, optimal, .. } = &r.outcome else {
        panic!("{}: {:?}", m.name, r.outcome)
    };
    assert!(optimal);
    // The plan replays through the rules to a win at the claimed cost.
    let (end, _) = resolve(m, &m.initial(), &plan.actions).expect("plan is legal");
    assert_eq!(end.status, Status::Won);
    assert_eq!(end.trace, plan.cost);
    assert_eq!(plan.actions.last(), Some(&Action::Jackout));
    let ends = plan.actions.iter().filter(|a| **a == Action::End).count();
    assert_eq!(usize::from(plan.turns), ends);
    (plan.cost, plan.turns)
}

#[test]
fn solver_agrees_with_the_independent_brute_force() {
    for m in [
        missions::tiny_4(0),
        missions::tiny_ghost(),
        missions::easy_4(0),
    ] {
        let brute = brute_force(&m);
        let (cost, turns) = plan_cost(&m, &Config::baseline());
        assert_eq!(brute.best_cost, Some(cost), "{}", m.name);
        assert_eq!(brute.best_turns, Some(turns), "{}", m.name);
        // The canonical action filters lose nothing.
        let mut raw = m.clone();
        raw.canonical_actions = false;
        let (cost_raw, turns_raw) = plan_cost(&raw, &Config::baseline());
        assert_eq!((cost_raw, turns_raw), (cost, turns), "{}", m.name);
    }
}

#[test]
fn every_configuration_finds_the_same_optimum() {
    for m in [
        missions::tiny_4(1),
        missions::easy_4(1),
        missions::audit_6(0),
    ] {
        let (cost, turns) = plan_cost(&m, &Config::baseline());
        for (name, cfg) in configs() {
            let (c, t) = plan_cost(&m, &cfg);
            assert_eq!(c, cost, "{} / {name}", m.name);
            // The tie-break on turns is exact without heuristic nor dominance.
            if !cfg.heuristic && !cfg.dominance {
                assert_eq!(t, turns, "{} / {name}", m.name);
            }
        }
    }
}

#[test]
fn feasibility_mode_agrees_with_the_optimum() {
    // "Within a cap" means the run is burned as soon as the Trace reaches the cap, even if
    // a later Spoof would have brought it back down: so the cap that admits the optimal
    // plan is its peak Trace + 1, and no cap at or under the optimum admits any plan.
    for m in [
        missions::tiny_4(1),
        missions::easy_4(1),
        missions::audit_6(0),
    ] {
        let r = solve(&m, &Config::tuned());
        let plan = r.plan().expect("solvable").clone();
        let mut s = m.initial();
        let mut peak = 0;
        for a in &plan.actions {
            s = step(&m, &s, *a).unwrap();
            peak = peak.max(s.trace);
        }
        let yes = solvable_within(&m, peak + 1, &Config::tuned());
        assert!(
            matches!(yes.outcome, Outcome::Solved { optimal: false, .. }),
            "{}",
            m.name
        );
        let no = solvable_within(&m, plan.cost, &Config::tuned());
        assert_eq!(no.outcome, Outcome::Unsolvable, "{}", m.name);
        let _ = Goal::Feasible;
    }
}

#[test]
fn unsolvable_missions_are_proven() {
    let m = missions::unsolvable_deadlock();
    for (name, cfg) in configs() {
        assert_eq!(solve(&m, &cfg).outcome, Outcome::Unsolvable, "{name}");
    }
    assert_eq!(brute_force(&m).best_cost, None);
}

#[test]
fn expansion_limit_returns_unknown_with_a_valid_bracket() {
    let m = missions::audit_6(1);
    let exact = plan_cost(&m, &Config::tuned()).0;
    let cfg = Config {
        max_expanded: 20,
        ..Config::tuned()
    };
    let b = bracket(&m, &cfg, 8);
    assert!(!b.exact);
    assert!(b.lower <= exact);
    // A narrow beam may fail, a wide one must find a plan no better than the optimum.
    let wide = bracket(&m, &cfg, 64);
    assert!(wide.upper.expect("a beam of 64 finds a plan") >= exact);
}

#[test]
fn beam_plan_is_legal_and_never_better_than_the_optimum() {
    for m in missions::small() {
        let exact = plan_cost(&m, &Config::tuned()).0;
        for width in [1, 4, 32] {
            if let Some(p) = beam_plan(&m, &m.initial(), width) {
                let (end, _) = resolve(&m, &m.initial(), &p.actions).expect("legal");
                assert_eq!((end.status, end.trace), (Status::Won, p.cost));
                assert!(p.cost >= exact, "{} width {width}", m.name);
            }
        }
    }
}

#[test]
fn optimum_is_consistent_along_the_optimal_plan() {
    let m = missions::audit_6(1);
    let rows = preview_each_turn(&m, &Config::tuned());
    assert!(rows.len() >= 3);
    let first = rows[0].cost;
    assert!(first.is_some());
    for r in &rows {
        assert_eq!(
            r.cost, first,
            "re-solving at turn {} changed the optimum",
            r.turn
        );
    }
}

#[test]
fn difficulty_presets_are_monotone() {
    // Invariant I21 on the solver's optimum: Story <= Normal <= Expert <= Hardcore.
    for bonus in [0, 1] {
        let costs: Vec<u8> = [
            Difficulty::Story,
            Difficulty::Normal,
            Difficulty::Expert,
            Difficulty::Hardcore,
        ]
        .into_iter()
        .map(|d| plan_cost(&missions::tiny_4_with(d, bonus), &Config::tuned()).0)
        .collect();
        assert!(costs.windows(2).all(|w| w[0] <= w[1]), "{costs:?}");
    }
}

#[test]
fn heuristic_is_admissible_on_random_states() {
    let mut rng = Pcg32::new(2024, 1);
    for m in [missions::tiny_4(1), missions::easy_4(0)] {
        let h = Heuristic::new(&m);
        for _ in 0..60 {
            let s = random_state(&m, &mut rng, 14);
            let truth = solve_from(&m, &s, &Config::baseline());
            let f = h.f(&m, &s, true);
            match truth.cost() {
                Some(c) => assert!(f <= i32::from(c), "{}: f {f} > cost {c} at {s:?}", m.name),
                // Unsolvable from s: any bound is admissible.
                None => assert!(matches!(truth.outcome, Outcome::Unsolvable)),
            }
            assert!(f < INFEASIBLE || truth.cost().is_none());
        }
    }
}

#[test]
fn generator_and_rules_agree() {
    let mut rng = Pcg32::new(7, 7);
    for m in missions::all().into_iter().chain([missions::tiny_4(0)]) {
        let mut all = Vec::new();
        for n in 0..m.n as u8 {
            all.push(Action::Move(n));
            for p in Program::ALL.into_iter().filter(|p| p.is_breach()) {
                all.push(Action::Breach {
                    node: n,
                    program: p,
                });
            }
        }
        all.extend([
            Action::Cloak,
            Action::Spoof,
            Action::Ghost,
            Action::Overclock,
            Action::Loot,
        ]);
        all.extend([Action::Jackout, Action::End]);
        for _ in 0..40 {
            let s = random_state(&m, &mut rng, 12);
            let mut legal = Vec::new();
            legal_actions(&m, &s, &mut legal);
            for a in &legal {
                assert!(
                    step(&m, &s, *a).is_ok(),
                    "{}: {a:?} offered but refused",
                    m.name
                );
            }
            for a in &all {
                if step(&m, &s, *a).is_ok() && !legal.contains(a) {
                    // Only the documented canonical filters may hide an accepted action.
                    let ok = matches!(a, Action::Ghost | Action::Spoof | Action::Cloak) || s.cloak;
                    assert!(ok, "{}: {a:?} accepted but not offered at {s:?}", m.name);
                }
            }
        }
    }
}

/// Reference scan rule, written apart from `step`: used to check the forecast.
fn expected_scan_total(m: &Mission, s: &State) -> u8 {
    let phase = (s.phase + 1) % m.period;
    let mut total = 0u8;
    for i in 0..m.sentinels.len() {
        let active = s.neutral[i] == 0
            && m.sentinels[i]
                .anchor
                .is_none_or(|a| s.progress[a as usize] < m.ice_strength[a as usize]);
        if !active {
            continue;
        }
        let at = m.sentinel_pos(i, phase) as usize;
        let d = m.dist[at][s.pos as usize];
        if d <= 1 || (s.trace >= 50 && d <= 2) {
            total += m.gain(5);
        }
    }
    total
}

#[test]
fn forecast_equals_resolution() {
    // Invariant I10, property style: the preview of the end of turn is what happens.
    let mut rng = Pcg32::new(99, 5);
    let set = [
        missions::tiny_4(0),
        missions::audit_6(0),
        missions::patrol_5(0),
        missions::cipher_6(0),
    ];
    let mut checked = 0;
    for m in &set {
        for _ in 0..250 {
            let s0 = random_state(m, &mut rng, 20);
            let plan = random_prefix(m, &s0, &mut rng, 6);
            let before = s0;
            let Ok(f) = forecast(m, &s0, &plan) else {
                continue;
            };
            assert_eq!(s0, before, "forecast must not mutate");
            let mut full = plan.clone();
            full.push(Action::End);
            let (end, events) = resolve(m, &s0, &full).expect("same plan resolves");
            assert_eq!(f.after_turn, end);
            let tail: Vec<Event> = events
                .iter()
                .copied()
                .skip_while(|e| {
                    !matches!(
                        e,
                        Event::PatrolMoved { .. }
                            | Event::Scan { .. }
                            | Event::ScanCancelled { .. }
                            | Event::Ambient { .. }
                    )
                })
                .collect();
            let end_events: Vec<Event> = f
                .events
                .iter()
                .copied()
                .filter(|e| !matches!(e, Event::Noise { .. }))
                .collect();
            let tail_f: Vec<Event> = tail
                .iter()
                .copied()
                .filter(|e| !matches!(e, Event::Noise { .. }))
                .collect();
            assert!(
                tail_f.ends_with(&end_events),
                "{}: {tail_f:?} vs {end_events:?}",
                m.name
            );
            // Independent check of the scan total (cloak cancels it all).
            if f.after_plan.status == Status::Running && !f.after_plan.cloak {
                let scans: u8 = f
                    .events
                    .iter()
                    .filter_map(|e| {
                        if let Event::Scan { added, .. } = e {
                            Some(*added)
                        } else {
                            None
                        }
                    })
                    .sum();
                assert_eq!(scans, expected_scan_total(m, &f.after_plan), "{}", m.name);
            }
            checked += 1;
        }
    }
    assert!(checked > 500);
}

#[test]
fn step_with_events_matches_step() {
    let mut rng = Pcg32::new(5, 5);
    let m = missions::cipher_6(0);
    for _ in 0..300 {
        let s = random_state(&m, &mut rng, 20);
        let mut legal = Vec::new();
        legal_actions(&m, &s, &mut legal);
        for a in legal {
            let mut ev = Vec::new();
            assert_eq!(step(&m, &s, a), step_with(&m, &s, a, &mut ev));
        }
    }
}

#[test]
fn pack_roundtrips_on_random_states() {
    let mut rng = Pcg32::new(31, 4);
    for m in missions::all() {
        for _ in 0..200 {
            let s = random_state(&m, &mut rng, 25);
            assert_eq!(State::unpack(&m, s.pack()), s, "{}", m.name);
        }
    }
}

#[test]
fn every_playout_terminates() {
    // Invariant I17: with ambient Trace > 0, waiting burns the run in at most cap/ambient turns.
    let m = missions::worst_8(Difficulty::Normal, 0);
    let mut s = m.initial();
    let mut turns = 0;
    while s.status == Status::Running {
        s = step(&m, &s, Action::End).expect("End is always legal");
        turns += 1;
        assert!(turns <= 100);
    }
    assert_eq!(s.status, Status::Burned);
    assert!(turns <= 50 + 1);
}

#[test]
fn preview_line_is_built_from_events() {
    let m = missions::audit_6(0);
    let s = m.initial();
    let f = forecast(&m, &s, &[]).unwrap();
    let line = f.line(&m);
    assert!(line.contains("ambient +2"), "{line}");
}

#[test]
fn a_burning_scan_stops_the_remaining_scans() {
    // Two sentinels see the player, the Trace cap is the value of one scan: the first scan
    // burns the run and the second one must neither be announced nor added.
    use neon_spike_solver::model::{Family, Loot, MissionBuilder};
    let mut b = MissionBuilder::new("burn", 1, Difficulty::Normal);
    let g = b.node("gateway", None, Loot::None);
    let s1 = b.node("guard-1", Some((Family::AiGuardian, 3)), Loot::None);
    let s2 = b.node("guard-2", Some((Family::AiGuardian, 3)), Loot::None);
    let v = b.node("vault", Some((Family::Network, 2)), Loot::Objective);
    b.edge(g, s1).edge(g, s2).edge(g, v);
    b.sentinel(&[s1], Some(s1)).sentinel(&[s2], Some(s2));
    b.program(Program::BruteForce).trace_cap(5);
    let m = b.build();
    let mut events = Vec::new();
    let end = step_with(&m, &m.initial(), Action::End, &mut events).unwrap();
    assert_eq!(end.status, Status::Burned);
    let scans: Vec<u8> = events
        .iter()
        .filter_map(|e| {
            if let Event::Scan { added, .. } = e {
                Some(*added)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(scans.len(), 1, "{events:?}");
    assert_eq!(
        scans.iter().sum::<u8>(),
        end.trace,
        "announced Trace must be the added Trace"
    );
    // The forecast shares the same path.
    let f = forecast(&m, &m.initial(), &[]).unwrap();
    assert_eq!(
        f.events
            .iter()
            .filter(|e| matches!(e, Event::Scan { .. }))
            .count(),
        1
    );
}

#[test]
fn tuned_is_the_measured_best_configuration() {
    let t = Config::tuned();
    assert!(!t.macro_turns, "end-of-turn merging never wins in time");
    assert!(t.heuristic && t.dominance && t.close_first);
    assert_eq!(t.tables, TableKind::Fx);
    assert_eq!(t.goal, Goal::Optimal);
}

#[test]
fn optimality_guarantees_are_reported_separately() {
    // The Trace is always optimal in `Goal::Optimal`, the number of turns only for the
    // plain search; wherever the turns differ the flag must say so.
    let base = Config::baseline();
    let mut differing = 0;
    for m in [
        missions::tiny_4(0),
        missions::tiny_4(1),
        missions::easy_4(1),
        missions::audit_6(0),
        missions::audit_6(1),
        missions::patrol_5(0),
    ] {
        let exact = solve(&m, &base);
        let Outcome::Solved {
            plan: p0,
            optimal,
            turns_minimal,
        } = &exact.outcome
        else {
            panic!()
        };
        assert!(*optimal && *turns_minimal, "plain search guarantees both");
        for cfg in [
            Config {
                heuristic: true,
                ..base
            },
            Config {
                dominance: true,
                ..base
            },
            Config::tuned(),
        ] {
            let r = solve(&m, &cfg);
            let Outcome::Solved {
                plan,
                optimal,
                turns_minimal,
            } = &r.outcome
            else {
                panic!()
            };
            assert!(*optimal);
            assert_eq!(plan.cost, p0.cost);
            assert!(
                !*turns_minimal,
                "heuristic or dominance: turns are not guaranteed minimal"
            );
            assert!(plan.turns >= p0.turns);
            differing += usize::from(plan.turns > p0.turns);
        }
    }
    // The flag is conservative: no counter-example was found on these missions, but A* with
    // an inconsistent bound or dominance does not prove the minimum of turns, so it is not claimed.
    let _ = differing;
}

#[test]
fn the_expansion_limit_is_exact() {
    let m = missions::audit_6(1);
    for limit in [1u64, 20, 100] {
        let cfg = Config {
            max_expanded: limit,
            ..Config::tuned()
        };
        let r = solve(&m, &cfg);
        assert!(
            matches!(r.outcome, Outcome::Unknown { .. }),
            "limit {limit}"
        );
        assert_eq!(
            r.stats.expanded, limit,
            "the state that hits the limit is not expanded"
        );
    }
}

#[test]
fn anytime_never_exceeds_its_budget() {
    use neon_spike_solver::analysis::anytime_plan;
    for m in [missions::audit_6(1), missions::patrol_5(2)] {
        for budget in [50u64, 300, 2_000] {
            if let Some((plan, used)) = anytime_plan(&m, &m.initial(), budget) {
                assert!(used <= budget, "{}: used {used} of {budget}", m.name);
                let (end, _) = resolve(&m, &m.initial(), &plan.actions).unwrap();
                assert_eq!(end.status, Status::Won);
            }
        }
    }
}
