//! The thresholds a report is checked against (`docs/spec/neon-sim.md` § 5).
//!
//! They are data (`data/sim/targets.toml`), never numbers in the code. A barrier (`P`) fails the
//! check; an objective (`O`) is reported and does not fail it, until the owner validates its
//! value and it is promoted.

use serde::Deserialize;

use crate::campaign::OPTIMIST;
use crate::record::difficulty_name;
use crate::report::{CampaignReport, Report, SitesReport};
use crate::sites::{SiteResult, SiteRow};
use neon_engine::campaign::Difficulty;

/// The thresholds shipped with the harness.
pub const EMBEDDED: &str = include_str!("../../../data/sim/targets.toml");

/// Whether a threshold fails the check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Level {
    /// A barrier: the check fails.
    P,
    /// An objective: reported only.
    O,
}

/// T1: the optimist finishes the campaign.
#[derive(Debug, Clone, Deserialize)]
pub struct Completion {
    /// P or O.
    pub level: Level,
    /// The share of the optimist's games that reach the epilogue, per mille.
    pub per_mille: u64,
    /// The endings that must be reached at least once, over all the reports of the optimist.
    pub endings: Vec<String>,
}

/// T2: every site can be won.
#[derive(Debug, Clone, Deserialize)]
pub struct Solvable {
    /// P or O.
    pub level: Level,
}

/// T3: the length of the best plan of a site.
#[derive(Debug, Clone, Deserialize)]
pub struct Length {
    /// P or O.
    pub level: Level,
    /// The fewest turns of the best plan.
    pub min_turns: u32,
    /// The most turns of the best plan.
    pub max_turns: u32,
}

/// T4: the margin the minimal deck leaves.
#[derive(Debug, Clone, Deserialize)]
pub struct Margin {
    /// The level of the order between difficulties: the margin never grows from Story to
    /// Hardcore.
    pub order_level: Level,
    /// The level of the gap between Story and Normal.
    pub gap_level: Level,
    /// How much more margin Story leaves than Normal, at least.
    pub story_over_normal: u8,
    /// The level of the bounds at Normal.
    pub bounds_level: Level,
    /// The least margin at Normal.
    pub normal_min: u8,
    /// The most margin at Normal.
    pub normal_max: u8,
}

/// All the thresholds.
#[derive(Debug, Clone, Deserialize)]
pub struct Targets {
    /// T1.
    pub t1: Completion,
    /// T2.
    pub t2: Solvable,
    /// T3.
    pub t3: Length,
    /// T4.
    pub t4: Margin,
}

impl Targets {
    /// Reads thresholds from TOML.
    ///
    /// # Errors
    ///
    /// The parse error.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// The thresholds shipped with the harness.
    ///
    /// # Errors
    ///
    /// The parse error (a test keeps the embedded file valid).
    pub fn embedded() -> Result<Self, toml::de::Error> {
        Self::parse(EMBEDDED)
    }
}

/// One verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The threshold, as numbered in the spec (`T1`...).
    pub id: &'static str,
    /// Whether it is a barrier or an objective.
    pub level: Level,
    /// Whether it holds.
    pub ok: bool,
    /// What was measured.
    pub detail: String,
}

impl Finding {
    /// Whether the finding fails the check.
    #[must_use]
    pub fn fails(&self) -> bool {
        !self.ok && self.level == Level::P
    }
}

/// Checks the reports against the thresholds. A threshold that the reports give no way to
/// measure is not listed: it is up to the caller to give the right reports.
#[must_use]
pub fn check(targets: &Targets, reports: &[Report]) -> Vec<Finding> {
    let campaigns: Vec<&CampaignReport> = reports
        .iter()
        .filter_map(|r| match r {
            Report::Campaign(c) if c.bot == OPTIMIST => Some(c),
            _ => None,
        })
        .collect();
    let sites: Vec<&SitesReport> = reports
        .iter()
        .filter_map(|r| match r {
            Report::Sites(s) => Some(s),
            Report::Campaign(_) => None,
        })
        .collect();
    let mut findings = Vec::new();
    if !campaigns.is_empty() {
        findings.extend(t1(&targets.t1, &campaigns));
    }
    let rows: Vec<&SiteRow> = sites.iter().flat_map(|s| &s.rows).collect();
    if !rows.is_empty() {
        findings.push(t2(&targets.t2, &rows));
        findings.push(t3(&targets.t3, &rows));
        findings.extend(t4(&targets.t4, &rows));
    }
    findings
}

