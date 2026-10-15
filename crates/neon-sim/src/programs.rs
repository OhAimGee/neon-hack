//! What each program is worth: for every site, how low the Trace goes with the whole deck of
//! its tier, and without each program of that deck in turn.
//!
//! A program that no site needs and that lowers the margin of no site is *dominated*: nothing
//! is lost by leaving it out, and nobody would pay for it (T9). The margin a program adds, site
//! by site, is also what its price should answer to. The measures are made at Normal,
//! with the objective alone and the exact search of the solver, so they are the same on every
//! machine.

use std::collections::BTreeMap;

use neon_engine::campaign::Difficulty;
use neon_engine::content::Content;
use neon_engine::content::runs::site_tier;
use neon_engine::run::Program;
use neon_engine::run::solve::{Outcome, optimum};
use serde::{Deserialize, Serialize};

/// What the solver says of a mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Margin {
    /// The lowest Trace is proven, the margin is the cap minus it.
    Left {
        /// The room under the cap.
        margin: u8,
    },
    /// Proven: no plan keeps the Trace under the cap.
    Lost,
    /// The search ran out of expansions before it knew.
    Unknown,
}

/// A program taken out of the deck of a site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Without {
    /// The program.
    pub program: String,
    /// What is left of the margin without it.
    pub margin: Margin,
}

/// A site, with its whole deck and without each program of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteDeck {
    /// The site.
    pub site: String,
    /// The tier of its deck.
    pub tier: u8,
    /// The programs of the deck.
    pub deck: Vec<String>,
    /// The margin with the whole deck.
    pub margin: Margin,
    /// The margin without each program.
    pub without: Vec<Without>,
}

/// What a program is worth over the sites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramRow {
    /// The program.
    pub program: String,
    /// The price of the item that sells it, if the stall sells it.
    pub price: Option<u32>,
    /// The sites that cannot be won without it.
    pub needed_at: Vec<String>,
    /// The sites it leaves more margin at, with how much more.
    pub lowers: Vec<(String, u8)>,
    /// The sites whose deck has it and that do no better with it.
    pub idle_at: Vec<String>,
    /// The margin it adds over all the sites it lowers the margin at.
    pub total_gain: u32,
    /// Whether nothing is lost by leaving it out: no site needs it and none does better with it.
    pub dominated: bool,
}

/// The analysis of the programs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramsReport {
    /// The version of the format.
    pub schema_version: u32,
    /// The difficulty the measures are made at.
    pub difficulty: String,
    /// The expansions the solver could spend on each measure.
    pub expansions: u64,
    /// The sites.
    pub sites: Vec<SiteDeck>,
    /// The programs.
    pub programs: Vec<ProgramRow>,
}

fn margin_of(mission: &neon_engine::run::Mission, expansions: u64) -> Margin {
    let cap = mission.trace_cap();
    match optimum(mission, expansions).outcome {
        Outcome::Solved {
            plan,
            optimal: true,
        } => Margin::Left {
            margin: cap.saturating_sub(plan.trace),
        },
        Outcome::Unsolvable => Margin::Lost,
        Outcome::Solved { .. } | Outcome::Unknown { .. } => Margin::Unknown,
    }
}

/// What a program is worth at a site: `None` for no difference, the margin lost otherwise
/// (`u16::MAX` when the site cannot be won without it).
fn worth(full: Margin, without: Margin) -> Option<u16> {
    match (full, without) {
        (Margin::Left { margin: a }, Margin::Left { margin: b }) if a > b => Some(u16::from(a - b)),
        (Margin::Left { .. }, Margin::Lost) => Some(u16::MAX),
        _ => None,
    }
}

