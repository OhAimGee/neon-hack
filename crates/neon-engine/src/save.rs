//! The save format: a versioned TOML envelope around the state of a game.
//!
//! The engine never touches the disk. It turns a game into text ([`encode`]) and text back
//! into a game ([`decode`]); the frontend decides where the text goes (see
//! `docs/design/architecture-rust.md`, section 7, and `DECISIONS.md` R-7).
//!
//! ```toml
//! version = 1
//!
//! [meta]    # what a list of saves shows, readable without the game
//! player = "Neon"
//! turn = 12
//!
//! [state]   # the game's own state
//! ...
//!
//! [end]     # always the last table: a truncated file cannot load
//! ok = true
//! ```
//!
//! Rules of evolution: a new optional field is `#[serde(default)]` and needs no new
//! version; an unknown key is ignored; any structural change raises [`SAVE_VERSION`] and
//! adds a [`Migration`], tested against a frozen file of the previous version. A file from
//! a newer game is refused with [`SaveError::TooNew`], never loaded approximately.

use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The version this build writes.
pub const SAVE_VERSION: u32 = 1;

/// Largest save accepted. A real campaign is a few dozen KiB; anything bigger is refused
/// before it is read into memory.
pub const MAX_SAVE_BYTES: u64 = 1 << 20;

/// How many manual slots exist.
pub const SLOT_COUNT: u8 = 9;

/// How many automatic checkpoints are kept, the newest first.
pub const CHECKPOINT_COUNT: u8 = 3;

/// Why the engine wants the game persisted now. The frontend picks the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveRequest {
    /// The current autosave, rewritten after every action that changes the game.
    Autosave,
    /// A point the player can come back to (before an irreversible choice). Checkpoints
    /// are kept in a short history that the autosave never overwrites.
    Checkpoint,
    /// A manual slot, `1..=`[`SLOT_COUNT`].
    Slot(u8),
}

/// What a list of saves shows without loading the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMeta {
    /// The player's name.
    pub player: String,
    /// How many actions the player has taken (the game's clock).
    pub turn: u32,
}

/// The state of a game that can be saved.
pub trait SaveState: Serialize + DeserializeOwned {
    /// Rejects a state the game could never have reached, so that an edited or damaged
    /// file cannot put the engine in an impossible situation.
    ///
    /// # Errors
    ///
    /// The reason, in English, for developers (it is not a catalog text).
    fn validate(&self) -> Result<(), String>;
}

/// A step from one version of the format to the next, on the untyped tree.
pub type Migration = fn(&mut toml::Table) -> Result<(), String>;

/// The migrations of this game: entry `n` turns version `n + 1` into version `n + 2`.
/// Empty while version 1 is the only one.
pub const MIGRATIONS: &[Migration] = &[];

/// Why a save could not be written or read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SaveError {
    /// The text is not valid TOML.
    #[error("not a valid save file: {0}")]
    Syntax(String),
    /// The version is missing, not a whole number, or below 1.
    #[error("the save has no usable version")]
    BadVersion,
    /// The file was written by a newer game.
    #[error("the save comes from a newer game (version {found}, this one reads up to {supported})")]
    TooNew {
        /// Version in the file.
        found: u32,
        /// Newest version this build reads.
        supported: u32,
    },
    /// The file is bigger than [`MAX_SAVE_BYTES`].
    #[error("the save is too large")]
    TooLarge,
    /// The file stops before its `[end]` marker.
    #[error("the save is incomplete")]
    Incomplete,
    /// A migration failed.
    #[error("cannot upgrade the save from version {from}: {reason}")]
    Migration {
        /// Version being upgraded.
        from: u32,
        /// What went wrong.
        reason: String,
    },
    /// The file does not have the shape of a save of this game.
    #[error("the save does not match the game: {0}")]
    Schema(String),
    /// The values are not a state the game can be in.
    #[error("the save holds an impossible state: {0}")]
    Invalid(String),
}

#[derive(Serialize, Deserialize)]
struct Envelope<S> {
    version: u32,
    meta: SaveMeta,
    state: S,
    end: End,
}

#[derive(Serialize, Deserialize)]
struct End {
    ok: bool,
}

/// Writes a game as save text.
///
/// # Errors
///
/// [`SaveError::Schema`] if the state cannot be written as TOML (a programming error).
pub fn encode<S: SaveState>(meta: &SaveMeta, state: &S) -> Result<String, SaveError> {
    let envelope = Envelope {
        version: SAVE_VERSION,
        meta: meta.clone(),
        state,
        end: End { ok: true },
    };
    toml::to_string(&envelope).map_err(|error| SaveError::Schema(error.to_string()))
}

/// Reads save text back into the meta and the state, upgrading an older version and
/// validating the state.
///
/// # Errors
///
/// A [`SaveError`] saying what is wrong; nothing is loaded in that case.
pub fn decode<S: SaveState>(text: &str) -> Result<(SaveMeta, S), SaveError> {
    decode_with(text, SAVE_VERSION, MIGRATIONS)
}

