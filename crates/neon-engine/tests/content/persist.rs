//! Saving and loading the mission state through the versioned save envelope.

use std::sync::OnceLock;

use neon_engine::content::schema::QuestStatus;
use neon_engine::content::{State, new_game};
use neon_engine::save::{self, SaveError, SaveMeta, SaveState};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

use crate::common::{q, shipped};
use crate::optimist::{all_plans, play};
use crate::props::{content, history, reached, warm};

/// What a campaign game will save: the state, validated against the shipped content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
struct Saved(State);

impl SaveState for Saved {
    fn validate(&self) -> Result<(), String> {
        self.0.validate(content())
    }
}

fn meta(state: &State) -> SaveMeta {
    SaveMeta {
        player: "Neon".to_owned(),
        turn: state.clock,
    }
}

fn write(state: &State) -> String {
    save::encode(&meta(state), &Saved(state.clone())).expect("a state can be written")
}

fn read(text: &str) -> Result<State, SaveError> {
    save::decode::<Saved>(text).map(|(_, saved)| saved.0)
}

/// The start of the real game, then the same game after a few steps, as save text.
fn new_game_state() -> State {
    new_game(&shipped()).0
}

/// A state in the middle of the campaign: a real play cut after 12 rounds.
fn midgame_state() -> State {
    static STATE: OnceLock<State> = OnceLock::new();
    STATE
        .get_or_init(|| {
            let c = shipped();
            let plan = all_plans(&c).into_iter().next().unwrap();
            let report = play(&c, &plan);
            let mut state = new_game(&c).0;
            for round in report.log.iter().take(12) {
                for fact in round {
                    state.apply(&c, fact);
                }
                state.refresh(&c);
            }
            state
        })
        .clone()
}

/// The text of a save, edited on the untyped tree.
fn edited(state: &State, edit: impl FnOnce(&mut toml::Table)) -> String {
    let mut table: toml::Table = write(state).parse().unwrap();
    let inner = table
        .get_mut("state")
        .and_then(toml::Value::as_table_mut)
        .unwrap();
    edit(inner);
    toml::to_string(&table).unwrap()
}

