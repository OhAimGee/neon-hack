//! Tests of the rules of the intrusion: the golden example of the spec, one test per rule,
//! and the properties that must hold on any mission (RUN-2, RUN-10, RUN-11).

use std::hash::{DefaultHasher, Hash, Hasher};

use proptest::prelude::*;

use super::*;
use crate::rng::Pcg32;

fn node(mission: &Mission, number: u8) -> NodeId {
    NodeId::from_number(number)
        .filter(|id| id.index() < mission.node_count())
        .expect("a node of the mission")
}

/// The sample mission of `docs/spec/run-screen.md` § 5: five nodes, one patrol.
pub(crate) fn payroll() -> Mission {
    let mut builder = MissionBuilder::new();
    let gateway = builder.node(None, None);
    let firewall = builder.node(
        Some(Ice {
            family: Family::Network,
            strength: 2,
        }),
        None,
    );
    let archives = builder.node(None, None);
    let vault = builder.node(
        Some(Ice {
            family: Family::Network,
            strength: 4,
        }),
        Some(LootKind::Objective),
    );
    let cameras = builder.node(
        Some(Ice {
            family: Family::AiGuardian,
            strength: 3,
        }),
        None,
    );
    for (a, b) in [
        (gateway, firewall),
        (gateway, archives),
        (firewall, archives),
        (archives, vault),
        (archives, cameras),
        (vault, cameras),
    ] {
        builder.link(a, b);
    }
    builder
        .patrol(&[cameras, archives, firewall, archives], None)
        .programs(&[Program::BruteForce, Program::Exploit, Program::Cloak])
        .intel_known(true);
    builder.build().expect("the payroll mission is valid")
}

fn breach(mission: &Mission, number: u8, program: Program) -> Action {
    Action::Breach {
        node: node(mission, number),
        program,
    }
}

#[test]
fn the_payroll_run_goes_as_the_spec_says() {
    let mission = payroll();
    let mut run = Run::new(mission.clone(), Undo::Unlimited);
    let m = &mission;

    // Turn 1: the forecast announces +7 (a scan of +5 and the ambient +2).
    let turn_one = run.forecast(1);
    assert_eq!(turn_one.total(), 7);
    assert!(turn_one.turns[0].cloak_usable);
    assert_eq!(
        turn_one.turns[0].facts,
        vec![
            Fact::PatrolMoved {
                patrol: 0,
                from: node(m, 5),
                to: node(m, 3),
            },
            Fact::Scanned {
                patrol: 0,
                at: node(m, 3),
                target: node(m, 1),
                added: 5,
            },
            Fact::TraceUp { added: 5 },
            Fact::Ambient { added: 2 },
            Fact::TraceUp { added: 2 },
            Fact::TurnStarted { turn: 2 },
        ]
    );
    assert_eq!(
        run.act(Action::Use(Utility::Cloak)),
        Ok(vec![Fact::CloakOn])
    );
    assert_eq!(run.forecast(1).total(), 2, "the scan is cancelled");
    let facts = run.act(breach(m, 2, Program::BruteForce)).unwrap();
    assert_eq!(
        facts,
        vec![
            Fact::Breached {
                node: node(m, 2),
                program: Program::BruteForce,
                power: 2,
                from: 0,
                to: 2,
            },
            Fact::Opened { node: node(m, 2) },
            Fact::TraceUp { added: 5 },
        ]
    );
    run.act(Action::Move(node(m, 2))).unwrap();
    assert_eq!(run.state().cycles(), 0);
    let end = run.act(Action::End).unwrap();
    assert!(end.contains(&Fact::ScanCancelled {
        patrol: 0,
        at: node(m, 3),
        target: node(m, 2),
    }));
    assert_eq!((run.state().turn(), run.state().trace()), (2, 7));

    // Turn 2.
    assert_eq!(run.forecast(1).total(), 7);
    run.act(Action::Move(node(m, 3))).unwrap();
    run.act(Action::Use(Utility::Cloak)).unwrap();
    assert_eq!(
        run.state().supply(m, Program::Cloak),
        Some(Supply::Limited(0))
    );
    run.act(breach(m, 4, Program::BruteForce)).unwrap();
    assert_eq!(run.state().progress(node(m, 4)), 2);
    run.act(Action::End).unwrap();
    assert_eq!((run.state().turn(), run.state().trace()), (3, 14));

    // Turn 3: no Cloak charge is left, so the forecast no longer mentions it.
    let turn_three = run.forecast(1);
    assert_eq!(turn_three.total(), 7);
    assert!(!turn_three.turns[0].cloak_usable);
    run.act(breach(m, 4, Program::Exploit)).unwrap();
    assert_eq!(run.state().trace(), 16);
    assert!(run.state().is_open(m, node(m, 4)));
    run.act(Action::Move(node(m, 4))).unwrap();
    run.act(Action::End).unwrap();
    assert_eq!((run.state().turn(), run.state().trace()), (4, 23));

    // Turn 4: take the ledger and jack out.
    assert_eq!(run.forecast(1).total(), 7);
    assert_eq!(
        run.act(Action::Loot),
        Ok(vec![Fact::Looted { node: node(m, 4) }])
    );
    assert_eq!(
        run.act(Action::Jackout),
        Ok(vec![Fact::Ended {
            status: Status::Won,
        }])
    );
    assert_eq!(run.status(), Status::Won);
    assert_eq!(run.state().trace(), 23);
}

