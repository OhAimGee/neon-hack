//! The thresholds a report is checked against (`docs/spec/neon-sim.md` § 5).
//!
//! They are data (`data/sim/targets.toml`), never numbers in the code. A barrier (`P`) fails the
//! check; an objective (`O`) is reported and does not fail it, until the owner validates its
//! value and it is promoted.

use serde::Deserialize;

use crate::bench::BenchReport;
use crate::campaign::{FARMER, HOARDER, OPTIMIST, PRUDENT, RECKLESS, STORY};
use crate::economy::EconomyReport;
use crate::programs::ProgramsReport;
use crate::record::difficulty_name;
use crate::report::{CampaignReport, MonkeyReport, Report, SitesReport, Spread};
use crate::sites::{Loot, SiteResult, SiteRow};
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
    /// The difficulty the plans are measured at: the reference one, the others scale from it.
    pub difficulty: String,
    /// The first tier the length applies to: the first two tiers are the tutorial, short on
    /// purpose.
    pub from_tier: u8,
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
    /// The first tier the gap and the bounds apply to: the first two tiers are the tutorial,
    /// meant to be short and forgiving. The order between difficulties holds for every site.
    pub from_tier: u8,
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

/// T5: how a reasonable player fares at Normal.
#[derive(Debug, Clone, Deserialize)]
pub struct Reasonable {
    /// P or O.
    pub level: Level,
    /// The share of campaigns the prudent bot finishes, per mille, at least.
    pub finished_per_mille_min: u64,
    /// The share of its intrusions that burn, per mille, at least.
    pub burned_per_mille_min: u64,
    /// The share of its intrusions that burn, per mille, at most.
    pub burned_per_mille_max: u64,
    /// The most burned runs in a row on one site.
    pub longest_streak_max: u64,
}

/// T6: how a reckless player fares: the floor of the difficulty.
#[derive(Debug, Clone, Deserialize)]
pub struct Floor {
    /// P or O.
    pub level: Level,
    /// The share of campaigns the reckless bot finishes at Expert, per mille, at most.
    pub expert_finished_per_mille_max: u64,
    /// The share of campaigns it finishes in Story, per mille, at least.
    pub story_finished_per_mille_min: u64,
}

/// T10: how long a campaign lasts for the prudent bot, at Normal.
#[derive(Debug, Clone, Deserialize)]
pub struct Duration {
    /// P or O.
    pub level: Level,
    /// The fewest command lines of a finished campaign (the median).
    pub commands_min: u64,
    /// The most command lines of a finished campaign (the median).
    pub commands_max: u64,
    /// The most of the steps of a game spent inside an intrusion, per mille (the worst game).
    pub in_intrusion_per_mille_max: u64,
}

/// T11: the Notoriety of the prudent and of the reckless bot, at Normal.
#[derive(Debug, Clone, Deserialize)]
pub struct Heat {
    /// P or O.
    pub level: Level,
    /// The highest Notoriety of a game the prudent bot may reach, at the median.
    pub prudent_median_max: u64,
    /// The share of the prudent bot's games that reach the Hunted band, per mille, at most.
    pub prudent_hunted_per_mille_max: u64,
    /// The share of the reckless bot's games that reach the Hunted band, per mille, at least.
    pub reckless_hunted_per_mille_min: u64,
}

/// T7: the economy: what the stall costs against what a campaign pays, what a tier pays against
/// the next item, and what cannot be done without.
#[derive(Debug, Clone, Deserialize)]
pub struct Economy {
    /// P or O.
    pub level: Level,
    /// The stall, as a percentage of the most a campaign pays, at least.
    pub catalogue_percent_min: u64,
    /// The stall, as a percentage of the most a campaign pays, at most.
    pub catalogue_percent_max: u64,
    /// The share of campaigns the hoarder (who buys nothing) finishes, per mille, at most.
    pub hoarder_finished_per_mille_max: u64,
}

/// T9: no program is dominated.
#[derive(Debug, Clone, Deserialize)]
pub struct Programs {
    /// P or O.
    pub level: Level,
}

/// T8: repeating things earns nothing.
#[derive(Debug, Clone, Deserialize)]
pub struct AntiFarm {
    /// P or O.
    pub level: Level,
    /// The fewest lines a farmer must have sent again in a game for the check to mean anything.
    pub min_repeated: u64,
}

/// T12: the monkey breaks nothing.
#[derive(Debug, Clone, Deserialize)]
pub struct Robustness {
    /// P or O.
    pub level: Level,
    /// The fewest games a report must hold to say anything (the quick check of the spec).
    pub min_games: u64,
}

/// T13: the harness is fast enough to be run often, and deterministic.
#[derive(Debug, Clone, Deserialize)]
pub struct Performance {
    /// P or O.
    pub level: Level,
    /// What a thousand campaigns may take, in milliseconds.
    pub thousand_campaigns_millis_max: u64,
    /// What the bounded resolution of a site may take, in microseconds, in the worst case.
    pub resolution_micros_max: u64,
    /// Whether two runs must give the same bytes.
    pub deterministic: bool,
}

