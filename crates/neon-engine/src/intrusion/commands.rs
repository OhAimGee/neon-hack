//! The commands of an intrusion (`docs/spec/commands.md` § 2.3).
//!
//! One table, read by dispatch, help and completion. `probe` (it needs a fog of war the rules
//! do not have yet) and `skip` (the automatic intrusion belongs to the campaign) are not in it.

use crate::command::{ArgKind, ArgSpec, CommandSpec, Context, HandledBy, Registry};

/// The context of the game while an intrusion is under way.
pub(super) const CONTEXT: Context = Context::Run;

/// The names `help <command>` accepts: every command of the table, in table order (a test
/// keeps it equal to [`SPECS`]).
pub(super) const NAMES: &[&str] = &[
    "help", "status", "map", "move", "breach", "use", "loot", "end", "jackout", "undo", "quit",
];

const HELP_ARGS: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Word(NAMES))];
const NODE: &[ArgSpec] = &[ArgSpec::required(ArgKind::Node)];
const BREACH_ARGS: &[ArgSpec] = &[
    ArgSpec::required(ArgKind::Node),
    ArgSpec::required(ArgKind::Program),
];
const PROGRAM: &[ArgSpec] = &[ArgSpec::required(ArgKind::Program)];

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

/// Every command of an intrusion.
pub(super) const SPECS: &[CommandSpec] = &[
    engine("help", &["h"], "run.help.help", Context::Anywhere, HELP_ARGS),
    engine("status", &["st"], "run.help.status", Context::Anywhere, &[]),
    engine("map", &[], "run.help.map", Context::Run, &[]),
    engine("move", &["go"], "run.help.move", Context::Run, NODE),
    engine("breach", &[], "run.help.breach", Context::Run, BREACH_ARGS),
    engine("use", &[], "run.help.use", Context::Run, PROGRAM),
    engine("loot", &[], "run.help.loot", Context::Run, &[]),
    engine("end", &[], "run.help.end", Context::Run, &[]),
    engine("jackout", &[], "run.help.jackout", Context::Run, &[]),
    engine("undo", &[], "run.help.undo", Context::Run, &[]),
    engine("quit", &["exit"], "run.help.quit", Context::Anywhere, &[]),
];

/// The table, as the registry that dispatch, help and completion read.
pub(super) const COMMANDS: Registry = Registry::new(SPECS);