#[test]
fn the_golden_run_is_the_forecast_turn_by_turn() {
    // Whatever is played, what the forecast announced is what ending the turn did.
    let mission = payroll();
    let mut run = Run::new(mission.clone(), Undo::NONE);
    for _ in 0..4 {
        let announced = run.forecast(1);
        let before = run.state().trace();
        let facts = run.act(Action::End).unwrap();
        assert_eq!(announced.turns[0].facts, facts);
        assert_eq!(&announced.turns[0].after, run.state());
        assert_eq!(
            u16::from(run.state().trace()),
            u16::from(before) + u16::from(announced.total())
        );
    }
}

fn builder_with_one_objective() -> (MissionBuilder, NodeId) {
    let mut builder = MissionBuilder::new();
    let first = builder.node(None, Some(LootKind::Objective));
    (builder, first)
}

#[test]
fn a_mission_is_checked_when_it_is_built() {
    let ice = |strength| Ice {
        family: Family::Network,
        strength,
    };
    assert_eq!(MissionBuilder::new().build(), Err(MissionError::NoNode));

    let mut builder = MissionBuilder::new();
    builder.node(None, None);
    assert_eq!(builder.build(), Err(MissionError::NoObjective));

    let (mut builder, first) = builder_with_one_objective();
    builder.link(first, first);
    assert_eq!(builder.build(), Err(MissionError::SelfLink(0)));

    let (mut builder, _) = builder_with_one_objective();
    builder.node(None, None);
    assert_eq!(builder.build(), Err(MissionError::Unreachable(1)));

    let (mut builder, _) = builder_with_one_objective();
    builder.node(Some(ice(0)), None);
    assert_eq!(builder.build(), Err(MissionError::BadStrength(1)));
    let (mut builder, _) = builder_with_one_objective();
    builder.node(Some(ice(MAX_STRENGTH + 1)), None);
    assert_eq!(builder.build(), Err(MissionError::BadStrength(1)));

    let (mut builder, _) = builder_with_one_objective();
    for _ in 0..MAX_NODES {
        builder.node(None, None);
    }
    assert_eq!(builder.build(), Err(MissionError::TooManyNodes));

    let (mut builder, first) = builder_with_one_objective();
    builder.patrol(&[], None);
    assert_eq!(builder.build(), Err(MissionError::EmptyRoute));
    builder = builder_with_one_objective().0;
    builder.patrol(&[first], Some(first));
    assert_eq!(builder.build(), Err(MissionError::AnchorWithoutIce(0)));

    // Routes of 3 and 5 turns repeat every 15 turns; adding 2 makes 30.
    let (mut builder, first) = builder_with_one_objective();
    builder
        .patrol(&[first; 3], None)
        .patrol(&[first; 5], None)
        .patrol(&[first; 2], None);
    assert_eq!(builder.build(), Err(MissionError::PeriodTooLong(30)));

    let (mut builder, first) = builder_with_one_objective();
    for _ in 0..=MAX_PATROLS {
        builder.patrol(&[first], None);
    }
    assert_eq!(builder.build(), Err(MissionError::TooManyPatrols));
}

#[test]
fn the_pace_of_a_mission_is_checked() {
    let check = |setup: fn(&mut MissionBuilder), error| {
        let (mut builder, _) = builder_with_one_objective();
        setup(&mut builder);
        assert_eq!(builder.build(), Err(error));
    };
    check(
        |b| {
            b.program_with(Program::Spoof, Supply::Unlimited);
        },
        MissionError::UnlimitedUtility(Program::Spoof),
    );
    check(
        |b| {
            b.program_with(Program::Overclock, Supply::Unlimited);
        },
        MissionError::UnlimitedUtility(Program::Overclock),
    );
    check(
        |b| {
            b.program_with(Program::Exploit, Supply::Limited(4));
        },
        MissionError::BadSupply(Program::Exploit),
    );
    check(
        |b| {
            b.program_with(Program::Exploit, Supply::Limited(0));
        },
        MissionError::BadSupply(Program::Exploit),
    );
    check(
        |b| {
            b.cycles_per_turn(0);
        },
        MissionError::BadCycles,
    );
    check(
        |b| {
            b.cycles_per_turn(MAX_CYCLES + 1);
        },
        MissionError::BadCycles,
    );
    check(
        |b| {
            b.gain_quarters(0);
        },
        MissionError::BadGain,
    );
    check(
        |b| {
            b.ambient(0);
        },
        MissionError::NoAmbient,
    );
    check(
        |b| {
            b.trace_cap(0);
        },
        MissionError::BadTraceCap,
    );
    // An unlimited Cloak is fine: only the two that would break Trace or cycles are refused.
    let (mut builder, _) = builder_with_one_objective();
    builder.program_with(Program::Cloak, Supply::Unlimited);
    assert!(builder.build().is_ok());
}

#[test]
fn nodes_are_numbered_from_one() {
    assert_eq!(NodeId::from_number(0), None);
    assert_eq!(NodeId::from_number(1).map(NodeId::index), Some(0));
    assert_eq!(NodeId::from_number(8).map(NodeId::number), Some(8));
    assert_eq!(NodeId::from_number(9), None);
    assert_eq!(NodeId::new(8), None);
}

