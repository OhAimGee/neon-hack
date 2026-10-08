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
    derive(c).into_iter().map(|(key, _)| key).collect()
}

/// Keys that two different things derive: `foo-bar` and `foo_bar` give the same key, so one
/// catalog entry would serve two ids. Returns `(key, first owner, second owner)`.
pub fn collisions(c: &Content) -> Vec<(String, String, String)> {
    let mut seen: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut out = Vec::new();
    for (key, owner) in derive(c) {
        match seen.get(&key) {
            Some(first) if *first != owner => out.push((key, first.clone(), owner)),
            Some(_) => {}
            None => {
                seen.insert(key, owner);
            }
        }
    }
    out
}

/// `(key, owner)` for every key, the owner being the id (or pair of ids) it comes from.
fn derive(c: &Content) -> Vec<(String, String)> {
    let owner = std::cell::RefCell::new(String::new());
    let mut keys = Vec::new();
    let mut add = |key: String| keys.push((key, owner.borrow().clone()));
    let own = |s: String| *owner.borrow_mut() = s;
    for q in &c.quests {
        own(format!("quest `{}`", q.id));
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
        own(format!("site `{}`", s.id));
        add(format!("node.{}.desc", k(s.id.as_str())));
        for f in &s.file {
            own(format!("file `{}` of site `{}`", f.id, s.id));
            add(format!(
                "file.{}.{}.desc",
                k(s.id.as_str()),
                k(f.id.as_str())
            ));
        }
    }
    for i in &c.items {
        own(format!("item `{}`", i.id));
        add(format!("item.{}.name", k(i.id.as_str())));
        add(format!("item.{}.desc", k(i.id.as_str())));
    }
    for ct in &c.contacts {
        own(format!("contact `{}`", ct.id));
        add(format!("contact.{}.name", k(ct.id.as_str())));
        add(format!("contact.{}.tagline", k(ct.id.as_str())));
    }
    for t in &c.topics {
        own(format!("topic `{}` of `{}`", t.id, t.contact));
        let base = format!(
            "contact.{}.topic.{}",
            k(t.contact.as_str()),
            k(t.id.as_str())
        );
        add(format!("{base}.q"));
        add(format!("{base}.a"));
    }
    for r in &c.readables {
        own(format!("readable `{}`", r.id));
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
        own(format!("decision `{}`", d.id));
        add(format!("decision.{}.prompt", k(d.id.as_str())));
        for ch in &d.choice {
            own(format!("choice `{}` of decision `{}`", ch.id, d.id));
            add(format!(
                "decision.{}.choice.{}.label",
                k(d.id.as_str()),
                k(ch.id.as_str())
            ));
        }
    }
    for e in &c.endings {
        own(format!("ending `{}`", e.id));
        add(format!("ending.{}.title", k(e.id.as_str())));
        for n in 1..=e.paragraphs {
            add(format!("ending.{}.p{n:02}", k(e.id.as_str())));
        }
    }
    for l in &c.epilogue {
        own(format!("epilogue line `{}`", l.id));
        add(format!("epilogue.{}", k(l.id.as_str())));
    }
    for cmd in &c.commands {
        own(format!("command `{}`", cmd.id));
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
