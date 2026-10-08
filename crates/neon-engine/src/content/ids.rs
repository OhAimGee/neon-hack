//! Stable textual identifiers of the content, one type per namespace.
//!
//! Ids are plain text in the data files and in saves. Every type shares the alphabet and the
//! length of [`crate::ids`] (`[a-z0-9_-]{1,48}`) and is checked when it is created or
//! deserialized, so a `QuestId` can never be passed where a `SiteId` is expected and a
//! malformed id never gets into the engine. [`ContactId`] is the very type the frontends use
//! to name a speaker. The content validator is stricter than the alphabet (see [`valid_id`]).

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub use crate::ids::{ContactId, IdError};

macro_rules! id_type {
    ($($(#[$meta:meta])* $name:ident),* $(,)?) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Arc<str>);

        impl $name {
            /// Creates an identifier, rejecting anything outside `[a-z0-9_-]{1,48}`.
            ///
            /// # Errors
            ///
            /// Returns an [`IdError`] saying why the text is not a valid identifier.
            pub fn new(id: &str) -> Result<Self, IdError> {
                crate::ids::check(id)?;
                Ok(Self(Arc::from(id)))
            }

            /// The identifier as text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(id: &str) -> Result<Self, Self::Err> {
                Self::new(id)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(id: String) -> Result<Self, Self::Error> {
                Self::new(&id)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0.as_ref().to_owned()
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

/// Whether `id` has the shape the content requires, `[a-z0-9][a-z0-9_-]{0,47}`: the id
/// alphabet, and no leading `-` or `_`. It also applies to the names that are not typed ids
/// (flag values, objective ids, command groups).
#[must_use]
pub fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    first_ok
        && id.len() <= 48
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests;