/// T14: the Story difficulty is played to its end by a player who only skips what lasts.
#[derive(Debug, Clone, Deserialize)]
pub struct StoryMode {
    /// P or O.
    pub level: Level,
    /// The share of campaigns the story bot finishes, per mille, at least.
    pub finished_per_mille_min: u64,
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
    /// T5.
    pub t5: Reasonable,
    /// T6.
    pub t6: Floor,
    /// T7.
    pub t7: Economy,
    /// T8.
    pub t8: AntiFarm,
    /// T9.
    pub t9: Programs,
    /// T10.
    pub t10: Duration,
    /// T11.
    pub t11: Heat,
    /// T12.
    pub t12: Robustness,
    /// T13.
    pub t13: Performance,
    /// T14.
    pub t14: StoryMode,
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
            Report::Campaign(_)
            | Report::Sites(_)
            | Report::Monkey(_)
            | Report::Economy(_)
            | Report::Programs(_) => None,
        })
        .collect();
    let all_campaigns: Vec<&CampaignReport> = reports
        .iter()
        .filter_map(|r| match r {
            Report::Campaign(c) => Some(c),
            Report::Sites(_) | Report::Monkey(_) | Report::Economy(_) | Report::Programs(_) => None,
        })
        .collect();
    let sites: Vec<&SitesReport> = reports
        .iter()
        .filter_map(|r| match r {
            Report::Sites(s) => Some(s),
            Report::Campaign(_) | Report::Monkey(_) | Report::Economy(_) | Report::Programs(_) => {
                None
            }
        })
        .collect();
    let monkeys: Vec<&MonkeyReport> = reports
        .iter()
        .filter_map(|r| match r {
            Report::Monkey(m) => Some(m),
            Report::Campaign(_) | Report::Sites(_) | Report::Economy(_) | Report::Programs(_) => {
                None
            }
        })
        .collect();
    let economies: Vec<&EconomyReport> = reports
        .iter()
        .filter_map(|r| match r {
            Report::Economy(e) => Some(e),
            Report::Campaign(_) | Report::Sites(_) | Report::Monkey(_) | Report::Programs(_) => {
                None
            }
        })
        .collect();
    let analyses: Vec<&ProgramsReport> = reports
        .iter()
        .filter_map(|r| match r {
            Report::Programs(p) => Some(p),
            Report::Campaign(_) | Report::Sites(_) | Report::Monkey(_) | Report::Economy(_) => None,
        })
        .collect();
    let mut findings = Vec::new();
    if !campaigns.is_empty() {
        findings.extend(t1(&targets.t1, &campaigns));
    }
    findings.extend(bots(targets, &all_campaigns));
    findings.extend(t8(&targets.t8, &all_campaigns));
    findings.extend(t12(&targets.t12, &monkeys));
    findings.extend(t7(&targets.t7, &economies, &all_campaigns));
    findings.extend(t9(&targets.t9, &analyses));
    let all_rows: Vec<&SiteRow> = sites.iter().flat_map(|s| &s.rows).collect();
    let (rows, full): (Vec<&SiteRow>, Vec<&SiteRow>) = all_rows
        .into_iter()
        .partition(|row| row.loot == Loot::Objective);
    if !rows.is_empty() {
        findings.push(t2(&targets.t2, &rows));
        findings.push(t3(&targets.t3, &rows));
        findings.extend(t4(&targets.t4, &rows));
    }
    if !full.is_empty() {
        findings.push(t2_full_loot(&targets.t2, &full));
    }
    findings
}

/// The report of a bot at a difficulty, if the caller gave one.
fn report_of<'a>(
    campaigns: &[&'a CampaignReport],
    bot: &str,
    difficulty: &str,
) -> Option<&'a CampaignReport> {
    campaigns
        .iter()
        .find(|c| c.bot == bot && c.difficulty == difficulty)
        .copied()
}

/// The share of the games a bot finished, per mille.
fn finished_per_mille(report: &CampaignReport) -> u64 {
    (report.finished * 1000)
        .checked_div(report.games)
        .unwrap_or(0)
}

/// T13: what the timings of `neon-sim bench` say.
#[must_use]
pub fn judge_bench(target: &Performance, report: &BenchReport) -> Vec<Finding> {
    let mut findings = vec![
        claim(
            "T13",
            target.level,
            report.thousand_campaigns_millis <= target.thousand_campaigns_millis_max,
            format!(
                "a thousand campaigns would take {} ms (mean of {} games, {} ms allowed)",
                report.thousand_campaigns_millis,
                report.games,
                target.thousand_campaigns_millis_max
            ),
        ),
        claim(
            "T13",
            target.level,
            report.resolution_worst_micros <= target.resolution_micros_max,
            format!(
                "the bounded resolution of a site takes {} us at the worst, at {} ({} us allowed, {} measures)",
                report.resolution_worst_micros,
                report.resolution_worst_at,
                target.resolution_micros_max,
                report.resolutions
            ),
        ),
    ];
    if target.deterministic {
        findings.push(claim(
            "T13",
            target.level,
            report.deterministic,
            format!(
                "two runs of the same games give {} bytes",
                if report.deterministic {
                    "the same"
                } else {
                    "different"
                }
            ),
        ));
    }
    findings
}

