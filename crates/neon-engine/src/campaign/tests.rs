//! Tests of the campaign game.
//!
//! `support` drives a game with typed lines and renders what it says; the other modules test a
//! family of behaviours each: the commands one by one, the story played through the commands,
//! the saves, and properties over random lines.

mod support;

use support::*;

#[test]
fn explore() {
    let mut game = Driver::new();
    let out = game.script(&[
        "Neon",
        "help",
        "status",
        "quests",
        "contacts",
        "net",
        "talk echo7",
        "0",
        "hack localhost",
        "y",
        "laylow",
        "quests m01",
        "shop",
        "messages",
    ]);
    println!("{out}");
}
