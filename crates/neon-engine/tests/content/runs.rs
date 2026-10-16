//! The tactical intrusions of the shipped content: every site has one, it builds at every
//! difficulty, and a simple player can win each of them.

use neon_engine::campaign::Difficulty;
use neon_engine::content::runs::site_tier;
use neon_engine::content::{Content, File, Sources};
use neon_engine::run::solve::{
    Outcome, PROOF_EXPANSIONS, Proof, Resolution, auto_resolve, optimum, prove_solvable,
};
use neon_engine::run::{
    Action, ActionError, Fact, Mission, NodeId, Pace, Program, Run, Status, Undo, Utility,
};

use crate::common::shipped;

fn site(id: &str) -> neon_engine::content::ids::SiteId {
    id.parse().unwrap()
}

fn node(number: u8) -> NodeId {
    NodeId::from_number(number).unwrap()
}

#[test]
fn every_site_has_an_intrusion_that_builds_at_every_difficulty() {
    let c = shipped();
    for s in &c.sites {
        for difficulty in Difficulty::ALL {
            let mission = c
                .mission(&s.id, difficulty.pace())
                .unwrap_or_else(|| panic!("{} has no mission at {difficulty:?}", s.id));
            assert!(mission.node_count() >= 3, "{}", s.id);
            let kit = c.kit(site_tier(s)).unwrap();
            assert_eq!(
                mission.cycles_per_turn(),
                kit.cycles + difficulty.pace().bonus_cycles
            );
        }
    }
    assert!(c.mission(&site("nowhere"), Pace::NORMAL).is_none());
}

#[test]
fn decks_only_grow_with_the_tier() {
    let c = shipped();
    for tier in 2..=6 {
        let before = &c.kit(tier - 1).unwrap().programs;
        let now = &c.kit(tier).unwrap().programs;
        assert!(before.iter().all(|p| now.contains(p)), "tier {tier}");
    }
    // The first deck has a breach program that never runs out.
    let first = &c.kit(1).unwrap().programs;
    assert!(first.contains(&Program::BruteForce));
}

#[test]
fn the_files_of_a_site_are_on_its_nodes_and_nowhere_else() {
    let c = shipped();
    for run in &c.runs {
        let site = c.site(&run.site).unwrap();
        let mission = c.mission(&run.site, Pace::NORMAL).unwrap();
        let mut found = 0;
        for id in mission.nodes() {
            let files = run.files_at(id);
            found += files.len();
            assert!(
                files.is_empty() || mission.loot(id).is_some(),
                "{}",
                run.site
            );
        }
        assert_eq!(found, site.file.len(), "{}", run.site);
    }
}

#[test]
fn corp_server_01_is_the_payroll_run_of_the_spec() {
    // `docs/spec/run-screen.md` § 5, played on the real data: four turns, Trace 7, 14, 23.
    // The deck of the example holds an Exploit, which the game no longer sells: the graph is the
    // shipped one, the deck is the one of the spec.
    let c = shipped();
    let def = c.run(&site("corp-server-01")).unwrap();
    let kit = c.kit(2).unwrap();
    let mut programs = kit.programs.clone();
    programs.push(Program::Exploit);
    let mission = def.mission(kit, &programs, Pace::NORMAL).unwrap();
    let mut run = Run::new(mission, Undo::NONE);
    let breach = |number, program| Action::Breach {
        node: node(number),
        program,
    };
    assert_eq!(run.forecast(1).total(), 7);
    run.act(Action::Use(Utility::Cloak)).unwrap();
    run.act(breach(2, Program::BruteForce)).unwrap();
    run.act(Action::Move(node(2))).unwrap();
    run.act(Action::End).unwrap();
    assert_eq!(run.state().trace(), 7);
    run.act(Action::Move(node(3))).unwrap();
    run.act(Action::Use(Utility::Cloak)).unwrap();
    run.act(breach(4, Program::BruteForce)).unwrap();
    run.act(Action::End).unwrap();
    assert_eq!(run.state().trace(), 14);
    run.act(breach(4, Program::Exploit)).unwrap();
    run.act(Action::Move(node(4))).unwrap();
    run.act(Action::End).unwrap();
    assert_eq!(run.state().trace(), 23);
    let facts = run.act(Action::Loot).unwrap();
    assert_eq!(facts, vec![Fact::Looted { node: node(4) }]);
    run.act(Action::Jackout).unwrap();
    assert_eq!(run.status(), Status::Won);
}