/// A verdict on one claim.
fn claim(id: &'static str, level: Level, ok: bool, detail: String) -> Finding {
    Finding {
        id,
        level,
        ok,
        detail,
    }
}

/// T5, T6, T10, T11 and T14: what the prudent, the reckless and the story bot go through.
fn bots(targets: &Targets, campaigns: &[&CampaignReport]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let prudent = report_of(campaigns, PRUDENT, "normal");
    let reckless_normal = report_of(campaigns, RECKLESS, "normal");
    if let Some(report) = prudent {
        let t = &targets.t5;
        let finished = finished_per_mille(report);
        findings.push(claim(
            "T5",
            t.level,
            finished >= t.finished_per_mille_min,
            format!(
                "prudent at Normal finishes {finished} per mille of its campaigns ({} wanted)",
                t.finished_per_mille_min
            ),
        ));
        let burned = report.intrusions.burned_per_mille;
        findings.push(claim(
            "T5",
            t.level,
            (t.burned_per_mille_min..=t.burned_per_mille_max).contains(&burned),
            format!(
                "prudent at Normal burns {burned} per mille of its {} intrusions ({} to {} wanted)",
                report.intrusions.total, t.burned_per_mille_min, t.burned_per_mille_max
            ),
        ));
        let streak = report.intrusions.longest_burn_streak;
        findings.push(claim(
            "T5",
            t.level,
            streak <= t.longest_streak_max,
            format!(
                "prudent at Normal burns at most {streak} runs in a row on a site ({} allowed)",
                t.longest_streak_max
            ),
        ));
        let d = &targets.t10;
        if let Some(commands) = report.commands {
            findings.push(claim(
                "T10",
                d.level,
                (d.commands_min..=d.commands_max).contains(&commands.median),
                format!(
                    "a prudent campaign at Normal takes {} command lines at the median ({} to {} wanted)",
                    commands.median, d.commands_min, d.commands_max
                ),
            ));
        }
        if let Some(share) = report.in_intrusion {
            findings.push(claim(
                "T10",
                d.level,
                share.max <= d.in_intrusion_per_mille_max,
                format!(
                    "at most {} per mille of a prudent game is spent in an intrusion ({} allowed)",
                    share.max, d.in_intrusion_per_mille_max
                ),
            ));
        }
        let h = &targets.t11;
        if let Some(notoriety) = report.max_notoriety {
            findings.push(claim(
                "T11",
                h.level,
                notoriety.median <= h.prudent_median_max,
                format!(
                    "the prudent bot's highest Notoriety is {} at the median ({} allowed)",
                    notoriety.median, h.prudent_median_max
                ),
            ));
        }
        findings.push(claim(
            "T11",
            h.level,
            report.hunted_per_mille <= h.prudent_hunted_per_mille_max,
            format!(
                "the prudent bot is Hunted in {} per mille of its games ({} allowed)",
                report.hunted_per_mille, h.prudent_hunted_per_mille_max
            ),
        ));
    }
    if let Some(report) = reckless_normal {
        let h = &targets.t11;
        findings.push(claim(
            "T11",
            h.level,
            report.hunted_per_mille >= h.reckless_hunted_per_mille_min,
            format!(
                "the reckless bot is Hunted in {} per mille of its games ({} wanted)",
                report.hunted_per_mille, h.reckless_hunted_per_mille_min
            ),
        ));
    }
    if let Some(report) = report_of(campaigns, STORY, "story") {
        let t = &targets.t14;
        let finished = finished_per_mille(report);
        findings.push(claim(
            "T14",
            t.level,
            finished >= t.finished_per_mille_min,
            format!(
                "story finishes {finished} per mille of its campaigns, skipping {} per mille of its {} intrusions ({} wanted)",
                report.intrusions.automatic_per_mille,
                report.intrusions.total,
                t.finished_per_mille_min
            ),
        ));
    }
    let t = &targets.t6;
    if let Some(report) = report_of(campaigns, RECKLESS, "expert") {
        let finished = finished_per_mille(report);
        findings.push(claim(
            "T6",
            t.level,
            finished <= t.expert_finished_per_mille_max,
            format!(
                "reckless at Expert finishes {finished} per mille of its campaigns ({} allowed)",
                t.expert_finished_per_mille_max
            ),
        ));
    }
    if let Some(report) = report_of(campaigns, RECKLESS, "story") {
        let finished = finished_per_mille(report);
        findings.push(claim(
            "T6",
            t.level,
            finished >= t.story_finished_per_mille_min,
            format!(
                "reckless in Story finishes {finished} per mille of its campaigns ({} wanted)",
                t.story_finished_per_mille_min
            ),
        ));
    }
    findings
}