#[test]
fn the_gain_rounds_up_so_a_noise_of_one_never_vanishes() {
    let build = |quarters| {
        let (mut builder, _) = builder_with_one_objective();
        builder.gain_quarters(quarters);
        builder.build().unwrap()
    };
    let story = build(2);
    assert_eq!(
        [story.gain(0), story.gain(1), story.gain(2), story.gain(5)],
        [0, 1, 1, 3]
    );
    let plain = build(4);
    assert_eq!(plain.gain(5), 5);
    let hard = build(6);
    assert_eq!(hard.gain(5), 8);
}

#[test]
fn trace_bands_follow_the_thresholds() {
    let bands = [0, 29, 30, 49, 50, 69, 70, 99].map(TraceBand::of);
    assert_eq!(
        bands,
        [
            TraceBand::Calm,
            TraceBand::Calm,
            TraceBand::Vigilance,
            TraceBand::Vigilance,
            TraceBand::Alert,
            TraceBand::Alert,
            TraceBand::Danger,
            TraceBand::Danger,
        ]
    );
}

/// A line of three nodes: a free start, a guarded middle, an objective at the end.
fn corridor(strength: u8, family: Family) -> (MissionBuilder, [NodeId; 3]) {
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, None);
    let b = builder.node(Some(Ice { family, strength }), None);
    let c = builder.node(None, Some(LootKind::Objective));
    builder.link(a, b).link(b, c);
    (builder, [a, b, c])
}

fn start_run(builder: &MissionBuilder) -> Run {
    Run::new(builder.build().unwrap(), Undo::Unlimited)
}

#[test]
fn ice_blocks_the_way_until_it_is_open() {
    let (mut builder, [_, b, c]) = corridor(2, Family::Network);
    builder.program(Program::BruteForce);
    let mut run = start_run(&builder);
    assert_eq!(run.act(Action::Move(b)), Err(ActionError::IceBlocks));
    assert_eq!(run.act(Action::Move(c)), Err(ActionError::NotLinked));
    assert_eq!(
        run.act(Action::Breach {
            node: c,
            program: Program::BruteForce,
        }),
        Err(ActionError::NotLinked)
    );
    run.act(Action::Breach {
        node: b,
        program: Program::BruteForce,
    })
    .unwrap();
    assert_eq!(
        run.act(Action::Breach {
            node: b,
            program: Program::BruteForce,
        }),
        Err(ActionError::NoIce),
        "an open node has nothing left to breach"
    );
    run.act(Action::Move(b)).unwrap();
    run.act(Action::Move(c)).unwrap();
}

#[test]
fn a_program_is_strong_against_its_family_and_weak_elsewhere() {
    let (mut builder, [_, b, _]) = corridor(6, Family::Encryption);
    builder.programs(&[Program::BruteForce, Program::Quantum]);
    let mut run = start_run(&builder);
    let progress = |run: &Run| run.state().progress(b);
    run.act(Action::Breach {
        node: b,
        program: Program::BruteForce,
    })
    .unwrap();
    assert_eq!(progress(&run), 1, "off-family power is 1");
    run.act(Action::Breach {
        node: b,
        program: Program::Quantum,
    })
    .unwrap();
    assert_eq!(progress(&run), 5, "Quantum is worth 4 against encryption");
    assert_eq!(run.state().cycles(), 0, "1 + 2 cycles of 3");
}

#[test]
fn progress_never_goes_past_the_strength() {
    let (mut builder, [_, b, _]) = corridor(1, Family::Network);
    builder.program(Program::Exploit);
    let mut run = start_run(&builder);
    let facts = run
        .act(Action::Breach {
            node: b,
            program: Program::Exploit,
        })
        .unwrap();
    assert!(matches!(
        facts[0],
        Fact::Breached {
            from: 0,
            to: 1,
            power: 3,
            ..
        }
    ));
}

#[test]
fn programs_must_be_in_the_deck_and_have_charges() {
    let (mut builder, [_, b, _]) = corridor(6, Family::Network);
    builder.program_with(Program::Exploit, Supply::Limited(1));
    let mut run = start_run(&builder);
    let exploit = Action::Breach {
        node: b,
        program: Program::Exploit,
    };
    assert_eq!(
        run.act(Action::Breach {
            node: b,
            program: Program::BruteForce,
        }),
        Err(ActionError::NotInDeck(Program::BruteForce))
    );
    assert_eq!(
        run.act(Action::Breach {
            node: b,
            program: Program::Cloak,
        }),
        Err(ActionError::NotABreach(Program::Cloak))
    );
    run.act(exploit).unwrap();
    run.act(Action::End).unwrap();
    assert_eq!(
        run.act(exploit),
        Err(ActionError::NoCharges(Program::Exploit))
    );
    assert_eq!(
        run.state().supply(run.mission(), Program::Exploit),
        Some(Supply::Limited(0))
    );
}

#[test]
fn a_refused_action_changes_nothing() {
    let (mut builder, [_, b, _]) = corridor(6, Family::Network);
    builder.program(Program::Exploit);
    let mut run = start_run(&builder);
    let before = *run.state();
    let exploit = Action::Breach {
        node: b,
        program: Program::Exploit,
    };
    run.act(exploit).unwrap();
    // 3 cycles: Exploit costs 2, so a second one does not fit.
    let during = *run.state();
    assert_eq!(run.act(exploit), Err(ActionError::NotEnoughCycles));
    assert_eq!(*run.state(), during);
    assert_ne!(during, before);
}

