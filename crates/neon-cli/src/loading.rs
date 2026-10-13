//! The `load` command, run by the frontends.
//!
//! Like `options` it is the interface's: it reads the saves folder, which the game knows
//! nothing about. Alone it lists the saves; with a name it asks first (a game under way is
//! replaced, and what was played since the last save is lost), and on a yes it hands the
//! text of the save to [`Game::load`], which replaces the game only if the save loads.
//!
//! The question is a real one: while it waits, [`Loading::question`] is the prompt the
//! frontend shows and answers, with the same yes-or-no keys as any other question, `Escape`
//! included. Both frontends share this module.

use neon_engine::event::{Event, Table};
use neon_engine::save::{CHECKPOINT_COUNT, SLOT_COUNT, SaveMeta, peek};
use neon_engine::text::Text;
use neon_engine::{Game, Input, Prompt, Step};

use crate::persist::Persistence;
use crate::store::{Store, Target};

/// What was typed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Request {
    /// `load`: what saves there are.
    List,
    /// `load <save>`: replace the game by this one.
    Load(String),
    /// `load` with more than a name.
    Usage,
}

/// Reads a line: `None` when it is not `load`, whatever the case of the word.
pub(crate) fn request(line: &str) -> Option<Request> {
    let mut words = line.split_whitespace();
    if !words.next()?.eq_ignore_ascii_case("load") {
        return None;
    }
    let name = words.next();
    Some(match (name, words.next()) {
        (None, _) => Request::List,
        (Some(name), None) => Request::Load(name.to_ascii_lowercase()),
        (Some(_), Some(_)) => Request::Usage,
    })
}

/// A save that was found and is waiting for a yes.
struct Pending {
    save: String,
    text: String,
    meta: SaveMeta,
}

/// What came of an answer.
pub(crate) enum Answer {
    /// Something to say; the game goes on as it was.
    Said(Vec<Event>),
    /// The game was replaced: this is the step to carry on from, already told that it was.
    Loaded(Step),
    /// The input closed while the question was asked: nothing is pending and the end of the
    /// input is for the game.
    Closed,
}

#[derive(Default)]
pub(crate) struct Loading {
    pending: Option<Pending>,
}

impl Loading {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Whether a question waits for its answer. The line-by-line interface asks
    /// [`Loading::question`] instead; the full-screen one needs to know before it routes a key.
    #[cfg(any(feature = "tui", test))]
    pub(crate) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// The question, as the prompt to show instead of the game's, while there is one.
    pub(crate) fn question(&self) -> Option<Prompt> {
        let pending = self.pending.as_ref()?;
        Some(Prompt::Confirm {
            question: Text::new("ui.load.confirm")
                .with_str("save", pending.save.clone())
                .with_str("player", pending.meta.player.clone())
                .with_int("turn", i64::from(pending.meta.turn)),
            default: false,
        })
    }

    /// Runs the command. A save that can be loaded makes the question pending, after a warning
    /// that the game under way goes; everything else is said at once.
    pub(crate) fn run(&mut self, request: Request, persistence: &Persistence) -> Vec<Event> {
        let Some(store) = persistence.store() else {
            return vec![Event::error(Text::new("ui.save.disabled"))];
        };
        match request {
            Request::List => list(store),
            Request::Usage => vec![Event::error(Text::new("ui.load.usage"))],
            Request::Load(name) => self.ask(store, &name),
        }
    }

    fn ask(&mut self, store: &Store, name: &str) -> Vec<Event> {
        let Ok(target) = Target::parse(name) else {
            return vec![Event::error(
                Text::new("ui.load.unknown")
                    .with_str("save", name.to_owned())
                    .with_int("checkpoints", i64::from(CHECKPOINT_COUNT))
                    .with_int("slots", i64::from(SLOT_COUNT)),
            )];
        };
        let found = store.read(target, |text| {
            peek(text).map(|meta| (text.to_owned(), meta))
        });
        match found {
            Ok(Some(loaded)) => {
                let (text, meta) = loaded.value;
                self.pending = Some(Pending {
                    save: target.label(),
                    text,
                    meta,
                });
                // Said once, in the log; the question itself stays short enough to be read
                // whole on the line it is asked on.
                vec![Event::system(Text::new("ui.load.warning"))]
            }
            Ok(None) => vec![Event::error(
                Text::new("ui.save.missing").with_str("save", target.label()),
            )],
            Err(error) => vec![Event::error(
                Text::new("ui.load.unreadable").with_str("details", error.to_string()),
            )],
        }
    }

    /// Takes the answer to the question. The game is replaced only on a yes and only if the
    /// save loads; whatever happens the question is over.
    pub(crate) fn answer(&mut self, input: &Input, game: &mut dyn Game) -> Answer {
        let Some(pending) = self.pending.take() else {
            return Answer::Said(Vec::new());
        };
        match input {
            Input::Confirm(true) => match game.load(&pending.text) {
                Ok(mut step) => {
                    step.events.insert(
                        0,
                        Event::system(Text::new("ui.load.done").with_str("save", pending.save)),
                    );
                    Answer::Loaded(step)
                }
                Err(error) => Answer::Said(vec![Event::error(
                    Text::new("ui.load.failed").with_str("reason", error.to_string()),
                )]),
            },
            Input::Confirm(false) | Input::Cancel => {
                Answer::Said(vec![Event::system(Text::new("ui.load.cancelled"))])
            }
            Input::Eof => Answer::Closed,
            _ => {
                // Not an answer to a question: ask again.
                self.pending = Some(pending);
                Answer::Said(vec![Event::error(Text::new("ui.invalid_confirm"))])
            }
        }
    }
}

/// The saves there are, with who and how far.
fn list(store: &Store) -> Vec<Event> {
    let listing = store.list();
    if listing.is_empty() {
        return vec![Event::system(Text::new("ui.save.none"))];
    }
    let rows = listing
        .into_iter()
        .map(|entry| {
            let save = Text::raw(entry.target.label());
            match entry.meta {
                Ok(meta) => vec![
                    save,
                    Text::raw(meta.player),
                    Text::raw(meta.turn.to_string()),
                ],
                Err(reason) => vec![
                    save,
                    Text::new("ui.load.damaged").with_str("reason", reason),
                    Text::raw("-"),
                ],
            }
        })
        .collect();
    vec![
        Event::Screen(Table::new(
            Text::new("ui.load.title"),
            ["ui.load.col_save", "ui.load.col_player", "ui.load.col_turn"]
                .iter()
                .map(|key| Text::new(key))
                .collect(),
            rows,
        )),
        Event::system(Text::new("ui.load.hint")),
    ]
}

#[cfg(test)]
mod tests;