/// T7, read in four ways: the stall against the revenues of the campaign, each tier against
/// the next item, whether everything can be bought, and whether the hoarder can do without.
fn t7(
    target: &Economy,
    economies: &[&EconomyReport],
    campaigns: &[&CampaignReport],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    // The revenues of a campaign do not depend on the difficulty: the optimist's tell them.
    let optimist: Vec<&&CampaignReport> = campaigns.iter().filter(|c| c.bot == OPTIMIST).collect();
    let most = optimist
        .iter()
        .filter_map(|c| c.credits.map(|s| s.max))
        .max();
    let least = optimist
        .iter()
        .filter_map(|c| c.credits.map(|s| s.min))
        .min();
    for economy in economies {
        let total = u64::from(economy.catalogue_total);
        if let Some(most) = most.filter(|most| *most > 0) {
            let percent = total * 100 / most;
            findings.push(claim(
                "T7",
                target.level,
                (target.catalogue_percent_min..=target.catalogue_percent_max).contains(&percent),
                format!(
                    "the stall costs {total} credits, {percent} % of the {most} a campaign pays at most ({} to {} % wanted)",
                    target.catalogue_percent_min, target.catalogue_percent_max
                ),
            ));
        }
        if let Some(least) = least {
            findings.push(claim(
                "T7",
                target.level,
                total > least,
                format!(
                    "the stall costs {total} credits and a campaign pays {least} at the least: {}",
                    if total > least {
                        "not everything can be bought"
                    } else {
                        "everything can be bought"
                    }
                ),
            ));
        }
        let short: Vec<String> = economy
            .tiers
            .iter()
            .filter_map(|tier| {
                let next = economy
                    .items
                    .iter()
                    .filter(|item| item.tier == tier.tier + 1)
                    .map(|item| item.price)
                    .max()?;
                (tier.main_credits < next)
                    .then(|| format!("tier {} pays {} for {next}", tier.tier, tier.main_credits))
            })
            .collect();
        findings.push(claim(
            "T7",
            target.level,
            short.is_empty(),
            if short.is_empty() {
                "the main quests of every tier pay for the item of the next".to_owned()
            } else {
                format!(
                    "the main quests of a tier do not pay for the item of the next: {}",
                    short.join(", ")
                )
            },
        ));
    }
    for hoarder in campaigns.iter().filter(|c| c.bot == HOARDER) {
        let finished = finished_per_mille(hoarder);
        findings.push(claim(
            "T7",
            target.level,
            finished <= target.hoarder_finished_per_mille_max,
            format!(
                "the hoarder finishes {finished} per mille of its campaigns at {} and is blocked in {} ({} allowed)",
                hoarder.difficulty, hoarder.blocked, target.hoarder_finished_per_mille_max
            ),
        ));
    }
    findings
}

/// T9: every program lowers the margin of some site, or a site cannot be won without it.
fn t9(target: &Programs, analyses: &[&ProgramsReport]) -> Vec<Finding> {
    analyses
        .iter()
        .map(|analysis| {
            let dominated: Vec<&str> = analysis
                .programs
                .iter()
                .filter(|row| row.dominated)
                .map(|row| row.program.as_str())
                .collect();
            claim(
                "T9",
                target.level,
                dominated.is_empty(),
                if dominated.is_empty() {
                    format!(
                        "each of the {} programs lowers the margin of a site, or a site needs it",
                        analysis.programs.len()
                    )
                } else {
                    format!(
                        "{} of {} programs lower the margin of no site and no site needs them: {}",
                        dominated.len(),
                        analysis.programs.len(),
                        dominated.join(", ")
                    )
                },
            )
        })
        .collect()
}

/// Whether a spread is nowhere above another, rank by rank.
fn nowhere_above(spread: &Spread, other: &Spread) -> bool {
    spread.min <= other.min
        && spread.p10 <= other.p10
        && spread.median <= other.median
        && spread.p90 <= other.p90
        && spread.max <= other.max
}

/// T8: at every difficulty both were played, the farmer, who sends again what the plan sent,
/// ends with no more credits and no more reputation than the optimist, on the same games.
fn t8(target: &AntiFarm, campaigns: &[&CampaignReport]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for farmer in campaigns.iter().filter(|c| c.bot == FARMER) {
        let Some(optimist) = report_of(campaigns, OPTIMIST, &farmer.difficulty) else {
            continue;
        };
        let (Some(f_credits), Some(o_credits)) = (&farmer.credits, &optimist.credits) else {
            continue;
        };
        let (Some(f_rep), Some(o_rep)) = (&farmer.reputation, &optimist.reputation) else {
            continue;
        };
        let repeated = farmer.repeated.map_or(0, |r| r.min);
        let same_games = farmer.games == optimist.games && farmer.seeds == optimist.seeds;
        let ok = same_games
            && repeated >= target.min_repeated
            && nowhere_above(f_credits, o_credits)
            && nowhere_above(f_rep, o_rep);
        findings.push(claim(
            "T8",
            target.level,
            ok,
            format!(
                "at {} the farmer sent again {repeated} lines at least per game and ends with {} credits and {} reputation at the median, the optimist with {} and {} ({} games each{})",
                farmer.difficulty,
                f_credits.median,
                f_rep.median,
                o_credits.median,
                o_rep.median,
                farmer.games,
                if same_games { "" } else { "; not the same games" }
            ),
        ));
    }
    findings
}

