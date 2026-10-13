//! The commands of an intrusion (`docs/spec/commands.md` § 2.3).
//!
//! [`RUN_SPECS`] are the commands that exist only during an intrusion; the campaign adds them
//! to its own table (one table, three contexts), and a game of one intrusion adds `help`,
//! `status` and `quit` to them. `probe` (it needs a fog of war the rules do not have yet) is
//! not in it.

use crate::command::{ArgKind, ArgSpec, CommandSpec, Context, HandledBy, Registry, concat};

/// The context of a game of one intrusion.
pub(super) const CONTEXT: Context = Context::Run;

const HELP_ARGS: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Word(NAMES))];
const NODE: &[ArgSpec] = &[ArgSpec::required(ArgKind::Node)];
const BREACH_ARGS: &[ArgSpec] = &[
    ArgSpec::required(ArgKind::Node),
    ArgSpec::required(ArgKind::Program),
];
const PROGRAM: &[ArgSpec] = &[ArgSpec::required(ArgKind::Program)];

/// The names `help <command>` accepts in a game of one intrusion, in table order (a test keeps
/// it equal to [`SPECS`]).
pub(super) const NAMES: &[&str] = &[
    "help", "status", "map", "move", "breach", "use", "loot", "end", "jackout", "undo", "skip",
    "quit",
];

const fn engine(
    name: &'static str,
    aliases: &'static [&'static str],
    help: &'static str,
    context: Context,
    args: &'static [ArgSpec],
) -> CommandSpec {
    CommandSpec {
        name,
        aliases,
        help,
        context,
        handled_by: HandledBy::Engine,
        args,
    }
}

/// The commands that exist only during an intrusion.
pub(crate) const RUN_SPECS: &[CommandSpec] = &[
    engine("map", &[], "run.help.map", Context::Run, &[]),
    engine("move", &["go"], "run.help.move", Context::Run, NODE),
    engine("breach", &[], "run.help.breach", Context::Run, BREACH_ARGS),
    engine("use", &[], "run.help.use", Context::Run, PROGRAM),
    engine("loot", &[], "run.help.loot", Context::Run, &[]),
    engine("end", &[], "run.help.end", Context::Run, &[]),
    engine("jackout", &[], "run.help.jackout", Context::Run, &[]),
    engine("undo", &[], "run.help.undo", Context::Run, &[]),
    engine("skip", &[], "run.help.skip", Context::Run, &[]),
];

const HEAD: &[CommandSpec] = &[
    engine(
        "help",
        &["h"],
        "run.help.help",
        Context::Anywhere,
        HELP_ARGS,
    ),
    engine("status", &["st"], "run.help.status", Context::Anywhere, &[]),
];

const TAIL: &[CommandSpec] = &[engine(
    "quit",
    &["exit"],
    "run.help.quit",
    Context::Anywhere,
    &[],
)];

const FRONT: [CommandSpec; 11] = concat(HEAD, RUN_SPECS);

/// Every command of a game of one intrusion.
pub(super) const SPECS: [CommandSpec; 12] = concat(&FRONT, TAIL);

/// The table, as the registry that dispatch, help and completion read.
pub(super) const COMMANDS: Registry = Registry::new(&SPECS);
