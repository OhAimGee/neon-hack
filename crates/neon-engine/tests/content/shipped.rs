//! The shipped content is valid and its declared text keys are the derived ones.

use std::fmt::Write as _;

use neon_engine::content::texts::{
    check_budgets, check_catalog_budgets, display_width, draft_keys, missing_keys, render_catalog,
    required_keys,
};
use neon_engine::content::{Content, Sources};
use neon_engine::text::{Catalog, Lang};

use crate::common::shipped;

/// Regenerates `data/world/texts.toml` from the structure. This is a writers' tool, not a
/// test: it is ignored, and refuses to run without the variable.
/// `NEON_REGEN_WORLD_TEXTS=1 cargo test -p neon-engine --test content regenerate_the_declared_text_keys -- --ignored`
#[test]
#[ignore = "writes data/world/texts.toml"]
#[allow(
    clippy::disallowed_methods,
    reason = "the regeneration tool reads its switch from the environment and writes the data file"
)]
fn regenerate_the_declared_text_keys() {
    assert!(
        std::env::var("NEON_REGEN_WORLD_TEXTS").is_ok(),
        "set NEON_REGEN_WORLD_TEXTS=1"
    );
    let c =
        Content::from_sources_without_texts(&Sources::embedded()).unwrap_or_else(|e| panic!("{e}"));
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/world/texts.toml");
    std::fs::write(path, render_catalog(&c)).unwrap();
}

/// Regenerates `data/text/<lang>/world_draft.toml`: a placeholder `TODO <key>` for every derived
/// key that no other text file of the language defines. Like the declared keys, this is a
/// writers' tool: it is ignored, and refuses to run without the variable. Writing a text moves
/// its key to a real file; run the tool again and its placeholder goes away.
/// `NEON_REGEN_WORLD_DRAFTS=1 cargo test -p neon-engine --test content regenerate_the_draft_texts -- --ignored`
#[test]
#[ignore = "writes data/text/<lang>/world_draft.toml"]
#[allow(
    clippy::disallowed_methods,
    reason = "the regeneration tool reads its switch from the environment and the other text files, and writes the draft file"
)]
fn regenerate_the_draft_texts() {
    assert!(
        std::env::var("NEON_REGEN_WORLD_DRAFTS").is_ok(),
        "set NEON_REGEN_WORLD_DRAFTS=1"
    );
    let c = shipped();
    for lang in Lang::ALL {
        let dir = format!(
            "{}/../../data/text/{}",
            env!("CARGO_MANIFEST_DIR"),
            lang.code()
        );
        let mut files: Vec<(String, String)> = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            if name.ends_with(".toml") && name != "world_draft.toml" {
                files.push((name, std::fs::read_to_string(&path).unwrap()));
            }
        }
        files.sort();
        let sources: Vec<(&str, &str)> = files
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str()))
            .collect();
        let catalog = Catalog::from_sources(lang, &sources).unwrap();
        let mut out = String::from(
            "# Placeholders for the derived text keys that are not written yet: `TODO <key>`.\n\
             # Generated, do not edit by hand. Writing a text means defining its key in a real\n\
             # file of this folder; then regenerate this one and the placeholder goes away.\n\
             # Regenerate: NEON_REGEN_WORLD_DRAFTS=1 cargo test -p neon-engine --test content regenerate_the_draft_texts -- --ignored\n\n",
        );
        for key in missing_keys(&c, &catalog) {
            writeln!(out, "\"{key}\" = \"TODO {key}\"").unwrap();
        }
        std::fs::write(format!("{dir}/world_draft.toml"), out).unwrap();
    }
}

#[test]
fn every_derived_key_has_a_text_in_every_language() {
    let c = shipped();
    for lang in Lang::ALL {
        let catalog = Catalog::embedded(lang).unwrap();
        let missing = missing_keys(&c, &catalog);
        assert!(
            missing.is_empty(),
            "{lang:?} lacks {} derived key(s), for example {:?}: regenerate the draft texts",
            missing.len(),
            missing.iter().take(3).collect::<Vec<_>>()
        );
    }
}

/// Says how many derived texts are still placeholders, per language. It never fails: the count
/// is the writers' progress (`data/text/<lang>/world_draft.toml` shrinks as texts are written).
#[test]
fn the_number_of_draft_texts_is_reported() {
    let c = shipped();
    let total = required_keys(&c).len();
    for lang in Lang::ALL {
        let catalog = Catalog::embedded(lang).unwrap();
        let drafts = draft_keys(&c, &catalog).len();
        // The test harness shows this with `--nocapture`.
        println!("{lang:?}: {drafts} of {total} derived texts are still drafts");
        assert!(drafts <= total);
    }
}

/// To be un-ignored at the end of R5, when every narrative text is written.
#[test]
#[ignore = "fails while a derived text is still a draft: un-ignore at the end of R5"]
fn no_draft_text_remains() {
    let c = shipped();
    for lang in Lang::ALL {
        let catalog = Catalog::embedded(lang).unwrap();
        let drafts = draft_keys(&c, &catalog);
        assert!(
            drafts.is_empty(),
            "{lang:?}: {} draft text(s) remain, for example {:?}",
            drafts.len(),
            drafts.iter().take(5).collect::<Vec<_>>()
        );
    }
}

#[test]
fn shipped_content_is_valid() {
    let c = shipped();
    assert_eq!(c.quests.len(), 23);
    assert_eq!(c.sites.len(), 16);
    assert_eq!(c.texts, required_keys(&c));
}

#[test]
fn the_shipped_text_file_is_what_the_tool_would_write() {
    let c = shipped();
    let declared: toml::Table = Sources::embedded().texts.parse().unwrap();
    let regenerated: toml::Table = render_catalog(&c).parse().unwrap();
    assert_eq!(declared["keys"], regenerated["keys"]);
}

#[test]
fn budgets_are_hooks_the_catalogs_plug_into() {
    let c = shipped();
    let long = |key: &str| (key == "quest.m05.title").then(|| "x".repeat(40));
    let errors = check_budgets(&c, long, display_width);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("quest.m05.title") && errors[0].contains("28"));
}

#[test]
fn the_real_catalogs_stay_within_the_budgets_of_the_keys_they_define() {
    // The texts of the campaign are written in lot R2.3. Until then the embedded catalogs
    // define few or none of the derived keys (`missing_keys` lists what is still owed); the
    // ones they do define must already respect their width budget in every language.
    let c = shipped();
    for lang in Lang::ALL {
        let catalog = Catalog::embedded(lang).unwrap();
        assert!(missing_keys(&c, &catalog).len() <= required_keys(&c).len());
        let overflows = check_catalog_budgets(&c, &catalog);
        assert!(overflows.is_empty(), "{lang:?}: {overflows:?}");
    }
}
