use std::fs;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use neon_engine::Game;
use neon_engine::demo::DemoGame;
use neon_engine::prompt::Input;
use tempfile::TempDir;

use super::*;

fn store() -> (TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().join("saves"));
    (dir, store)
}

/// The text of a real save after `scans` scans, so two different numbers are two different
/// valid saves.
fn save_text(scans: usize) -> String {
    let mut game = DemoGame::new(7);
    game.start();
    for input in [
        Input::Continue,
        Input::Line("Neon".into()),
        Input::Confirm(true),
    ] {
        game.handle(input);
    }
    for _ in 0..scans {
        game.handle(Input::Line("scan".into()));
    }
    game.snapshot().unwrap()
}

fn load(store: &Store, target: Target) -> Option<Loaded<DemoGame>> {
    store.read(target, DemoGame::from_save).unwrap()
}

fn file(store: &Store, name: &str) -> PathBuf {
    store.dir.join(name)
}

fn names(store: &Store) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(&store.dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn labels_round_trip_and_bad_ones_say_what_is_accepted() {
    for target in Target::all() {
        assert_eq!(Target::parse(&target.label()), Ok(target));
    }
    assert_eq!(Target::parse("checkpoint"), Ok(Target::Checkpoint(1)));
    for bad in [
        "",
        "Auto",
        "slot-0",
        "slot-10",
        "checkpoint-4",
        "slot",
        "slot-x",
        "2",
    ] {
        let error = Target::parse(bad).unwrap_err();
        assert!(error.contains("slot-9"), "{bad}: {error}");
    }
}

#[test]
fn a_save_written_is_a_save_read() {
    let (_dir, store) = store();
    assert!(load(&store, Target::Auto).is_none(), "nothing yet");
    let text = save_text(2);
    store.write(SaveRequest::Autosave, &text).unwrap();
    let loaded = load(&store, Target::Auto).unwrap();
    assert!(!loaded.from_backup);
    assert_eq!(loaded.value.snapshot().unwrap(), text);
    assert_eq!(
        names(&store),
        ["auto.toml"],
        "no temporary file is left behind"
    );
}

#[test]
fn each_write_keeps_the_previous_valid_save_as_backup() {
    let (_dir, store) = store();
    let (first, second, third) = (save_text(1), save_text(2), save_text(3));
    store.write(SaveRequest::Autosave, &first).unwrap();
    assert_eq!(names(&store), ["auto.toml"], "nothing to back up yet");
    store.write(SaveRequest::Autosave, &second).unwrap();
    store.write(SaveRequest::Autosave, &third).unwrap();
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml")).unwrap(),
        third
    );
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml.bak")).unwrap(),
        second
    );
}

#[test]
fn a_damaged_save_is_set_aside_and_never_replaces_the_good_backup() {
    let (_dir, store) = store();
    let (first, second, third) = (save_text(1), save_text(2), save_text(3));
    store.write(SaveRequest::Autosave, &first).unwrap();
    store.write(SaveRequest::Autosave, &second).unwrap();
    fs::write(file(&store, "auto.toml"), "garbage").unwrap();

    store.write(SaveRequest::Autosave, &third).unwrap();
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml")).unwrap(),
        third
    );
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml.corrupt")).unwrap(),
        "garbage"
    );
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml.bak")).unwrap(),
        first,
        "the backup of the damaged file would have destroyed the last good copy"
    );
}

#[test]
fn a_damaged_save_falls_back_to_the_backup_and_says_so() {
    let (_dir, store) = store();
    let (first, second) = (save_text(1), save_text(2));
    store.write(SaveRequest::Autosave, &first).unwrap();
    store.write(SaveRequest::Autosave, &second).unwrap();
    // A cut file, as after a crash on a file system without atomic rename.
    fs::write(file(&store, "auto.toml"), &second[..second.len() / 2]).unwrap();

    let loaded = load(&store, Target::Auto).unwrap();
    assert!(loaded.from_backup);
    assert_eq!(loaded.value.snapshot().unwrap(), first);
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml.bak")).unwrap(),
        first,
        "loading changes nothing on disk"
    );
}

#[test]
fn without_a_valid_copy_the_error_explains_and_nothing_is_touched() {
    let (_dir, store) = store();
    fs::create_dir_all(&store.dir).unwrap();
    fs::write(file(&store, "auto.toml"), "garbage").unwrap();
    fs::write(file(&store, "auto.toml.bak"), "also garbage").unwrap();
    let error = store.read(Target::Auto, DemoGame::from_save).err().unwrap();
    assert!(matches!(error, StoreError::Unreadable { .. }), "{error}");
    assert!(
        error.to_string().contains("no valid previous copy"),
        "{error}"
    );
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml")).unwrap(),
        "garbage"
    );
    assert_eq!(names(&store), ["auto.toml", "auto.toml.bak"]);
}

#[test]
fn a_save_from_a_newer_game_is_refused_and_left_alone() {
    let (_dir, store) = store();
    let newer = save_text(1).replacen("version = 1", "version = 2", 1);
    fs::create_dir_all(&store.dir).unwrap();
    fs::write(file(&store, "auto.toml"), &newer).unwrap();
    let error = store.read(Target::Auto, DemoGame::from_save).err().unwrap();
    assert!(error.to_string().contains("newer game"), "{error}");
    assert_eq!(
        fs::read_to_string(file(&store, "auto.toml")).unwrap(),
        newer
    );
}