#[test]
fn social_engineering_needs_intel() {
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, Some(LootKind::Intel));
    let b = builder.node(
        Some(Ice {
            family: Family::Human,
            strength: 3,
        }),
        Some(LootKind::Objective),
    );
    builder.link(a, b).program(Program::Social);
    let mut run = start_run(&builder);
    let social = Action::Breach {
        node: b,
        program: Program::Social,
    };
    assert_eq!(run.act(social), Err(ActionError::NeedsIntel));
    assert!(!run.legal_actions().contains(&social));
    run.act(Action::Loot).unwrap();
    assert_eq!(run.act(social).map(|_| ()), Ok(()));

    builder.intel_known(true);
    let run = start_run(&builder);
    assert!(run.legal_actions().contains(&social));
}

#[test]
fn spoof_never_goes_below_the_trace_of_the_start_of_the_turn() {
    let (mut builder, [_, b, _]) = corridor(6, Family::Network);
    builder
        .program(Program::BruteForce)
        .program_with(Program::Spoof, Supply::Limited(3));
    let mut run = start_run(&builder);
    // At the start of turn 1 the Trace is 0: nothing to erase.
    assert_eq!(
        run.act(Action::Use(Utility::Spoof)),
        Err(ActionError::NothingToErase)
    );
    let breach = Action::Breach {
        node: b,
        program: Program::BruteForce,
    };
    run.act(breach).unwrap();
    assert_eq!(run.state().trace(), 5);
    assert_eq!(
        run.act(Action::Use(Utility::Spoof)),
        Ok(vec![Fact::Erased { removed: 5 }]),
        "5 earned this turn, so only 5 can go, though a charge is worth 10"
    );
    assert_eq!(run.state().trace(), 0);
    run.act(breach).unwrap();
    run.act(Action::End).unwrap();
    // The turn made 5 and the ambient 2: the next turn starts at 7 and 7 is its floor.
    assert_eq!(run.state().trace(), 7);
    assert_eq!(run.state().turn_floor(), 7);
    assert_eq!(
        run.act(Action::Use(Utility::Spoof)),
        Err(ActionError::NothingToErase),
        "the trace of the turns before is kept"
    );
    assert_eq!(
        run.state().supply(run.mission(), Program::Spoof),
        Some(Supply::Limited(2)),
        "a refused Spoof costs no charge"
    );
}

#[test]
fn cloak_cancels_the_scans_of_the_turn_and_lasts_one_turn() {
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, Some(LootKind::Objective));
    builder.patrol(&[a], None).program(Program::Cloak);
    let mut run = start_run(&builder);
    assert_eq!(
        run.act(Action::Use(Utility::Cloak)),
        Ok(vec![Fact::CloakOn])
    );
    assert_eq!(
        run.act(Action::Use(Utility::Cloak)),
        Err(ActionError::AlreadyActive(Utility::Cloak))
    );
    run.act(Action::End).unwrap();
    assert!(!run.state().cloak_active());
    assert_eq!(run.state().trace(), 2, "only the ambient Trace");
    let facts = run.act(Action::End).unwrap();
    assert!(
        facts
            .iter()
            .any(|fact| matches!(fact, Fact::Scanned { .. }))
    );
    assert_eq!(run.state().trace(), 2 + 5 + 2);
}

#[test]
fn ghost_silences_what_follows_but_not_scans_nor_the_ambient_trace() {
    let (mut builder, [_, b, _]) = corridor(6, Family::Network);
    builder
        .programs(&[Program::BruteForce, Program::Ghost])
        .patrol(&[b], None);
    let mut run = start_run(&builder);
    let loud = Action::Breach {
        node: b,
        program: Program::BruteForce,
    };
    run.act(loud).unwrap();
    assert_eq!(run.state().trace(), 5);
    run.act(Action::Use(Utility::Ghost)).unwrap();
    run.act(loud).unwrap();
    assert_eq!(run.state().trace(), 5, "the second breach made no noise");
    run.act(Action::End).unwrap();
    // The patrol on the middle node is next to the player: scan +5, then ambient +2.
    assert_eq!(run.state().trace(), 5 + 5 + 2);
}

#[test]
fn the_danger_band_makes_every_breach_louder() {
    let (mut builder, [_, b, _]) = corridor(6, Family::Network);
    builder.program(Program::BruteForce).trace_cap(200);
    let mission = builder.build().unwrap();
    let loud = Action::Breach {
        node: b,
        program: Program::BruteForce,
    };
    let gain_at = |trace: u8| {
        let mut state = mission.initial_state();
        state.trace = trace;
        let next = apply(&mission, &state, loud, &mut ()).unwrap();
        next.trace() - trace
    };
    assert_eq!(gain_at(0), 5);
    assert_eq!(gain_at(69), 5, "just under the danger band");
    assert_eq!(gain_at(70), 7, "two more per breach in the danger band");
}

#[test]
fn overclock_gives_cycles_for_a_little_noise_and_costs_none() {
    let (mut builder, _) = builder_with_one_objective();
    builder.program(Program::Overclock);
    let mut run = start_run(&builder);
    assert_eq!(
        run.act(Action::Use(Utility::Overclock)),
        Ok(vec![
            Fact::ExtraCycles { count: 2 },
            Fact::TraceUp { added: 3 }
        ])
    );
    assert_eq!(run.state().cycles(), 5);
    assert_eq!(
        run.act(Action::Use(Utility::Overclock)),
        Err(ActionError::NoCharges(Program::Overclock))
    );
}

