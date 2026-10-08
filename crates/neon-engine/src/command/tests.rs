use proptest::prelude::*;

use super::*;
use crate::text::{Catalog, Lang, RenderMode, render};

// ---- Fixtures -------------------------------------------------------------------------------

const fn plain(
    name: &'static str,
    aliases: &'static [&'static str],
    help: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        aliases,
        help,
        context: Context::Anywhere,
        handled_by: HandledBy::Engine,
        args: &[],
    }
}

/// A table of names and aliases only: the registry of R1.4a.
const SPECS: &[CommandSpec] = &[
    plain("exploit", &["pwn"], "help.exploit"),
    plain("help", &["h"], "help.help"),
    plain("quit", &["exit", "bye"], "help.quit"),
    plain("scan", &["ls"], "help.scan"),
    plain("shop", &["buy"], "help.shop"),
];
const REGISTRY: Registry = Registry::new(SPECS);

const BROKEN: &[CommandSpec] = &[
    plain("Scan", &["s c"], "h"),
    plain("go", &["go", ""], "h"),
    plain("go", &[], "h"),
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

const BRIEF_NORMAL_FULL: &[&str] = &["brief", "normal", "full"];
const CALM_COOL: &[&str] = &["calm", "cool"];

/// A small full game: three contexts, frontend commands, every kind of argument.
const GAME_SPECS: &[CommandSpec] = &[
    plain("help", &["h"], "help.help"),
    CommandSpec {
        args: &[ArgSpec::optional(ArgKind::Word(BRIEF_NORMAL_FULL))],
        ..plain("verbosity", &[], "help.verbosity")
    },
    CommandSpec {
        args: &[ArgSpec::optional(ArgKind::Word(CALM_COOL))],
        ..plain("mood", &[], "help.mood")
    },
    CommandSpec {
        handled_by: HandledBy::Frontend(Scope::TuiOnly),
        ..plain("panel", &[], "help.panel")
    },
    CommandSpec {
        handled_by: HandledBy::Frontend(Scope::AnyFrontend),
        args: &[ArgSpec::optional(ArgKind::Path)],
        ..plain("export", &[], "help.export")
    },
    CommandSpec {
        context: Context::Hub,
        handled_by: HandledBy::Frontend(Scope::AnyFrontend),
        args: &[ArgSpec::optional(ArgKind::Slot)],
        ..plain("load", &[], "help.load")
    },
    CommandSpec {
        context: Context::Hub,
        args: &[ArgSpec::optional(ArgKind::Quest)],
        ..plain("quests", &["journal", "q"], "help.quests")
    },
    CommandSpec {
        context: Context::Hub,
        args: &[ArgSpec::required(ArgKind::Item)],
        ..plain("buy", &[], "help.buy")
    },
    CommandSpec {
        context: Context::Hub,
        args: &[ArgSpec::optional(ArgKind::Number)],
        ..plain("wait", &[], "help.wait")
    },
    CommandSpec {
        context: Context::Hub,
        ..plain("hack", &[], "help.hack")
    },
    CommandSpec {
        context: Context::Run,
        ..plain("map", &[], "help.map")
    },
    CommandSpec {
        context: Context::Run,
        args: &[ArgSpec::required(ArgKind::Node)],
        ..plain("move", &["go"], "help.move")
    },
    CommandSpec {
        context: Context::Run,
        args: &[
            ArgSpec::required(ArgKind::Node),
            ArgSpec::required(ArgKind::Program),
        ],
        ..plain("breach", &[], "help.breach")
    },
    CommandSpec {
        context: Context::Run,
        args: &[
            ArgSpec::required(ArgKind::Program),
            ArgSpec::optional(ArgKind::Node),
        ],
        ..plain("use", &[], "help.use")
    },
    CommandSpec {
        context: Context::Run,
        ..plain("jackout", &[], "help.jackout")
    },
];
const GAME: Registry = Registry::new(GAME_SPECS);

/// The game's things. Rows never leave their list: an owned or unaffordable item stays,
/// closed, under the same number.
#[derive(Debug, Clone)]
struct Fake {
    credits: u32,
    items: Vec<(&'static str, u32, bool)>,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            credits: 100,
            items: vec![
                ("proxy", 30, false),
                ("cloak", 60, false),
                ("deck", 120, false),
            ],
        }
    }
}

impl Resolver for Fake {
    fn list(&self, kind: ArgKind, command: &CommandSpec) -> Listing {
        let rows = match kind {
            ArgKind::Node => vec![
                Row::open("n.gate", &["gateway", "gw"]),
                Row::open("n.vault", &["vault"]),
                Row::closed("n.wall", &["firewall"], Text::new("reason.adjacent")),
            ],
            ArgKind::Program => vec![
                Row::open("p.brute", &["brute"]),
                Row::open("p.spoof", &["spoof"]),
                // Stealth cannot be used, but it can be named by `breach`, which refuses it
                // with its own reason: availability is per command.
                if command.name == "use" {
                    Row::closed("p.stealth", &["stealth"], Text::new("reason.cycles"))
                } else {
                    Row::open("p.stealth", &["stealth"])
                },
                Row::open("p.eclair", &["Éclair", "flash"]),
            ],
            ArgKind::Item => self
                .items
                .iter()
                .map(|(id, price, owned)| {
                    if *owned {
                        Row::closed(*id, &[*id], Text::new("reason.owned"))
                    } else if *price > self.credits {
                        Row::closed(*id, &[*id], Text::new("reason.price"))
                    } else {
                        Row::open(*id, &[*id])
                    }
                })
                .collect(),
            ArgKind::Slot => (1..=3)
                .map(|n| Row::open(format!("slot.{n}"), &[format!("slot{n}").as_str()]))
                .collect(),
            _ => Vec::new(),
        };
        let see = match kind {
            ArgKind::Node => "map",
            ArgKind::Program => "deck",
            ArgKind::Item => "shop",
            ArgKind::Slot => "load",
            _ => "quests",
        };
        Listing { see, rows }
    }
}

fn lookup(line: &str, context: Context, capabilities: Capabilities) -> Lookup {
    GAME.parse(line, context, capabilities, open, &Fake::default())
}

fn run(line: &str) -> Lookup {
    lookup(line, Context::Run, Capabilities::TUI)
}

fn hub(line: &str) -> Lookup {
    lookup(line, Context::Hub, Capabilities::TUI)
}

