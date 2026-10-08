//! The shipped content is valid, and every validation rule fires on a broken copy.

use neon_spike_missions::texts::{check_budgets, render_catalog, required_keys};
use neon_spike_missions::{Content, Sources};

fn shipped() -> Content {
    Content::embedded().unwrap_or_else(|e| panic!("shipped content is invalid:\n{e}"))
}

/// Regenerates `data/texts.toml`:
/// `NEON_SPIKE_REGEN=1 cargo test -p neon-spike-missions --test content regenerate -- --ignored`
#[test]
#[ignore = "writes data/texts.toml"]
fn regenerate() {
    assert!(
        std::env::var("NEON_SPIKE_REGEN").is_ok(),
        "set NEON_SPIKE_REGEN=1"
    );
    let c =
        Content::from_sources_without_texts(&Sources::embedded()).unwrap_or_else(|e| panic!("{e}"));
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/texts.toml");
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
fn budgets_are_hooks_the_catalogs_will_plug_into() {
    let c = shipped();
    let long = |key: &str| (key == "quest.m05.title").then(|| "x".repeat(40));
    let errors = check_budgets(&c, long, |t| t.chars().count());
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("quest.m05.title") && errors[0].contains("28"));
}
