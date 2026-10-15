//! `neon-sim`: the command line of the balancing harness.
//!
//! ```text
//! neon-sim run   --difficulty all --seeds 1..=3 --plans representative --out target/sim/optimist.json
//! neon-sim solve --out target/sim/sites.json
//! neon-sim check target/sim/optimist.json target/sim/sites.json
//! ```

use std::fs;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use neon_engine::campaign::Difficulty;
use neon_engine::content::Content;
use neon_sim::campaign::{OPTIMIST, PlanSet, plans, run_optimist};
use neon_sim::record::difficulty_name;
use neon_sim::report::{CampaignReport, Report, SCHEMA_VERSION, SitesReport};
use neon_sim::sites::{DEFAULT_EXPANSIONS, SiteResult, measure};
use neon_sim::targets::{Level, Targets, check};

#[derive(Parser)]
#[command(name = "neon-sim", version, about = "Balancing harness of Neon Hack")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Play the optimist through the campaign and report what it met.
    Run(RunArgs),
    /// Measure the sites with the solver: can each be won, with what margin.
    Solve(SolveArgs),
    /// Check reports against the thresholds; exits with 1 when a barrier fails.
    Check(CheckArgs),
}

#[derive(Args)]
struct RunArgs {
    /// The difficulty, or `all`.
    #[arg(long, value_enum, default_value = "all")]
    difficulty: Which,
    /// The seeds of the games: `7` or `1..=20`.
    #[arg(long, default_value = "1", value_parser = parse_seeds)]
    seeds: RangeInclusive<u64>,
    /// Which decisions the optimist takes.
    #[arg(long, value_enum, default_value = "representative")]
    plans: Plans,
    /// Only the first plans of the set (a quick check).
    #[arg(long)]
    limit: Option<usize>,
    /// Where to write the report; with `all` difficulties, one file per difficulty with its
    /// name before the extension. Without it, the summary is the only output.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct SolveArgs {
    /// Only this site.
    #[arg(long)]
    site: Option<String>,
    /// The difficulty, or `all`.
    #[arg(long, value_enum, default_value = "all")]
    difficulty: Which,
    /// The expansions the solver may spend on each measure.
    #[arg(long, default_value_t = DEFAULT_EXPANSIONS)]
    expansions: u64,
    /// Where to write the report.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct CheckArgs {
    /// The thresholds; the ones shipped with the harness by default.
    #[arg(long)]
    targets: Option<PathBuf>,
    /// The reports to check.
    #[arg(required = true)]
    reports: Vec<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Which {
    All,
    Story,
    Normal,
    Expert,
    Hardcore,
}

impl Which {
    fn difficulties(self) -> Vec<Difficulty> {
        match self {
            Self::All => Difficulty::ALL.to_vec(),
            Self::Story => vec![Difficulty::Story],
            Self::Normal => vec![Difficulty::Normal],
            Self::Expert => vec![Difficulty::Expert],
            Self::Hardcore => vec![Difficulty::Hardcore],
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Plans {
    All,
    Representative,
}

fn parse_seeds(text: &str) -> Result<RangeInclusive<u64>, String> {
    let number = |part: &str| {
        part.trim()
            .parse::<u64>()
            .map_err(|_| format!("`{part}` is not a seed"))
    };
    let (first, last) = if let Some((first, last)) = text.split_once("..=") {
        (number(first)?, number(last)?)
    } else {
        let seed = number(text)?;
        (seed, seed)
    };
    if first > last {
        return Err(format!("`{text}` is an empty range of seeds"));
    }
    Ok(first..=last)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Run(args) => run(&args),
        Command::Solve(args) => solve(&args),
        Command::Check(args) => check_reports(&args),
    };
    match result {
        Ok(code) => code,
        Err(message) => {
            console::error(&message);
            ExitCode::from(2)
        }
    }
}

fn run(args: &RunArgs) -> Result<ExitCode, String> {
    let set = match args.plans {
        Plans::All => PlanSet::All,
        Plans::Representative => PlanSet::Representative,
    };
    let mut plans = plans(set).map_err(|e| format!("the game content does not load: {e}"))?;
    if let Some(limit) = args.limit {
        plans.truncate(limit);
    }
    let difficulties = args.difficulty.difficulties();
    let many = difficulties.len() > 1;
    let mut all_finished = true;
    for difficulty in difficulties {
        let (report, _) = run_optimist(&plans, difficulty, &args.seeds);
        console::line(&summary(&report));
        all_finished &= report.stuck == 0;
        if let Some(out) = &args.out {
            let path = if many {
                with_suffix(out, difficulty_name(difficulty))
            } else {
                out.clone()
            };
            write(&path, &Report::Campaign(report))?;
        }
    }
    Ok(if all_finished {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn solve(args: &SolveArgs) -> Result<ExitCode, String> {
    let content =
        Content::embedded().map_err(|e| format!("the game content does not load: {e}"))?;
    let difficulty = match args.difficulty {
        Which::All => None,
        other => other.difficulties().first().copied(),
    };
    let rows = measure(&content, args.site.as_deref(), difficulty, args.expansions);
    if rows.is_empty() {
        return Err("no such site".to_owned());
    }
    for row in &rows {
        console::line(&format!(
            "{} {} (tier {}, cap {}): {}",
            row.site,
            row.difficulty,
            row.tier,
            row.cap,
            match &row.result {
                SiteResult::Solved {
                    trace,
                    margin,
                    turns,
                    optimal,
                    expanded,
                } => format!(
                    "won with Trace {trace}, margin {margin}, {turns} turns{} ({expanded} states)",
                    if *optimal { "" } else { ", best found" }
                ),
                SiteResult::Unsolvable => "cannot be won".to_owned(),
                SiteResult::Unknown { expanded } => format!("unknown after {expanded} states"),
            }
        ));
    }
    if let Some(out) = &args.out {
        write(
            out,
            &Report::Sites(SitesReport {
                schema_version: SCHEMA_VERSION,
                expansions: args.expansions,
                rows,
            }),
        )?;
    }
    Ok(ExitCode::SUCCESS)
}

fn check_reports(args: &CheckArgs) -> Result<ExitCode, String> {
    let targets = match &args.targets {
        Some(path) => {
            let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            Targets::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => Targets::embedded().map_err(|e| format!("the thresholds do not load: {e}"))?,
    };
    let mut reports = Vec::new();
    for path in &args.reports {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        reports.push(Report::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    let findings = check(&targets, &reports);
    if findings.is_empty() {
        return Err("the reports give nothing to check".to_owned());
    }
    let mut failed = false;
    for finding in &findings {
        let mark = match (finding.ok, finding.level) {
            (true, _) => "ok  ",
            (false, Level::P) => "FAIL",
            (false, Level::O) => "warn",
        };
        console::line(&format!(
            "[{mark}] {} ({:?}): {}",
            finding.id, finding.level, finding.detail
        ));
        failed |= finding.fails();
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// A line that sums up a campaign report.
fn summary(report: &CampaignReport) -> String {
    let endings: Vec<String> = report
        .endings
        .iter()
        .map(|(ending, n)| format!("{ending} {n}"))
        .collect();
    let commands = report.commands.map_or_else(String::new, |s| {
        format!(", {} commands (median {})", s.max, s.median)
    });
    format!(
        "{OPTIMIST} {}: {} games, {} finished, {} stuck{commands}; endings: {}",
        report.difficulty,
        report.games,
        report.finished,
        report.stuck,
        if endings.is_empty() {
            "none".to_owned()
        } else {
            endings.join(", ")
        }
    )
}

/// `out.json` and `story` give `out-story.json`.
fn with_suffix(path: &std::path::Path, suffix: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let name = match path.extension() {
        Some(ext) => format!("{stem}-{suffix}.{}", ext.to_string_lossy()),
        None => format!("{stem}-{suffix}"),
    };
    path.with_file_name(name)
}

fn write(path: &std::path::Path, report: &Report) -> Result<(), String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let json = report
        .to_json()
        .map_err(|e| format!("the report does not serialize: {e}"))?;
    fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))
}

/// Console output of the harness.
mod console {
    #![allow(clippy::print_stdout, clippy::print_stderr)]

    pub(super) fn line(text: &str) {
        println!("{text}");
    }

    pub(super) fn error(text: &str) {
        eprintln!("neon-sim: {text}");
    }
}
