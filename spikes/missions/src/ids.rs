//! Stable textual identifiers, one type per namespace.
//!
//! Ids are plain text in data files and saves. Shape validation (`[a-z0-9][a-z0-9_-]{0,47}`)
//! is done by the content validator, not at deserialization, so that every bad id of a file
//! is reported with its line instead of stopping at the first one.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($($(#[$meta:meta])* $name:ident),* $(,)?) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps a string without checking its shape.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The id as text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self::new(id)
            }
        }
    )*};
}

id_type!(
    /// A quest (`m05`, `s12`).
    QuestId,
    /// A macro-map site (`nexus-mainframe`).
    SiteId,
    /// A loot file of a site (`neural_maps`).
    FileId,
    /// A contact (`r4z0r`).
    ContactId,
    /// A shop item or quest item (`stealth_module`).
    ItemId,
    /// A fragment, mail, scene or encrypted document (`f08`, `doc_ledger`).
    ReadableId,
    /// A typed narrative flag (`d2`, `phoenix_state`).
    FlagId,
    /// A player decision (`d1`).
    DecisionId,
    /// One option of a decision (`betray`).
    ChoiceId,
    /// A dialogue topic of a contact.
    TopicId,
    /// A command the player can have used (`scan`).
    CommandId,
    /// An ending (`e1a`).
    EndingId,
    /// A line of the epilogue montage.
    LineId,
);

/// Whether `id` has the shape `[a-z0-9][a-z0-9_-]{0,47}`.
pub fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    first_ok
        && id.len() <= 48
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}
