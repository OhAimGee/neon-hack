use super::*;

const SETTINGS: &str = r#"
[settings]
narrative_prefixes = ["quest."]
exempt_prefixes = ["ui.tag."]
"#;

fn glossary(concepts: &str) -> Glossary {
    Glossary::parse(&format!("{SETTINGS}{concepts}")).unwrap()
}

fn catalog(lang: Lang, source: &str) -> Catalog {
    Catalog::from_sources(lang, &[("test.toml", source)]).unwrap()
}

const HEAT: &str = r#"
[[concept]]
id = "trace"
note = "gauge"
[concept.forbid]
en = ["heat", "alert level"]
fr = ["chaleur"]
"#;

#[test]
fn the_embedded_glossary_loads_and_has_unique_ids() {
    let glossary = Glossary::embedded().unwrap();
    assert!(glossary.concepts().len() > 50);
    let ids: BTreeSet<&str> = glossary.concepts().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), glossary.concepts().len());
}

#[test]
fn a_malformed_glossary_is_refused_with_a_reason() {
    assert!(matches!(
        Glossary::parse("not toml ["),
        Err(ParseError::Toml(_))
    ));
    let bad_id = format!("{SETTINGS}[[concept]]\nid = \"Trace\"\nnote = \"x\"\n");
    assert!(matches!(
        Glossary::parse(&bad_id),
        Err(ParseError::BadId(_))
    ));
    let twice = format!("{SETTINGS}{HEAT}{HEAT}");
    assert!(matches!(
        Glossary::parse(&twice),
        Err(ParseError::DuplicateId(id)) if id == "trace"
    ));
    let language = format!(
        "{SETTINGS}[[concept]]\nid = \"a\"\nnote = \"x\"\n[concept.forbid]\nde = [\"hitze\"]\n"
    );
    assert!(matches!(
        Glossary::parse(&language),
        Err(ParseError::UnknownLanguage { code, .. }) if code == "de"
    ));
    let empty = format!(
        "{SETTINGS}[[concept]]\nid = \"a\"\nnote = \"x\"\n[concept.forbid]\nen = [\" \"]\n"
    );
    assert!(matches!(
        Glossary::parse(&empty),
        Err(ParseError::EmptyVariant(_))
    ));
    let unknown_field = format!("{SETTINGS}[[concept]]\nid = \"a\"\nnote = \"x\"\nloud = true\n");
    assert!(matches!(
        Glossary::parse(&unknown_field),
        Err(ParseError::Toml(_))
    ));
}

#[test]
fn a_word_is_found_only_as_a_whole_word() {
    assert!(contains_word("the heat rises", "heat"));
    assert!(contains_word("heat", "heat"));
    assert!(contains_word("(heat)", "heat"));
    assert!(contains_word("the alert level is high", "alert level"));
    assert!(!contains_word("wheat", "heat"));
    assert!(!contains_word("heating", "heat"));
    assert!(!contains_word("the heater", "heat"));
    assert!(contains_word("le niveau d'alerte monte", "niveau d'alerte"));
    assert!(!contains_word("rien", "noeud"));
}

fn both(en: &str, fr: &str) -> [Catalog; 2] {
    [catalog(Lang::En, en), catalog(Lang::Fr, fr)]
}

#[test]
fn a_consistent_vocabulary_has_no_issue() {
    let glossary = glossary(HEAT);
    let catalogs = both(
        "[term]\ntrace = \"Trace\"\n[ui]\nline = \"Your Trace is {n}.\"\n",
        "[term]\ntrace = \"Trace\"\n[ui]\nline = \"Votre Trace est {n}.\"\n",
    );
    assert_eq!(glossary.check(&catalogs), []);
}

