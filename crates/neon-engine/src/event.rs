//! What the engine says: events with a semantic role, never styled text.
//!
//! Every frontend consumes the same [`Event`]s: the plain frontend writes them as text,
//! the TUI shows them in its log and panel, `neon-sim` and the tests read them as data.
//! Nothing the TUI shows is missing from the plain output, and the other way round.

use crate::ids::ContactId;
use crate::text::Text;

/// How serious an alert is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Worth knowing.
    Notice,
    /// Needs attention.
    Warning,
    /// Immediate danger.
    Danger,
}

/// Who or what is speaking, which decides how a message is presented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    /// The narrator.
    Narration,
    /// A character, rendered `NAME: line` (never by colour alone).
    Dialogue {
        /// The contact speaking.
        speaker: ContactId,
    },
    /// Neutral information from the game.
    System,
    /// A warning about the player's situation.
    Alert(Severity),
    /// A gain: credits, reputation, an item.
    Reward,
    /// A refused or invalid action.
    Error,
}

impl Role {
    /// Alerts, rewards and errors can never be hidden by a verbosity setting.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Alert(_) | Self::Reward | Self::Error)
    }
}

/// How much a message matters, used to filter by verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Importance {
    /// Always shown.
    Essential,
    /// Shown unless the player asked for brief output.
    Normal,
    /// Atmosphere, shown only at full verbosity.
    Flavor,
}

/// A value followed by the interface: a gauge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gauge {
    /// How close security is to the player.
    Trace,
    /// What the world knows of the player: the gauge of the campaign.
    Notoriety,
}

impl Gauge {
    /// Catalog key of the gauge name.
    #[must_use]
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Trace => "gauge.trace",
            Self::Notoriety => "gauge.notoriety",
        }
    }
}

/// A structured screen: a shop, a map, a mail box, a journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Title of the screen.
    pub title: Text,
    /// Column headings.
    pub columns: Vec<Text>,
    /// One row per entry, as many cells as columns.
    pub rows: Vec<Vec<Text>>,
}

/// One thing the engine says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A message with a role.
    Message {
        /// Who speaks.
        role: Role,
        /// How much it matters.
        importance: Importance,
        /// What is said.
        text: Text,
    },
    /// Pure decoration (a logo): normal importance, so brief verbosity drops it. Replaced by
    /// `alt` in ASCII mode and dropped for screen readers.
    Decor {
        /// Lines of art.
        art: &'static [&'static str],
        /// A short text standing for the art.
        alt: Text,
    },
    /// A structured screen.
    Screen(Table),
    /// A gauge moved.
    Changed {
        /// Which gauge.
        gauge: Gauge,
        /// Value before.
        from: i32,
        /// Value after.
        to: i32,
        /// Name of the level the gauge is now in (`calm`, `critical`...).
        band: Text,
    },
    /// A blank separation.
    Break,
}

impl Event {
    /// A message. Alerts, rewards and errors are always [`Importance::Essential`].
    #[must_use]
    pub fn message(role: Role, importance: Importance, text: Text) -> Self {
        let importance = if role.is_critical() {
            Importance::Essential
        } else {
            importance
        };
        Self::Message {
            role,
            importance,
            text,
        }
    }

    /// Narration of normal importance.
    #[must_use]
    pub fn narration(text: Text) -> Self {
        Self::message(Role::Narration, Importance::Normal, text)
    }

    /// Atmosphere, dropped at brief and normal verbosity.
    #[must_use]
    pub fn flavor(text: Text) -> Self {
        Self::message(Role::Narration, Importance::Flavor, text)
    }

    /// A line spoken by a contact.
    #[must_use]
    pub fn say(speaker: ContactId, text: Text) -> Self {
        Self::message(Role::Dialogue { speaker }, Importance::Normal, text)
    }

    /// Neutral information the player asked for.
    #[must_use]
    pub fn system(text: Text) -> Self {
        Self::message(Role::System, Importance::Essential, text)
    }

    /// A warning.
    #[must_use]
    pub fn alert(severity: Severity, text: Text) -> Self {
        Self::message(Role::Alert(severity), Importance::Essential, text)
    }

    /// A gain.
    #[must_use]
    pub fn reward(text: Text) -> Self {
        Self::message(Role::Reward, Importance::Essential, text)
    }

    /// A refused action.
    #[must_use]
    pub fn error(text: Text) -> Self {
        Self::message(Role::Error, Importance::Essential, text)
    }

    /// The importance that applies, whatever way the event was built: a critical message
    /// is essential even if its fields were filled in by hand.
    #[must_use]
    pub fn importance(&self) -> Importance {
        match self {
            Self::Message {
                role, importance, ..
            } if !role.is_critical() => *importance,
            // Decoration is dropped at brief verbosity, like any other non-essential content.
            Self::Decor { .. } => Importance::Normal,
            _ => Importance::Essential,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn contact() -> ContactId {
        ContactId::new("echo7").unwrap()
    }

    fn any_role() -> impl Strategy<Value = Role> {
        prop_oneof![
            Just(Role::Narration),
            Just(Role::Dialogue { speaker: contact() }),
            Just(Role::System),
            Just(Role::Alert(Severity::Notice)),
            Just(Role::Alert(Severity::Danger)),
            Just(Role::Reward),
            Just(Role::Error),
        ]
    }

    fn any_importance() -> impl Strategy<Value = Importance> {
        prop_oneof![
            Just(Importance::Essential),
            Just(Importance::Normal),
            Just(Importance::Flavor),
        ]
    }

    proptest! {
        #[test]
        fn critical_messages_are_always_essential(role in any_role(), importance in any_importance()) {
            let critical = role.is_critical();
            let event = Event::message(role.clone(), importance, Text::new("k"));
            if critical {
                prop_assert_eq!(event.importance(), Importance::Essential);
                // The constructor also stores it, so reading the field directly is safe too.
                let Event::Message { importance: stored, .. } = &event else {
                    panic!("`message` must build a message");
                };
                prop_assert_eq!(*stored, Importance::Essential);
            }
            // Even an event built by hand cannot dodge the rule.
            let by_hand = Event::Message { role, importance, text: Text::new("k") };
            if critical {
                prop_assert_eq!(by_hand.importance(), Importance::Essential);
            } else {
                prop_assert_eq!(by_hand.importance(), importance);
            }
        }
    }

    #[test]
    fn constructors_pick_the_expected_roles() {
        let text = || Text::new("k");
        assert!(matches!(
            Event::error(text()),
            Event::Message {
                role: Role::Error,
                importance: Importance::Essential,
                ..
            }
        ));
        assert!(matches!(
            Event::flavor(text()),
            Event::Message {
                role: Role::Narration,
                importance: Importance::Flavor,
                ..
            }
        ));
        assert_eq!(Event::flavor(text()).importance(), Importance::Flavor);
        assert_eq!(Event::Break.importance(), Importance::Essential);
    }

    #[test]
    fn decoration_is_not_essential() {
        let decor = Event::Decor {
            art: &["+--+"],
            alt: Text::new("k"),
        };
        assert_eq!(decor.importance(), Importance::Normal);
    }
}
