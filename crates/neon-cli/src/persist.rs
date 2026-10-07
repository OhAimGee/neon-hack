//! Writing the saves the engine asks for, and telling the player when that goes wrong.
//!
//! Both frontends call [`Persistence::after_step`] after every step. The engine decides
//! *when* and *what kind* of save (`Step::save`); this decides where it goes and what is
//! worth saying: nothing when an automatic save works, a line when a manual one works, and
//! an error that is shown once for automatic saves (the game goes on) but every time for a
//! manual one, because the player asked for it.

use neon_engine::event::Event;
use neon_engine::save::SaveRequest;
use neon_engine::text::Text;
use neon_engine::{Game, Step};

use crate::store::Store;

pub(crate) struct Persistence {
    store: Option<Store>,
    warned: bool,
}

impl Persistence {
    /// Writes into `store`.
    pub(crate) fn new(store: Store) -> Self {
        Self {
            store: Some(store),
            warned: false,
        }
    }

    /// Writes nothing (`--no-save`, and the frontends' tests).
    pub(crate) fn disabled() -> Self {
        Self {
            store: None,
            warned: false,
        }
    }

    /// Honours the save the step asks for, if any, and adds to the step the line the
    /// player should read about it.
    pub(crate) fn after_step(&mut self, game: &dyn Game, step: &mut Step) {
        if let Some(request) = step.save
            && let Some(notice) = self.persist(game, request)
        {
            step.events.push(notice);
        }
    }

    fn persist(&mut self, game: &dyn Game, request: SaveRequest) -> Option<Event> {
        let manual = matches!(request, SaveRequest::Slot(_));
        let Some(store) = &self.store else {
            return manual.then(|| Event::error(Text::new("ui.save.disabled")));
        };
        let written = game
            .snapshot()
            .map_err(|error| error.to_string())
            .and_then(|text| {
                store
                    .write(request, &text)
                    .map_err(|error| error.to_string())
            });
        match (written, request) {
            (Ok(()), SaveRequest::Slot(slot)) => Some(Event::system(
                Text::new("ui.save.slot_done").with_int("slot", i64::from(slot)),
            )),
            (Ok(()), _) => None,
            (Err(reason), _) => {
                let first_failure = !std::mem::replace(&mut self.warned, true);
                (first_failure || manual)
                    .then(|| Event::error(Text::new("ui.save.failed").with_str("reason", reason)))
            }
        }
    }
}

#[cfg(test)]
mod tests;
