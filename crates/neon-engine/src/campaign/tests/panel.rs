//! What the side panel lists: the sections of the view, their rows and their tones, and the
//! commands that open for a full-screen frontend.

use super::support::*;
use crate::command::Capabilities;
use crate::game::{Game, PanelSection, Tone, View};
use crate::text::{Lang, RenderMode, render};

fn words(text: &crate::text::Text, lang: Lang) -> String {
    render(text, &catalog(lang), RenderMode::FULL)
}

fn titles(view: &View) -> Vec<String> {
    view.sections
        .iter()
        .map(|section| words(&section.title, Lang::En))
        .collect()
}

fn section<'a>(view: &'a View, title: &str) -> &'a PanelSection {
    view.sections
        .iter()
        .find(|section| words(&section.title, Lang::En) == title)
        .unwrap_or_else(|| panic!("no section {title}: {:?}", titles(view)))
}

fn rows(section: &PanelSection, lang: Lang) -> Vec<String> {
    section
        .rows
        .iter()
        .map(|row| words(&row.text, lang))
        .collect()
}

#[test]
fn nothing_is_listed_before_the_handle_is_known() {
    let driver = Driver::raw();
    assert!(driver.game.view().sections.is_empty());
}

#[test]
fn a_new_game_lists_the_quest_the_net_and_the_purse() {
    let driver = Driver::new();
    let view = driver.game.view();
    assert_eq!(titles(&view), ["Quests", "The net", "Inventory"]);

    let quests = rows(section(&view, "Quests"), Lang::En);
    assert_eq!(quests.first().map(String::as_str), Some("First Steps"));
    assert!(
        quests
            .iter()
            .any(|row| row == "- Open the journal (quests)"),
        "{quests:?}"
    );
    assert_eq!(
        view.sections.first().map(|s| s.rows[0].tone),
        Some(Tone::Strong)
    );

    let net = rows(section(&view, "The net"), Lang::En);
    assert_eq!(net, ["localhost: known"]);

    let inventory = rows(section(&view, "Inventory"), Lang::En);
    assert_eq!(
        inventory,
        [
            format!("Credits: {}", driver.credits()),
            "No item yet.".to_owned()
        ]
    );
}

#[test]
fn the_panel_is_in_french_too() {
    let driver = Driver::new();
    let view = driver.game.view();
    let titles: Vec<String> = view
        .sections
        .iter()
        .map(|section| words(&section.title, Lang::Fr))
        .collect();
    assert_eq!(titles, ["Quêtes", "Le réseau", "Inventaire"]);
    let net = rows(&view.sections[1], Lang::Fr);
    assert_eq!(net, ["localhost : connu"]);
}

#[test]
fn a_pierced_site_is_good_news_and_what_was_bought_is_listed() {
    let mut driver = Driver::new();
    driver.finish_m01();
    driver.lines(&["buy 1", "y"]);
    let view = driver.game.view();
    let net = section(&view, "The net");
    let localhost = net.rows.first().expect("localhost is listed");
    assert_eq!(words(&localhost.text, Lang::En), "localhost: pierced");
    assert_eq!(localhost.tone, Tone::Good);
    let inventory = rows(section(&view, "Inventory"), Lang::En);
    assert!(
        inventory.iter().any(|row| row == "Ghost Protocol"),
        "{inventory:?}"
    );
    assert!(!inventory.iter().any(|row| row == "No item yet."));
}

/// The rows of the section called `title`, or none when the panel has no such section.
fn rows_of(view: &View, title: &str) -> Vec<String> {
    view.sections
        .iter()
        .find(|section| words(&section.title, Lang::En) == title)
        .map(|section| rows(section, Lang::En))
        .unwrap_or_default()
}

#[test]
fn what_waits_to_be_read_is_listed_until_it_is_read() {
    let mut driver = Driver::new();
    assert!(rows_of(&driver.game.view(), "To read").is_empty());
    driver.finish_m01();
    let view = driver.game.view();
    let waiting = rows_of(&view, "To read").join(" / ");
    assert!(waiting.contains("unread message"), "{waiting}");
    assert!(
        section(&view, "To read")
            .rows
            .iter()
            .all(|row| row.tone == Tone::Warning)
    );
    let inbox = driver.game.state.inbox.clone();
    for id in inbox {
        driver.line(&format!("read {id}"));
    }
    let waiting = rows_of(&driver.game.view(), "To read").join(" / ");
    assert!(!waiting.contains("unread message"), "{waiting}");
}

#[test]
fn the_view_is_the_same_for_both_languages_and_changes_nothing() {
    let driver = Driver::new();
    let before = driver.game.state.clone();
    let view = driver.game.view();
    let _ = driver.game.view();
    assert_eq!(driver.game.state, before);
    assert_eq!(view, driver.game.view());
}

#[test]
fn panel_is_offered_to_the_full_screen_interface_only() {
    let mut driver = Driver::new();
    assert!(driver.game.complete("pan").is_empty());
    assert!(driver.line("panel").contains("Only in full screen."));
    assert!(driver.line("plain").contains("Only in full screen."));
    driver.game.set_capabilities(Capabilities::TUI);
    assert_eq!(driver.game.complete("pan"), ["panel"]);
    // `plain` is the full-screen interface's too: it hands the game to the line-by-line one.
    assert_eq!(driver.game.complete("pl"), ["plain"]);
    // Help lists it, and says what it is for.
    let said = driver.line("help panel");
    assert!(said.contains("Show or hide the side panel"), "{said}");
    assert!(!driver.errored(), "{said}");
}
