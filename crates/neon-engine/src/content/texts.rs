//! Text keys: derived from the structure, never written by hand in the content.
//!
//! The structure (`data/world/*.toml`) holds no story text. Every text a frontend will need
//! has a key that follows from the ids by a fixed rule, so the French and English catalogs
//! have the same keys by construction and the validator can list what is missing.
//! `texts.toml` only *declares* the keys (the catalog of what the writers owe); a test keeps
//! it equal to what [`required_keys`] derives. Ids use `_` in keys wherever they use `-`.
//!
//! The width budgets ([`budget`]) are measured in terminal columns with [`display_width`];
//! [`check_budgets`] takes the measure as a parameter so tests can use their own, and
//! [`check_catalog_budgets`] plugs in the real catalogs and the real measure.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use unicode_width::UnicodeWidthStr;

use crate::content::Content;
use crate::content::schema::{QuestKind, ReadableKind};
use crate::text::{Catalog, RenderMode, Text, render};

fn k(id: &str) -> String {
    id.replace('-', "_")
}

/// Every text key the content needs.
#[must_use]
pub fn required_keys(c: &Content) -> BTreeSet<String> {
    derive(c).into_iter().map(|(key, _)| key).collect()
}

/// Keys that two different things derive: `foo-bar` and `foo_bar` give the same key, so one
/// catalog entry would serve two ids. Returns `(key, first owner, second owner)`.
#[must_use]
pub fn collisions(c: &Content) -> Vec<(String, String, String)> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
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

/// The keys being derived, each tagged with the thing it comes from.
struct Keys {
    owner: String,
    keys: Vec<(String, String)>,
}

impl Keys {
    /// Starts the keys of a new owner.
    fn own(&mut self, owner: String) {
        self.owner = owner;
    }

    /// Adds a key of the current owner.
    fn add(&mut self, key: String) {
        self.keys.push((key, self.owner.clone()));
    }
}

/// `(key, owner)` for every key, the owner being the id (or pair of ids) it comes from.
fn derive(c: &Content) -> Vec<(String, String)> {
    let mut out = Keys {
        owner: String::new(),
        keys: Vec::new(),
    };
    for q in &c.quests {
        out.own(format!("quest `{}`", q.id));
        let id = k(q.id.as_str());
        for field in ["title", "title_short", "desc", "lore", "loc", "debrief"] {
            out.add(format!("quest.{id}.{field}"));
        }
        for n in 1..=q.objective.len() {
            out.add(format!("quest.{id}.obj.{n}"));
            out.add(format!("quest.{id}.hint.{n}.1"));
            if q.kind == QuestKind::Main {
                out.add(format!("quest.{id}.hint.{n}.2"));
            }
        }
    }
    for s in &c.sites {
        out.own(format!("site `{}`", s.id));
        out.add(format!("node.{}.desc", k(s.id.as_str())));
        for f in &s.file {
            out.own(format!("file `{}` of site `{}`", f.id, s.id));
            out.add(format!(
                "file.{}.{}.desc",
                k(s.id.as_str()),
                k(f.id.as_str())
            ));
        }
    }
    for i in &c.items {
        out.own(format!("item `{}`", i.id));
        out.add(format!("item.{}.name", k(i.id.as_str())));
        out.add(format!("item.{}.desc", k(i.id.as_str())));
    }
    for ct in &c.contacts {
        out.own(format!("contact `{}`", ct.id));
        out.add(format!("contact.{}.name", k(ct.id.as_str())));
        out.add(format!("contact.{}.tagline", k(ct.id.as_str())));
    }
    for t in &c.topics {
        out.own(format!("topic `{}` of `{}`", t.id, t.contact));
        let base = format!(
            "contact.{}.topic.{}",
            k(t.contact.as_str()),
            k(t.id.as_str())
        );
        out.add(format!("{base}.q"));
        out.add(format!("{base}.a"));
    }
    for r in &c.readables {
        out.own(format!("readable `{}`", r.id));
        let id = k(r.id.as_str());
        match r.kind {
            ReadableKind::Fragment => {
                out.add(format!("frag.{id}.title"));
                out.add(format!("frag.{id}.body"));
            }
            ReadableKind::Mail => {
                out.add(format!("mail.{id}.subject"));
                out.add(format!("mail.{id}.body"));
            }
            ReadableKind::Scene => {
                for n in 1..=r.paragraphs {
                    out.add(format!("cutscene.{id}.p{n:02}"));
                }
            }
            ReadableKind::Document => out.add(format!("doc.{id}.title")),
        }
    }
    for d in &c.decisions {
        out.own(format!("decision `{}`", d.id));
        out.add(format!("decision.{}.prompt", k(d.id.as_str())));
        for ch in &d.choice {
            out.own(format!("choice `{}` of decision `{}`", ch.id, d.id));
            out.add(format!(
                "decision.{}.choice.{}.label",
                k(d.id.as_str()),
                k(ch.id.as_str())
            ));
        }
    }
    for e in &c.endings {
        out.own(format!("ending `{}`", e.id));
        out.add(format!("ending.{}.title", k(e.id.as_str())));
        for n in 1..=e.paragraphs {
            out.add(format!("ending.{}.p{n:02}", k(e.id.as_str())));
        }
    }
    for l in &c.epilogue {
        out.own(format!("epilogue line `{}`", l.id));
        out.add(format!("epilogue.{}", k(l.id.as_str())));
    }
    for cmd in &c.commands {
        out.own(format!("command `{}`", cmd.id));
        out.add(format!("command.{}.help", k(cmd.id.as_str())));
    }
    out.keys
}

