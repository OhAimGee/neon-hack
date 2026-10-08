//! Embeds the text catalogs (`data/text/<lang>/*.toml`), the glossary (`data/glossary.toml`)
//! and the campaign content (`data/world/*.toml`) into the crate.
//!
//! The engine does no I/O at run time: it receives strings. This script, which only runs
//! at build time, walks the data directory and generates the list of files with
//! `include_str!`, so a file can neither be forgotten nor read from a stray place, and
//! debug and release builds behave the same.

#![allow(
    clippy::print_stdout,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    reason = "a build script talks to Cargo on stdout and reads the data directory"
)]

use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The files of `data/world/`, as `(file stem, generated constant)`. The content loader reads
/// exactly these; a missing file or a stray one stops the build instead of being ignored.
const WORLD_FILES: &[(&str, &str)] = &[
    ("catalog", "WORLD_CATALOG"),
    ("flags", "WORLD_FLAGS"),
    ("rewards", "WORLD_REWARDS"),
    ("quests", "WORLD_QUESTS"),
    ("decisions", "WORLD_DECISIONS"),
    ("topics", "WORLD_TOPICS"),
    ("texts", "WORLD_TEXTS"),
];

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").ok_or("no manifest dir")?);
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("no out dir")?);
    let text_dir = manifest_dir.join("../../data/text");
    println!("cargo:rerun-if-changed={}", text_dir.display());
    let glossary = manifest_dir.join("../../data/glossary.toml");
    println!("cargo:rerun-if-changed={}", glossary.display());

    let world_dir = manifest_dir.join("../../data/world");
    println!("cargo:rerun-if-changed={}", world_dir.display());

    let mut files = Vec::new();
    for language in sorted_entries(&text_dir)? {
        if !language.is_dir() {
            continue;
        }
        let code = file_name(&language)?;
        for file in sorted_entries(&language)? {
            if file
                .extension()
                .is_some_and(|extension| extension == "toml")
            {
                files.push((code.clone(), file_name(&file)?, file));
            }
        }
    }

    let mut generated = String::from(
        "/// `(language code, file name, contents)` of every embedded text file.\n\
         const EMBEDDED_TEXT: &[(&str, &str, &str)] = &[\n",
    );
    for (code, name, path) in &files {
        writeln!(
            generated,
            "    ({code:?}, {name:?}, include_str!({:?})),",
            path.to_string_lossy()
        )?;
    }
    generated.push_str("];\n");
    fs::write(out_dir.join("embedded_text.rs"), generated)?;
    fs::write(
        out_dir.join("embedded_glossary.rs"),
        format!(
            "/// The contents of `data/glossary.toml`.\nconst EMBEDDED_GLOSSARY: &str = include_str!({:?});\n",
            glossary.to_string_lossy()
        ),
    )?;
    fs::write(
        out_dir.join("embedded_world.rs"),
        world_constants(&world_dir)?,
    )?;
    Ok(())
}

/// One `include_str!` constant per world file, after checking the directory holds exactly
/// the expected files.
fn world_constants(world_dir: &Path) -> Result<String, Box<dyn Error>> {
    for entry in sorted_entries(world_dir)? {
        let name = file_name(&entry)?;
        if !WORLD_FILES
            .iter()
            .any(|(stem, _)| name == format!("{stem}.toml"))
        {
            return Err(format!(
                "data/world/{name} is not a known world file: declare it in build.rs and in the loader"
            )
            .into());
        }
    }
    let mut generated = String::new();
    for (stem, constant) in WORLD_FILES {
        let path = world_dir.join(format!("{stem}.toml"));
        if !path.is_file() {
            return Err(format!("data/world/{stem}.toml is missing").into());
        }
        writeln!(
            generated,
            "/// The contents of `data/world/{stem}.toml`.\nconst {constant}: &str = include_str!({:?});",
            path.to_string_lossy()
        )?;
    }
    Ok(generated)
}

fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut entries = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries)
}

fn file_name(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("a data file name is not UTF-8")?
        .to_owned())
}
