//! The screens a frontend keeps open: each is the table its command shows, drawn again from
//! the game as it is now, and drawing it changes nothing.

use super::support::*;
use crate::event::{Event, ScreenKind, Table};
use crate::game::Game;
use crate::text::{Lang, RenderMode, render};

/// The pinned table the last step showed, if any.
fn shown(driver: &Driver) -> Option<Table> {
    driver.last.events.iter().find_map(|event| match event {
        Event::Screen(table) if table.pin.is_some() => Some(table.clone()),
        _ => None,
    })
}

/// A game past the first quest, where the stall, the inbox and the archives are open.
fn open_hub() -> Driver {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver
}

#[test]
fn every_list_command_shows_a_table_that_names_its_kind() {
    let mut driver = open_hub();
    for (line, kind) in [
        ("shop", ScreenKind::Shop),
        ("laylow", ScreenKind::Services),
        ("contacts", ScreenKind::Contacts),
        ("messages", ScreenKind::Mail),
        ("archives", ScreenKind::Archives),
        ("quests", ScreenKind::Quests),
        ("net", ScreenKind::Net),
    ] {
        driver.line(line);
        let table = shown(&driver).unwrap_or_else(|| panic!("`{line}` shows no pinned table"));
        assert_eq!(table.pin, Some(kind), "{line}");
    }
}

#[test]
fn the_help_is_read_once_and_not_pinned() {
    let mut driver = open_hub();
    driver.line("help");
    assert!(shown(&driver).is_none());
}

#[test]
fn a_screen_is_the_very_table_its_command_showed() {
    let mut driver = open_hub();
    for (line, kind) in [
        ("shop", ScreenKind::Shop),
        ("laylow", ScreenKind::Services),
        ("contacts", ScreenKind::Contacts),
        ("messages", ScreenKind::Mail),
        ("archives", ScreenKind::Archives),
        ("quests", ScreenKind::Quests),
        ("net", ScreenKind::Net),
    ] {
        driver.line(line);
        let by_command = shown(&driver).unwrap();
        // The events of a step are glossed, and so is a screen: the two are equal as data.
        assert_eq!(driver.game.screen(kind), Some(by_command), "{line}");
    }
}

#[test]
fn drawing_a_screen_changes_nothing() {
    let driver = open_hub();
    let before = driver.game.state.clone();
    for kind in [
        ScreenKind::Shop,
        ScreenKind::Services,
        ScreenKind::Contacts,
        ScreenKind::Mail,
        ScreenKind::Archives,
        ScreenKind::Quests,
        ScreenKind::Net,
    ] {
        let _ = driver.game.screen(kind);
    }
    assert_eq!(driver.game.state, before);
}

#[test]
fn a_screen_follows_the_game() {
    let mut driver = open_hub();
    let ghost_row = |driver: &Driver| -> String {
        let table = driver.game.screen(ScreenKind::Shop).unwrap();
        let cat = catalog(Lang::En);
        table
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| render(cell, &cat, RenderMode::FULL))
                    .collect::<Vec<_>>()
                    .join(" - ")
            })
            .find(|row| row.contains("Ghost Protocol"))
            .expect("the shop lists Ghost Protocol")
    };
    assert!(
        ghost_row(&driver).ends_with("available"),
        "{}",
        ghost_row(&driver)
    );
    driver.lines(&["buy ghost", "y"]);
    assert!(
        ghost_row(&driver).ends_with("owned"),
        "{}",
        ghost_row(&driver)
    );
}

#[test]
fn there_is_no_screen_before_the_opening_is_over() {
    let driver = Driver::raw();
    assert!(driver.game.screen(ScreenKind::Shop).is_none());
}