#[test]
fn looting_costs_a_cycle_and_happens_once() {
    let (builder, _) = builder_with_one_objective();
    let mut run = start_run(&builder);
    assert_eq!(
        run.act(Action::Jackout),
        Err(ActionError::ObjectivesMissing)
    );
    run.act(Action::Loot).unwrap();
    assert_eq!(run.state().cycles(), 2);
    assert_eq!(run.act(Action::Loot), Err(ActionError::NothingToLoot));
    assert!(run.state().objectives_done(run.mission()));
    run.act(Action::Jackout).unwrap();
    assert_eq!(run.act(Action::End), Err(ActionError::RunOver));
    assert!(run.legal_actions().is_empty());
}

#[test]
fn nothing_to_loot_where_there_is_nothing() {
    let (mut builder, [a, ..]) = corridor(1, Family::Network);
    builder.program(Program::BruteForce);
    let mut run = start_run(&builder);
    assert_eq!(run.state().position(), a);
    assert_eq!(run.act(Action::Loot), Err(ActionError::NothingToLoot));
}

#[test]
fn an_alert_trace_lets_patrols_see_two_nodes_away() {
    // A line of three: the patrol sits on the last node and the player on the first.
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, Some(LootKind::Objective));
    let b = builder.node(None, None);
    let c = builder.node(None, None);
    builder
        .link(a, b)
        .link(b, c)
        .patrol(&[c], None)
        .trace_cap(200);
    let mission = builder.build().unwrap();
    let mut state = mission.initial_state();
    let facts_of_end = |state: &RunState| {
        let mut facts = Vec::new();
        apply(&mission, state, Action::End, &mut facts).unwrap();
        facts
    };
    assert!(
        !facts_of_end(&state)
            .iter()
            .any(|fact| matches!(fact, Fact::Scanned { .. })),
        "two nodes away is out of sight while the Trace is low"
    );
    state.trace = 50;
    assert!(
        facts_of_end(&state)
            .iter()
            .any(|fact| matches!(fact, Fact::Scanned { .. })),
        "in the alert band the patrol sees two nodes away"
    );
}

#[test]
fn a_virus_puts_down_the_patrols_on_the_node_for_two_turns() {
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, Some(LootKind::Objective));
    let b = builder.node(
        Some(Ice {
            family: Family::AiGuardian,
            strength: 6,
        }),
        None,
    );
    builder.link(a, b);
    // A guardian process of the middle node, and a patrol that is elsewhere.
    builder.patrol(&[b], Some(b)).patrol(&[a, a], None);
    builder.program(Program::Virus).trace_cap(200);
    let mut run = start_run(&builder);
    let facts = run
        .act(Action::Breach {
            node: b,
            program: Program::Virus,
        })
        .unwrap();
    assert!(facts.contains(&Fact::PatrolDown { patrol: 0 }));
    assert!(!facts.contains(&Fact::PatrolDown { patrol: 1 }));
    assert_eq!(run.state().down_turns(0), 2);
    // The guardian does not scan for two ends of turn, then does again.
    let scans = |facts: &[Fact]| {
        facts
            .iter()
            .filter(|fact| matches!(fact, Fact::Scanned { patrol: 0, .. }))
            .count()
    };
    let first = run.act(Action::End).unwrap();
    let second = run.act(Action::End).unwrap();
    let third = run.act(Action::End).unwrap();
    // Patrol 1 scans too (it stands on the player): count only patrol 0.
    assert_eq!((scans(&first), scans(&second), scans(&third)), (0, 0, 1));
}

#[test]
fn a_guardian_process_stops_when_its_node_is_open() {
    let (mut builder, [_, b, _]) = corridor(1, Family::Network);
    builder.patrol(&[b], Some(b)).program(Program::BruteForce);
    let mut run = start_run(&builder);
    let facts = run.act(Action::End).unwrap();
    assert!(
        facts
            .iter()
            .any(|fact| matches!(fact, Fact::Scanned { .. }))
    );
    run.act(Action::Breach {
        node: b,
        program: Program::BruteForce,
    })
    .unwrap();
    assert!(!run.state().patrol_scans(run.mission(), 0));
    let facts = run.act(Action::End).unwrap();
    assert!(
        !facts
            .iter()
            .any(|fact| matches!(fact, Fact::Scanned { .. }))
    );
}

#[test]
fn a_static_patrol_does_not_announce_a_move() {
    let (mut builder, [_, b, _]) = corridor(1, Family::Network);
    builder.patrol(&[b], None);
    let mut run = start_run(&builder);
    let facts = run.act(Action::End).unwrap();
    assert!(
        !facts
            .iter()
            .any(|fact| matches!(fact, Fact::PatrolMoved { .. }))
    );
}

#[test]
fn the_run_is_burned_when_the_trace_reaches_the_cap() {
    let (mut builder, _) = builder_with_one_objective();
    builder.trace_cap(3).ambient(2);
    let mut run = start_run(&builder);
    run.act(Action::End).unwrap();
    assert_eq!(run.state().trace(), 2);
    let facts = run.act(Action::End).unwrap();
    assert_eq!(run.status(), Status::Burned);
    assert_eq!(
        facts.last(),
        Some(&Fact::Ended {
            status: Status::Burned,
        }),
        "the last thing said is that the run is burned"
    );
    assert!(
        !facts
            .iter()
            .any(|fact| matches!(fact, Fact::TurnStarted { .. }))
    );
    assert_eq!(run.act(Action::End), Err(ActionError::RunOver));
}

