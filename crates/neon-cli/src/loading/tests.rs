use std::fs;

use neon_engine::campaign::{CampaignGame, Difficulty};
use neon_engine::demo::DemoGame;
use neon_engine::save::SaveRequest;
use neon_engine::text::RenderMode;
use tempfile::TempDir;

use super::*;
use crate::render::{Renderer, Verbosity};
use crate::test_support::catalog_en;

fn campaign() -> CampaignGame {
    CampaignGame::new(Difficulty::Normal, 1).unwrap()
}

/// A campaign past its opening, and its save text.
fn played(handle: &str) -> (CampaignGame, String) {
    let mut game = campaign();
    game.start();
    game.handle(Input::Line(handle.to_owned()));
    game.handle(Input::Continue);
    game.handle(Input::Continue);
    game.handle(Input::Confirm(false));
    let text = game.snapshot().unwrap();
    (game, text)
}

/// A saves folder, with saves written the way the game writes them.
fn folder_with(saves: &[(SaveRequest, &str)]) -> (TempDir, Persistence) {
    let folder = TempDir::new().unwrap();
    let store = Store::new(folder.path().to_path_buf());
    for (request, text) in saves {
        store.write(*request, text).unwrap();
    }
    (folder, Persistence::new(store))
}

fn said(events: &[Event]) -> String {
    let catalog = catalog_en();
    let renderer = Renderer {
        catalog: &catalog,
        mode: RenderMode::FULL,
        verbosity: Verbosity::Normal,
        colors: None,
    };
    events
        .iter()
        .flat_map(|event| renderer.event(event))
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn question_text(loading: &Loading) -> String {
    let Some(Prompt::Confirm { question, default }) = loading.question() else {
        panic!("a question was expected");
    };
    assert!(!default, "the safe answer is no");
    let catalog = catalog_en();
    neon_engine::text::render(&question, &catalog, RenderMode::FULL)
}

#[test]
fn only_load_is_taken_whatever_its_case_and_with_at_most_one_name() {
    assert_eq!(request("load"), Some(Request::List));
    assert_eq!(request("  LOAD  "), Some(Request::List));
    assert_eq!(request("load slot-1"), Some(Request::Load("slot-1".into())));
    assert_eq!(request("Load SLOT-1"), Some(Request::Load("slot-1".into())));
    assert_eq!(request("load slot-1 slot-2"), Some(Request::Usage));
    for other in ["", "   ", "loader", "unload", "export", "load_game x"] {
        assert_eq!(request(other), None, "{other:?}");
    }
}

#[test]
fn the_saves_are_listed_with_who_and_how_far_and_a_damaged_one_is_said() {
    let (_, text) = played("Neon");
    let (folder, persistence) = folder_with(&[(SaveRequest::Slot(2), &text)]);
    fs::write(folder.path().join("slot-3.toml"), "garbage").unwrap();
    let mut loading = Loading::new();
    let events = loading.run(Request::List, &persistence);
    assert!(matches!(&events[0], Event::Screen(table) if table.rows.len() == 2));
    let Event::Screen(table) = &events[0] else {
        unreachable!()
    };
    let catalog = catalog_en();
    let cell = |row: usize, column: usize| {
        neon_engine::text::render(&table.rows[row][column], &catalog, RenderMode::FULL)
    };
    assert_eq!((cell(0, 0), cell(0, 1)), ("slot-2".into(), "Neon".into()));
    assert!(cell(1, 1).starts_with("damaged"), "{}", cell(1, 1));
    assert!(!loading.is_pending(), "a list asks nothing");
}

#[test]
fn no_saves_and_saving_turned_off_are_said() {
    let (_folder, persistence) = folder_with(&[]);
    let mut loading = Loading::new();
    assert!(said(&loading.run(Request::List, &persistence)).contains("No saved game."));
    let off = Persistence::disabled();
    assert!(said(&loading.run(Request::List, &off)).contains("Saving is turned off"));
    assert!(said(&loading.run(Request::Load("auto".into()), &off)).contains("turned off"));
}

#[test]
fn a_name_that_is_no_save_or_a_save_that_is_not_there_is_said_and_asks_nothing() {
    let (_folder, persistence) = folder_with(&[]);
    let mut loading = Loading::new();
    let unknown = said(&loading.run(Request::Load("nowhere".into()), &persistence));
    assert!(
        unknown.contains("no save called nowhere") && unknown.contains("slot-9"),
        "{unknown}"
    );
    let missing = said(&loading.run(Request::Load("slot-4".into()), &persistence));
    assert!(
        missing.contains("There is no save called slot-4."),
        "{missing}"
    );
    assert!(said(&loading.run(Request::Usage, &persistence)).contains("Usage"));
    assert!(!loading.is_pending());
}

#[test]
fn a_damaged_save_with_no_good_copy_is_refused() {
    let (folder, persistence) = folder_with(&[]);
    fs::write(folder.path().join("slot-1.toml"), "garbage").unwrap();
    let mut loading = Loading::new();
    let said = said(&loading.run(Request::Load("slot-1".into()), &persistence));
    assert!(
        said.contains("damaged") && said.contains("Nothing was changed"),
        "{said}"
    );
    assert!(!loading.is_pending());
}

#[test]
fn a_save_that_can_be_loaded_asks_first_with_who_and_how_far() {
    let (_, text) = played("Zoe");
    let (_folder, persistence) = folder_with(&[(SaveRequest::Slot(1), &text)]);
    let mut loading = Loading::new();
    let warning = said(&loading.run(Request::Load("slot-1".into()), &persistence));
    assert!(warning.contains("is lost"), "{warning}");
    assert!(loading.is_pending());
    let question = question_text(&loading);
    assert!(question.starts_with("Load slot-1 (Zoe,"), "{question}");
    assert!(
        question.chars().count() < 60,
        "short enough for one line: {question}"
    );
}

#[test]
fn yes_replaces_the_game_and_says_so() {
    let (saved, text) = played("Zoe");
    let (_folder, persistence) = folder_with(&[(SaveRequest::Slot(1), &text)]);
    let (mut current, _) = played("Neon");
    let mut loading = Loading::new();
    loading.run(Request::Load("slot-1".into()), &persistence);
    let Answer::Loaded(step) = loading.answer(&Input::Confirm(true), &mut current) else {
        panic!("the game should have been loaded");
    };
    assert_eq!(current.state(), saved.state());
    assert!(
        said(&step.events).starts_with("Loaded slot-1."),
        "{}",
        said(&step.events)
    );
    assert_eq!(step.prompt, Prompt::Command);
    assert!(!loading.is_pending());
}

#[test]
fn no_and_escape_load_nothing() {
    let (_, text) = played("Zoe");
    let (_folder, persistence) = folder_with(&[(SaveRequest::Slot(1), &text)]);
    for answer in [Input::Confirm(false), Input::Cancel] {
        let (mut current, _) = played("Neon");
        let before = current.state().clone();
        let mut loading = Loading::new();
        loading.run(Request::Load("slot-1".into()), &persistence);
        let Answer::Said(events) = loading.answer(&answer, &mut current) else {
            panic!("nothing should have been loaded");
        };
        assert!(said(&events).contains("Nothing was loaded."));
        assert_eq!(current.state(), &before);
        assert!(!loading.is_pending());
    }
}

#[test]
fn the_end_of_the_input_drops_the_question_and_is_the_games() {
    let (_, text) = played("Zoe");
    let (_folder, persistence) = folder_with(&[(SaveRequest::Slot(1), &text)]);
    let (mut current, _) = played("Neon");
    let mut loading = Loading::new();
    loading.run(Request::Load("slot-1".into()), &persistence);
    assert!(matches!(
        loading.answer(&Input::Eof, &mut current),
        Answer::Closed
    ));
    assert!(!loading.is_pending());
}

#[test]
fn a_save_of_another_game_is_refused_and_the_game_is_untouched() {
    let demo = DemoGame::new(1).snapshot().unwrap();
    let (_folder, persistence) = folder_with(&[(SaveRequest::Slot(1), &demo)]);
    let (mut current, _) = played("Neon");
    let before = current.state().clone();
    let mut loading = Loading::new();
    loading.run(Request::Load("slot-1".into()), &persistence);
    let Answer::Said(events) = loading.answer(&Input::Confirm(true), &mut current) else {
        panic!("it should not load");
    };
    let said = said(&events);
    assert!(
        said.contains("Could not load") && said.contains("Nothing was changed"),
        "{said}"
    );
    assert_eq!(current.state(), &before);
    assert!(!loading.is_pending());
}

#[test]
fn a_line_that_is_not_an_answer_asks_again() {
    let (_, text) = played("Zoe");
    let (_folder, persistence) = folder_with(&[(SaveRequest::Slot(1), &text)]);
    let (mut current, _) = played("Neon");
    let mut loading = Loading::new();
    loading.run(Request::Load("slot-1".into()), &persistence);
    let Answer::Said(events) = loading.answer(&Input::Line("what".into()), &mut current) else {
        panic!("a question is not answered by a line");
    };
    assert!(!events.is_empty());
    assert!(loading.is_pending(), "the question stands");
}