/// Like [`decode`], for a format at `current` version with its `migrations`.
///
/// # Errors
///
/// A [`SaveError`] saying what is wrong.
pub fn decode_with<S: SaveState>(
    text: &str,
    current: u32,
    migrations: &[Migration],
) -> Result<(SaveMeta, S), SaveError> {
    let envelope: Envelope<S> = read_envelope(text, current, migrations)?;
    envelope.state.validate().map_err(SaveError::Invalid)?;
    Ok((envelope.meta, envelope.state))
}

/// Reads only what a list of saves needs, checking everything about the file except the
/// shape of the game's own state.
///
/// # Errors
///
/// A [`SaveError`] saying what is wrong.
pub fn peek(text: &str) -> Result<SaveMeta, SaveError> {
    let envelope: Envelope<IgnoredAny> = read_envelope(text, SAVE_VERSION, MIGRATIONS)?;
    Ok(envelope.meta)
}

fn read_envelope<S: DeserializeOwned>(
    text: &str,
    current: u32,
    migrations: &[Migration],
) -> Result<Envelope<S>, SaveError> {
    if u64::try_from(text.len()).map_or(true, |size| size > MAX_SAVE_BYTES) {
        return Err(SaveError::TooLarge);
    }
    let mut table: toml::Table = text.parse().map_err(|error: toml::de::Error| {
        SaveError::Syntax(error.to_string().trim_end().to_owned())
    })?;
    let found = match table.get("version") {
        Some(toml::Value::Integer(version)) if *version >= 1 => {
            u32::try_from(*version).unwrap_or(u32::MAX)
        }
        _ => return Err(SaveError::BadVersion),
    };
    if found > current {
        return Err(SaveError::TooNew {
            found,
            supported: current,
        });
    }
    for from in found..current {
        let index = usize::try_from(from - 1).unwrap_or(usize::MAX);
        let Some(migrate) = migrations.get(index) else {
            return Err(SaveError::Migration {
                from,
                reason: "no migration is defined for this version".to_owned(),
            });
        };
        migrate(&mut table).map_err(|reason| SaveError::Migration { from, reason })?;
        table.insert(
            "version".to_owned(),
            toml::Value::Integer(i64::from(from) + 1),
        );
    }
    let complete = matches!(
        table.get("end"),
        Some(toml::Value::Table(end)) if end.get("ok") == Some(&toml::Value::Boolean(true))
    );
    if !complete {
        return Err(SaveError::Incomplete);
    }
    toml::Value::Table(table)
        .try_into()
        .map_err(|error: toml::de::Error| {
            SaveError::Schema(error.to_string().trim_end().to_owned())
        })
}