#[test]
fn later_patrols_do_not_announce_trace_after_the_run_is_burned() {
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, Some(LootKind::Objective));
    builder.patrol(&[a], None).patrol(&[a], None).trace_cap(5);
    let mut run = start_run(&builder);
    let facts = run.act(Action::End).unwrap();
    let scans = facts
        .iter()
        .filter(|fact| matches!(fact, Fact::Scanned { .. }))
        .count();
    assert_eq!(scans, 1);
    assert_eq!(run.status(), Status::Burned);
}

#[test]
fn the_forecast_stops_when_the_run_would_be_burned_and_never_changes_the_state() {
    let mut builder = MissionBuilder::new();
    let a = builder.node(None, Some(LootKind::Objective));
    builder.trace_cap(5).ambient(2);
    let run = start_run(&builder);
    let before = *run.state();
    let forecast = run.forecast(3);
    // Turn 1 ends at 2, turn 2 at 4, turn 3 at 6 = burned: three turns announced.
    assert_eq!(forecast.turns.len(), 3);
    assert_eq!(forecast.turns[2].after.status(), Status::Burned);
    assert_eq!(forecast.turns[1].turn, 2);
    assert_eq!(*run.state(), before);
    let _ = a;
    // Asking further announces nothing more once burned.
    assert_eq!(run.forecast(9).turns.len(), 3);
    assert_eq!(run.forecast(0).total(), 0);
}

#[test]
fn undo_takes_back_the_whole_turn() {
    let (mut builder, [_, b, _]) = corridor(2, Family::Network);
    builder.program(Program::BruteForce);
    let mut run = Run::new(builder.build().unwrap(), Undo::Left(2));
    let start = *run.state();
    assert_eq!(run.undo(), Err(UndoError::NothingToTakeBack));
    run.act(Action::Breach {
        node: b,
        program: Program::BruteForce,
    })
    .unwrap();
    run.act(Action::Move(b)).unwrap();
    assert_eq!(run.undo(), Ok(()));
    assert_eq!(*run.state(), start, "both actions of the turn are gone");
    assert_eq!(run.undo_left(), Undo::Left(1));
    assert_eq!(run.undo(), Err(UndoError::NothingToTakeBack));

    run.act(Action::End).unwrap();
    let second = *run.state();
    assert_eq!(run.turn_start(), &second);
    run.act(Action::Loot).unwrap_err();
    // Nothing done in turn 2: undo goes back to the start of turn 1.
    assert_eq!(run.undo(), Ok(()));
    assert_eq!(*run.state(), start);
    assert_eq!(run.undo_left(), Undo::NONE);
    run.act(Action::End).unwrap();
    assert_eq!(run.undo(), Err(UndoError::NoneLeft));
}

#[test]
fn undo_is_refused_when_the_run_is_over() {
    let (builder, _) = builder_with_one_objective();
    let mut run = start_run(&builder);
    run.act(Action::Loot).unwrap();
    run.act(Action::Jackout).unwrap();
    assert_eq!(run.undo(), Err(UndoError::RunOver));
}

#[test]
fn unlimited_undo_is_never_spent() {
    let (builder, _) = builder_with_one_objective();
    let mut run = start_run(&builder);
    for _ in 0..5 {
        run.act(Action::Loot).unwrap();
        run.undo().unwrap();
    }
    assert_eq!(run.undo_left(), Undo::Unlimited);
}

// -- Properties on random missions ------------------------------------------------------

/// A small dice over the engine generator, so a seed makes the same mission everywhere.
pub(crate) struct Dice(Pcg32);

impl Dice {
    pub(crate) fn new(seed: u64, stream: u64) -> Self {
        Self(Pcg32::new(seed, stream))
    }

    /// A number in `0..bound` (`0` when `bound` is `0`).
    pub(crate) fn below(&mut self, bound: usize) -> usize {
        let bound = u32::try_from(bound).unwrap_or(u32::MAX);
        self.0.next_below(bound).unwrap_or(0) as usize
    }

    pub(crate) fn byte(&mut self, bound: u8) -> u8 {
        u8::try_from(self.below(usize::from(bound))).unwrap_or(0)
    }
}