fn t1(target: &Completion, reports: &[&CampaignReport]) -> Vec<Finding> {
    let games: u64 = reports.iter().map(|r| r.games).sum();
    let finished: u64 = reports.iter().map(|r| r.finished).sum();
    let share = (finished * 1000).checked_div(games).unwrap_or(0);
    let mut findings = vec![Finding {
        id: "T1",
        level: target.level,
        ok: games > 0 && share >= target.per_mille,
        detail: format!(
            "the optimist finished {finished} of {games} games ({share} per mille, {} wanted)",
            target.per_mille
        ),
    }];
    let missing: Vec<&str> = target
        .endings
        .iter()
        .filter(|ending| {
            !reports
                .iter()
                .any(|r| r.endings.get(ending.as_str()).is_some_and(|n| *n > 0))
        })
        .map(String::as_str)
        .collect();
    findings.push(Finding {
        id: "T1",
        level: target.level,
        ok: missing.is_empty(),
        detail: if missing.is_empty() {
            format!("every ending was reached ({})", target.endings.join(", "))
        } else {
            format!("endings never reached: {}", missing.join(", "))
        },
    });
    findings
}

fn t2(target: &Solvable, rows: &[&SiteRow]) -> Finding {
    let bad: Vec<String> = rows
        .iter()
        .filter(|r| !matches!(r.result, SiteResult::Solved { .. }))
        .map(|r| format!("{} {}", r.site, r.difficulty))
        .collect();
    Finding {
        id: "T2",
        level: target.level,
        ok: bad.is_empty(),
        detail: if bad.is_empty() {
            format!(
                "every site is won, at every difficulty ({} measures)",
                rows.len()
            )
        } else {
            format!("not won: {}", bad.join(", "))
        },
    }
}

fn t3(target: &Length, rows: &[&SiteRow]) -> Finding {
    let bad: Vec<String> = rows
        .iter()
        .filter_map(|r| match r.result {
            SiteResult::Solved { turns, .. }
                if turns < target.min_turns || turns > target.max_turns =>
            {
                Some(format!("{} {} ({turns} turns)", r.site, r.difficulty))
            }
            _ => None,
        })
        .collect();
    Finding {
        id: "T3",
        level: target.level,
        ok: bad.is_empty(),
        detail: if bad.is_empty() {
            format!(
                "the best plan of every site takes {} to {} turns",
                target.min_turns, target.max_turns
            )
        } else {
            format!(
                "outside {} to {} turns: {}",
                target.min_turns,
                target.max_turns,
                bad.join(", ")
            )
        },
    }
}

/// The margin of a site at a difficulty, when it was measured and won.
fn margin_of(rows: &[&SiteRow], site: &str, difficulty: Difficulty) -> Option<u8> {
    rows.iter()
        .find(|r| r.site == site && r.difficulty == difficulty_name(difficulty))
        .and_then(|r| match r.result {
            SiteResult::Solved { margin, .. } => Some(margin),
            _ => None,
        })
}