fn name_of(lookup: &Lookup) -> Option<&'static str> {
    match lookup {
        Lookup::Found { spec, .. }
        | Lookup::Locked { spec, .. }
        | Lookup::WrongContext { spec, .. }
        | Lookup::BadArg { spec, .. } => Some(spec.name),
        Lookup::Empty | Lookup::Unknown(_) => None,
    }
}

fn found_ids(lookup: &Lookup) -> Vec<String> {
    let Lookup::Found { args, .. } = lookup else {
        panic!("expected a command, got {lookup:?}");
    };
    args.iter()
        .map(|arg| match arg {
            ArgRef::Listed { id, .. } => id.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn bad_arg(lookup: Lookup) -> ArgError {
    let Lookup::BadArg { error, .. } = lookup else {
        panic!("expected a bad argument, got {lookup:?}");
    };
    error
}

/// Plain registry parse, for the tests that predate arguments.
fn parse_basic(line: &str, availability: fn(&CommandSpec) -> Availability) -> Lookup {
    REGISTRY.parse(
        line,
        Context::Hub,
        Capabilities::PLAIN,
        availability,
        &NoLists,
    )
}

fn complete_basic(prefix: &str, availability: fn(&CommandSpec) -> Availability) -> Vec<String> {
    REGISTRY.complete(prefix, Context::Hub, Capabilities::PLAIN, availability)
}

fn listed_names(
    registry: &Registry,
    context: Context,
    capabilities: Capabilities,
    availability: impl Fn(&CommandSpec) -> Availability,
) -> Vec<String> {
    let Event::Screen(table) = registry.help(
        Text::new("t"),
        &[Text::new("c1"), Text::new("c2")],
        context,
        capabilities,
        availability,
    ) else {
        panic!("help is a screen");
    };
    table
        .rows
        .iter()
        .map(|row| match &row[0].args[..] {
            [(_, crate::text::Arg::Str(name))] => name.clone(),
            other => panic!("not a raw name: {other:?}"),
        })
        .collect()
}

// ---- Names, aliases, locked and unknown (R1.4a) ---------------------------------------------

#[test]
fn a_name_or_an_alias_finds_the_command_whatever_its_case() {
    for typed in ["scan", "SCAN", "Scan", "  scan  ", "ls", "LS"] {
        assert_eq!(
            name_of(&parse_basic(typed, open)),
            Some("scan"),
            "{typed:?}"
        );
    }
    assert_eq!(name_of(&parse_basic("exit", open)), Some("quit"));
    assert_eq!(name_of(&parse_basic("H", open)), Some("help"));
}

#[test]
fn empty_and_unknown_lines_are_told_apart_from_each_other_and_from_locked_ones() {
    assert_eq!(parse_basic("", open), Lookup::Empty);
    assert_eq!(parse_basic("   \t", open), Lookup::Empty);
    assert_eq!(
        parse_basic("Teleport now", open),
        Lookup::Unknown("teleport".to_owned())
    );
    let Lookup::Locked { spec, reason } = parse_basic("PWN target", tiered) else {
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
    assert_eq!(complete_basic("s", open), ["scan", "shop"]);
    assert_eq!(complete_basic("SH", open), ["shop"]);
    assert_eq!(
        complete_basic("", open),
        ["exploit", "help", "quit", "scan", "shop"]
    );
    assert!(complete_basic("zz", open).is_empty());
}

#[test]
fn an_alias_is_completed_only_when_no_official_name_fits() {
    assert_eq!(
        complete_basic("e", open),
        ["exploit"],
        "an official name wins over `exit`"
    );
    assert_eq!(complete_basic("ex", open), ["exploit"]);
    assert_eq!(
        complete_basic("exi", open),
        ["exit"],
        "no name fits, so the alias does"
    );
    assert_eq!(
        complete_basic("b", open),
        ["bye", "buy"],
        "aliases in table order"
    );
}

#[test]
fn a_locked_command_is_never_completed_nor_listed() {
    assert_eq!(
        complete_basic("e", tiered),
        ["exit"],
        "only the alias of an open command"
    );
    assert!(complete_basic("exp", tiered).is_empty());
    assert!(
        complete_basic("pw", tiered).is_empty(),
        "nor through its alias"
    );
    let listed = listed_names(&REGISTRY, Context::Hub, Capabilities::PLAIN, tiered);
    assert_eq!(listed, ["help", "quit", "scan", "shop"]);
}

#[test]
fn help_lists_exactly_the_open_commands_with_their_help_keys() {
    let Event::Screen(table) = REGISTRY.help(
        Text::new("t"),
        &[Text::new("c1"), Text::new("c2")],
        Context::Hub,
        Capabilities::PLAIN,
        open,
    ) else {
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

// ---- Contexts and frontends -----------------------------------------------------------------

#[test]
fn a_command_of_another_context_answers_wrong_context_not_unknown_nor_locked() {
    let Lookup::WrongContext { spec, reason } = hub("map") else {
        panic!("map is a run command");
    };
    assert_eq!(spec.name, "map");
    assert_eq!(
        reason,
        Text::new("error.wrong_context.run").with_term("run", "run")
    );

    let Lookup::WrongContext { spec, reason } = run("hack") else {
        panic!("hack is a hub command");
    };
    assert_eq!(spec.name, "hack");
    assert_eq!(
        reason,
        Text::new("error.wrong_context.hub").with_term("run", "run")
    );

    assert_eq!(
        hub("bogus"),
        Lookup::Unknown("bogus".to_owned()),
        "a name that exists nowhere stays unknown"
    );
    // Wrong context wins over locked: a locked run command at the hub reveals nothing more
    // than "only during a run".
    let locked = |_: &CommandSpec| Availability::Locked(Text::new("locked.never"));
    let answer = GAME.parse("map", Context::Hub, Capabilities::TUI, locked, &NoLists);
    assert!(matches!(answer, Lookup::WrongContext { .. }));
    let answer = GAME.parse("jackout", Context::Run, Capabilities::TUI, locked, &NoLists);
    assert!(matches!(answer, Lookup::Locked { .. }), "{answer:?}");
}

#[test]
fn a_command_of_another_context_is_neither_listed_nor_completed() {
    let at_hub = listed_names(&GAME, Context::Hub, Capabilities::TUI, open);
    assert!(at_hub.contains(&"quests".to_owned()));
    for run_only in ["map", "move", "breach", "use", "jackout"] {
        assert!(!at_hub.contains(&run_only.to_owned()), "{run_only}");
    }
    let in_run = listed_names(&GAME, Context::Run, Capabilities::TUI, open);
    assert!(in_run.contains(&"map".to_owned()));
    for hub_only in ["quests", "buy", "hack", "wait", "load"] {
        assert!(!in_run.contains(&hub_only.to_owned()), "{hub_only}");
    }
    // Anywhere commands are in both.
    for anywhere in ["help", "verbosity", "export", "panel"] {
        assert!(at_hub.contains(&anywhere.to_owned()), "{anywhere}");
        assert!(in_run.contains(&anywhere.to_owned()), "{anywhere}");
    }

    let complete = |prefix: &str, context| GAME.complete(prefix, context, Capabilities::TUI, open);
    assert_eq!(complete("jac", Context::Run), ["jackout"]);
    assert!(
        complete("jac", Context::Hub).is_empty(),
        "no `jackout` at the hub"
    );
    assert_eq!(complete("m", Context::Hub), ["mood"]);
    assert_eq!(complete("m", Context::Run), ["mood", "map", "move"]);
    // The alias of a command of another context is not completed either.
    assert_eq!(
        complete("j", Context::Hub),
        ["journal"],
        "an alias of a hub command"
    );
    assert_eq!(complete("j", Context::Run), ["jackout"]);
    assert!(complete("jo", Context::Run).is_empty());
    assert_eq!(complete("go", Context::Run), ["go"]);
    assert!(complete("go", Context::Hub).is_empty());
}

#[test]
fn locked_unknown_and_wrong_context_are_three_different_answers() {
    let locked = |spec: &CommandSpec| {
        if spec.name == "hack" {
            Availability::Locked(Text::new("locked.m01"))
        } else {
            Availability::Open
        }
    };
    let parse = |line: &str| GAME.parse(line, Context::Hub, Capabilities::TUI, locked, &NoLists);
    assert!(matches!(parse("hack"), Lookup::Locked { .. }));
    assert!(matches!(parse("map"), Lookup::WrongContext { .. }));
    assert!(matches!(parse("bogus"), Lookup::Unknown(_)));
    assert!(matches!(parse("wait"), Lookup::Found { .. }));
    // A locked command is not listed, and neither is a wrong-context one.
    assert!(
        !GAME
            .complete("", Context::Hub, Capabilities::TUI, locked)
            .contains(&"hack".to_owned())
    );
}

#[test]
fn a_tui_only_command_does_not_exist_for_the_plain_frontend() {
    let plain_run = |line: &str| lookup(line, Context::Hub, Capabilities::PLAIN);
    let Lookup::WrongContext { spec, reason } = plain_run("panel") else {
        panic!("panel is not for the plain frontend");
    };
    assert_eq!(spec.name, "panel");
    assert_eq!(reason, Text::new("error.wrong_context.tui"));
    assert!(matches!(hub("panel"), Lookup::Found { .. }));

    let plain_names = listed_names(&GAME, Context::Hub, Capabilities::PLAIN, open);
    assert!(!plain_names.contains(&"panel".to_owned()));
    assert!(plain_names.contains(&"export".to_owned()), "any frontend");
    assert!(plain_names.contains(&"load".to_owned()));
    let tui_names = listed_names(&GAME, Context::Hub, Capabilities::TUI, open);
    assert!(tui_names.contains(&"panel".to_owned()));
    assert_eq!(tui_names.len(), plain_names.len() + 1);

    assert!(
        GAME.complete("pa", Context::Hub, Capabilities::PLAIN, open)
            .is_empty()
    );
    assert_eq!(
        GAME.complete("pa", Context::Hub, Capabilities::TUI, open),
        ["panel"]
    );
}

#[test]
fn what_help_lists_runs_and_what_runs_is_in_help() {
    for context in [Context::Hub, Context::Run] {
        for capabilities in [Capabilities::PLAIN, Capabilities::TUI] {
            let listed = listed_names(&GAME, context, capabilities, open);
            for spec in GAME.specs() {
                let answer = lookup(spec.name, context, capabilities);
                let runs = matches!(answer, Lookup::Found { .. } | Lookup::BadArg { .. });
                assert_eq!(
                    runs,
                    listed.contains(&spec.name.to_owned()),
                    "{} in {context:?} / {capabilities:?}: {answer:?}",
                    spec.name
                );
                // Every alias runs the same command.
                for alias in spec.aliases {
                    assert_eq!(
                        name_of(&lookup(alias, context, capabilities)),
                        Some(spec.name)
                    );
                }
            }
        }
    }
}

// ---- Arguments ------------------------------------------------------------------------------

#[test]
fn arguments_are_resolved_before_the_command_sees_them() {
    assert_eq!(found_ids(&run("breach 2 spoof")), ["n.vault", "p.spoof"]);
    assert_eq!(found_ids(&run("breach gate 1")), ["n.gate", "p.brute"]);
    assert_eq!(found_ids(&run("BREACH Vault BRU")), ["n.vault", "p.brute"]);
    let Lookup::Found { args, .. } = run("move 2") else {
        panic!("move 2");
    };
    assert_eq!(
        args,
        [ArgRef::Listed {
            kind: ArgKind::Node,
            number: 2,
            id: "n.vault".to_owned()
        }]
    );
    assert_eq!(
        name_of(&run("go gw")),
        Some("move"),
        "an alias of the command"
    );
}

#[test]
fn a_command_without_required_arguments_behaves_as_before() {
    let Lookup::Found { args, .. } = hub("quests") else {
        panic!("quests lists");
    };
    assert!(args.is_empty());
    let Lookup::Found { args, .. } = hub("hack") else {
        panic!("hack");
    };
    assert!(args.is_empty());
    // Surplus words are ignored; an optional argument is read when present.
    let Lookup::Found { args, .. } = hub("hack now please") else {
        panic!("hack");
    };
    assert!(args.is_empty());
    let Lookup::Found { args, .. } = run("use brute") else {
        panic!("use brute");
    };
    assert_eq!(args.len(), 1);
    assert_eq!(found_ids(&run("use brute gw")), ["p.brute", "n.gate"]);
    assert_eq!(found_ids(&run("use brute gw extra words")).len(), 2);
}

#[test]
fn a_missing_required_argument_points_to_the_usage_of_the_command() {
    for line in ["breach", "breach 2", "move", "move   ", "use"] {
        let error = bad_arg(run(line));
        assert!(
            matches!(error, ArgError::Missing { .. }),
            "{line:?}: {error:?}"
        );
        assert_eq!(error.text().key, "error.arg.missing");
    }
    let ArgError::Missing { usage } = bad_arg(run("breach")) else {
        panic!("breach needs arguments");
    };
    assert_eq!(usage, Text::dynamic("help.breach.usage".to_owned()));
    let ArgError::Missing { usage } = bad_arg(hub("buy")) else {
        panic!("buy needs an item");
    };
    assert_eq!(usage, Text::dynamic("help.buy.usage".to_owned()));
}

#[test]
fn errors_are_reported_for_the_first_bad_argument_from_the_left() {
    let error = bad_arg(run("breach 9"));
    assert!(matches!(
        error,
        ArgError::Unknown {
            kind: ArgKind::Node,
            ..
        }
    ));
    let error = bad_arg(run("breach vault zzz"));
    assert!(matches!(
        error,
        ArgError::Unknown {
            kind: ArgKind::Program,
            ..
        }
    ));
    let ArgError::Unknown { kind, word, see } = bad_arg(run("move 9")) else {
        panic!("9 is not a node");
    };
    assert_eq!(
        (kind, word.as_str(), see.as_str()),
        (ArgKind::Node, "9", "map")
    );
}

#[test]
fn an_ambiguous_word_lists_its_candidates_with_their_numbers() {
    let ArgError::Ambiguous {
        kind,
        word,
        candidates,
    } = bad_arg(run("breach vault s"))
    else {
        panic!("s fits spoof and stealth");
    };
    assert_eq!(kind, ArgKind::Program);
    assert_eq!(word, "s");
    assert_eq!(
        candidates,
        [
            Candidate {
                number: Some(2),
                name: "spoof".to_owned()
            },
            Candidate {
                number: Some(3),
                name: "stealth".to_owned()
            },
        ]
    );
}

#[test]
fn a_listed_but_refused_thing_answers_with_its_own_reason_and_keeps_its_number() {
    // Per command: stealth is fine for `breach` and refused by `use`.
    assert_eq!(
        found_ids(&run("breach vault stealth")),
        ["n.vault", "p.stealth"]
    );
    let error = bad_arg(run("use 3"));
    assert_eq!(
        error,
        ArgError::Unavailable {
            name: "stealth".to_owned(),
            reason: Text::new("reason.cycles")
        }
    );
    assert_eq!(error.text().key, "error.arg.unavailable");
    // By number, by name and by prefix, the refusal is the same.
    assert_eq!(bad_arg(run("use stealth")), error);
    assert_eq!(bad_arg(run("use ste")), error);
    // The closed node is named, not unknown.
    assert!(matches!(
        bad_arg(run("move 3")),
        ArgError::Unavailable { .. }
    ));
    assert!(matches!(
        bad_arg(run("move fire")),
        ArgError::Unavailable { .. }
    ));
}

#[test]
fn items_closed_by_credits_stay_listed_under_their_number() {
    let broke = Fake {
        credits: 40,
        ..Fake::default()
    };
    let at_hub = |line: &str| GAME.parse(line, Context::Hub, Capabilities::TUI, open, &broke);
    assert_eq!(found_ids(&at_hub("buy 1")), ["proxy"]);
    let error = bad_arg(at_hub("buy 3"));
    assert_eq!(
        error,
        ArgError::Unavailable {
            name: "deck".to_owned(),
            reason: Text::new("reason.price")
        }
    );
}

#[test]
fn a_number_argument_is_a_plain_decimal_number() {
    let number = |line: &str| match hub(line) {
        Lookup::Found { args, .. } => Ok(args),
        Lookup::BadArg { error, .. } => Err(error),
        other => panic!("{other:?}"),
    };
    assert_eq!(number("wait 12"), Ok(vec![ArgRef::Number(12)]));
    assert_eq!(number("wait 0"), Ok(vec![ArgRef::Number(0)]));
    assert_eq!(number("wait"), Ok(vec![]));
    for bad in ["x", "-1", "+1", "1.5", "99999999999999999999", "٣"] {
        assert!(
            matches!(
                number(&format!("wait {bad}")),
                Err(ArgError::Unknown {
                    kind: ArgKind::Number,
                    ..
                })
            ),
            "{bad}"
        );
    }
    let Err(error) = number("wait x") else {
        panic!("x is not a number")
    };
    let ArgError::Unknown { see, .. } = &error else {
        panic!("{error:?}")
    };
    assert_eq!(see, "help wait");
}

#[test]
fn a_word_argument_resolves_among_its_fixed_words() {
    let word = |line: &str| match hub(line) {
        Lookup::Found { args, .. } => Ok(args),
        Lookup::BadArg { error, .. } => Err(error),
        other => panic!("{other:?}"),
    };
    assert_eq!(word("verbosity full"), Ok(vec![ArgRef::Word("full")]));
    assert_eq!(word("verbosity FULL"), Ok(vec![ArgRef::Word("full")]));
    assert_eq!(
        word("verbosity b"),
        Ok(vec![ArgRef::Word("brief")]),
        "unique prefix"
    );
    assert_eq!(word("verbosity"), Ok(vec![]));
    assert!(matches!(
        word("verbosity 2"),
        Err(ArgError::Unknown {
            kind: ArgKind::Word(_),
            ..
        })
    ));
    assert!(matches!(
        word("verbosity loud"),
        Err(ArgError::Unknown { .. })
    ));
    // Two words share a prefix: ambiguous, without numbers; the exact word still wins.
    let Err(ArgError::Ambiguous { candidates, .. }) = word("mood c") else {
        panic!("c fits calm and cool");
    };
    assert_eq!(
        candidates
            .iter()
            .map(|c| (c.number, c.name.as_str()))
            .collect::<Vec<_>>(),
        [(None, "calm"), (None, "cool")]
    );
    assert_eq!(word("mood ca"), Ok(vec![ArgRef::Word("calm")]));
}

#[test]
fn a_path_takes_the_rest_of_the_line_verbatim() {
    let path = |line: &str| match hub(line) {
        Lookup::Found { args, .. } => args,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        path("export  My  Notes é.TXT  "),
        [ArgRef::Path("My  Notes é.TXT".to_owned())],
        "spaces inside kept, case and accents untouched, the edges trimmed"
    );
    assert_eq!(
        path("EXPORT /tmp/a b/c"),
        [ArgRef::Path("/tmp/a b/c".to_owned())]
    );
    assert_eq!(
        path("export 3"),
        [ArgRef::Path("3".to_owned())],
        "no number"
    );
    assert_eq!(path("export"), []);
    assert_eq!(path("export   "), []);
}

#[test]
fn a_number_wins_over_a_name_that_looks_like_it() {
    let rows = [Row::open("a", &["2"]), Row::open("b", &["1"])];
    assert_eq!(
        resolve("2", &rows),
        Resolution::Found(2),
        "row number 2, not the row called 2"
    );
    assert_eq!(
        resolve("0001", &rows),
        Resolution::Found(1),
        "leading zeros are the same number"
    );
    assert_eq!(resolve("0", &rows), Resolution::Unknown);
    assert_eq!(resolve("3", &rows), Resolution::Unknown);
}

#[test]
fn accents_and_case_are_ignored_both_ways() {
    let rows = [
        Row::open("a", &["élevé"]),
        Row::open("b", &["Passerelle"]),
        Row::open("c", &["Œuvre"]),
    ];
    for typed in ["eleve", "ELEVE", "Élevé", "élevé", "ELEVÉ", "el", "ÉL"] {
        assert_eq!(resolve(typed, &rows), Resolution::Found(1), "{typed}");
    }
    for typed in ["passerelle", "PASSERELLE", "pass", "Pàss"] {
        assert_eq!(resolve(typed, &rows), Resolution::Found(2), "{typed}");
    }
    assert_eq!(resolve("oeuvre", &rows), Resolution::Found(3));
    assert_eq!(
        resolve("e", &rows),
        Resolution::Found(1),
        "a prefix of one row only"
    );
    assert_eq!(
        resolve("", &rows),
        Resolution::Unknown,
        "nothing typed fits nothing"
    );
    assert_eq!(
        resolve("\u{301}", &rows),
        Resolution::Unknown,
        "a lone accent folds to nothing"
    );
}

#[test]
fn an_exact_name_wins_over_longer_names_that_start_with_it() {
    let rows = [Row::open("a", &["cloak"]), Row::open("b", &["cloaker"])];
    assert_eq!(resolve("cloak", &rows), Resolution::Found(1));
    assert_eq!(resolve("cloa", &rows), Resolution::Ambiguous(vec![1, 2]));
    assert_eq!(resolve("cloake", &rows), Resolution::Found(2));
    // Two rows with the same name cannot be told apart by name.
    let twins = [Row::open("a", &["x1"]), Row::open("b", &["x1"])];
    assert_eq!(resolve("x1", &twins), Resolution::Ambiguous(vec![1, 2]));
}

#[test]
fn a_slot_is_a_listed_thing() {
    assert_eq!(found_ids(&hub("load 2")), ["slot.2"]);
    assert_eq!(found_ids(&hub("load slot3")), ["slot.3"]);
    assert_eq!(found_ids(&hub("load")), Vec::<String>::new());
    assert!(matches!(
        bad_arg(hub("load 4")),
        ArgError::Unknown {
            kind: ArgKind::Slot,
            ..
        }
    ));
}

// ---- Completion of arguments ----------------------------------------------------------------

fn complete_args(line: &str, context: Context) -> Vec<String> {
    GAME.complete_args(line, context, Capabilities::TUI, open, &Fake::default())
}

#[test]
fn argument_completion_proposes_only_open_things_for_the_word_being_typed() {
    assert_eq!(
        complete_args("breach ", Context::Run),
        ["gateway", "vault"],
        "the firewall is listed but closed: never proposed"
    );
    assert_eq!(
        complete_args("use ", Context::Run),
        ["brute", "spoof", "Éclair"],
        "stealth is closed for `use`: never proposed"
    );
    assert_eq!(complete_args("use s", Context::Run), ["spoof"]);
    assert!(
        complete_args("use st", Context::Run).is_empty(),
        "closed: no spoiler"
    );
    assert_eq!(complete_args("use ECL", Context::Run), ["Éclair"]);
    assert_eq!(complete_args("use ecl", Context::Run), ["Éclair"]);
    assert_eq!(complete_args("breach vault ", Context::Run).len(), 4);
    assert_eq!(
        complete_args("breach vault s", Context::Run),
        ["spoof", "stealth"]
    );
    assert!(complete_args("breach vault zzz", Context::Run).is_empty());
}

#[test]
fn argument_completion_prefers_official_names_over_aliases() {
    // `fl` fits only the alias `flash` of the eclair program.
    assert_eq!(complete_args("use fl", Context::Run), ["flash"]);
    // `e` fits the official name Éclair: the alias is not added.
    assert_eq!(complete_args("use e", Context::Run), ["Éclair"]);
    // `g` fits the official `gateway` and not the alias `gw`.
    assert_eq!(complete_args("move g", Context::Run), ["gateway"]);
    assert_eq!(complete_args("move gw", Context::Run), ["gw"]);
}

#[test]
fn argument_completion_follows_the_position_in_the_line() {
    assert_eq!(
        complete_args("use brute ", Context::Run),
        ["gateway", "vault"]
    );
    assert!(
        complete_args("use brute gw ", Context::Run).is_empty(),
        "past the last argument"
    );
    assert!(
        complete_args("move", Context::Run).is_empty(),
        "still the command"
    );
    assert!(complete_args("", Context::Run).is_empty());
    assert!(complete_args("bogus ", Context::Run).is_empty());
    assert_eq!(complete_args("  move  g", Context::Run), ["gateway"]);
}

#[test]
fn argument_completion_follows_context_frontend_and_lock() {
    assert!(
        complete_args("move ", Context::Hub).is_empty(),
        "a run command at the hub"
    );
    assert!(complete_args("buy ", Context::Run).is_empty());
    assert_eq!(
        complete_args("buy ", Context::Hub),
        ["proxy", "cloak"],
        "deck costs too much"
    );
    let locked = |_: &CommandSpec| Availability::Locked(Text::new("locked.never"));
    assert!(
        GAME.complete_args(
            "move ",
            Context::Run,
            Capabilities::TUI,
            locked,
            &Fake::default()
        )
        .is_empty()
    );
    assert!(
        GAME.complete_args(
            "panel ",
            Context::Hub,
            Capabilities::PLAIN,
            open,
            &Fake::default()
        )
        .is_empty()
    );
}

#[test]
fn numbers_paths_and_words_are_completed_as_designed() {
    assert!(
        complete_args("wait ", Context::Hub).is_empty(),
        "numbers are not completed"
    );
    assert!(
        complete_args("export /tm", Context::Hub).is_empty(),
        "files are the frontend's"
    );
    assert!(complete_args("export a ", Context::Hub).is_empty());
    assert_eq!(
        complete_args("verbosity ", Context::Hub),
        ["brief", "normal", "full"]
    );
    assert_eq!(complete_args("verbosity N", Context::Hub), ["normal"]);
    assert_eq!(complete_args("mood c", Context::Hub), ["calm", "cool"]);
    assert_eq!(
        complete_args("load ", Context::Hub),
        ["slot1", "slot2", "slot3"]
    );
}

// ---- Table checks ---------------------------------------------------------------------------

#[test]
fn a_well_formed_table_has_no_issue_and_mistakes_are_found() {
    assert_eq!(REGISTRY.issues(), []);
    assert_eq!(GAME.issues(), []);

    let issues = Registry::new(BROKEN).issues();
    assert!(issues.contains(&Issue::BadName("Scan")));
    assert!(issues.contains(&Issue::BadName("s c")));
    assert!(issues.contains(&Issue::BadName("")));
    assert!(issues.contains(&Issue::Duplicate("go")));
}

#[test]
fn argument_declarations_are_checked() {
    const EMPTY_SET: &[&str] = &[];
    const WRONG: &[CommandSpec] = &[
        CommandSpec {
            args: &[ArgSpec::required(ArgKind::Word(EMPTY_SET))],
            ..plain("emptyset", &[], "h")
        },
        CommandSpec {
            args: &[
                ArgSpec::optional(ArgKind::Node),
                ArgSpec::required(ArgKind::Program),
            ],
            ..plain("order", &[], "h")
        },
        CommandSpec {
            args: &[
                ArgSpec::required(ArgKind::Path),
                ArgSpec::required(ArgKind::Node),
            ],
            ..plain("pathfirst", &[], "h")
        },
        CommandSpec {
            args: &[
                ArgSpec::required(ArgKind::Node),
                ArgSpec::optional(ArgKind::Path),
            ],
            ..plain("fine", &[], "h")
        },
        CommandSpec {
            args: &[
                ArgSpec::optional(ArgKind::Node),
                ArgSpec::optional(ArgKind::Program),
            ],
            ..plain("alloptional", &[], "h")
        },
    ];
    assert_eq!(
        Registry::new(WRONG).issues(),
        [
            Issue::EmptyWordSet("emptyset"),
            Issue::RequiredAfterOptional("order"),
            Issue::PathNotLast("pathfirst"),
        ]
    );
}

#[test]
fn a_run_command_is_never_listed_at_the_hub() {
    // The check uses the same predicate as the listing; the real guarantee is this loop.
    for spec in GAME.specs().iter().filter(|s| s.context == Context::Run) {
        let at_hub = listed_names(&GAME, Context::Hub, Capabilities::TUI, open);
        assert!(!at_hub.contains(&spec.name.to_owned()), "{}", spec.name);
        assert!(matches!(hub(spec.name), Lookup::WrongContext { .. }));
    }
    assert!(Context::Anywhere.serves(Context::Run));
    assert!(Context::Hub.serves(Context::Hub));
    assert!(!Context::Run.serves(Context::Hub));
    assert!(!Context::Hub.serves(Context::Run));
}

fn english_with(extra: &str) -> Catalog {
    Catalog::from_sources(Lang::En, &[("extra", extra)]).unwrap()
}

#[test]
fn the_catalog_must_have_the_help_and_the_usage_of_commands_with_arguments() {
    const SPEC: &[CommandSpec] = &[
        CommandSpec {
            context: Context::Run,
            args: &[ArgSpec::required(ArgKind::Node)],
            ..plain("breach", &[], "help.breach")
        },
        CommandSpec {
            context: Context::Run,
            ..plain("map", &[], "help.map")
        },
    ];
    let complete = english_with(
        r#"
        [help]
        "breach" = "attack"
        "breach.usage" = "Usage: breach <node> <program>"
        "map" = "show the graph"
        "#,
    );
    let registry = Registry::new(SPEC);
    assert_eq!(registry.catalog_issues(&complete), []);

    let partial = english_with("[help]\n\"breach\" = \"attack\"\n");
    assert_eq!(
        registry.catalog_issues(&partial),
        [
            CatalogIssue::MissingUsage("help.breach.usage".to_owned()),
            CatalogIssue::MissingHelp("help.map".to_owned()),
        ]
    );
    // A command with no argument needs no usage; the old tables need none at all.
    assert_eq!(
        Registry::new(SPECS).catalog_issues(&english_with(
            "[help]\nexploit=\"a\"\nhelp=\"b\"\nquit=\"c\"\nscan=\"d\"\nshop=\"e\"\n"
        )),
        []
    );
}

// ---- Error texts, as the player reads them --------------------------------------------------

const TEST_TEXTS_EN: &str = r#"
[help]
"breach.usage" = "Usage: `breach <node> <program>`. Example: `breach 3 brute`."
"buy.usage" = "Usage: `buy <item>`."

[reason]
adjacent = "not adjacent"
cycles = "not enough cycles"
price = "too expensive"
owned = "already owned"
"#;

const TEST_TEXTS_FR: &str = r#"
[help]
"breach.usage" = "Usage : `breach <nœud> <programme>`. Exemple : `breach 3 brute`."
"buy.usage" = "Usage : `buy <objet>`."

[reason]
adjacent = "nœud non adjacent"
cycles = "cycles insuffisants"
price = "crédits manquants"
owned = "déjà possédé"
"#;

fn catalog_of(lang: Lang) -> Catalog {
    let (ui, terms, extra) = match lang {
        Lang::En => (
            include_str!("../../../../data/text/en/ui.toml"),
            include_str!("../../../../data/text/en/terms.toml"),
            TEST_TEXTS_EN,
        ),
        Lang::Fr => (
            include_str!("../../../../data/text/fr/ui.toml"),
            include_str!("../../../../data/text/fr/terms.toml"),
            TEST_TEXTS_FR,
        ),
    };
    Catalog::from_sources(lang, &[("ui", ui), ("terms", terms), ("extra", extra)]).unwrap()
}

fn said(text: &Text, lang: Lang) -> String {
    render(text, &catalog_of(lang), RenderMode::FULL)
}

/// What the player reads after typing `line` in `context`.
fn answer(line: &str, context: Context, capabilities: Capabilities, lang: Lang) -> String {
    let text = match GAME.parse(line, context, capabilities, open, &Fake::default()) {
        Lookup::BadArg { error, .. } => error.text(),
        Lookup::WrongContext { reason, .. } | Lookup::Locked { reason, .. } => reason,
        Lookup::Unknown(word) => Text::new("error.unknown_command").with_str("command", word),
        other => panic!("not an error: {other:?}"),
    };
    said(&text, lang)
}

#[test]
fn the_error_texts_read_well_in_english() {
    let say = |line, context| answer(line, context, Capabilities::PLAIN, Lang::En);
    assert_eq!(
        say("bogus", Context::Hub),
        "Unknown command: bogus. See `help`."
    );
    assert_eq!(say("map", Context::Hub), "Only during a run.");
    assert_eq!(say("hack", Context::Run), "Not during a run.");
    assert_eq!(say("panel", Context::Hub), "Only in full screen.");
    assert_eq!(
        say("breach 2", Context::Run),
        "Missing argument. Usage: `breach <node> <program>`. Example: `breach 3 brute`."
    );
    assert_eq!(
        say("buy", Context::Hub),
        "Missing argument. Usage: `buy <item>`."
    );
    assert_eq!(say("move 9", Context::Run), "Unknown node: 9. See `map`.");
    assert_eq!(
        say("move nope", Context::Run),
        "Unknown node: nope. See `map`."
    );
    assert_eq!(
        say("wait x", Context::Hub),
        "Unknown number: x. See `help wait`."
    );
    assert_eq!(
        say("breach vault s", Context::Run),
        "\"s\" fits several programs: 2. spoof, 3. stealth."
    );
    assert_eq!(
        say("mood c", Context::Hub),
        "\"c\" fits several words: calm, cool."
    );
    assert_eq!(
        say("use 3", Context::Run),
        "stealth is unavailable: not enough cycles."
    );
    assert_eq!(
        say("move 3", Context::Run),
        "firewall is unavailable: not adjacent."
    );
    assert_eq!(
        say("buy 3", Context::Hub),
        "deck is unavailable: too expensive."
    );
}

#[test]
fn the_error_texts_read_well_in_french() {
    let say = |line, context| answer(line, context, Capabilities::PLAIN, Lang::Fr);
    assert_eq!(
        say("bogus", Context::Hub),
        "Commande inconnue : bogus. Voir `help`."
    );
    assert_eq!(say("map", Context::Hub), "Seulement pendant une intrusion.");
    assert_eq!(say("hack", Context::Run), "Pas pendant une intrusion.");
    assert_eq!(say("panel", Context::Hub), "Seulement en plein écran.");
    assert_eq!(
        say("breach 2", Context::Run),
        "Argument manquant. Usage : `breach <nœud> <programme>`. Exemple : `breach 3 brute`."
    );
    assert_eq!(
        say("buy", Context::Hub),
        "Argument manquant. Usage : `buy <objet>`."
    );
    assert_eq!(say("move 9", Context::Run), "Inconnu : nœud 9. Voir `map`.");
    assert_eq!(
        say("wait x", Context::Hub),
        "Inconnu : numéro x. Voir `help wait`."
    );
    assert_eq!(
        say("breach vault s", Context::Run),
        "« s » convient à plusieurs programmes : 2. spoof, 3. stealth."
    );
    assert_eq!(
        say("mood c", Context::Hub),
        "« c » convient à plusieurs mots : calm, cool."
    );
    assert_eq!(
        say("use 3", Context::Run),
        "stealth est indisponible : cycles insuffisants."
    );
    assert_eq!(
        say("buy 3", Context::Hub),
        "deck est indisponible : crédits manquants."
    );
}

#[test]
fn the_error_texts_survive_screen_reader_and_ascii_modes() {
    let text = bad_arg(run("move 9")).text();
    for mode in [RenderMode::SCREEN_READER, RenderMode::ASCII] {
        let said = render(&text, &catalog_of(Lang::Fr), mode);
        let expected = if mode.ascii {
            "Inconnu : noeud 9. Voir `map`."
        } else {
            "Inconnu : nœud 9. Voir `map`."
        };
        assert_eq!(said, expected);
    }
}

#[test]
fn every_kind_of_argument_has_a_term_in_both_languages() {
    const KINDS: [ArgKind; 13] = [
        ArgKind::Number,
        ArgKind::Quest,
        ArgKind::Contact,
        ArgKind::Message,
        ArgKind::Document,
        ArgKind::Site,
        ArgKind::Node,
        ArgKind::Program,
        ArgKind::Item,
        ArgKind::Slot,
        ArgKind::Service,
        ArgKind::Word(&["a"]),
        ArgKind::Path,
    ];
    for lang in Lang::ALL {
        let catalog = catalog_of(lang);
        for kind in KINDS {
            let key = format!("term.{}", kind.term());
            assert!(catalog.get(&key).is_some(), "{lang:?}: `{key}` is missing");
        }
    }
}

// ---- Properties -----------------------------------------------------------------------------

/// Words a player (or a fuzzer) might type: names, numbers, accents, spacing tricks.
fn word_soup() -> impl Strategy<Value = String> {
    let piece = prop_oneof![
        Just("breach".to_owned()),
        Just("use".to_owned()),
        Just("export".to_owned()),
        Just("verbosity".to_owned()),
        Just("MOVE".to_owned()),
        Just("0".to_owned()),
        Just("3".to_owned()),
        Just("99999999999999999999999".to_owned()),
        Just("é".to_owned()),
        Just("Éclair".to_owned()),
        Just("\u{a0}".to_owned()),
        Just("\t".to_owned()),
        Just("  ".to_owned()),
        Just("\u{301}".to_owned()),
        Just("日本".to_owned()),
        "[a-zA-Z0-9 ]{0,6}",
    ];
    proptest::collection::vec(piece, 0..8).prop_map(|pieces| pieces.join(" "))
}

proptest! {
    #[test]
    fn the_argument_parser_never_panics_on_any_input(line in any::<String>()) {
        for context in [Context::Hub, Context::Run, Context::Anywhere] {
            for capabilities in [Capabilities::PLAIN, Capabilities::TUI] {
                let _ = GAME.parse(&line, context, capabilities, open, &Fake::default());
                let _ = GAME.complete_args(&line, context, capabilities, open, &Fake::default());
                let _ = GAME.complete(&line, context, capabilities, open);
            }
        }
        let rows = Fake::default().list(ArgKind::Program, &GAME_SPECS[0]).rows;
        let _ = resolve(&line, &rows);
        let _ = fold(&line);
    }

    #[test]
    fn the_argument_parser_never_panics_on_command_shaped_input(line in word_soup()) {
        for context in [Context::Hub, Context::Run] {
            let parsed = GAME.parse(&line, context, Capabilities::TUI, open, &Fake::default());
            if let Lookup::BadArg { error, .. } = parsed {
                let _ = error.text();
            }
            let _ = GAME.complete_args(&line, context, Capabilities::TUI, open, &Fake::default());
        }
    }

    #[test]
    fn a_number_inside_the_list_is_found_and_outside_it_is_unknown(
        len in 0usize..12,
        number in 0usize..30,
    ) {
        let rows: Vec<Row> = (1..=len).map(|n| Row::open(format!("id{n}"), &[format!("name{n}").as_str()])).collect();
        let resolved = resolve(&number.to_string(), &rows);
        if (1..=len).contains(&number) {
            prop_assert_eq!(resolved, Resolution::Found(number));
        } else {
            prop_assert_eq!(resolved, Resolution::Unknown);
        }
    }

    #[test]
    fn an_exact_name_is_found_whatever_its_case_and_accents(
        names in proptest::collection::btree_set("[a-z]{3,8}", 1..6),
        pick in 0usize..6,
        upper in any::<bool>(),
    ) {
        // Names that are not prefixes of each other are unambiguous even by prefix, but an
        // exact name must win even when it is the prefix of another one.
        let names: Vec<String> = names.into_iter().collect();
        let rows: Vec<Row> = names.iter().map(|n| Row::open(n.as_str(), &[n.as_str()])).collect();
        let index = pick % names.len();
        let typed = if upper { names[index].to_uppercase() } else { names[index].clone() };
        prop_assert_eq!(resolve(&typed, &rows), Resolution::Found(index + 1));
    }

    #[test]
    fn a_unique_prefix_is_found_and_a_shared_one_is_ambiguous(
        stem in "[a-z]{2,5}",
        tails in proptest::collection::btree_set("[0-9]{1,3}", 2..5),
    ) {
        let tails: Vec<String> = tails.into_iter().collect();
        let rows: Vec<Row> = tails.iter().map(|t| {
            let name = format!("{stem}{t}");
            Row::open(name.clone(), &[name.as_str()])
        }).collect();
        // The stem fits every row: ambiguous, with every number in order.
        let all: Vec<usize> = (1..=rows.len()).collect();
        prop_assert_eq!(resolve(&stem, &rows), Resolution::Ambiguous(all));
        // The whole name of one row is exact; so is a prefix that only it shares.
        for (index, row) in rows.iter().enumerate() {
            prop_assert_eq!(resolve(&row.names[0], &rows), Resolution::Found(index + 1));
        }
        // Something no row starts with is unknown.
        let nothing = format!("{stem}x");
        prop_assert_eq!(resolve(&nothing, &rows), Resolution::Unknown);
    }

    #[test]
    fn folding_is_stable(text in any::<String>()) {
        let once = fold(&text);
        prop_assert_eq!(fold(&once), once.clone());
    }

    #[test]
    fn folding_ignores_case_and_accents(text in "[a-zA-ZàâäçéèêëîïôöùûüÿœÀÂÉÈÊÎÔÙÛÇ0-9 -]{0,16}") {
        prop_assert_eq!(fold(&text.to_uppercase()), fold(&text.to_lowercase()));
        prop_assert!(fold(&text).is_ascii());
    }

    #[test]
    fn a_row_is_found_through_any_of_its_names_and_counts_once(
        word in "[a-z]{2,6}",
    ) {
        let rows = vec![
            Row::open("a", &[format!("{word}1").as_str(), format!("{word}2").as_str()]),
            Row::open("b", &["zzz"]),
        ];
        // Both names of row `a` start with the word: still one row.
        prop_assert_eq!(resolve(&word, &rows), Resolution::Found(1));
    }

    #[test]
    fn rows_never_change_number_while_their_list_exists(
        operations in proptest::collection::vec(operation(), 0..60),
    ) {
        let mut game = Fake { credits: 0, items: Vec::new() };
        game.items.push(("a0", 5, false));
        // (id, number it was first shown with)
        let mut shown: Vec<(&'static str, usize)> = vec![("a0", 1)];
        let mut discovered = 1usize;
        for operation in operations {
            match operation {
                Operation::Earn(amount) => game.credits = game.credits.saturating_add(amount),
                Operation::Spend(amount) => game.credits = game.credits.saturating_sub(amount),
                Operation::Buy(index) => {
                    let len = game.items.len();
                    if let Some(item) = game.items.get_mut(index % len) {
                        item.2 = true;
                    }
                }
                Operation::Restock(index) => {
                    let len = game.items.len();
                    if let Some(item) = game.items.get_mut(index % len) {
                        item.2 = false;
                    }
                }
                Operation::Discover => {
                    // A new article is appended: it never shifts the ones already shown.
                    let id = ids()[discovered % ids().len()];
                    if !game.items.iter().any(|(known, _, _)| *known == id) {
                        let price = u32::try_from(discovered % 7).unwrap_or(0) * 10;
                        game.items.push((id, price, false));
                        shown.push((id, game.items.len()));
                    }
                    discovered += 1;
                }
            }
            let listing = game.list(ArgKind::Item, &GAME_SPECS[7]);
            prop_assert_eq!(listing.rows.len(), game.items.len());
            for (id, number) in &shown {
                // By number: the row is the same thing.
                let by_number = resolve(&number.to_string(), &listing.rows);
                prop_assert_eq!(by_number, Resolution::Found(*number));
                prop_assert_eq!(&listing.rows[number - 1].id, id);
                // By name: the same number.
                prop_assert_eq!(resolve(id, &listing.rows), Resolution::Found(*number));
                // Through the registry: either that thing, or its refusal, never another.
                let parsed = GAME.parse(&format!("buy {id}"), Context::Hub, Capabilities::TUI, open, &game);
                match parsed {
                    Lookup::Found { args, .. } => prop_assert_eq!(
                        &args,
                        &[ArgRef::Listed { kind: ArgKind::Item, number: *number, id: (*id).to_owned() }]
                    ),
                    Lookup::BadArg { error: ArgError::Unavailable { name, .. }, .. } => {
                        prop_assert_eq!(name.as_str(), *id);
                    }
                    other => prop_assert!(false, "unexpected {:?}", other),
                }
            }
        }
    }
}

fn ids() -> [&'static str; 9] {
    ["b1", "b2", "b3", "b4", "b5", "b6", "b7", "b8", "b9"]
}

#[derive(Debug, Clone)]
enum Operation {
    Earn(u32),
    Spend(u32),
    Buy(usize),
    Restock(usize),
    Discover,
}

fn operation() -> impl Strategy<Value = Operation> {
    prop_oneof![
        (0u32..200).prop_map(Operation::Earn),
        (0u32..200).prop_map(Operation::Spend),
        any::<usize>().prop_map(Operation::Buy),
        any::<usize>().prop_map(Operation::Restock),
        Just(Operation::Discover),
    ]
}