fn set(table: &mut toml::Table, path: &[&str], value: toml::Value) {
    let (last, parents) = path.split_last().unwrap();
    let mut cursor = table;
    for key in parents {
        cursor = cursor
            .entry((*key).to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
    }
    cursor.insert((*last).to_owned(), value);
}

fn rejected(text: &str) -> SaveError {
    read(text).expect_err("the damaged state was accepted")
}

#[test]
fn the_start_of_the_game_has_the_documented_shape() {
    let text = write(&new_game_state());
    let expected = r#"version = 1

[meta]
player = "Neon"
turn = 0

[state]
clock = 0
tier = 1
reputation = 0
earned = 100
claimed = ["open-m01"]

[state.heat]
at = 0
heat = 0

[state.contacts]
angel = "offline"
aura = "offline"
broker = "offline"
echo7 = "available"
ghost = "offline"
insider = "offline"
miner = "offline"
phoenix = "offline"
r4z0r = "offline"

[state.quests.m01]
status = "active"
opened_at = 0

[end]
ok = true
"#;
    assert_eq!(text, expected);
    assert_eq!(read(&text).unwrap(), new_game_state());
}

#[test]
fn the_middle_of_the_campaign_uses_text_keys_everywhere() {
    let state = midgame_state();
    assert!(state.clock > 10 && !state.talks.is_empty(), "a real game");
    assert_ne!(
        state.status(&q("m14")),
        QuestStatus::Completed,
        "not the end"
    );
    let text = write(&state);
    let table: toml::Table = text.parse().unwrap();
    let inner = table["state"].as_table().unwrap();
    // Maps keyed by turn use the turn as text; sets are sorted arrays; nothing is positional.
    let peaks = inner.get("heat_peaks").and_then(toml::Value::as_table);
    assert!(peaks.is_none_or(|p| p.keys().all(|k| k.parse::<u32>().is_ok())));
    assert!(inner["extracted"].is_table() && inner["compromised"].is_table());
    assert!(inner["quests"].as_table().unwrap().contains_key("m01"));
    assert_eq!(read(&text).unwrap(), state);
}

#[test]
fn every_finished_campaign_round_trips_and_validates() {
    let c = shipped();
    for plan in &all_plans(&c) {
        let state = play(&c, plan).state;
        assert_eq!(state.validate(&c), Ok(()), "{:?}", plan.choices);
        let text = write(&state);
        assert_eq!(read(&text).unwrap(), state, "{:?}", plan.choices);
        // Writing what was read gives the same text: the layout is deterministic.
        assert_eq!(write(&read(&text).unwrap()), text);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// `parse(write(s)) == s` for every state the engine can reach, and it validates.
    #[test]
    fn any_reachable_state_round_trips(w in warm(), h in history()) {
        let (s, _) = reached(w, &h);
        prop_assert_eq!(s.validate(content()), Ok(()));
        let text = write(&s);
        prop_assert_eq!(read(&text).unwrap(), s);
    }
}

#[test]
fn values_outside_their_bounds_are_refused_when_read() {
    let state = midgame_state();
    let cases: [(&[&str], toml::Value); 5] = [
        (&["reputation"], 1_000_001.into()),
        (&["reputation"], (-1_000_001).into()),
        (&["earned"], 1_000_000_001.into()),
        (&["earned"], (-5).into()),
        (
            &["payments", "1"],
            vec![toml::Value::from(2_000_000_000)].into(),
        ),
    ];
    for (path, value) in cases {
        let text = edited(&state, |t| set(t, path, value.clone()));
        assert!(
            matches!(rejected(&text), SaveError::Schema(_)),
            "{path:?} = {value}"
        );
    }
    // Exactly at the caps is fine.
    let text = edited(&state, |t| {
        set(t, &["reputation"], 1_000_000.into());
        set(t, &["earned"], 1_000_000_000.into());
    });
    assert!(read(&text).is_ok());
}

#[test]
fn a_state_the_game_could_not_have_reached_is_refused_with_a_reason() {
    let state = midgame_state();
    let invalid = |path: &[&str], value: toml::Value, needle: &str| {
        let text = edited(&state, |t| set(t, path, value));
        match rejected(&text) {
            SaveError::Invalid(reason) => {
                assert!(reason.contains(needle), "{path:?}: {reason}");
            }
            other => panic!("{path:?}: expected an impossible state, got {other:?}"),
        }
    };
    invalid(&["tier"], 0.into(), "tier 0");
    invalid(&["tier"], 7.into(), "tier 7");
    invalid(&["heat", "heat"], 101.into(), "heat 101");
    invalid(&["clock"], 0.into(), "later than the clock");
    invalid(&["flags", "no_such_flag"], true.into(), "unknown flag");
    invalid(
        &["flags", "d1"],
        "nonsense".into(),
        "does not fit flag `d1`",
    );
    invalid(
        &["flags", "adv"],
        vec![toml::Value::from("nope")].into(),
        "does not fit flag `adv`",
    );
    invalid(
        &["contacts", "nobody"],
        "available".into(),
        "unknown contact",
    );
    invalid(
        &["owned"],
        vec![toml::Value::from("nothing")].into(),
        "unknown item",
    );
    invalid(
        &["opened"],
        vec![toml::Value::from("f99")].into(),
        "unknown readable",
    );
    invalid(
        &["used"],
        vec![toml::Value::from("hack")].into(),
        "unknown command",
    );
    invalid(&["compromised", "nowhere"], 1.into(), "unknown site");
    invalid(
        &["extracted", "localhost"],
        vec![toml::Value::from("no_such_file")].into(),
        "unknown file",
    );
    invalid(&["links", "echo7"], 9.into(), "link level 9");
    invalid(&["bonus_trust", "echo7"], 2_000_000.into(), "trust");
    invalid(&["decisions", "d1"], "maybe".into(), "unknown choice");
    invalid(&["restarts", "m99"], 1.into(), "unknown quest");
    for (claim, needle) in [
        ("quest-zz", "does not belong"),
        ("quest-m14", "did not happen"),
    ] {
        let text = edited(&state, |t| {
            t["claimed"].as_array_mut().unwrap().push(claim.into());
        });
        match rejected(&text) {
            SaveError::Invalid(reason) => assert!(reason.contains(needle), "{claim}: {reason}"),
            other => panic!("{claim}: {other:?}"),
        }
    }
    invalid(
        &["quests", "m01", "done"],
        vec![toml::Value::from(vec![toml::Value::from(99), 0.into()])].into(),
        "no objective (99, 0)",
    );
    invalid(
        &["quests", "m99"],
        toml::Value::Table(toml::toml! { status = "active"
        opened_at = 0 }),
        "unknown quest",
    );
}

#[test]
fn the_heat_frontier_must_strictly_decrease() {
    let state = midgame_state();
    let text = edited(&state, |t| {
        set(t, &["heat_peaks", "1"], 50.into());
        set(t, &["heat_peaks", "2"], 50.into());
        let clock = t["clock"].as_integer().unwrap();
        assert!(clock >= 2);
    });
    match rejected(&text) {
        SaveError::Invalid(reason) => assert!(reason.contains("strictly decrease"), "{reason}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_quest_status_without_its_claim_is_refused() {
    let state = midgame_state();
    let text = edited(&state, |t| {
        t.remove("claimed");
    });
    assert!(matches!(rejected(&text), SaveError::Invalid(_)));
}

#[test]
fn shapes_that_are_not_a_state_are_refused_by_the_reader() {
    let state = midgame_state();
    for (path, value) in [
        (&["heat_peaks", "abc"][..], toml::Value::from(10)),
        (&["payments", "-1"][..], vec![toml::Value::from(10)].into()),
        (&["contacts", "Echo 7"][..], "available".into()),
        (&["tier"][..], "three".into()),
        (&["quests", "m01", "status"][..], "paused".into()),
        (&["flags", "d1"][..], toml::Value::Table(toml::Table::new())),
    ] {
        let text = edited(&state, |t| set(t, path, value.clone()));
        assert!(
            matches!(rejected(&text), SaveError::Schema(_)),
            "{path:?} = {value}"
        );
    }
    // A missing required value is a shape error too; an unknown key is ignored.
    let missing = edited(&state, |t| {
        t.remove("earned");
    });
    assert!(matches!(rejected(&missing), SaveError::Schema(_)));
    let extra = edited(&state, |t| set(t, &["from_the_future"], 1.into()));
    assert_eq!(read(&extra).unwrap(), state);
}

#[test]
fn extreme_values_are_refused_rather_than_wrapped() {
    let state = midgame_state();
    for value in [i64::MIN, i64::MAX, -1, 4_294_967_296] {
        let text = edited(&state, |t| set(t, &["clock"], value.into()));
        assert!(read(&text).is_err(), "clock = {value}");
    }
}

fn mutate(text: &str, pos: usize, len: usize, junk: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = pos % (chars.len() + 1);
    let end = (start + len).min(chars.len());
    let mut out: String = chars.iter().take(start).collect();
    out.push_str(junk);
    out.extend(chars.iter().skip(end));
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A damaged or hostile save is an error or a valid state, never a panic.
    #[test]
    fn hostile_save_text_never_panics(
        pos in 0usize..20_000,
        len in 0usize..120,
        junk in "\\PC{0,24}",
        noise in "\\PC{0,200}",
        whole in any::<bool>(),
    ) {
        let text = write(&midgame_state());
        let damaged = if whole { noise } else { mutate(&text, pos, len, &junk) };
        if let Ok(state) = read(&damaged) {
            prop_assert_eq!(state.validate(content()), Ok(()));
        }
    }

    /// Whatever the number a key is given, reading it back never panics.
    #[test]
    fn numbers_in_any_position_are_refused_or_valid(n in any::<i64>(), key in 0usize..6) {
        let path: &[&str] = match key {
            0 => &["clock"],
            1 => &["tier"],
            2 => &["heat", "heat"],
            3 => &["heat", "at"],
            4 => &["reputation"],
            _ => &["earned"],
        };
        let text = edited(&midgame_state(), |t| set(t, path, n.into()));
        if let Ok(state) = read(&text) {
            prop_assert_eq!(state.validate(content()), Ok(()));
        }
    }
}
