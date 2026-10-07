use neon_engine::demo::DemoGame;
use neon_engine::event::{Event, Role};
use neon_engine::save::SaveRequest;
use neon_engine::{Game, Input, Step};

use super::*;

fn game() -> DemoGame {
    let mut game = DemoGame::new(7);
    game.start();
    for input in [
        Input::Continue,
        Input::Line("Neon".into()),
        Input::Confirm(true),
    ] {
        game.handle(input);
    }
    game
}

fn step(request: Option<SaveRequest>) -> Step {
    Step {
        events: Vec::new(),
        prompt: neon_engine::Prompt::Command,
        save: request,
    }
}

fn keys(step: &Step) -> Vec<(String, bool)> {
    step.events
        .iter()
        .filter_map(|event| match event {
            Event::Message { role, text, .. } => {
                Some((text.key.to_string(), matches!(role, Role::Error)))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn an_automatic_save_is_written_silently() {
    let dir = tempfile::tempdir().unwrap();
    let mut persistence = Persistence::new(Store::new(dir.path().join("saves")));
    let mut done = step(Some(SaveRequest::Autosave));
    persistence.after_step(&game(), &mut done);
    assert!(done.events.is_empty());
    assert!(dir.path().join("saves/auto.toml").exists());

    let mut checkpoint = step(Some(SaveRequest::Checkpoint));
    persistence.after_step(&game(), &mut checkpoint);
    assert!(checkpoint.events.is_empty());
    assert!(dir.path().join("saves/checkpoint-1.toml").exists());
}

#[test]
fn a_manual_save_says_where_it_went() {
    let dir = tempfile::tempdir().unwrap();
    let mut persistence = Persistence::new(Store::new(dir.path().join("saves")));
    let mut done = step(Some(SaveRequest::Slot(4)));
    persistence.after_step(&game(), &mut done);
    assert_eq!(keys(&done), [("ui.save.slot_done".to_owned(), false)]);
    assert!(dir.path().join("saves/slot-4.toml").exists());
}

#[test]
fn a_step_without_a_request_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut persistence = Persistence::new(Store::new(dir.path().join("saves")));
    let mut quiet = step(None);
    persistence.after_step(&game(), &mut quiet);
    assert!(quiet.events.is_empty());
    assert!(!dir.path().join("saves").exists());
}

#[test]
fn a_failing_automatic_save_is_reported_once_and_a_manual_one_every_time() {
    let dir = tempfile::tempdir().unwrap();
    // The "folder" is a file, so nothing can be written under it.
    let blocked = dir.path().join("saves");
    std::fs::write(&blocked, "in the way").unwrap();
    let mut persistence = Persistence::new(Store::new(blocked));

    let failed = |persistence: &mut Persistence, request| {
        let mut result = step(Some(request));
        persistence.after_step(&game(), &mut result);
        keys(&result)
    };
    let error = [("ui.save.failed".to_owned(), true)];
    assert_eq!(failed(&mut persistence, SaveRequest::Autosave), error);
    assert_eq!(failed(&mut persistence, SaveRequest::Autosave), []);
    assert_eq!(failed(&mut persistence, SaveRequest::Checkpoint), []);
    assert_eq!(failed(&mut persistence, SaveRequest::Slot(1)), error);
    assert_eq!(failed(&mut persistence, SaveRequest::Slot(1)), error);
}

#[test]
fn with_saving_off_only_a_manual_request_is_answered() {
    let mut persistence = Persistence::disabled();
    for request in [SaveRequest::Autosave, SaveRequest::Checkpoint] {
        let mut result = step(Some(request));
        persistence.after_step(&game(), &mut result);
        assert!(result.events.is_empty(), "{request:?}");
    }
    let mut manual = step(Some(SaveRequest::Slot(1)));
    persistence.after_step(&game(), &mut manual);
    assert_eq!(keys(&manual), [("ui.save.disabled".to_owned(), true)]);
}