#[test]
fn an_oversized_file_is_refused_without_being_read() {
    let (_dir, store) = store();
    fs::create_dir_all(&store.dir).unwrap();
    let big = vec![b' '; usize::try_from(MAX_SAVE_BYTES).unwrap() + 1];
    fs::write(file(&store, "auto.toml"), big).unwrap();
    let error = store.read(Target::Auto, DemoGame::from_save).err().unwrap();
    assert!(error.to_string().contains("too large"), "{error}");
    // A text that is not UTF-8 is just another unreadable file.
    fs::write(file(&store, "slot-1.toml"), [0xff, 0xfe, 0x00]).unwrap();
    assert!(store.read(Target::Slot(1), DemoGame::from_save).is_err());
    // And saving over the oversized one sets it aside.
    store.write(SaveRequest::Autosave, &save_text(1)).unwrap();
    assert!(file(&store, "auto.toml.corrupt").exists());
}

#[test]
fn checkpoints_keep_a_history_that_the_autosave_never_touches() {
    let (_dir, store) = store();
    let saves: Vec<String> = (1..=5).map(save_text).collect();
    for text in &saves {
        store.write(SaveRequest::Checkpoint, text).unwrap();
        store.write(SaveRequest::Autosave, text).unwrap();
    }
    let read = |n| fs::read_to_string(file(&store, &format!("checkpoint-{n}.toml"))).unwrap();
    assert_eq!(
        [read(1), read(2), read(3)],
        [saves[4].clone(), saves[3].clone(), saves[2].clone()],
        "newest first, three kept"
    );
    assert!(!file(&store, "checkpoint-4.toml").exists());
    assert_eq!(
        load(&store, Target::Checkpoint(2))
            .unwrap()
            .value
            .snapshot()
            .unwrap(),
        saves[3]
    );
}

#[test]
fn manual_slots_are_independent_files_with_their_own_backup() {
    let (_dir, store) = store();
    let (first, second) = (save_text(1), save_text(2));
    store.write(SaveRequest::Slot(3), &first).unwrap();
    store.write(SaveRequest::Slot(3), &second).unwrap();
    store.write(SaveRequest::Slot(9), &first).unwrap();
    assert_eq!(
        names(&store),
        ["slot-3.toml", "slot-3.toml.bak", "slot-9.toml"]
    );
    assert_eq!(
        load(&store, Target::Slot(3))
            .unwrap()
            .value
            .snapshot()
            .unwrap(),
        second
    );
    assert!(load(&store, Target::Slot(4)).is_none());
    for bad in [0, 10, 255] {
        assert!(matches!(
            store.write(SaveRequest::Slot(bad), &first),
            Err(StoreError::NoSuchSlot(n)) if n == bad
        ));
    }
}

#[test]
fn the_list_shows_what_exists_and_survives_broken_files() {
    let (_dir, store) = store();
    assert!(store.list().is_empty());
    store.write(SaveRequest::Autosave, &save_text(2)).unwrap();
    store.write(SaveRequest::Slot(2), &save_text(1)).unwrap();
    fs::write(file(&store, "slot-5.toml"), "garbage").unwrap();
    let listing = store.list();
    let labels: Vec<String> = listing.iter().map(|entry| entry.target.label()).collect();
    assert_eq!(labels, ["auto", "slot-2", "slot-5"]);
    let auto = listing[0].meta.as_ref().unwrap();
    assert_eq!((auto.player.as_str(), auto.turn), ("Neon", 2));
    assert!(listing[2].meta.is_err());
}

#[test]
fn a_failure_just_before_the_rename_leaves_the_old_file_intact_and_no_litter() {
    let (_dir, store) = store();
    let old = save_text(1);
    store.write(SaveRequest::Autosave, &old).unwrap();
    let path = file(&store, "auto.toml");

    let result = write_atomic(&path, save_text(2).as_bytes(), || {
        Err(io::Error::other("power cut"))
    });
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), old);
    assert_eq!(
        names(&store),
        ["auto.toml"],
        "the temporary file is removed"
    );
}

#[test]
fn a_reader_never_sees_a_mix_of_two_saves() {
    let (_dir, store) = store();
    let (small, large) = (save_text(1), save_text(30));
    store.write(SaveRequest::Autosave, &small).unwrap();
    let path = file(&store, "auto.toml");
    let done = AtomicBool::new(false);
    thread::scope(|scope| {
        scope.spawn(|| {
            for round in 0..60 {
                let text = if round % 2 == 0 { &large } else { &small };
                write_atomic(&path, text.as_bytes(), || Ok(())).unwrap();
            }
            done.store(true, Ordering::SeqCst);
        });
        while !done.load(Ordering::SeqCst) {
            let seen = fs::read_to_string(&path).unwrap();
            assert!(
                seen == small || seen == large,
                "a torn read of {} bytes",
                seen.len()
            );
        }
    });
}
