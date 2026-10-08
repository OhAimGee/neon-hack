//! The embedded catalogs, as the program ships them.

use neon_engine::text::glossary::Glossary;
use neon_engine::text::{Catalog, Lang, check};

fn load_all() -> Vec<Catalog> {
    Lang::ALL
        .into_iter()
        .map(|lang| {
            Catalog::embedded(lang)
                .unwrap_or_else(|errors| panic!("the {lang:?} catalog does not load:\n{errors}"))
        })
        .collect()
}

#[test]
fn every_language_has_an_embedded_catalog() {
    for catalog in load_all() {
        assert!(
            catalog.len() > 50,
            "{:?} has only {} texts",
            catalog.lang(),
            catalog.len()
        );
        assert!(catalog.get("ui.raw").is_some());
    }
}

#[test]
fn the_embedded_content_follows_every_rule() {
    let issues = check(&load_all());
    assert!(
        issues.is_empty(),
        "{} issue(s):\n{}",
        issues.len(),
        issues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_embedded_content_follows_the_glossary() {
    let glossary = Glossary::embedded().expect("the embedded glossary loads");
    let issues = glossary.check(&load_all());
    assert!(
        issues.is_empty(),
        "{} issue(s):\n{}",
        issues.len(),
        issues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}