/// Serializes a `u64` as 16 hexadecimal digits. TOML integers are signed 64-bit, and a
/// `u64` above `i64::MAX` cannot be read back through the untyped tree that migrations use.
pub mod hex_u64 {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    /// Writes the value as 16 hex digits.
    ///
    /// # Errors
    ///
    /// Whatever the serializer reports.
    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{value:016x}"))
    }

    /// Reads 16 hex digits.
    ///
    /// # Errors
    ///
    /// A deserialization error when the text is not exactly 16 hex digits.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.len() != 16 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(D::Error::custom("expected 16 hexadecimal digits"));
        }
        u64::from_str_radix(&text, 16).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Toy {
        credits: i32,
        #[serde(default)]
        note: String,
        #[serde(with = "hex_u64")]
        seed: u64,
        items: Vec<Item>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum Item {
        Key { id: String },
        Coin,
    }

    impl SaveState for Toy {
        fn validate(&self) -> Result<(), String> {
            if self.credits < 0 {
                return Err("negative credits".to_owned());
            }
            Ok(())
        }
    }

    fn toy() -> Toy {
        Toy {
            credits: 12,
            note: "hi".to_owned(),
            seed: u64::MAX,
            items: vec![Item::Key { id: "a".to_owned() }, Item::Coin],
        }
    }

    fn meta() -> SaveMeta {
        SaveMeta {
            player: "Neon".to_owned(),
            turn: 3,
        }
    }

    #[test]
    fn a_save_round_trips_and_has_the_documented_shape() {
        let text = encode(&meta(), &toy()).unwrap();
        assert!(text.starts_with("version = 1\n"), "{text}");
        assert!(text.trim_end().ends_with("[end]\nok = true"), "{text}");
        assert!(text.contains("seed = \"ffffffffffffffff\""), "{text}");
        let (read_meta, state) = decode::<Toy>(&text).unwrap();
        assert_eq!((read_meta, state), (meta(), toy()));
        assert_eq!(peek(&text).unwrap(), meta());
    }

    #[test]
    fn every_strict_prefix_of_a_save_is_rejected() {
        let text = encode(&meta(), &toy()).unwrap();
        let body = text.trim_end();
        for (cut, _) in body.char_indices() {
            assert!(decode::<Toy>(&body[..cut]).is_err(), "prefix {cut} loaded");
            assert!(peek(&body[..cut]).is_err(), "prefix {cut} peeked");
        }
        assert!(
            decode::<Toy>(body).is_ok(),
            "the final newline may be missing"
        );
    }

    #[test]
    fn unusable_versions_are_refused() {
        let good = encode(&meta(), &toy()).unwrap();
        let with_version = |value: &str| good.replacen("version = 1", value, 1);
        for bad in [
            "version = 0",
            "version = -3",
            "version = 1.5",
            "version = \"1\"",
            "# none",
        ] {
            assert_eq!(
                decode::<Toy>(&with_version(bad)),
                Err(SaveError::BadVersion),
                "{bad}"
            );
        }
        assert_eq!(
            decode::<Toy>(&with_version("version = 2")),
            Err(SaveError::TooNew {
                found: 2,
                supported: 1
            })
        );
        assert_eq!(
            decode::<Toy>(&with_version("version = 99999999999")),
            Err(SaveError::TooNew {
                found: u32::MAX,
                supported: 1
            })
        );
    }

    #[test]
    fn a_missing_end_marker_means_incomplete() {
        let good = encode(&meta(), &toy()).unwrap();
        let cut = good.replace("\n[end]\nok = true\n", "\n");
        assert_eq!(decode::<Toy>(&cut), Err(SaveError::Incomplete));
        let false_end = good.replace("ok = true", "ok = false");
        assert_eq!(decode::<Toy>(&false_end), Err(SaveError::Incomplete));
    }

    #[test]
    fn shape_and_state_errors_are_told_apart() {
        let good = encode(&meta(), &toy()).unwrap();
        let wrong_type = good.replace("credits = 12", "credits = \"twelve\"");
        assert!(matches!(
            decode::<Toy>(&wrong_type),
            Err(SaveError::Schema(_))
        ));
        let impossible = good.replace("credits = 12", "credits = -5");
        assert_eq!(
            decode::<Toy>(&impossible),
            Err(SaveError::Invalid("negative credits".to_owned()))
        );
        let bad_hex = good.replace("ffffffffffffffff", "zzzzzzzzzzzzzzzz");
        assert!(matches!(decode::<Toy>(&bad_hex), Err(SaveError::Schema(_))));
        assert!(matches!(
            decode::<Toy>("this is = = not toml"),
            Err(SaveError::Syntax(_))
        ));
    }

    #[test]
    fn unknown_keys_are_ignored_and_missing_optional_fields_default() {
        let good = encode(&meta(), &toy()).unwrap();
        let extra = good.replace("[end]", "future_field = 7\n\n[end]");
        assert_eq!(decode::<Toy>(&extra).unwrap().1, toy());
        let without_note = good.replace("note = \"hi\"\n", "");
        assert_eq!(decode::<Toy>(&without_note).unwrap().1.note, "");
    }

    #[test]
    fn oversized_text_is_refused_before_parsing() {
        let limit = usize::try_from(MAX_SAVE_BYTES).unwrap();
        assert_eq!(
            decode::<Toy>(&" ".repeat(limit + 1)),
            Err(SaveError::TooLarge)
        );
        // Exactly at the limit it is read (and found to be no save), not refused for size.
        assert_ne!(decode::<Toy>(&" ".repeat(limit)), Err(SaveError::TooLarge));
    }

    fn rename_credits(table: &mut toml::Table) -> Result<(), String> {
        let state = table
            .get_mut("state")
            .and_then(toml::Value::as_table_mut)
            .ok_or("no state")?;
        let money = state.remove("money").ok_or("no money")?;
        state.insert("credits".to_owned(), money);
        Ok(())
    }

    #[test]
    fn older_versions_are_migrated_in_order_on_the_untyped_tree() {
        // A version-1 file kept `credits` under the name `money`.
        let v1 = encode(&meta(), &toy())
            .unwrap()
            .replace("credits = 12", "money = 12");
        let (_, state) = decode_with::<Toy>(&v1, 2, &[rename_credits]).unwrap();
        assert_eq!(state, toy());

        let failing = |_: &mut toml::Table| Err("boom".to_owned());
        assert_eq!(
            decode_with::<Toy>(&v1, 2, &[failing]),
            Err(SaveError::Migration {
                from: 1,
                reason: "boom".to_owned()
            })
        );
        assert_eq!(
            decode_with::<Toy>(&v1, 3, &[rename_credits]),
            Err(SaveError::Migration {
                from: 2,
                reason: "no migration is defined for this version".to_owned()
            })
        );
    }

    #[test]
    fn the_migration_table_covers_every_published_version() {
        assert_eq!(
            MIGRATIONS.len(),
            usize::try_from(SAVE_VERSION - 1).unwrap(),
            "raising SAVE_VERSION needs a migration and a frozen file of the old version"
        );
    }
}
