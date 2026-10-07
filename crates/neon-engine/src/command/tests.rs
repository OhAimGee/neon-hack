use super::*;

const SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "exploit",
        aliases: &["pwn"],
        help: "help.exploit",
    },
    CommandSpec {
        name: "help",
        aliases: &["h"],
        help: "help.help",
    },
    CommandSpec {
        name: "quit",
        aliases: &["exit", "bye"],
        help: "help.quit",
    },
    CommandSpec {
        name: "scan",
        aliases: &["ls"],
        help: "help.scan",
    },
    CommandSpec {
        name: "shop",
        aliases: &["buy"],
        help: "help.shop",
    },
];
const REGISTRY: Registry = Registry::new(SPECS);

const BROKEN: &[CommandSpec] = &[
    CommandSpec {
        name: "Scan",
        aliases: &["s c"],
        help: "h",
    },
    CommandSpec {
        name: "go",
        aliases: &["go", ""],
        help: "h",
    },
    CommandSpec {
        name: "go",
        aliases: &[],
        help: "h",
    },
];

/// `exploit` is locked until tier 2; everything else is open.
fn tiered(spec: &CommandSpec) -> Availability {
    if spec.name == "exploit" {
        Availability::Locked(Text::new("locked.tier").with_int("tier", 2))
    } else {
        Availability::Open
    }
}

fn open(_: &CommandSpec) -> Availability {
    Availability::Open
}

fn name_of(lookup: &Lookup<'_>) -> Option<&'static str> {
    match lookup {
        Lookup::Found { spec, .. } | Lookup::Locked { spec, .. } => Some(spec.name),
        Lookup::Empty | Lookup::Unknown(_) => None,
    }
}

#[test]
fn a_name_or_an_alias_finds_the_command_whatever_its_case() {
    for typed in ["scan", "SCAN", "Scan", "  scan  ", "ls", "LS"] {
        assert_eq!(
            name_of(&REGISTRY.parse(typed, open)),
            Some("scan"),
            "{typed:?}"
        );
    }
    assert_eq!(name_of(&REGISTRY.parse("exit", open)), Some("quit"));
    assert_eq!(name_of(&REGISTRY.parse("H", open)), Some("help"));
}

#[test]
fn the_words_after_the_command_are_its_arguments() {
    let Lookup::Found { spec, args } = REGISTRY.parse("shop  proxy  now", open) else {
        panic!("shop is open");
    };
    assert_eq!((spec.name, args), ("shop", vec!["proxy", "now"]));
    let Lookup::Found { args, .. } = REGISTRY.parse("scan", open) else {
        panic!("scan is open");
    };
    assert!(args.is_empty());
}

#[test]
fn empty_and_unknown_lines_are_told_apart_from_each_other_and_from_locked_ones() {
    assert_eq!(REGISTRY.parse("", open), Lookup::Empty);
    assert_eq!(REGISTRY.parse("   \t", open), Lookup::Empty);
    assert_eq!(
        REGISTRY.parse("Teleport now", open),
        Lookup::Unknown("teleport".to_owned())
    );
    let Lookup::Locked { spec, reason } = REGISTRY.parse("PWN target", tiered) else {
        panic!("exploit is locked");
    };
    assert_eq!(
        spec.name, "exploit",
        "an alias of a locked command is locked too"
    );
    assert_eq!(reason, Text::new("locked.tier").with_int("tier", 2));
}

#[test]
fn completion_offers_open_names_in_table_order() {
    assert_eq!(REGISTRY.complete("s", open), ["scan", "shop"]);
    assert_eq!(REGISTRY.complete("SH", open), ["shop"]);
    assert_eq!(
        REGISTRY.complete("", open),
        ["exploit", "help", "quit", "scan", "shop"]
    );
    assert!(REGISTRY.complete("zz", open).is_empty());
}

#[test]
fn an_alias_is_completed_only_when_no_official_name_fits() {
    assert_eq!(
        REGISTRY.complete("e", open),
        ["exploit"],
        "an official name wins over `exit`"
    );
    assert_eq!(REGISTRY.complete("ex", open), ["exploit"]);
    assert_eq!(
        REGISTRY.complete("exi", open),
        ["exit"],
        "no name fits, so the alias does"
    );
    assert_eq!(
        REGISTRY.complete("b", open),
        ["bye", "buy"],
        "aliases in table order"
    );
}

#[test]
fn a_locked_command_is_never_completed_nor_listed() {
    assert_eq!(
        REGISTRY.complete("e", tiered),
        ["exit"],
        "only the alias of an open command"
    );
    assert!(REGISTRY.complete("exp", tiered).is_empty());
    assert!(
        REGISTRY.complete("pw", tiered).is_empty(),
        "nor through its alias"
    );

    let Event::Screen(table) =
        REGISTRY.help(Text::new("t"), &[Text::new("c1"), Text::new("c2")], tiered)
    else {
        panic!("help is a screen");
    };
    let listed: Vec<&Text> = table.rows.iter().map(|row| &row[0]).collect();
    assert_eq!(listed.len(), 4);
    assert!(!listed.contains(&&Text::raw("exploit")));
}

#[test]
fn help_lists_exactly_the_open_commands_with_their_help_keys() {
    let Event::Screen(table) =
        REGISTRY.help(Text::new("t"), &[Text::new("c1"), Text::new("c2")], open)
    else {
        panic!("help is a screen");
    };
    assert_eq!(table.columns, [Text::new("c1"), Text::new("c2")]);
    assert_eq!(table.rows.len(), SPECS.len());
    for (row, spec) in table.rows.iter().zip(SPECS) {
        assert_eq!(row[0], Text::raw(spec.name));
        assert_eq!(row[1], Text::dynamic(spec.help.to_owned()));
    }
    assert_eq!(
        REGISTRY.help_keys().collect::<Vec<_>>(),
        [
            "help.exploit",
            "help.help",
            "help.quit",
            "help.scan",
            "help.shop"
        ]
    );
}

#[test]
fn a_well_formed_table_has_no_issue_and_mistakes_are_found() {
    assert_eq!(REGISTRY.issues(), []);

    let issues = Registry::new(BROKEN).issues();
    assert!(issues.contains(&Issue::BadName("Scan")));
    assert!(issues.contains(&Issue::BadName("s c")));
    assert!(issues.contains(&Issue::BadName("")));
    assert!(issues.contains(&Issue::Duplicate("go")));
}
