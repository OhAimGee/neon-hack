//! The command table and the texts the campaign points at.

use std::collections::BTreeSet;

use super::support::*;
use crate::campaign::commands::{COMMANDS, NAMES, SPECS};
use crate::campaign::resolver::Lists;
use crate::command::{ArgKind, Capabilities, Context, HandledBy, Resolver, Scope};
use crate::text::glossary::Glossary;
use crate::text::{Catalog, Lang, RenderMode, Text, render};

#[test]
fn the_table_is_well_formed_and_has_every_text_in_every_language() {
    assert_eq!(COMMANDS.issues(), []);
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        assert_eq!(COMMANDS.catalog_issues(&catalog), [], "{lang:?}");
    }
    // No run command: the tactical run is lot R4.
    assert!(SPECS.iter().all(|spec| spec.context != Context::Run));
}

#[test]
fn help_word_set_is_the_table_in_table_order() {
    let names: Vec<&str> = SPECS.iter().map(|spec| spec.name).collect();
    assert_eq!(names, NAMES);
    assert_eq!(crate::campaign::command_names().collect::<Vec<_>>(), NAMES);
}

#[test]
fn no_command_name_is_a_variant_the_glossary_forbids() {
    let glossary = Glossary::embedded().unwrap();
    for spec in SPECS {
        for name in std::iter::once(&spec.name).chain(spec.aliases) {
            for concept in glossary.concepts() {
                for lang in Lang::ALL {
                    assert!(
                        !concept
                            .forbidden(lang)
                            .any(|variant| variant.eq_ignore_ascii_case(name)),
                        "`{name}` is forbidden for `{}` in {lang:?}",
                        concept.id
                    );
                }
            }
        }
    }
}

#[test]
fn the_frontend_commands_are_declared_with_their_scope() {
    let scope = |name: &str| {
        SPECS
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.handled_by)
    };
    assert_eq!(scope("load"), Some(HandledBy::Frontend(Scope::AnyFrontend)));
    assert_eq!(
        scope("export"),
        Some(HandledBy::Frontend(Scope::AnyFrontend))
    );
    assert_eq!(scope("panel"), Some(HandledBy::Frontend(Scope::TuiOnly)));
    assert_eq!(scope("plain"), Some(HandledBy::Frontend(Scope::TuiOnly)));
    assert_eq!(scope("hack"), Some(HandledBy::Engine));
    // The plain frontend never sees the commands of the full-screen one.
    let open = COMMANDS.complete("pa", Context::Hub, Capabilities::PLAIN, |_| {
        crate::command::Availability::Open
    });
    assert!(open.is_empty(), "{open:?}");
}

#[test]
fn every_kind_of_argument_the_table_uses_has_a_resolver_that_points_back_at_a_command() {
    let mut driver = Driver::new();
    driver.finish_m02();
    let lists = Lists {
        c: driver.game.content,
        s: &driver.game.state,
    };
    let names: BTreeSet<&str> = SPECS.iter().map(|spec| spec.name).collect();
    for spec in SPECS {
        for arg in spec.args {
            if matches!(arg.kind, ArgKind::Number | ArgKind::Word(_) | ArgKind::Path) {
                continue;
            }
            let listing = lists.list(arg.kind, spec);
            let command = listing.see.split(' ').next().unwrap();
            assert!(
                names.contains(command),
                "{:?} points at `{}`",
                arg.kind,
                listing.see
            );
            assert!(
                !listing.rows.is_empty(),
                "{:?} lists nothing in a game under way",
                arg.kind
            );
            // A row can be named: every row has at least one name, in one word.
            for row in &listing.rows {
                assert!(!row.names.is_empty() && row.names.iter().all(|n| !n.contains(' ')));
            }
        }
    }
}

/// Every key written as a string literal `"campaign.…"` in the sources of the game exists in
/// both catalogs, so no text the commands can say is missing.
#[test]
fn every_campaign_key_the_code_points_at_is_in_both_catalogs() {
    let sources = [
        include_str!("../commands.rs"),
        include_str!("../dispatch.rs"),
        include_str!("../hack.rs"),
        include_str!("../hub.rs"),
        include_str!("../mod.rs"),
        include_str!("../play.rs"),
        include_str!("../resolver.rs"),
        include_str!("../talk.rs"),
        include_str!("../trade.rs"),
    ];
    let mut keys = BTreeSet::new();
    for source in sources {
        let mut rest = source;
        while let Some(start) = rest.find("\"campaign.") {
            let tail = &rest[start + 1..];
            let end = tail.find('"').unwrap();
            let key = &tail[..end];
            if key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
            {
                keys.insert(key.to_owned());
            }
            rest = &tail[end..];
        }
    }
    assert!(keys.len() > 100, "{} keys found", keys.len());
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        let missing: Vec<&String> = keys
            .iter()
            .filter(|key| catalog.get(key).is_none())
            .collect();
        assert!(missing.is_empty(), "{lang:?}: {missing:?}");
    }
}

/// The reverse: a text of `campaign.toml` that nothing points at is dead weight (the command
/// help and the dynamic families are checked apart).
#[test]
fn campaign_texts_that_no_code_uses_are_found() {
    let used = [
        include_str!("../commands.rs"),
        include_str!("../dispatch.rs"),
        include_str!("../hack.rs"),
        include_str!("../hub.rs"),
        include_str!("../mod.rs"),
        include_str!("../play.rs"),
        include_str!("../resolver.rs"),
        include_str!("../talk.rs"),
        include_str!("../trade.rs"),
    ]
    .concat();
    let catalog = catalog(Lang::En);
    // Families built from a name: states, statuses, relations and the usage of each command.
    let dynamic = [
        "campaign.contact_state.",
        "campaign.quest_status.",
        "campaign.relation.",
        "campaign.site_status.",
        "campaign.archive.",
        "campaign.message.",
        "campaign.help.",
        "campaign.col.",
    ];
    let dead: Vec<&str> = catalog
        .iter()
        .map(|(key, _)| key)
        .filter(|key| key.starts_with("campaign.") && !key.contains('@'))
        .filter(|key| !dynamic.iter().any(|prefix| key.starts_with(prefix)))
        .filter(|key| !used.contains(&format!("\"{key}\"")))
        .collect();
    assert!(dead.is_empty(), "unused texts: {dead:?}");
}

#[test]
fn the_gauge_of_the_notoriety_is_named_like_the_term() {
    for lang in Lang::ALL {
        let catalog = catalog(lang);
        let gauge = render(&Text::new("gauge.notoriety"), &catalog, RenderMode::FULL);
        let term = render(
            &Text::dynamic("term.notoriety".to_owned()),
            &catalog,
            RenderMode::FULL,
        );
        assert_eq!(gauge, term, "{lang:?}");
    }
}

#[test]
fn the_names_of_the_world_are_defined_for_every_contact_and_item() {
    let driver = Driver::new();
    let c = driver.game.content;
    for lang in Lang::ALL {
        let catalog: Catalog = catalog(lang);
        for contact in &c.contacts {
            let key = format!("contact.{}.name", contact.id);
            assert!(catalog.get(&key).is_some(), "{lang:?} {key}");
            let shown = render(&Text::dynamic(key.clone()), &catalog, RenderMode::FULL);
            assert!(
                !shown.starts_with("TODO"),
                "{lang:?} {key} is still a draft"
            );
        }
        for item in &c.items {
            let shown = render(
                &Text::dynamic(format!("item.{}.name", item.id)),
                &catalog,
                RenderMode::FULL,
            );
            assert!(
                !shown.starts_with("TODO"),
                "{lang:?} {} is still a draft",
                item.id
            );
        }
    }
}