/// A simple player: walks the shortest way to the nearest objective, opens what is in the way
/// with the program that makes the most progress, covers every end of turn with Cloak when it
/// has one, loots and jacks out. Returns the Trace it ended with, or why it failed.
fn simple_player(mission: &Mission) -> Result<u8, String> {
    let mut run = Run::new(mission.clone(), Undo::NONE);
    for _ in 0..400 {
        let state = *run.state();
        match state.status() {
            Status::Won => return Ok(state.trace()),
            Status::Burned => return Err(format!("burned on turn {}", state.turn())),
            Status::Running => {}
        }
        if state.objectives_done(mission) {
            run.act(Action::Jackout).map_err(|e| e.to_string())?;
            continue;
        }
        let here = state.position();
        let wants = |id: NodeId| {
            mission
                .loot(id)
                .is_some_and(neon_engine::run::LootKind::is_objective)
                && !state.is_looted(id)
        };
        let action = if wants(here) && state.cycles() > 0 {
            Action::Loot
        } else {
            let target = mission
                .nodes()
                .filter(|id| wants(*id))
                .min_by_key(|id| mission.distance(here, *id))
                .ok_or("no objective")?;
            let hop = mission
                .neighbours(here)
                .min_by_key(|id| mission.distance(*id, target))
                .ok_or("dead end")?;
            if state.is_open(mission, hop) {
                Action::Move(hop)
            } else {
                let best = run
                    .legal_actions()
                    .into_iter()
                    .filter_map(|a| match a {
                        Action::Breach { node, program } if node == hop => {
                            let family = mission.ice(hop)?.family;
                            Some((
                                program.power_against(family),
                                std::cmp::Reverse(program.spec().noise),
                                a,
                            ))
                        }
                        _ => None,
                    })
                    .max_by_key(|(power, quiet, _)| (*power, *quiet));
                match best {
                    Some((_, _, a)) => a,
                    None => Action::End,
                }
            }
        };
        let acting = if action == Action::End
            && run.legal_actions().contains(&Action::Use(Utility::Cloak))
        {
            Action::Use(Utility::Cloak)
        } else {
            action
        };
        match run.act(acting) {
            Ok(_) => {}
            Err(ActionError::NotEnoughCycles) => {
                run.act(Action::End).map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(format!("{acting:?}: {e}")),
        }
    }
    Err("did not finish".into())
}

#[test]
fn a_simple_player_wins_every_intrusion() {
    let c = shipped();
    for s in &c.sites {
        for difficulty in [Difficulty::Story, Difficulty::Normal] {
            let mission = c.mission(&s.id, difficulty.pace()).unwrap();
            if let Err(why) = simple_player(&mission) {
                panic!("{} {difficulty:?}: {why}", s.id);
            }
        }
    }
}

fn broken(edit: impl FnOnce(&mut Sources)) -> Vec<neon_engine::content::Diagnostic> {
    let mut src = Sources::embedded();
    edit(&mut src);
    match Content::from_sources(&src) {
        Ok(_) => panic!("the broken copy was accepted"),
        Err(e) => e.0,
    }
}

fn says(errors: &[neon_engine::content::Diagnostic], needle: &str) -> bool {
    errors
        .iter()
        .any(|d| d.file == File::Runs && d.message.contains(needle))
}

#[test]
fn a_broken_intrusion_is_reported_at_its_line() {
    let errors = broken(|s| {
        s.runs = s
            .runs
            .replacen("link = [[1, 2], [2, 3]]", "link = [[1, 2], [2, 9]]", 1);
    });
    let hit = errors
        .iter()
        .find(|d| d.message.contains("node 9 does not exist"))
        .unwrap();
    assert_eq!(hit.file, File::Runs);
    assert!(hit.line > 1);

    let errors = broken(|s| {
        s.runs = s.runs.replacen("files = [\"user_data\"]", "files = []", 1);
    });
    assert!(
        says(&errors, "the file `user_data` is on no node"),
        "{errors:?}"
    );

    let errors = broken(|s| {
        s.runs = s.runs.replacen(
            "files = [\"user_data\"]",
            "files = [\"user_data\", \"ghost\"]",
            1,
        );
    });
    assert!(
        says(&errors, "names the unknown file `ghost`"),
        "{errors:?}"
    );

    let errors = broken(|s| {
        s.runs = s
            .runs
            .replacen("site = \"localhost\"", "site = \"nowhere\"", 1);
    });
    assert!(says(&errors, "unknown site"), "{errors:?}");
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("site `localhost` has no intrusion")),
        "{errors:?}"
    );

    let errors = broken(|s| {
        s.runs = s.runs.replacen("loot = \"objective\"", "", 1);
    });
    assert!(says(&errors, "no objective loot"), "{errors:?}");

    let errors = broken(|s| {
        s.runs = s.runs.replacen(
            "link = [[1, 2], [2, 3]]",
            "link = [[1, 2], [2, 1], [2, 3]]",
            1,
        );
    });
    assert!(says(&errors, "written twice"), "{errors:?}");

    let errors = broken(|s| {
        s.runs = s
            .runs
            .replacen("link = [[1, 2], [2, 3]]", "link = [[1, 2]]", 1);
    });
    assert!(says(&errors, "cannot be reached"), "{errors:?}");
}