/// T12: no panic, no game that cannot be left, no save that does not round trip, in enough games.
fn t12(target: &Robustness, reports: &[&MonkeyReport]) -> Vec<Finding> {
    if reports.is_empty() {
        return Vec::new();
    }
    let games: u64 = reports.iter().map(|r| r.games).sum();
    let sum = |wanted: fn(&MonkeyReport) -> u64| reports.iter().map(|r| wanted(r)).sum::<u64>();
    let (panics, stuck, saves, contract) = (
        sum(|r| r.panics),
        sum(|r| r.not_quittable),
        sum(|r| r.save_faults),
        sum(|r| r.contract_faults),
    );
    vec![
        claim(
            "T12",
            target.level,
            games >= target.min_games,
            format!(
                "the monkey played {games} games ({} wanted to say anything)",
                target.min_games
            ),
        ),
        claim(
            "T12",
            target.level,
            panics + stuck + saves + contract == 0,
            format!(
                "the monkey found {panics} panics, {stuck} games that cannot be left, {saves} saves that do not round trip and {contract} steps at odds with the game"
            ),
        ),
    ]
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

/// T2 again, asked the way the campaign asks it: every loot of the site is taken (the files of
/// the quests are on the nodes that are not the objective), with the whole deck of the tier.
fn t2_full_loot(target: &Solvable, rows: &[&SiteRow]) -> Finding {
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
                "every site is won taking all its loot, at every difficulty ({} measures)",
                rows.len()
            )
        } else {
            format!("not won taking all the loot: {}", bad.join(", "))
        },
    }
}

