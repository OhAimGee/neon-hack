//! The command table of the campaign (docs/spec/commands.md, sections 2.1 to 2.2).
//!
//! One table, read by dispatch, help, completion and the content validator. The commands of
//! the tactical run (`deck`, `map`, `breach`...) belong to lot R4 and are not declared yet.

use crate::command::{
    ArgKind, ArgSpec, Capabilities, CommandSpec, Context, HandledBy, Registry, Scope,
};

/// The game is at the hub: there is no run to be in until lot R4.
pub(super) const CONTEXT: Context = Context::Hub;

/// The campaign does not know which frontend runs it: it answers as a plain one would.
pub(super) const FRONTEND: Capabilities = Capabilities::PLAIN;

/// The names `help <command>` accepts: every command of the table, in table order (a test
/// keeps it equal to [`SPECS`]).
pub(super) const NAMES: &[&str] = &[
    "help", "status", "quit", "load", "panel", "plain", "export", "quests", "accept", "contacts",
    "talk", "messages", "read", "archives", "decrypt", "net", "hack", "shop", "buy", "laylow",
    "hint", "save", "tutorial",
];

const HELP_ARGS: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Word(NAMES))];
const QUEST_OPTIONAL: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Quest)];
const QUEST: &[ArgSpec] = &[ArgSpec::required(ArgKind::Quest)];
const CONTACT: &[ArgSpec] = &[ArgSpec::required(ArgKind::Contact)];
const MESSAGE: &[ArgSpec] = &[ArgSpec::required(ArgKind::Message)];
const DOCUMENT_OPTIONAL: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Document)];
const DOCUMENT: &[ArgSpec] = &[ArgSpec::required(ArgKind::Document)];
const SITE_OPTIONAL: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Site)];
const SITE: &[ArgSpec] = &[ArgSpec::required(ArgKind::Site)];
const ITEM: &[ArgSpec] = &[ArgSpec::required(ArgKind::Item)];
const SERVICE_OPTIONAL: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Service)];
const SLOT_OPTIONAL: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Slot)];
const PATH_OPTIONAL: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Path)];
/// The two things `tutorial` can be told besides its bare form.
pub(super) const TUTORIAL_WORDS: &[&str] = &["skip", "restart"];
const TUTORIAL_ARGS: &[ArgSpec] = &[ArgSpec::optional(ArgKind::Word(TUTORIAL_WORDS))];

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

const fn frontend(
    name: &'static str,
    help: &'static str,
    context: Context,
    scope: Scope,
    args: &'static [ArgSpec],
) -> CommandSpec {
    CommandSpec {
        name,
        aliases: &[],
        help,
        context,
        handled_by: HandledBy::Frontend(scope),
        args,
    }
}

/// Every command of the campaign. Commands that only the frontend can run (`load`, `panel`,
/// `plain`, `export`) are declared here too, so that they are listed, completed and checked
/// like the others.
pub(super) const SPECS: &[CommandSpec] = &[
    engine(
        "help",
        &["h"],
        "campaign.help.help",
        Context::Anywhere,
        HELP_ARGS,
    ),
    engine(
        "status",
        &["st"],
        "campaign.help.status",
        Context::Anywhere,
        &[],
    ),
    engine(
        "quit",
        &["exit"],
        "campaign.help.quit",
        Context::Anywhere,
        &[],
    ),
    frontend(
        "load",
        "campaign.help.load",
        Context::Hub,
        Scope::AnyFrontend,
        PATH_OPTIONAL,
    ),
    frontend(
        "panel",
        "campaign.help.panel",
        Context::Anywhere,
        Scope::TuiOnly,
        &[],
    ),
    frontend(
        "plain",
        "campaign.help.plain",
        Context::Anywhere,
        Scope::TuiOnly,
        &[],
    ),
    frontend(
        "export",
        "campaign.help.export",
        Context::Anywhere,
        Scope::AnyFrontend,
        PATH_OPTIONAL,
    ),
    engine(
        "quests",
        &["journal", "q"],
        "campaign.help.quests",
        Context::Hub,
        QUEST_OPTIONAL,
    ),
    engine("accept", &[], "campaign.help.accept", Context::Hub, QUEST),
    engine("contacts", &[], "campaign.help.contacts", Context::Hub, &[]),
    engine(
        "talk",
        &["contact"],
        "campaign.help.talk",
        Context::Hub,
        CONTACT,
    ),
    engine(
        "messages",
        &["inbox"],
        "campaign.help.messages",
        Context::Hub,
        &[],
    ),
    engine("read", &[], "campaign.help.read", Context::Hub, MESSAGE),
    engine(
        "archives",
        &[],
        "campaign.help.archives",
        Context::Hub,
        DOCUMENT_OPTIONAL,
    ),
    engine(
        "decrypt",
        &[],
        "campaign.help.decrypt",
        Context::Hub,
        DOCUMENT,
    ),
    engine(
        "net",
        &["sites"],
        "campaign.help.net",
        Context::Hub,
        SITE_OPTIONAL,
    ),
    engine("hack", &[], "campaign.help.hack", Context::Hub, SITE),
    engine("shop", &[], "campaign.help.shop", Context::Hub, &[]),
    engine("buy", &[], "campaign.help.buy", Context::Hub, ITEM),
    engine(
        "laylow",
        &[],
        "campaign.help.laylow",
        Context::Hub,
        SERVICE_OPTIONAL,
    ),
    engine("hint", &[], "campaign.help.hint", Context::Hub, &[]),
    engine(
        "save",
        &[],
        "campaign.help.save",
        Context::Hub,
        SLOT_OPTIONAL,
    ),
    engine(
        "tutorial",
        &[],
        "tutorial.help",
        Context::Hub,
        TUTORIAL_ARGS,
    ),
];

/// The table, as the registry that dispatch, help and completion read.
pub(super) const COMMANDS: Registry = Registry::new(SPECS);

/// The official names of the commands of the campaign, in table order: what the content
/// validator checks the rules of `unlocks.toml` against.
pub fn command_names() -> impl Iterator<Item = &'static str> {
    SPECS.iter().map(|spec| spec.name)
}