fn t4(target: &Margin, rows: &[&SiteRow]) -> Vec<Finding> {
    // One entry per site, in the order of the reports: the rows of a site are spread over the
    // reports (one per difficulty), so a consecutive `dedup` would not be enough.
    let mut sites: Vec<&str> = Vec::new();
    for row in rows {
        if !sites.contains(&row.site.as_str()) {
            sites.push(row.site.as_str());
        }
    }
    let mut disorder = Vec::new();
    let mut flat = Vec::new();
    let mut outside = Vec::new();
    for site in sites {
        let at = |difficulty| margin_of(rows, site, difficulty);
        let (Some(story), Some(normal), Some(expert), Some(hardcore)) = (
            at(Difficulty::Story),
            at(Difficulty::Normal),
            at(Difficulty::Expert),
            at(Difficulty::Hardcore),
        ) else {
            continue;
        };
        if story < normal || normal < expert || expert < hardcore {
            disorder.push(format!(
                "{site} (story {story}, normal {normal}, expert {expert}, hardcore {hardcore})"
            ));
        }
        if u16::from(story) < u16::from(normal) + u16::from(target.story_over_normal) {
            flat.push(format!("{site} ({story} against {normal})"));
        }
        if normal < target.normal_min || normal > target.normal_max {
            outside.push(format!("{site} ({normal})"));
        }
    }
    let verdict = |id, level, list: &[String], good: String, bad: String| Finding {
        id,
        level,
        ok: list.is_empty(),
        detail: if list.is_empty() {
            good
        } else {
            format!("{bad}: {}", list.join("; "))
        },
    };
    vec![
        verdict(
            "T4",
            target.order_level,
            &disorder,
            "the margin never grows from Story to Hardcore, at any site".to_owned(),
            "margins out of order".to_owned(),
        ),
        verdict(
            "T4",
            target.gap_level,
            &flat,
            format!(
                "Story leaves at least {} more margin than Normal, at every site",
                target.story_over_normal
            ),
            format!(
                "Story leaves less than {} more margin than Normal",
                target.story_over_normal
            ),
        ),
        verdict(
            "T4",
            target.bounds_level,
            &outside,
            format!(
                "the margin at Normal is {} to {} at every site",
                target.normal_min, target.normal_max
            ),
            format!(
                "margin at Normal outside {} to {}",
                target.normal_min, target.normal_max
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "test code: a failed expectation is the failure to report"
    )]

    use super::*;
    use crate::report::SCHEMA_VERSION;

    fn row(site: &str, difficulty: &str, margin: u8, turns: u32) -> SiteRow {
        SiteRow {
            site: site.to_owned(),
            tier: 1,
            difficulty: difficulty.to_owned(),
            cap: 100,
            result: SiteResult::Solved {
                trace: 100 - margin,
                margin,
                turns,
                optimal: true,
                expanded: 1,
            },
        }
    }

    fn sites(rows: Vec<SiteRow>) -> Report {
        Report::Sites(SitesReport {
            schema_version: SCHEMA_VERSION,
            expansions: 1,
            rows,
        })
    }

    /// A site whose margin falls with the difficulty, as the thresholds want.
    fn well_behaved(site: &str, margins: [u8; 4], turns: u32) -> Vec<SiteRow> {
        ["story", "normal", "expert", "hardcore"]
            .iter()
            .zip(margins)
            .map(|(difficulty, margin)| row(site, difficulty, margin, turns))
            .collect()
    }

    fn targets() -> Targets {
        Targets::embedded().unwrap()
    }

    fn find<'a>(findings: &'a [Finding], id: &str, nth: usize) -> &'a Finding {
        findings.iter().filter(|f| f.id == id).nth(nth).unwrap()
    }

    #[test]
    fn a_site_inside_every_bound_passes_everything() {
        let report = sites(well_behaved("a", [80, 60, 50, 40], 5));
        let findings = check(&targets(), &[report]);
        assert!(findings.iter().all(|f| f.ok), "{findings:?}");
    }

    #[test]
    fn a_site_that_cannot_be_won_fails_t2() {
        let mut rows = well_behaved("a", [80, 60, 50, 40], 5);
        rows[2].result = SiteResult::Unsolvable;
        rows[3].result = SiteResult::Unknown { expanded: 9 };
        let findings = check(&targets(), &[sites(rows)]);
        let t2 = find(&findings, "T2", 0);
        assert!(!t2.ok && t2.fails());
        assert!(t2.detail.contains("a expert") && t2.detail.contains("a hardcore"));
    }

    #[test]
    fn a_margin_that_grows_with_the_difficulty_fails_the_barrier_of_t4() {
        let report = sites(well_behaved("a", [80, 60, 70, 40], 5));
        let findings = check(&targets(), &[report]);
        let order = find(&findings, "T4", 0);
        assert!(order.fails(), "{}", order.detail);
        assert!(order.detail.contains("a (story 80, normal 60, expert 70"));
    }

    #[test]
    fn the_objectives_warn_and_do_not_fail() {
        // Too short a plan (T3), Story too close to Normal and Normal over the bound (T4).
        let report = sites(well_behaved("a", [95, 92, 86, 80], 1));
        let findings = check(&targets(), &[report]);
        let objectives: Vec<&Finding> = findings
            .iter()
            .filter(|f| matches!(f.id, "T3" | "T4") && f.level == Level::O)
            .collect();
        assert_eq!(objectives.len(), 3);
        assert!(objectives.iter().all(|f| !f.ok && !f.fails()));
        assert!(findings.iter().all(|f| !f.fails()));
    }

    #[test]
    fn a_site_measured_in_several_reports_is_named_once() {
        // `neon-sim solve` writes one report per difficulty: the rows of a site are interleaved.
        let reports: Vec<Report> = well_behaved("a", [95, 92, 86, 80], 5)
            .into_iter()
            .map(|r| sites(vec![r]))
            .collect();
        let findings = check(&targets(), &reports);
        let flat = find(&findings, "T4", 1);
        assert_eq!(flat.detail.matches("a (95 against 92)").count(), 1, "{}", flat.detail);
    }

    #[test]
    fn nothing_to_measure_gives_no_finding() {
        assert!(check(&targets(), &[]).is_empty());
    }
}
