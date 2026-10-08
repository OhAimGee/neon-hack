//! The shipped content is valid and its declared text keys are the derived ones.

use neon_engine::content::texts::{
    check_budgets, check_catalog_budgets, display_width, missing_keys, render_catalog,
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
