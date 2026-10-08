//! Every validation rule fires on a broken copy of the shipped data, with a file and a line.

use neon_spike_missions::{Content, Diagnostic, File, Sources};

/// Loads a copy of the shipped data after `edit`, and returns its error report.
fn broken(edit: impl FnOnce(&mut Sources)) -> Vec<Diagnostic> {
    let mut src = Sources::embedded();
    edit(&mut src);
    match Content::from_sources(&src) {
        Ok(_) => panic!("the broken copy was accepted"),
        Err(e) => e.0,
    }
}

fn replace_once(text: &mut String, from: &str, to: &str) {
    assert!(text.contains(from), "pattern not found: {from}");
    *text = text.replacen(from, to, 1);
}

fn has(errors: &[Diagnostic], file: File, needle: &str) -> bool {
    errors
        .iter()
        .any(|d| d.file == file && d.message.contains(needle))
}

#[test]
fn an_unknown_reference_is_reported_at_its_line() {
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "site = \"freeport-02\"",
            "site = \"freeport-0x\"",
        )
    });
    let line = Sources::embedded()
        .quests
        .lines()
        .position(|l| l.contains("freeport-02"))
        .unwrap()
        + 1;
    let hit = errors
        .iter()
        .find(|d| d.message.contains("unknown site `freeport-0x`"))
        .unwrap();
    assert_eq!(hit.file, File::Quests);
    // An objective is located at its `[[quest.objective]]` header, two lines above `site`.
    assert_eq!(hit.line as usize, line - 2, "{hit}");
    assert!(hit.to_string().starts_with("data/quests.toml:"));
}

#[test]
fn a_duplicate_id_is_reported() {
    let errors = broken(|s| replace_once(&mut s.quests, "id = \"m02\"", "id = \"m01\""));
    assert!(has(&errors, File::Quests, "duplicate quest id `m01`"));
}

#[test]
fn prerequisites_must_come_first_and_cannot_loop() {
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "tier = 1\nreward",
            "tier = 1\nprereq = [\"m02\"]\nreward",
        )
    });
    assert!(has(&errors, File::Quests, "written after it"));
    assert!(has(&errors, File::Quests, "prerequisite cycle"));
}

#[test]
fn flags_are_typed_and_enum_values_must_be_declared() {
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "value = \"compromised\" }",
            "value = \"comprimised\" }",
        )
    });
    assert!(has(
        &errors,
        File::Quests,
        "does not fit flag `phoenix_state`"
    ));
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "flag = \"hub\", value = \"safehouse\"",
            "flag = \"hub\", value = true",
        )
    });
    assert!(has(&errors, File::Quests, "does not fit flag `hub`"));
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "name = \"angel_state\", is = \"free\"",
            "name = \"angel_state\", is = \"freed\"",
        )
    });
    assert!(has(&errors, File::Quests, "wrong or undeclared value"));
    let errors = broken(|s| {
        replace_once(
            &mut s.decisions,
            "has = \"echo_log\"",
            "has = \"echo_logs\"",
        )
    });
    assert!(has(&errors, File::Decisions, "wrong or undeclared value"));
}

#[test]
fn rewards_must_resolve() {
    let errors = broken(|s| replace_once(&mut s.rewards, "id = \"L\"", "id = \"M\""));
    assert!(has(
        &errors,
        File::Rewards,
        "size L must be defined exactly once"
    ));
    let errors =
        broken(|s| replace_once(&mut s.rewards, "id = 5\nprice = 800", "id = 5\nprice = 0"));
    assert!(has(&errors, File::Rewards, "tier 5 needs"));
}

#[test]
fn a_quest_has_a_bounded_number_of_objectives() {
    let extra = "[[quest.objective]]\nkind = \"use\"\ncommand = \"help\"\n".repeat(7);
    let errors = broken(|s| {
        let from = "[[quest.on_complete]]\nthen = [\n    { kind = \"grant_tier\", tier = 3 },";
        replace_once(&mut s.quests, from, &format!("{extra}{from}"));
    });
    assert!(has(
        &errors,
        File::Quests,
        "10 objectives, the limit is 1 to 8"
    ));
}