#[test]
fn a_broken_deck_is_reported() {
    let errors = broken(|s| {
        s.runs = s.runs.replacen(
            "programs = [\"brute\", \"backdoor\", \"cloak\"]",
            "programs = [\"cloak\"]",
            1,
        );
    });
    assert!(
        says(&errors, "needs a breach program with no charge limit"),
        "{errors:?}"
    );

    let errors = broken(|s| {
        s.runs = s.runs.replacen(
            "\"decrypt\", \"spoof\", \"quantum\"]",
            "\"decrypt\", \"quantum\"]",
            1,
        );
    });
    assert!(
        says(
            &errors,
            "deck of tier 3: `spoof` of the tier before is missing"
        ),
        "{errors:?}"
    );

    let errors = broken(|s| {
        s.runs = s.runs.replacen("cycles = 3", "cycles = 8", 1);
    });
    assert!(says(&errors, "a turn gives 1 to 7 cycles"), "{errors:?}");

    let errors = broken(|s| {
        s.runs = s.runs.replacen(
            "[\"brute\", \"backdoor\", \"cloak\"]",
            "[\"brute\", \"brute\"]",
            1,
        );
    });
    assert!(says(&errors, "`brute` twice"), "{errors:?}");

    let errors = broken(|s| {
        s.runs = s.runs.replacen("\"cloak\"]", "\"hack\"]", 1);
    });
    assert!(
        errors
            .iter()
            .any(|d| d.file == File::Runs && d.message.contains("unknown program `hack`")),
        "{errors:?}"
    );
}

/// Plays a plan from the start and returns where it ends.
fn replay(mission: &Mission, plan: &neon_engine::run::solve::Plan) -> neon_engine::run::RunState {
    let mut state = mission.initial_state();
    for action in &plan.actions {
        state = neon_engine::run::apply(mission, &state, *action, &mut ()).unwrap();
    }
    state
}

#[test]
fn every_intrusion_is_provably_solvable_with_the_deck_every_player_has() {
    // Decision S-10: the minimal deck is what never runs out. The proof is the search itself:
    // it finds a plan that plays and ends under the cap, at every difficulty.
    let c = shipped();
    for s in &c.sites {
        for difficulty in Difficulty::ALL {
            let mission = c
                .mission(&s.id, difficulty.pace())
                .unwrap()
                .unlimited_only();
            match prove_solvable(&mission, PROOF_EXPANSIONS) {
                Proof::Solvable(plan) => {
                    let end = replay(&mission, &plan);
                    assert_eq!(end.status(), Status::Won, "{} {difficulty:?}", s.id);
                    assert_eq!(end.trace(), plan.trace, "{} {difficulty:?}", s.id);
                    assert!(plan.trace < mission.trace_cap());
                }
                other => panic!("{} {difficulty:?}: {other:?}", s.id),
            }
        }
    }
}

#[test]
fn the_margin_of_difficulty_is_exact_and_grows_with_the_difficulty() {
    // The margin is the cap minus the lowest Trace the minimal deck can reach. It is computed
    // exactly (the search finishes), and a harder difficulty never makes the run easier.
    let c = shipped();
    for s in &c.sites {
        let mut previous = 0u8;
        for difficulty in Difficulty::ALL {
            let mission = c
                .mission(&s.id, difficulty.pace())
                .unwrap()
                .unlimited_only();
            let report = optimum(&mission, PROOF_EXPANSIONS);
            let Outcome::Solved { plan, optimal } = report.outcome else {
                panic!("{} {difficulty:?}: {:?}", s.id, report.outcome);
            };
            assert!(optimal, "{} {difficulty:?}", s.id);
            assert_eq!(replay(&mission, &plan).trace(), plan.trace);
            assert!(
                plan.trace >= previous,
                "{} {difficulty:?}: {} after {previous}",
                s.id,
                plan.trace
            );
            previous = plan.trace;
        }
    }
}

#[test]
fn an_automatic_intrusion_always_finds_a_plan_and_never_beats_the_exact_one() {
    // The bounded chain of story mode, on the full deck of each tier: a plan that plays, and
    // when it is proven optimal, no worse than the simple player.
    let c = shipped();
    for s in &c.sites {
        for difficulty in [Difficulty::Story, Difficulty::Normal] {
            let mission = c.mission(&s.id, difficulty.pace()).unwrap();
            let resolution = auto_resolve(&mission, &mission.initial_state());
            let (Resolution::Optimal(plan) | Resolution::Best(plan)) = &resolution else {
                panic!("{} {difficulty:?}: {resolution:?}", s.id);
            };
            let end = replay(&mission, plan);
            assert_eq!((end.status(), end.trace()), (Status::Won, plan.trace));
            if matches!(resolution, Resolution::Optimal(_)) {
                let simple = simple_player(&mission).unwrap();
                assert!(plan.trace <= simple, "{} {difficulty:?}", s.id);
            }
        }
    }
}