/// A random valid mission made from a seed: same seed, same mission.
pub(crate) fn random_mission(seed: u64) -> Mission {
    let mut dice = Dice::new(seed, 7);
    let count = 1 + dice.below(7);
    let mut builder = MissionBuilder::new();
    let mut nodes = Vec::new();
    for index in 0..count {
        let ice = (index > 0 && dice.below(3) > 0).then(|| Ice {
            family: Family::ALL[dice.below(4)],
            strength: 1 + dice.byte(MAX_STRENGTH),
        });
        let loot = match dice.below(5) {
            0 => Some(LootKind::Intel),
            1 => Some(LootKind::Extra),
            _ => None,
        };
        nodes.push(builder.node(ice, loot));
    }
    // The last node holds the objective; a chain keeps the graph connected.
    nodes.push(builder.node(None, Some(LootKind::Objective)));
    for pair in nodes.windows(2) {
        builder.link(pair[0], pair[1]);
    }
    for _ in 0..dice.below(4) {
        let (a, b) = (dice.below(nodes.len()), dice.below(nodes.len()));
        if a != b {
            builder.link(nodes[a], nodes[b]);
        }
    }
    for _ in 0..dice.below(4) {
        let length = 1 + dice.below(4);
        let route: Vec<NodeId> = (0..length)
            .map(|_| nodes[dice.below(nodes.len())])
            .collect();
        builder.patrol(&route, None);
    }
    for program in Program::ALL {
        if dice.below(2) == 0 {
            let supply = match program.spec().supply {
                Supply::Limited(_) => Supply::Limited(1 + dice.byte(3)),
                unlimited @ Supply::Unlimited => unlimited,
            };
            builder.program_with(program, supply);
        }
    }
    builder
        .cycles_per_turn(1 + dice.byte(6))
        .gain_quarters(1 + dice.byte(8))
        .ambient(1 + dice.byte(4))
        .trace_cap(20 + dice.byte(81))
        .intel_known(dice.below(2) == 0);
    builder.build().expect("random missions are valid")
}

/// Every action that exists, to check `legal_actions` against the rules.
pub(crate) fn every_action(mission: &Mission) -> Vec<Action> {
    let mut all = vec![Action::Loot, Action::End, Action::Jackout];
    all.extend(Utility::ALL.map(Action::Use));
    for node in mission.nodes() {
        all.push(Action::Move(node));
        all.extend(Program::ALL.map(|program| Action::Breach { node, program }));
    }
    all
}

/// Plays up to `steps` random legal actions, calling `check` on every state reached.
pub(crate) fn random_play(
    mission: &Mission,
    seed: u64,
    steps: usize,
    mut check: impl FnMut(&RunState, Option<Action>),
) -> RunState {
    let mut dice = Dice::new(seed, 11);
    let mut state = mission.initial_state();
    check(&state, None);
    for _ in 0..steps {
        let legal = legal_actions(mission, &state);
        let Some(&action) = legal.get(dice.below(legal.len())) else {
            break;
        };
        state = apply(mission, &state, action, &mut ()).unwrap();
        check(&state, Some(action));
    }
    state
}

fn hash_of(state: &RunState) -> u64 {
    let mut hasher = DefaultHasher::new();
    state.hash(&mut hasher);
    hasher.finish()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// `legal_actions` is exactly what `apply` accepts.
    #[test]
    fn legal_actions_are_what_apply_accepts(seed in any::<u64>()) {
        let mission = random_mission(seed);
        let universe = every_action(&mission);
        random_play(&mission, seed, 60, |state, _| {
            let legal = legal_actions(&mission, state);
            let accepted: Vec<Action> = universe
                .iter()
                .copied()
                .filter(|action| apply(&mission, state, *action, &mut ()).is_ok())
                .collect();
            let mut sorted = legal.clone();
            sorted.sort_by_key(|action| universe.iter().position(|other| other == action));
            let mut expected = accepted;
            expected.sort_by_key(|action| universe.iter().position(|other| other == action));
            assert_eq!(sorted, expected);
            assert_eq!(legal.len(), sorted.len());
        });
    }

    /// RUN-2: the forecast is the resolution, fact for fact and to the digit.
    #[test]
    fn the_forecast_is_the_resolution(seed in any::<u64>()) {
        let mission = random_mission(seed);
        random_play(&mission, seed, 60, |state, _| {
            let forecast = forecast(&mission, state, 1);
            let mut facts = Vec::new();
            let after = apply(&mission, state, Action::End, &mut facts);
            match after {
                Ok(after) => {
                    let announced = &forecast.turns[0];
                    assert_eq!(announced.facts, facts);
                    assert_eq!(announced.after, after);
                    let added: u32 = facts.iter().map(|fact| match fact {
                        Fact::TraceUp { added } => u32::from(*added),
                        _ => 0,
                    }).sum();
                    assert_eq!(u32::from(announced.total), added);
                    assert_eq!(
                        u32::from(after.trace()),
                        u32::from(state.trace()) + added
                    );
                }
                Err(_) => assert!(forecast.turns.is_empty()),
            }
        });
    }

    /// RUN-10 (I17): the Trace of the start of a turn only goes up, by at least the ambient
    /// Trace each turn, so a run lasts at most `cap / ambient + 1` turns whatever is played.
    #[test]
    fn every_run_ends_in_bounded_turns(seed in any::<u64>()) {
        let mission = random_mission(seed);
        let ambient = u32::from(mission.gain(mission.ambient()));
        let bound = u32::from(mission.trace_cap()).div_ceil(ambient) + 1;
        let mut last_floor = 0;
        random_play(&mission, seed, 400, |state, _| {
            assert!(state.turn() <= bound, "turn {} > {bound}", state.turn());
            assert!(state.turn_floor() >= last_floor);
            assert!(state.trace() >= state.turn_floor() || !state.is_running());
            last_floor = state.turn_floor();
        });
        // And waiting alone burns the run within that bound.
        let mut run = Run::new(mission.clone(), Undo::NONE);
        while run.status() == Status::Running {
            run.act(Action::End).unwrap();
        }
        assert_eq!(run.status(), Status::Burned);
        assert!(run.state().turn() <= bound);
    }

    /// RUN-11 (I18): undo, then the same actions, gives the same state, hash included.
    #[test]
    fn undo_then_the_same_actions_gives_the_same_state(seed in any::<u64>()) {
        let mission = random_mission(seed);
        let mut dice = Dice::new(seed, 13);
        let mut run = Run::new(mission, Undo::Unlimited);
        // Some turns first, so the undo has a past, ending on a turn start.
        for _ in 0..40 {
            let legal = run.legal_actions();
            let Some(&action) = legal.get(dice.below(legal.len())) else {
                return Ok(());
            };
            if action == Action::Jackout {
                break;
            }
            run.act(action).unwrap();
        }
        if !run.state().is_running() || run.act(Action::End).is_err() || !run.state().is_running()
        {
            return Ok(());
        }
        // Play out a turn with a recorded plan, remember the state, undo, replay.
        let start = *run.turn_start();
        let mut plan = Vec::new();
        let mut probe = run.clone();
        for _ in 0..6 {
            let legal = probe.legal_actions();
            let Some(&action) = legal.get(dice.below(legal.len())) else {
                break;
            };
            if matches!(action, Action::End | Action::Jackout) {
                break;
            }
            probe.act(action).unwrap();
            plan.push(action);
        }
        let mut played = run.clone();
        for &action in &plan {
            played.act(action).unwrap();
        }
        // A burned run is over: it cannot be taken back (the hub takes over).
        if plan.is_empty() || !played.state().is_running() {
            return Ok(());
        }
        let reached = *played.state();
        played.undo().unwrap();
        assert_eq!(*played.state(), start);
        for &action in &plan {
            played.act(action).unwrap();
        }
        assert_eq!(*played.state(), reached);
        assert_eq!(hash_of(played.state()), hash_of(&reached));
    }
}

