use crate::text::{Catalog, Lang};

use super::*;

fn shipped() -> Content {
    Content::embedded().unwrap_or_else(|e| panic!("shipped content is invalid:\n{e}"))
}

fn catalog(toml: &str) -> Catalog {
    Catalog::from_sources(Lang::En, &[("world.toml", toml)]).unwrap()
}

#[test]
fn the_display_width_counts_columns_not_characters() {
    assert_eq!(display_width("hello"), 5);
    assert_eq!(display_width("é"), 1);
    // Wide characters take two columns, which is why the budgets are not in characters.
    assert_eq!(display_width("数据"), 4);
    assert_eq!(display_width("a\u{0301}"), 1);
}

#[test]
fn the_budget_of_a_key_follows_its_shape() {
    assert_eq!(budget("quest.m05.title"), Some(28));
    assert_eq!(budget("quest.m05.hint.2.1"), Some(160));
    assert_eq!(budget("contact.echo7.topic.advice.a"), Some(340));
    assert_eq!(budget("cutscene.cs02.p01"), Some(280));
    assert_eq!(budget("command.scan.help"), None);
}

#[test]
fn a_catalog_over_budget_is_reported_with_the_real_measure() {
    let c = shipped();
    // 15 wide characters are 15 characters but 30 columns: over the title budget of 28.
    let wide = "数".repeat(15);
    let fine = "x".repeat(28);
    let cat = catalog(&format!(
        "[quest.m05]\ntitle = \"{wide}\"\n[quest.m04]\ntitle = \"{fine}\"\n"
    ));
    let errors = check_catalog_budgets(&c, &cat);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("quest.m05.title") && errors[0].contains("30 columns"));
}

#[test]
fn missing_keys_lists_what_the_writers_still_owe() {
    let c = shipped();
    let cat = catalog("[quest.m05]\ntitle = \"The Eye\"\n");
    let missing = missing_keys(&c, &cat);
    assert_eq!(missing.len(), required_keys(&c).len() - 1);
    assert!(!missing.iter().any(|k| k == "quest.m05.title"));
    assert!(missing.iter().any(|k| k == "quest.m05.desc"));
}

#[test]
fn the_regenerated_file_declares_exactly_the_derived_keys() {
    let c = shipped();
    let text = render_catalog(&c);
    let declared: toml::Table = text.parse().unwrap();
    let keys: BTreeSet<String> = declared["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(keys, required_keys(&c));
}
