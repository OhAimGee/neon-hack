//! Where saves live on disk: files, backups, quarantine and atomic writes.
//!
//! The engine only produces and reads save text (`neon_engine::save`). This module owns the
//! files, following `docs/design/architecture-rust.md` section 7.4-7.5 and `DECISIONS.md` R-7:
//!
//! ```text
//! <dir>/auto.toml            the autosave, rewritten after every action that changes the game
//! <dir>/auto.toml.bak        the previous VALID autosave, kept before each write
//! <dir>/auto.toml.corrupt    an unreadable file, set aside instead of overwritten
//! <dir>/checkpoint-1.toml    the newest checkpoint (kept: 3), never touched by the autosave
//! <dir>/slot-1.toml          manual slots (9), with the same .bak and .corrupt rules
//! ```

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use neon_engine::save::{
    CHECKPOINT_COUNT, MAX_SAVE_BYTES, SLOT_COUNT, SaveError, SaveMeta, SaveRequest, peek,
};
use thiserror::Error;

/// A save file the player can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Auto,
    /// 1 is the newest.
    Checkpoint(u8),
    Slot(u8),
}

impl Target {
    /// Every target, in the order a list shows them.
    fn all() -> impl Iterator<Item = Self> {
        std::iter::once(Self::Auto)
            .chain((1..=CHECKPOINT_COUNT).map(Self::Checkpoint))
            .chain((1..=SLOT_COUNT).map(Self::Slot))
    }

    /// The name shown to the player and used on the command line: `auto`, `checkpoint-2`,
    /// `slot-3`.
    pub(crate) fn label(self) -> String {
        match self {
            Self::Auto => "auto".to_owned(),
            Self::Checkpoint(number) => format!("checkpoint-{number}"),
            Self::Slot(number) => format!("slot-{number}"),
        }
    }

    /// Reads a label back. `checkpoint` alone is the newest checkpoint.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let invalid = || {
            format!(
                "`{text}` is not a save: use auto, checkpoint-1 to checkpoint-{CHECKPOINT_COUNT}, \
                 or slot-1 to slot-{SLOT_COUNT}"
            )
        };
        let numbered = |prefix: &str, max: u8| {
            let number = text.strip_prefix(prefix)?.parse::<u8>().ok()?;
            (1..=max).contains(&number).then_some(number)
        };
        if text == "auto" {
            Ok(Self::Auto)
        } else if text == "checkpoint" {
            Ok(Self::Checkpoint(1))
        } else if let Some(number) = numbered("checkpoint-", CHECKPOINT_COUNT) {
            Ok(Self::Checkpoint(number))
        } else if let Some(number) = numbered("slot-", SLOT_COUNT) {
            Ok(Self::Slot(number))
        } else {
            Err(invalid())
        }
    }
}

/// Why a save could not be written or read.
#[derive(Debug, Error)]
pub(crate) enum StoreError {
    #[error("{}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("{} cannot be read ({reason}) and no valid previous copy exists", path.display())]
    Unreadable { path: PathBuf, reason: String },
    #[error("there is no slot {0}")]
    NoSuchSlot(u8),
}

/// A game read from disk.
pub(crate) struct Loaded<T> {
    pub(crate) value: T,
    /// The latest file was damaged and the previous copy was used instead.
    pub(crate) from_backup: bool,
}

/// One line of the list of saves.
pub(crate) struct Listing {
    pub(crate) target: Target,
    /// What the file holds, or why it cannot be read.
    pub(crate) meta: Result<SaveMeta, String>,
}

/// The saves folder.
pub(crate) struct Store {
    dir: PathBuf,
}

