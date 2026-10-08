//! Text keys: derived from the structure, never written by hand in the content.
//!
//! The structure (`data/*.toml`) holds no story text. Every text a frontend will need has a
//! key that follows from the ids by a fixed rule, so the French and English catalogs have
//! the same keys by construction and the validator can list what is missing. `texts.toml`
//! only *declares* the keys (the catalog of what the writers owe); a test keeps it equal to
//! what [`required_keys`] derives. Ids use `_` in keys wherever they use `-`.

use std::collections::BTreeSet;

use crate::content::Content;
use crate::schema::{QuestKind, ReadableKind};

fn k(id: &str) -> String {
    id.replace('-', "_")
}

/// Every text key the content needs.
pub fn required_keys(c: &Content) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let mut add = |key: String| {
        keys.insert(key);
    };
    for q in &c.quests {
        let id = k(q.id.as_str());
        for field in ["title", "title_short", "desc", "lore", "loc", "debrief"] {
            add(format!("quest.{id}.{field}"));
        }
        for n in 1..=q.objective.len() {
            add(format!("quest.{id}.obj.{n}"));
            add(format!("quest.{id}.hint.{n}.1"));
            if q.kind == QuestKind::Main {
                add(format!("quest.{id}.hint.{n}.2"));
            }
        }
    }
    for s in &c.sites {
        add(format!("node.{}.desc", k(s.id.as_str())));
        for f in &s.file {
            add(format!(
                "file.{}.{}.desc",
                k(s.id.as_str()),
                k(f.id.as_str())
            ));
        }
    }
    for i in &c.items {
        add(format!("item.{}.name", k(i.id.as_str())));
        add(format!("item.{}.desc", k(i.id.as_str())));
    }
    for ct in &c.contacts {
        add(format!("contact.{}.name", k(ct.id.as_str())));
        add(format!("contact.{}.tagline", k(ct.id.as_str())));
    }
    for t in &c.topics {
        let base = format!(
            "contact.{}.topic.{}",
            k(t.contact.as_str()),
            k(t.id.as_str())
        );
        add(format!("{base}.q"));
        add(format!("{base}.a"));
    }
    for r in &c.readables {
        let id = k(r.id.as_str());
        match r.kind {
            ReadableKind::Fragment => {
                add(format!("frag.{id}.title"));
                add(format!("frag.{id}.body"));
            }
            ReadableKind::Mail => {
                add(format!("mail.{id}.subject"));
                add(format!("mail.{id}.body"));
            }
            ReadableKind::Scene => {
                for n in 1..=r.paragraphs {
                    add(format!("cutscene.{id}.p{n:02}"));
                }
            }
            ReadableKind::Document => add(format!("doc.{id}.title")),
        }
    }
    for d in &c.decisions {
        add(format!("decision.{}.prompt", k(d.id.as_str())));
        for ch in &d.choice {
            add(format!(
                "decision.{}.choice.{}.label",
                k(d.id.as_str()),
                k(ch.id.as_str())
            ));
        }
    }
    for e in &c.endings {
        add(format!("ending.{}.title", k(e.id.as_str())));
        for n in 1..=e.paragraphs {
            add(format!("ending.{}.p{n:02}", k(e.id.as_str())));
        }
    }
    for l in &c.epilogue {
        add(format!("epilogue.{}", k(l.id.as_str())));
    }
    for cmd in &c.commands {
        add(format!("command.{}.help", k(cmd.id.as_str())));
    }
    keys
}

/// Display-width budget of a key, in terminal columns (bible 6.2), when it has one.
pub fn budget(key: &str) -> Option<usize> {
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        ["quest", _, "title"] => Some(28),
        ["quest", _, "title_short"] => Some(18),
        ["quest", _, "desc"] => Some(110),
        ["quest", _, "lore"] => Some(420),
        ["quest", _, "obj", _] => Some(64),
        ["quest", _, "hint", _, _] => Some(160),
        ["quest", _, "debrief"] => Some(200),
        ["contact", _, "tagline"] => Some(40),
        ["contact", _, "topic", _, "q"] => Some(48),
        ["contact", _, "topic", _, "a"] => Some(340),
        ["mail", _, "subject"] => Some(48),
        ["mail", _, "body"] => Some(600),
        ["cutscene", _, _] => Some(280),
        ["frag", _, "title"] => Some(40),
        ["frag", _, "body"] => Some(520),
        ["epilogue", _] | ["ending", _, _] => Some(300),
        ["node", _, "desc"] => Some(60),
        ["file", _, _, "desc"] => Some(40),
        _ => None,
    }
}

/// Checks the budgets of a catalog. `text` returns the text of a key in one language,
/// `width` measures it (display width in the real engine, with `unicode-width`). Returns one
/// message per overflow.
pub fn check_budgets(
    c: &Content,
    text: impl Fn(&str) -> Option<String>,
    width: impl Fn(&str) -> usize,
) -> Vec<String> {
    required_keys(c)
        .into_iter()
        .filter_map(|key| {
            let max = budget(&key)?;
            let w = width(&text(&key)?);
            (w > max).then(|| format!("{key}: {w} columns for a budget of {max}"))
        })
        .collect()
}

/// The text of `texts.toml` for the current structure (used to regenerate the file).
pub fn render_catalog(c: &Content) -> String {
    let mut out = String::from(
        "# Text keys the content needs (docs/design/narrative-bible.md, section 6.3).\n\
         # Derived from the structure by `texts::required_keys`: do not edit by hand.\n\
         # Regenerate: NEON_SPIKE_REGEN=1 cargo test -p neon-spike-missions --test content regenerate -- --ignored\n\
         keys = [\n",
    );
    for key in required_keys(c) {
        out.push_str(&format!("    \"{key}\",\n"));
    }
    out.push_str("]\n");
    out
}