/// Measures the sites (or only `only`) at Normal, with the whole deck of the tier and without
/// each program.
#[must_use]
pub fn analyse(content: &Content, expansions: u64, only: Option<&str>) -> ProgramsReport {
    let pace = Difficulty::Normal.pace();
    let mut sites = Vec::new();
    for site in &content.sites {
        if only.is_some_and(|wanted| wanted != site.id.as_str()) {
            continue;
        }
        let (Some(run), Some(kit)) = (content.run(&site.id), content.kit(site_tier(site))) else {
            continue;
        };
        let build = |programs: &[Program]| run.mission(kit, programs, pace).ok();
        let Some(whole) = build(&kit.programs) else {
            continue;
        };
        let margin = margin_of(&whole, expansions);
        let mut without = Vec::new();
        for program in &kit.programs {
            let rest: Vec<Program> = kit
                .programs
                .iter()
                .copied()
                .filter(|other| other != program)
                .collect();
            if let Some(mission) = build(&rest) {
                without.push(Without {
                    program: program.id().to_owned(),
                    margin: margin_of(&mission, expansions),
                });
            }
        }
        sites.push(SiteDeck {
            site: site.id.to_string(),
            tier: site_tier(site),
            deck: kit.programs.iter().map(|p| p.id().to_owned()).collect(),
            margin,
            without,
        });
    }
    // What each program is worth, site by site.
    let mut worths: BTreeMap<String, BTreeMap<String, u16>> = BTreeMap::new();
    let mut present: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for deck in &sites {
        for program in &deck.deck {
            present
                .entry(program.clone())
                .or_default()
                .push(deck.site.clone());
        }
        for without in &deck.without {
            if let Some(value) = worth(deck.margin, without.margin) {
                worths
                    .entry(without.program.clone())
                    .or_default()
                    .insert(deck.site.clone(), value);
            }
        }
    }
    let price_of = |program: &str| {
        content
            .items
            .iter()
            .find(|item| item.program.is_some_and(|p| p.id() == program))
            .map(|item| item.price.get())
    };
    let programs: Vec<ProgramRow> = present
        .iter()
        .map(|(program, at)| {
            let mine = worths.get(program);
            let needed_at: Vec<String> = mine
                .map(|m| {
                    m.iter()
                        .filter(|(_, v)| **v == u16::MAX)
                        .map(|(s, _)| s.clone())
                        .collect()
                })
                .unwrap_or_default();
            let lowers: Vec<(String, u8)> = mine
                .map(|m| {
                    m.iter()
                        .filter(|(_, v)| **v != u16::MAX)
                        .map(|(s, v)| (s.clone(), u8::try_from(*v).unwrap_or(u8::MAX)))
                        .collect()
                })
                .unwrap_or_default();
            let idle_at: Vec<String> = at
                .iter()
                .filter(|site| mine.is_none_or(|m| !m.contains_key(*site)))
                .cloned()
                .collect();
            ProgramRow {
                program: program.clone(),
                price: price_of(program),
                total_gain: lowers
                    .iter()
                    .map(|(_, gain): &(String, u8)| u32::from(*gain))
                    .sum(),
                dominated: needed_at.is_empty() && lowers.is_empty(),
                needed_at,
                lowers,
                idle_at,
            }
        })
        .collect();
    ProgramsReport {
        schema_version: crate::report::SCHEMA_VERSION,
        difficulty: crate::record::difficulty_name(Difficulty::Normal).to_owned(),
        expansions,
        sites,
        programs,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "test code: a failed expectation is the failure to report"
    )]

    use super::*;

    #[test]
    fn a_site_is_measured_with_its_whole_deck_and_without_each_program_of_it() {
        let content = Content::embedded().unwrap();
        let report = analyse(&content, 2_000_000, Some("corp-server-01"));
        assert_eq!(report.sites.len(), 1);
        let site = &report.sites[0];
        assert_eq!((site.site.as_str(), site.tier), ("corp-server-01", 2));
        assert_eq!(site.without.len(), site.deck.len());
        assert!(matches!(site.margin, Margin::Left { .. }));
        // Every program of the deck is told, with the sites it is in.
        assert_eq!(report.programs.len(), site.deck.len());
        for row in &report.programs {
            assert_eq!(
                row.needed_at.len() + row.lowers.len() + row.idle_at.len(),
                1,
                "{row:?}"
            );
            assert_eq!(
                row.dominated,
                row.needed_at.is_empty() && row.lowers.is_empty()
            );
        }
    }

    #[test]
    fn what_a_program_is_worth_is_the_margin_it_takes_away_or_the_site_it_loses() {
        let left = |margin| Margin::Left { margin };
        assert_eq!(worth(left(30), left(30)), None);
        assert_eq!(worth(left(30), left(27)), Some(3));
        // Without a program the site can even be easier: the program is worth nothing there.
        assert_eq!(worth(left(30), left(33)), None);
        assert_eq!(worth(left(30), Margin::Lost), Some(u16::MAX));
        assert_eq!(worth(left(30), Margin::Unknown), None);
    }

    #[test]
    fn the_same_content_gives_the_same_report() {
        let content = Content::embedded().unwrap();
        assert_eq!(
            analyse(&content, 2_000_000, Some("localhost")),
            analyse(&content, 2_000_000, Some("localhost"))
        );
    }
}