impl Store {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// The saves folder, shown to the player so that they can find their files.
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, target: Target) -> PathBuf {
        self.dir.join(format!("{}.toml", target.label()))
    }

    /// Writes a save for the engine's request.
    ///
    /// The autosave and the slots keep the previous valid file as `.bak` and set a damaged
    /// one aside as `.corrupt`; a checkpoint pushes the older ones down the history.
    pub(crate) fn write(&self, request: SaveRequest, text: &str) -> Result<(), StoreError> {
        fs::create_dir_all(&self.dir).map_err(|source| io_error(&self.dir, source))?;
        match request {
            SaveRequest::Autosave => self.write_current(Target::Auto, text),
            SaveRequest::Slot(number) if (1..=SLOT_COUNT).contains(&number) => {
                self.write_current(Target::Slot(number), text)
            }
            SaveRequest::Slot(number) => Err(StoreError::NoSuchSlot(number)),
            SaveRequest::Checkpoint => {
                for number in (1..CHECKPOINT_COUNT).rev() {
                    let from = self.path(Target::Checkpoint(number));
                    let to = self.path(Target::Checkpoint(number + 1));
                    match rename_replace(&from, &to) {
                        Ok(()) => {}
                        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                        Err(source) => return Err(io_error(&from, source)),
                    }
                }
                let path = self.path(Target::Checkpoint(1));
                write_atomic(&path, text.as_bytes(), || Ok(()))
                    .map_err(|source| io_error(&path, source))
            }
        }
    }

    fn write_current(&self, target: Target, text: &str) -> Result<(), StoreError> {
        let path = self.path(target);
        match read_limited(&path) {
            Ok(None) => {}
            Ok(Some(old)) if peek(&old).is_ok() => {
                let backup = sibling(&path, "bak");
                write_atomic(&backup, old.as_bytes(), || Ok(()))
                    .map_err(|source| io_error(&backup, source))?;
            }
            // Unreadable, too large, or not a save: keep it, but out of the way.
            Ok(Some(_)) | Err(_) => {
                let corrupt = sibling(&path, "corrupt");
                rename_replace(&path, &corrupt).map_err(|source| io_error(&path, source))?;
            }
        }
        write_atomic(&path, text.as_bytes(), || Ok(())).map_err(|source| io_error(&path, source))
    }

    /// The text of the latest file of a save, without any fallback to its backup: `None` when
    /// there is no such file or it cannot be read.
    pub(crate) fn primary_text(&self, target: Target) -> Option<String> {
        read_limited(&self.path(target)).ok().flatten()
    }

    /// Reads a save. `Ok(None)` means there is none. A damaged file falls back to its
    /// `.bak`; when that fails too the error says so and nothing is touched.
    pub(crate) fn read<T>(
        &self,
        target: Target,
        parse: impl Fn(&str) -> Result<T, SaveError>,
    ) -> Result<Option<Loaded<T>>, StoreError> {
        let path = self.path(target);
        let try_file = |file: &Path| -> Result<Option<T>, String> {
            match read_limited(file) {
                Ok(None) => Ok(None),
                Ok(Some(text)) => parse(&text).map(Some).map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        };
        let reason = match try_file(&path) {
            Ok(Some(value)) => {
                return Ok(Some(Loaded {
                    value,
                    from_backup: false,
                }));
            }
            Ok(None) => None,
            Err(reason) => Some(reason),
        };
        match try_file(&sibling(&path, "bak")) {
            Ok(Some(value)) => Ok(Some(Loaded {
                value,
                from_backup: true,
            })),
            Ok(None) | Err(_) => match reason {
                None => Ok(None),
                Some(reason) => Err(StoreError::Unreadable { path, reason }),
            },
        }
    }

    /// Every save that exists, without loading any game: only the header is read, and a
    /// broken file is listed as such.
    pub(crate) fn list(&self) -> Vec<Listing> {
        Target::all()
            .filter_map(|target| {
                let meta = match read_limited(&self.path(target)) {
                    Ok(None) => return None,
                    Ok(Some(text)) => peek(&text).map_err(|error| error.to_string()),
                    Err(error) => Err(error.to_string()),
                };
                Some(Listing { target, meta })
            })
            .collect()
    }
}

fn io_error(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// `auto.toml` and `bak` give `auto.toml.bak`.
fn sibling(path: &Path, extension: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(extension);
    PathBuf::from(name)
}

/// The text of a file, `None` when it does not exist. A file over the size limit is refused
/// before it is read into memory.
fn read_limited(path: &Path) -> io::Result<Option<String>> {
    let size = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if size > MAX_SAVE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the file is too large for a save",
        ));
    }
    fs::read_to_string(path).map(Some)
}

/// Replaces `path` with `data` so that a reader sees the old file or the new one, never a
/// mix, and a failure leaves the old file untouched. `before_rename` lets a test inject a
/// failure at the worst moment.
fn write_atomic(
    path: &Path,
    data: &[u8],
    before_rename: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    // Same folder, so the same file system, so the rename is atomic.
    let temporary = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        before_rename()?;
        rename_replace(&temporary, path)
    })();
    if result.is_err() {
        // Best effort: the error being reported is the original one.
        let _ = fs::remove_file(&temporary);
        return result;
    }
    sync_dir(dir);
    Ok(())
}

/// Renames over an existing file. On Windows a destination held open by another program
/// (an antivirus, an indexer) fails with `PermissionDenied` for a moment: try again a few
/// times. That loop is only compiled on Windows and has no test.
fn rename_replace(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        let mut delay = std::time::Duration::from_millis(10);
        for _ in 0..5 {
            match fs::rename(from, to) {
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    std::thread::sleep(delay);
                    delay *= 2;
                }
                result => return result,
            }
        }
    }
    fs::rename(from, to)
}

/// Makes the rename itself durable. Not available on Windows, where it is skipped.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(dir) {
        // A failure here only costs durability after a power cut, not correctness.
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

#[cfg(test)]
mod tests;
