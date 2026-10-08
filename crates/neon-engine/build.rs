//! Embeds the text catalogs (`data/text/<lang>/*.toml`) and the glossary (`data/glossary.toml`)
//! into the crate.
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

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").ok_or("no manifest dir")?);
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("no out dir")?);
    let text_dir = manifest_dir.join("../../data/text");
    println!("cargo:rerun-if-changed={}", text_dir.display());
    let glossary = manifest_dir.join("../../data/glossary.toml");
    println!("cargo:rerun-if-changed={}", glossary.display());

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
    Ok(())
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