#[test]
fn text_keys_are_checked_both_ways() {
    let errors = broken(|s| replace_once(&mut s.texts, "    \"quest.m05.title\",\n", ""));
    assert!(has(
        &errors,
        File::Texts,
        "missing text key `quest.m05.title`"
    ));
    let errors = broken(|s| {
        replace_once(
            &mut s.texts,
            "keys = [\n",
            "keys = [\n    \"quest.zz.title\",\n",
        )
    });
    let orphan = errors
        .iter()
        .find(|d| d.message.contains("orphan text key `quest.zz.title`"))
        .unwrap();
    assert!(orphan.line > 1, "the orphan is located in the file");
}

#[test]
fn two_unordered_writers_of_a_flag_are_refused() {
    let errors = broken(|s| {
        s.topics.push_str("\n[[topic]]\nid = \"redecorate\"\ncontact = \"echo7\"\nthen = [{ then = [{ kind = \"set_flag\", flag = \"hub\", value = \"studio\" }] }]\n");
    });
    assert!(has(
        &errors,
        File::Topics,
        "flag `hub` is written by two unordered sources"
    ));
}

#[test]
fn objective_shapes_are_checked() {
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "kind = \"talk\"\ncontact = \"echo7\"\n",
            "kind = \"talk\"\n",
        )
    });
    assert!(has(&errors, File::Quests, "needs `contact`"));
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "kind = \"compromise\"\ncount = 3\n",
            "kind = \"compromise\"\ncount = 3\nsite = \"localhost\"\n",
        )
    });
    assert!(has(&errors, File::Quests, "`site` or `count`, not both"));
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "kind = \"buy\"\nitem = \"stealth_module\"\n",
            "kind = \"buy\"\nitem = \"stealth_module\"\nsite = \"localhost\"\n",
        )
    });
    assert!(has(
        &errors,
        File::Quests,
        "not used by objective kind `buy`"
    ));
}

#[test]
fn schema_errors_carry_a_line() {
    let errors = broken(|s| replace_once(&mut s.quests, "giver = \"echo7\"", "gvier = \"echo7\""));
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].message.contains("unknown field `gvier`"),
        "{}",
        errors[0]
    );
    assert!(errors[0].line >= 1 && errors[0].file == File::Quests);
}

#[test]
fn conditions_cannot_nest_forever() {
    let mut deep = String::from("{ kind = \"quest\", id = \"m01\", is = \"completed\" }");
    for _ in 0..8 {
        deep = format!("{{ kind = \"not\", of = {deep} }}");
    }
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "available_if = { kind = \"flag\", name = \"d2\", is = \"save\" }",
            &format!("available_if = {deep}"),
        );
    });
    assert!(has(&errors, File::Quests, "nested deeper than 6"));
}

#[test]
fn a_readable_without_a_source_and_a_forgotten_decision_objective_are_caught() {
    let errors = broken(|s| {
        s.catalog
            .push_str("\n[[readable]]\nid = \"f99\"\nkind = \"fragment\"\n")
    });
    assert!(has(&errors, File::Catalog, "readable `f99` has no source"));
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "kind = \"choice\"\ndecision = \"d3\"\n",
            "kind = \"use\"\ncommand = \"help\"\n",
        )
    });
    assert!(has(
        &errors,
        File::Decisions,
        "has no `choice` objective for it"
    ));
}

#[test]
fn a_main_quest_cannot_fail_and_a_failable_one_needs_a_reason() {
    let errors = broken(|s| {
        replace_once(
            &mut s.quests,
            "code = \"EPILOGUE\"\nkind = \"main\"",
            "code = \"EPILOGUE\"\nkind = \"main\"\nfailable = true",
        )
    });
    assert!(has(&errors, File::Quests, "a main quest cannot fail"));
    let errors = broken(|s| replace_once(&mut s.quests, "failable = true\n", ""));
    assert!(has(&errors, File::Quests, "need `failable = true`"));
}