// ------------------------------------------------------------------- the deck of a player

#[test]
fn a_program_the_stall_sells_is_in_the_deck_only_of_the_player_who_owns_its_item() {
    use neon_engine::content::Stock;
    use std::collections::BTreeSet;

    let c = shipped();
    let id = site("corp-server-01");
    let has = |mission: &Mission, program: Program| mission.supply(program).is_some();
    let count = |mission: &Mission| Program::ALL.iter().filter(|p| has(mission, **p)).count();
    let none = BTreeSet::new();
    let owning: BTreeSet<_> = ["prog_decrypt".parse().unwrap()].into_iter().collect();

    let everything = c
        .mission_holding(&id, Pace::NORMAL, Stock::Everything)
        .unwrap();
    let empty_handed = c
        .mission_holding(&id, Pace::NORMAL, Stock::Owned(&none))
        .unwrap();
    let with_decrypt = c
        .mission_holding(&id, Pace::NORMAL, Stock::Owned(&owning))
        .unwrap();

    // The tools see the whole deck of the tier, which is the deck `mission` has always given.
    assert!(has(&everything, Program::Spoof));
    assert_eq!(
        count(&c.mission(&id, Pace::NORMAL).unwrap()),
        count(&everything)
    );
    // The player who bought nothing keeps the programs nobody sells.
    for base in [Program::BruteForce, Program::Backdoor, Program::Cloak] {
        assert!(has(&empty_handed, base), "{base:?}");
    }
    assert!(!has(&empty_handed, Program::Spoof));
    assert!(!has(&empty_handed, Program::Decrypt));
    // Owning an item puts its program in the deck, and only that one.
    assert!(has(&with_decrypt, Program::Decrypt));
    assert!(!has(&with_decrypt, Program::Spoof));
    assert!(count(&with_decrypt) > count(&empty_handed));
}

#[test]
fn every_program_of_a_deck_beyond_the_base_one_is_sold_at_the_tier_it_appears() {
    let c = shipped();
    for kit in &c.kits {
        for program in &kit.programs {
            if !c.is_sold(*program) {
                continue;
            }
            let item = c
                .items
                .iter()
                .find(|item| item.program == Some(*program))
                .unwrap();
            let sold_from = item.unlock.tier.unwrap_or(1);
            assert!(
                sold_from <= kit.tier,
                "{program:?} is in the deck of tier {} but sold from tier {sold_from}",
                kit.tier
            );
        }
    }
}

#[test]
fn an_upgrade_adds_one_charge_to_a_limited_program_of_the_deck_of_the_player_who_owns_it() {
    use neon_engine::content::Stock;
    use neon_engine::run::Supply;
    use std::collections::BTreeSet;

    let c = shipped();
    let id = site("corp-server-01");
    let charges = |mission: &Mission, program: Program| match mission.supply(program) {
        Some(Supply::Limited(count)) => Some(count),
        _ => None,
    };
    let none = BTreeSet::new();
    let owning: BTreeSet<_> = ["up_cloak".parse().unwrap()].into_iter().collect();
    let plain = c
        .mission_holding(&id, Pace::NORMAL, Stock::Owned(&none))
        .unwrap();
    let upgraded = c
        .mission_holding(&id, Pace::NORMAL, Stock::Owned(&owning))
        .unwrap();
    let everything = c
        .mission_holding(&id, Pace::NORMAL, Stock::Everything)
        .unwrap();

    // The Cloak of every deck has two charges; the upgrade makes it three, and only for its owner.
    assert_eq!(charges(&plain, Program::Cloak), Some(2));
    assert_eq!(charges(&upgraded, Program::Cloak), Some(3));
    assert_eq!(charges(&everything, Program::Cloak), Some(3));
    // Nothing else moves: the other limited programs of the deck keep their charges.
    assert_eq!(
        charges(&plain, Program::Spoof),
        charges(&upgraded, Program::Spoof)
    );
    // An upgrade for a program the deck does not have adds nothing to the deck: the first tier
    // has no Virus, and the owner of its upgrade still has none.
    let virus: BTreeSet<_> = ["up_virus".parse().unwrap()].into_iter().collect();
    let first = c
        .mission_holding(&site("localhost"), Pace::NORMAL, Stock::Owned(&virus))
        .unwrap();
    assert_eq!(first.supply(Program::Virus), None);
}