#[test]
fn a_forbidden_variant_is_reported_with_the_key_and_the_concept() {
    let glossary = glossary(HEAT);
    let catalogs = both(
        "[term]\ntrace = \"Trace\"\n[ui]\nline = \"The Heat is {n}.\"\nok = \"Wheat\"\n",
        "[term]\ntrace = \"Trace\"\n[ui]\nline = \"La Trace est {n}.\"\n",
    );
    assert_eq!(
        glossary.check(&catalogs),
        [Issue::Forbidden {
            lang: Lang::En,
            key: "ui.line".to_owned(),
            concept: "trace".to_owned(),
            variant: "heat".to_owned(),
        }]
    );
}

#[test]
fn the_variants_of_one_language_do_not_apply_to_the_other() {
    let glossary = glossary(HEAT);
    let catalogs = both(
        "[term]\ntrace = \"Trace\"\n[ui]\nline = \"A chaleur of sorts\"\n",
        "[term]\ntrace = \"Trace\"\n[ui]\nline = \"Heat\"\n",
    );
    assert_eq!(glossary.check(&catalogs), []);
}

#[test]
fn role_tags_are_exempt_and_narration_is_only_checked_for_strict_concepts() {
    let loose = glossary(HEAT);
    let strict = glossary(&HEAT.replace("note = \"gauge\"", "note = \"gauge\"\nstrict = true"));
    let catalogs = both(
        "[term]\ntrace = \"Trace\"\n[ui.tag]\nheat = \"[HEAT]\"\n[quest.m01]\nbeat = \"The heat is on.\"\n",
        "[term]\ntrace = \"Trace\"\n[ui.tag]\nheat = \"[CHALEUR]\"\n[quest.m01]\nbeat = \"La chaleur monte.\"\n",
    );
    assert_eq!(loose.check(&catalogs), []);
    let issues = strict.check(&catalogs);
    assert_eq!(issues.len(), 2, "{issues:?}");
    assert!(issues.iter().all(|issue| matches!(
        issue,
        Issue::Forbidden { key, .. } if key == "quest.m01.beat"
    )));
}

#[test]
fn a_concept_without_a_term_and_a_term_without_a_concept_are_reported() {
    let glossary = glossary(HEAT);
    let catalogs = both("[term]\nheat = \"Heat\"\n", "[term]\ntrace = \"Trace\"\n");
    let issues = glossary.check(&catalogs);
    assert!(issues.contains(&Issue::MissingTerm {
        lang: Lang::En,
        id: "trace".to_owned()
    }));
    assert!(issues.contains(&Issue::UnknownTerm {
        lang: Lang::En,
        key: "term.heat".to_owned()
    }));
    assert_eq!(issues.len(), 2, "{issues:?}");
}

#[test]
fn two_concepts_cannot_share_a_word_and_a_forbidden_word_cannot_be_a_term() {
    let glossary = glossary(
        r#"
[[concept]]
id = "a"
note = "x"
[[concept]]
id = "b"
note = "x"
[concept.forbid]
en = ["alpha"]
"#,
    );
    let catalogs = [
        catalog(Lang::En, "[term]\na = \"Alpha\"\nb = \"alpha\"\n"),
        catalog(Lang::Fr, "[term]\na = \"un\"\nb = \"deux\"\n"),
    ];
    let issues = glossary.check(&catalogs);
    assert!(issues.contains(&Issue::DuplicateTerm {
        lang: Lang::En,
        word: "alpha".to_owned(),
        ids: vec!["a".to_owned(), "b".to_owned()],
    }));
    assert!(issues.contains(&Issue::ForbiddenIsCanonical {
        lang: Lang::En,
        concept: "b".to_owned(),
        variant: "alpha".to_owned(),
        owner: "a, b".to_owned(),
    }));
}

#[test]
fn variants_of_a_term_do_not_count_as_unknown_terms() {
    let glossary = glossary(HEAT);
    let catalogs = both(
        "[term]\ntrace = \"Trace\"\n\"trace@sr\" = \"Trace\"\n",
        "[term]\ntrace = \"Trace\"\n",
    );
    assert_eq!(glossary.check(&catalogs), []);
}
