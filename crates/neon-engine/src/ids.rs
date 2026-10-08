//! Stable textual identifiers.
//!
//! Identifiers are plain text in data files and saves, so reordering a table never
//! corrupts anything. They are validated once, when they are created or deserialized.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_LEN: usize = 48;

/// Why a string is not a valid identifier.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdError {
    /// The identifier is empty.
    #[error("identifier is empty")]
    Empty,
    /// The identifier is longer than 48 characters.
    #[error("identifier is longer than 48 characters")]
    TooLong,
    /// Only `a-z`, `0-9`, `-` and `_` are allowed.
    #[error("identifier contains the forbidden character {0:?}")]
    ForbiddenChar(char),
}

/// Checks the alphabet and the length shared by every identifier of the game, whatever its
/// namespace: `[a-z0-9_-]{1,48}`.
///
/// # Errors
///
/// Returns an [`IdError`] saying why the text is not a valid identifier.
pub(crate) fn check(id: &str) -> Result<(), IdError> {
    if id.is_empty() {
        return Err(IdError::Empty);
    }
    if id.chars().count() > MAX_LEN {
        return Err(IdError::TooLong);
    }
    let forbidden = id
        .chars()
        .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_'));
    match forbidden {
        Some(c) => Err(IdError::ForbiddenChar(c)),
        None => Ok(()),
    }
}

/// Identifier of a contact: a character the player can talk to (`echo7`, `r4z0r`...).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContactId(Arc<str>);

impl ContactId {
    /// Creates an identifier, rejecting anything outside `[a-z0-9_-]{1,48}`.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] saying why the text is not a valid identifier.
    pub fn new(id: &str) -> Result<Self, IdError> {
        check(id)?;
        Ok(Self(Arc::from(id)))
    }

    /// The identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ContactId {
    type Err = IdError;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        Self::new(id)
    }
}

impl TryFrom<String> for ContactId {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl From<ContactId> for String {
    fn from(id: ContactId) -> Self {
        id.0.as_ref().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn accepts_lowercase_digits_dash_and_underscore() {
        for id in ["echo7", "r4z0r", "shadow-broker", "data_miner"] {
            assert_eq!(ContactId::new(id).unwrap().as_str(), id);
        }
    }

    #[test]
    fn rejects_bad_identifiers_with_a_reason() {
        assert_eq!(ContactId::new(""), Err(IdError::Empty));
        assert_eq!(ContactId::new(&"a".repeat(49)), Err(IdError::TooLong));
        assert_eq!(ContactId::new("Echo7"), Err(IdError::ForbiddenChar('E')));
        assert_eq!(ContactId::new("a b"), Err(IdError::ForbiddenChar(' ')));
        assert_eq!(ContactId::new("é"), Err(IdError::ForbiddenChar('é')));
    }

    #[test]
    fn deserialization_validates_too() {
        let ok: Result<ContactId, _> = ContactId::try_from("echo7".to_owned());
        assert!(ok.is_ok());
        let bad: Result<ContactId, _> = ContactId::try_from("ECHO 7".to_owned());
        assert!(bad.is_err());
    }

    proptest! {
        #[test]
        fn valid_ids_round_trip_through_string(id in "[a-z0-9_-]{1,48}") {
            let parsed = ContactId::new(&id).unwrap();
            prop_assert_eq!(String::from(parsed), id);
        }

        #[test]
        fn any_accepted_id_matches_the_documented_alphabet(id in ".{0,60}") {
            if let Ok(parsed) = ContactId::new(&id) {
                prop_assert!(parsed.as_str().chars().all(
                    |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'
                ));
                prop_assert!(!parsed.as_str().is_empty() && parsed.as_str().chars().count() <= 48);
            }
        }
    }
}