// -- Taking a run up again ---------------------------------------------------------------

#[test]
fn a_state_goes_through_a_save_and_comes_back_equal() {
    for seed in 0..60 {
        let mission = random_mission(seed);
        let end = random_play(&mission, seed, 12, |_, _| {});
        let text = toml::to_string(&end).unwrap();
        assert_eq!(toml::from_str::<RunState>(&text).unwrap(), end, "{text}");
    }
    let undo = [Undo::Unlimited, Undo::Left(3), Undo::NONE];
    for undo in undo {
        let text = toml::to_string(&toml::Table::from_iter([(
            "undo".to_owned(),
            toml::Value::try_from(undo).unwrap(),
        )]))
        .unwrap();
        let back: toml::Table = text.parse().unwrap();
        assert_eq!(back["undo"].clone().try_into::<Undo>().unwrap(), undo);
    }
}

#[test]
fn only_the_start_of_a_turn_of_this_mission_is_accepted() {
    for seed in 0..120 {
        let mission = random_mission(seed);
        // Every turn start a random play reaches is accepted.
        let mut state = mission.initial_state();
        assert_eq!(mission.turn_start_error(&state), None);
        let mut dice = Dice::new(seed, 17);
        for _ in 0..40 {
            let legal = legal_actions(&mission, &state);
            let Some(&action) = legal.get(dice.below(legal.len())) else {
                break;
            };
            state = apply(&mission, &state, action, &mut ()).unwrap();
            if action == Action::End && state.is_running() {
                assert_eq!(mission.turn_start_error(&state), None, "seed {seed}");
            }
        }
    }
    let mission = payroll();
    let start = mission.initial_state();
    let mut broken = start;
    broken.trace = 40;
    assert!(
        mission.turn_start_error(&broken).is_some(),
        "a Trace that is not the floor"
    );
    let mut broken = start;
    broken.cloak = true;
    assert!(mission.turn_start_error(&broken).is_some());
    let mut broken = start;
    broken.progress[0] = 5;
    assert!(
        mission.turn_start_error(&broken).is_some(),
        "progress where there is no ICE"
    );
    let mut broken = start;
    broken.loot_taken = 0b1;
    assert!(
        mission.turn_start_error(&broken).is_some(),
        "loot where there is none"
    );
    let mut broken = start;
    broken.charges[Program::Cloak.index()] = 9;
    assert!(
        mission.turn_start_error(&broken).is_some(),
        "more charges than the deck gave"
    );
    let mut broken = start;
    broken.status = Status::Won;
    assert!(mission.turn_start_error(&broken).is_some());
}

#[test]
fn a_run_taken_up_again_can_take_back_only_the_turn_under_way() {
    let mission = payroll();
    let mut run = Run::new(mission.clone(), Undo::Left(2));
    run.act(Action::End).unwrap();
    let turn_start = *run.turn_start();
    let mut again = Run::resume(mission, turn_start, run.undo_left());
    assert_eq!(again.undo(), Err(UndoError::NothingToTakeBack));
    again.act(Action::Use(Utility::Cloak)).unwrap();
    assert_eq!(again.undo(), Ok(()));
    assert_eq!(*again.state(), turn_start);
}

#[test]
fn an_automatic_end_takes_every_loot_and_wins_at_the_trace_it_is_given() {
    let mission = payroll();
    let settled = mission.settled_at(&mission.initial_state(), 40);
    assert_eq!(settled.status(), Status::Won);
    assert_eq!(settled.trace(), 40);
    assert!(settled.objectives_done(&mission));
    // Never at or past the cap, which would be a burned run.
    assert!(mission.settled_at(&mission.initial_state(), 255).trace() < mission.trace_cap());
}