/// Display-width budget of a key, in terminal columns (bible 6.2), when it has one.
#[must_use]
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
        ["contact", _, "tagline"] | ["frag", _, "title"] | ["file", _, _, "desc"] => Some(40),
        ["contact", _, "topic", _, "q"] | ["mail", _, "subject"] => Some(48),
        ["contact", _, "topic", _, "a"] => Some(340),
        ["mail", _, "body"] => Some(600),
        ["cutscene", _, _] => Some(280),
        ["frag", _, "body"] => Some(520),
        ["epilogue", _] | ["ending", _, _] => Some(300),
        ["node", _, "desc"] => Some(60),
        _ => None,
    }
}

/// Width of a text in terminal columns: wide (East Asian) characters count for two, combining
/// marks for none. This is the measure the budgets are written in.
#[must_use]
pub fn display_width(text: &str) -> usize {
    text.width()
}

/// Checks the budgets of a catalog. `text` returns the text of a key in one language,
/// `width` measures it ([`display_width`] in the real engine). Returns one message per
/// overflow.
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

/// Checks the budgets of the texts a language catalog holds, measured with
/// [`display_width`]. A key the catalog lacks is not reported here (see [`missing_keys`]).
#[must_use]
pub fn check_catalog_budgets(c: &Content, catalog: &Catalog) -> Vec<String> {
    check_budgets(
        c,
        |key| {
            catalog
                .get(key)
                .map(|_| render(&Text::dynamic(key.to_owned()), catalog, RenderMode::FULL))
        },
        display_width,
    )
}

/// The derived keys a language catalog does not define yet: what the writers still owe.
#[must_use]
pub fn missing_keys(c: &Content, catalog: &Catalog) -> Vec<String> {
    required_keys(c)
        .into_iter()
        .filter(|key| catalog.get(key).is_none())
        .collect()
}

/// The text of `texts.toml` for the current structure (used to regenerate the file).
#[must_use]
pub fn render_catalog(c: &Content) -> String {
    let mut out = String::from(
        "# Text keys the content needs (docs/design/narrative-bible.md, section 6.3).\n\
         # Derived from the structure by `texts::required_keys`: do not edit by hand.\n\
         # Regenerate: NEON_REGEN_WORLD_TEXTS=1 cargo test -p neon-engine --test content regenerate_the_declared_text_keys -- --ignored\n\
         keys = [\n",
    );
    for key in required_keys(c) {
        // Writing to a String cannot fail.
        let _ = writeln!(out, "    \"{key}\",");
    }
    out.push_str("]\n");
    out
}

#[cfg(test)]
mod tests;