fn t3(target: &Length, rows: &[&SiteRow]) -> Finding {
    let bad: Vec<String> = rows
        .iter()
        .filter(|r| r.difficulty == target.difficulty && r.tier >= target.from_tier)
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
    let mut sites: Vec<(&str, u8)> = Vec::new();
    for row in rows {
        if !sites.iter().any(|(site, _)| *site == row.site) {
            sites.push((row.site.as_str(), row.tier));
        }
    }
    let mut disorder = Vec::new();
    let mut flat = Vec::new();
    let mut outside = Vec::new();
    for (site, tier) in sites {
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
        if tier < target.from_tier {
            continue;
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

    fn row(site: &str, tier: u8, difficulty: &str, margin: u8, turns: u32) -> SiteRow {
        SiteRow {
            site: site.to_owned(),
            tier,
            difficulty: difficulty.to_owned(),
            cap: 100,
            loot: Loot::Objective,
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
        behaving_at_tier(site, 3, margins, turns)
    }

    fn behaving_at_tier(site: &str, tier: u8, margins: [u8; 4], turns: u32) -> Vec<SiteRow> {
        ["story", "normal", "expert", "hardcore"]
            .iter()
            .zip(margins)
            .map(|(difficulty, margin)| row(site, tier, difficulty, margin, turns))
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
        // The same thresholds, all as objectives: a plan too short (T3), Story too close to
        // Normal and Normal over the bound (T4).
        let objectives = Targets::parse(&EMBEDDED.replace("\"P\"", "\"O\"")).unwrap();
        let report = sites(well_behaved("a", [95, 92, 86, 80], 1));
        let findings = check(&objectives, &[report]);
        let warned: Vec<&Finding> = findings
            .iter()
            .filter(|f| matches!(f.id, "T3" | "T4") && f.level == Level::O && !f.ok)
            .collect();
        assert_eq!(warned.len(), 3);
        assert!(warned.iter().all(|f| !f.fails()));
        assert!(findings.iter().all(|f| !f.fails()));
    }

    #[test]
    fn the_shipped_barriers_fail_on_a_site_that_is_too_easy_or_too_short() {
        let report = sites(well_behaved("a", [95, 92, 86, 80], 1));
        let findings = check(&targets(), &[report]);
        assert!(find(&findings, "T3", 0).fails());
        assert!(find(&findings, "T4", 1).fails());
        assert!(find(&findings, "T4", 2).fails());
    }

    #[test]
    fn the_tutorial_is_exempt_from_the_length_the_gap_and_the_bounds_but_not_from_the_order() {
        let easy = behaving_at_tier("tutorial", 2, [95, 92, 86, 80], 1);
        let findings = check(&targets(), &[sites(easy)]);
        assert!(findings.iter().all(|f| f.ok), "{findings:?}");
        let disordered = behaving_at_tier("tutorial", 2, [80, 60, 70, 40], 3);
        let findings = check(&targets(), &[sites(disordered)]);
        assert!(find(&findings, "T4", 0).fails());
    }

    #[test]
    fn only_the_reference_difficulty_is_held_to_the_length() {
        // One turn at Story and at Expert is what the difficulty does; Normal takes four.
        let mut rows = well_behaved("a", [80, 60, 50, 40], 1);
        if let Some(normal) = rows.get_mut(1)
            && let SiteResult::Solved { turns, .. } = &mut normal.result
        {
            *turns = 4;
        }
        let findings = check(&targets(), &[sites(rows)]);
        assert!(
            find(&findings, "T3", 0).ok,
            "{}",
            find(&findings, "T3", 0).detail
        );
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
        assert_eq!(
            flat.detail.matches("a (95 against 92)").count(),
            1,
            "{}",
            flat.detail
        );
    }

    #[test]
    fn nothing_to_measure_gives_no_finding() {
        assert!(check(&targets(), &[]).is_empty());
    }

    // ------------------------------------------------------------------ the bots

    use crate::record::{GameRecord, IntrusionRecord, Outcome};
    use crate::report::CampaignReport;

    /// A campaign of a bot: `finished` games reached an ending, the others were lost.
    fn campaign(
        bot: &str,
        difficulty: &str,
        finished: usize,
        lost: usize,
        burned_per_ten: usize,
        commands: u64,
        notoriety: u8,
    ) -> Report {
        let intrusions = (0..10)
            .map(|n| IntrusionRecord {
                site: format!("site{n}"),
                turns: 4,
                trace: 50,
                burned: n < burned_per_ten,
                automatic: false,
            })
            .collect::<Vec<_>>();
        let game = |outcome| GameRecord {
            plan: "p".to_owned(),
            difficulty: difficulty.to_owned(),
            seed: 1,
            outcome,
            commands,
            credits: 0,
            reputation: 0,
            max_notoriety: notoriety,
            intrusion_per_mille: 200,
            intrusions: intrusions.clone(),
            repeated: None,
        };
        let records: Vec<GameRecord> = (0..finished)
            .map(|_| {
                game(Outcome::Finished {
                    ending: "e2".to_owned(),
                })
            })
            .chain((0..lost).map(|_| {
                game(Outcome::Lost {
                    site: "x".to_owned(),
                })
            }))
            .collect();
        Report::Campaign(CampaignReport::of(
            bot,
            difficulty,
            [1, 1],
            records.len() as u64,
            &records,
        ))
    }

    #[test]
    fn a_prudent_bot_inside_every_bound_passes_t5_t10_and_t11() {
        let reports = [
            // 2 of 10 intrusions burn, 2000 commands, Notoriety 40: all inside.
            campaign("prudent", "normal", 20, 0, 2, 2000, 40),
            campaign("reckless", "normal", 20, 0, 5, 1000, 90),
            campaign("reckless", "story", 20, 0, 0, 1000, 60),
            campaign("reckless", "expert", 1, 19, 8, 1000, 90),
        ];
        let findings = check(&targets(), &reports);
        assert!(!findings.is_empty());
        let bad: Vec<_> = findings.iter().filter(|f| !f.ok).collect();
        assert!(bad.is_empty(), "{bad:?}");
        let ids: Vec<&str> = findings.iter().map(|f| f.id).collect();
        assert_eq!(ids.iter().filter(|id| **id == "T5").count(), 3);
        assert_eq!(ids.iter().filter(|id| **id == "T6").count(), 2);
        assert_eq!(ids.iter().filter(|id| **id == "T10").count(), 2);
        assert_eq!(ids.iter().filter(|id| **id == "T11").count(), 3);
    }

    #[test]
    fn a_bot_outside_the_bounds_is_reported_by_claim_and_none_of_them_fails_the_check() {
        let reports = [
            // Never burns, 500 commands, always Hunted: every claim about the prudent bot fails.
            campaign("prudent", "normal", 20, 0, 0, 500, 100),
            // The reckless bot is never Hunted and finishes Expert.
            campaign("reckless", "normal", 20, 0, 5, 1000, 10),
            campaign("reckless", "expert", 20, 0, 0, 1000, 10),
        ];
        let findings = check(&targets(), &reports);
        let failed: Vec<(&str, &str)> = findings
            .iter()
            .filter(|f| !f.ok)
            .map(|f| (f.id, f.detail.as_str()))
            .collect();
        for needle in [
            "burns 0 per mille",
            "500 command lines",
            "highest Notoriety is 100",
            "prudent bot is Hunted in 1000",
            "reckless bot is Hunted in 0",
            "reckless at Expert finishes 1000",
        ] {
            assert!(
                failed.iter().any(|(_, detail)| detail.contains(needle)),
                "{needle} is not among {failed:?}"
            );
        }
        assert!(findings.iter().all(|f| !f.fails()), "they are objectives");
    }

    #[test]
    fn the_story_bot_is_read_at_story_only_and_says_how_much_it_skipped() {
        let reports = [
            campaign("story", "story", 20, 0, 0, 600, 20),
            campaign("story", "story", 10, 10, 0, 600, 20),
        ];
        let findings = check(&targets(), &reports[..1]);
        assert_eq!(findings.len(), 1);
        let finding = findings.first().unwrap();
        assert_eq!((finding.id, finding.ok), ("T14", true));
        assert!(
            finding.detail.contains("1000 per mille"),
            "{}",
            finding.detail
        );
        assert!(
            finding.detail.contains("skipping 0 per mille"),
            "{}",
            finding.detail
        );
        // Half of the games lost: the objective is not met, and it stays an objective.
        let findings = check(&targets(), &reports[1..]);
        assert!(
            findings
                .iter()
                .all(|f| f.id == "T14" && !f.ok && !f.fails())
        );
        // The story bot at another difficulty is read by nothing.
        assert!(
            check(
                &targets(),
                &[campaign("story", "normal", 20, 0, 0, 600, 20)]
            )
            .is_empty()
        );
    }

    // ------------------------------------------------------------------ the farmer

    /// The report of a bot at Normal with the given rewards in every game, and the lines a
    /// farmer sent again.
    fn rewarded(bot: &str, credits: i64, reputation: i64, repeated: Option<u64>) -> Report {
        let game = |seed: u64| GameRecord {
            plan: "p".to_owned(),
            difficulty: "normal".to_owned(),
            seed,
            outcome: Outcome::Finished {
                ending: "e2".to_owned(),
            },
            commands: 600,
            credits,
            reputation,
            max_notoriety: 20,
            intrusion_per_mille: 200,
            intrusions: Vec::new(),
            repeated,
        };
        let records = [game(1), game(2)];
        Report::Campaign(CampaignReport::of(bot, "normal", [1, 2], 1, &records))
    }

    #[test]
    fn a_farmer_that_earns_what_the_optimist_earns_passes_t8() {
        let reports = [
            rewarded("optimist", 18_000, 500, None),
            rewarded("farmer", 18_000, 500, Some(1_200)),
        ];
        let findings: Vec<_> = check(&targets(), &reports)
            .into_iter()
            .filter(|f| f.id == "T8")
            .collect();
        assert_eq!(findings.len(), 1);
        assert!(findings[0].ok, "{}", findings[0].detail);
        assert_eq!(targets().t8.level, Level::P);
    }

    #[test]
    fn a_farmer_that_earns_more_or_repeats_nothing_fails_the_barrier() {
        for (credits, reputation, repeated) in [
            (18_001, 500, Some(1_200)),
            (18_000, 501, Some(1_200)),
            (18_000, 500, Some(0)),
            (18_000, 500, None),
        ] {
            let reports = [
                rewarded("optimist", 18_000, 500, None),
                rewarded("farmer", credits, reputation, repeated),
            ];
            let findings: Vec<_> = check(&targets(), &reports)
                .into_iter()
                .filter(|f| f.id == "T8")
                .collect();
            assert!(
                findings.len() == 1 && findings[0].fails(),
                "{credits} {reputation} {repeated:?}: {findings:?}"
            );
        }
    }

    #[test]
    fn a_farmer_without_an_optimist_to_compare_with_gives_no_finding() {
        let reports = [rewarded("farmer", 18_000, 500, Some(1_200))];
        assert!(check(&targets(), &reports).iter().all(|f| f.id != "T8"));
    }

    // ------------------------------------------------------------------ the economy

    use crate::economy::{ItemRow, TierRow};

    /// An economy of two tiers: the stall sells one item at each.
    fn economy_of(prices: [u32; 2], main_credits: [u32; 2]) -> Report {
        let item = |tier: u8, price: u32| ItemRow {
            id: format!("item{tier}"),
            tier,
            price,
            required_by: vec!["m01".to_owned()],
        };
        let tier = |tier: u8, main: u32| TierRow {
            tier,
            flagship_price: 100,
            main_credits: main,
            all_credits: main,
        };
        Report::Economy(EconomyReport {
            schema_version: SCHEMA_VERSION,
            start_credits: 100,
            catalogue_total: prices[0] + prices[1],
            items: vec![item(1, prices[0]), item(2, prices[1])],
            services: Vec::new(),
            tiers: vec![tier(1, main_credits[0]), tier(2, main_credits[1])],
        })
    }

    fn t7_claims(reports: &[Report]) -> Vec<Finding> {
        check(&targets(), reports)
            .into_iter()
            .filter(|f| f.id == "T7")
            .collect()
    }

    #[test]
    fn an_economy_inside_every_bound_passes_t7() {
        // Revenues of 1000; a stall of 1250 (125 %), more than anyone earns; tier 1 pays 800
        // for the item of 750 at tier 2.
        let reports = [
            economy_of([500, 750], [800, 600]),
            rewarded("optimist", 1_000, 0, None),
            campaign("hoarder", "normal", 0, 0, 0, 0, 0),
        ];
        let claims = t7_claims(&reports);
        assert!(claims.iter().all(|f| f.ok), "{claims:?}");
        assert_eq!(targets().t7.level, Level::O);
    }

    #[test]
    fn a_thin_stall_and_a_tier_that_cannot_pay_the_next_item_are_told_apart() {
        let reports = [
            // 100 + 200 against a campaign of 1000: 30 %, and everything is affordable;
            // tier 1 pays 50 for an item of 200.
            economy_of([100, 200], [50, 600]),
            rewarded("optimist", 1_000, 0, None),
        ];
        let claims = t7_claims(&reports);
        let failed: Vec<&str> = claims
            .iter()
            .filter(|f| !f.ok)
            .map(|f| f.detail.as_str())
            .collect();
        assert_eq!(failed.len(), 3, "{failed:?}");
        assert!(failed.iter().any(|d| d.contains("30 % of the 1000")));
        assert!(
            failed
                .iter()
                .any(|d| d.contains("everything can be bought"))
        );
        assert!(failed.iter().any(|d| d.contains("tier 1 pays 50 for 200")));
        assert!(claims.iter().all(|f| !f.fails()), "they are objectives");
    }

    #[test]
    fn a_hoarder_that_finishes_the_campaign_fails_the_claim_that_it_cannot() {
        let reports = [campaign("hoarder", "normal", 5, 0, 0, 500, 20)];
        let claims = t7_claims(&reports);
        assert_eq!(claims.len(), 1);
        assert!(!claims[0].ok, "{}", claims[0].detail);
    }

    // ------------------------------------------------------------------ the programs

    use crate::programs::ProgramRow;

    fn analysis(rows: &[(&str, bool)]) -> Report {
        Report::Programs(ProgramsReport {
            schema_version: SCHEMA_VERSION,
            difficulty: "normal".to_owned(),
            expansions: 1,
            sites: Vec::new(),
            programs: rows
                .iter()
                .map(|(program, dominated)| ProgramRow {
                    program: (*program).to_owned(),
                    price: None,
                    needed_at: Vec::new(),
                    lowers: if *dominated {
                        Vec::new()
                    } else {
                        vec![("localhost".to_owned(), 2)]
                    },
                    idle_at: Vec::new(),
                    total_gain: if *dominated { 0 } else { 2 },
                    dominated: *dominated,
                })
                .collect(),
        })
    }

    #[test]
    fn programs_that_all_lower_a_margin_pass_t9_and_the_dominated_ones_are_named() {
        let good = check(
            &targets(),
            &[analysis(&[("spoof", false), ("cloak", false)])],
        );
        assert_eq!(good.len(), 1);
        assert!(good[0].ok && good[0].id == "T9", "{}", good[0].detail);
        assert!(good[0].detail.contains("each of the 2 programs"));
        let bad = check(
            &targets(),
            &[analysis(&[
                ("spoof", false),
                ("exploit", true),
                ("social", true),
            ])],
        );
        assert!(!bad[0].ok && !bad[0].fails(), "an objective");
        assert!(
            bad[0].detail.contains("2 of 3 programs") && bad[0].detail.contains("exploit, social"),
            "{}",
            bad[0].detail
        );
        assert_eq!(targets().t9.level, Level::O);
    }

    // ------------------------------------------------------------------ the monkey

    use crate::monkey::{Fault, MonkeyRecord};

    fn monkey(games: u64, fault: Option<&Fault>) -> Report {
        let records: Vec<MonkeyRecord> = (0..games)
            .map(|seed| MonkeyRecord {
                seed,
                difficulty: "normal".to_owned(),
                inputs: 300,
                intrusion_inputs: 150,
                damaged_refused: 6,
                damaged_accepted: 0,
                fault: if seed == 0 { fault.cloned() } else { None },
                trail: Vec::new(),
            })
            .collect();
        Report::Monkey(MonkeyReport::of([0, games], 300, &records))
    }

    #[test]
    fn a_monkey_that_found_nothing_in_enough_games_passes_t12() {
        let findings = check(&targets(), &[monkey(100, None)]);
        assert_eq!(findings.len(), 2);
        assert!(
            findings.iter().all(|f| f.id == "T12" && f.ok),
            "{findings:?}"
        );
        assert_eq!(targets().t12.level, Level::P);
    }

    #[test]
    fn too_few_games_or_one_fault_fails_the_barrier() {
        let few = check(&targets(), &[monkey(99, None)]);
        assert!(
            few.iter()
                .any(|f| !f.ok && f.fails() && f.detail.contains("99 games"))
        );
        for fault in [
            Fault::Panic {
                message: "boom".to_owned(),
            },
            Fault::NotQuittable {
                why: "stuck".to_owned(),
            },
            Fault::Save {
                why: "differs".to_owned(),
            },
            Fault::Contract {
                why: "differs".to_owned(),
            },
        ] {
            let findings = check(&targets(), &[monkey(100, Some(&fault))]);
            assert!(findings.iter().any(Finding::fails), "{findings:?}");
        }
    }

    #[test]
    fn a_bot_nobody_asked_about_gives_no_finding() {
        // Prudent at Expert: no threshold reads it.
        let reports = [campaign("prudent", "expert", 0, 5, 8, 0, 100)];
        assert!(check(&targets(), &reports).is_empty());
    }
    fn timings(thousand_millis: u64, worst_micros: u64, deterministic: bool) -> BenchReport {
        BenchReport {
            games: 8,
            campaigns_total_micros: thousand_millis * 8,
            campaign_median_micros: thousand_millis,
            campaign_worst_micros: thousand_millis,
            thousand_campaigns_millis: thousand_millis,
            resolutions: 64,
            resolution_worst_micros: worst_micros,
            resolution_worst_at: "corp-server-01 at normal".to_owned(),
            deterministic,
        }
    }

    #[test]
    fn fast_and_deterministic_timings_pass_t13() {
        let target = targets().t13;
        let findings = judge_bench(&target, &timings(59_999, 50_000, true));
        assert_eq!(findings.len(), 3);
        assert!(
            findings.iter().all(|f| f.id == "T13" && f.ok),
            "{findings:?}"
        );
    }

    #[test]
    fn slow_timings_or_different_bytes_fail_t13() {
        let target = targets().t13;
        let slow = judge_bench(&target, &timings(60_001, 50_001, true));
        assert_eq!(slow.iter().filter(|f| !f.ok).count(), 2, "{slow:?}");
        let drifting = judge_bench(&target, &timings(1, 1, false));
        assert!(
            drifting
                .iter()
                .any(|f| !f.ok && f.detail.contains("different")),
            "{drifting:?}"
        );
        // An objective warns, a barrier would fail: the level is the data's.
        assert_eq!(target.level, Level::O);
        assert!(slow.iter().all(|f| !f.fails()));
    }
}
