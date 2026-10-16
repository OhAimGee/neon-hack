//! `neon-sim`: the command line of the balancing harness.
//!
//! ```text
//! neon-sim run   --difficulty all --seeds 1..=3 --plans representative --out target/sim/optimist.json
//! neon-sim solve --out target/sim/sites.json
//! neon-sim economy --out target/sim/economy.json
//! neon-sim check target/sim/optimist.json target/sim/sites.json target/sim/economy.json
//! ```

use std::fs;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use neon_engine::campaign::Difficulty;
use neon_engine::content::Content;
use neon_sim::bench::bench;
use neon_sim::campaign::{Bot, PlanSet, plans, run_bot};
use neon_sim::economy::measure as measure_economy;
use neon_sim::monkey::{DEFAULT_STEPS, play_monkey};
use neon_sim::programs::analyse as analyse_programs;
use neon_sim::record::difficulty_name;
use neon_sim::report::{CampaignReport, MonkeyReport, Report, SCHEMA_VERSION, SitesReport};
use neon_sim::sites::{DEFAULT_EXPANSIONS, Loot, SiteResult, measure};
use neon_sim::targets::{Finding, Level, Targets, check, judge_bench};

#[derive(Parser)]
#[command(name = "neon-sim", version, about = "Balancing harness of Neon Hack")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Play a bot through the campaign and report what it met.
    Run(RunArgs),
    /// Measure the sites with the solver: can each be won, with what margin.
    Solve(SolveArgs),
    /// What the stall sells and the quests pay: the economy of the content.
    Economy(EconomyArgs),
    /// What each program is worth: the margin of every site with and without each program.
    Programs(ProgramsArgs),
    /// Time the harness (T13): campaigns, the bounded resolution of a site, a double run.
    Bench(BenchArgs),
    /// Check reports against the thresholds; exits with 1 when a barrier fails.
    Check(CheckArgs),
}

#[derive(Args)]
struct BenchArgs {
    /// How many campaigns to time (the 216 plans in turn, at Normal): a thousand, as T13 says.
    #[arg(long, default_value_t = 1000)]
    games: usize,
    /// Where to write the timings (`bench.json`: not a canonical report, it holds durations).
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct ProgramsArgs {
    /// Only this site.
    #[arg(long)]
    site: Option<String>,
    /// The expansions the solver may spend on each measure.
    #[arg(long, default_value_t = DEFAULT_EXPANSIONS)]
    expansions: u64,
    /// Where to write the report.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct EconomyArgs {
    /// Where to write the report.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct RunArgs {
    /// The bot that plays: the optimist follows the solver, the prudent, the reckless and the
    /// story bot play by hand, the farmer repeats everything, the monkey types at random.
    #[arg(long, value_enum, default_value = "optimist")]
    bot: BotArg,
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
    /// The inputs the monkey is given in each game before it leaves (the monkey only).
    #[arg(long, default_value_t = DEFAULT_STEPS)]
    steps: usize,
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
    /// What to ask of the intrusion: `objective` (the room the site leaves, with the deck every
    /// player has) or `all` (take every loot, with the whole deck of the tier, as the optimist
    /// does).
    #[arg(long, value_enum, default_value = "objective")]
    loot: LootArg,
    /// The expansions the solver may spend on each measure.
    #[arg(long, default_value_t = DEFAULT_EXPANSIONS)]
    expansions: u64,
    /// Where to write the report.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum LootArg {
    Objective,
    All,
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
enum BotArg {
    Optimist,
    Prudent,
    Reckless,
    Story,
    Farmer,
    Hoarder,
    Monkey,
}

impl BotArg {
    /// The bot that plays campaigns; the monkey does not play one.
    const fn bot(self) -> Option<Bot> {
        match self {
            Self::Optimist => Some(Bot::Optimist),
            Self::Prudent => Some(Bot::Prudent),
            Self::Reckless => Some(Bot::Reckless),
            Self::Story => Some(Bot::Story),
            Self::Farmer => Some(Bot::Farmer),
            Self::Hoarder => Some(Bot::Hoarder),
            Self::Monkey => None,
        }
    }
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
        Command::Economy(args) => economy(&args),
        Command::Programs(args) => programs(&args),
        Command::Bench(args) => bench_command(&args),
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
    let Some(bot) = args.bot.bot() else {
        return run_monkey(args);
    };
    let set = match args.plans {
        Plans::All => PlanSet::All,
        Plans::Representative => PlanSet::Representative,
    };
    let mut plans = plans(set).map_err(|e| format!("the game content does not load: {e}"))?;
    if let Some(limit) = args.limit {
        plans.truncate(limit);
    }
    let mut difficulties = args.difficulty.difficulties();
    if let Some(only) = bot.only_difficulty() {
        if !matches!(args.difficulty, Which::All) && difficulties != [only] {
            return Err(format!(
                "the {} bot only plays {}",
                bot.name(),
                difficulty_name(only)
            ));
        }
        difficulties = vec![only];
    }
    let many = difficulties.len() > 1;
    let mut all_finished = true;
    for difficulty in difficulties {
        let (report, _) = run_bot(bot, &plans, difficulty, &args.seeds);
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

/// The monkey: a game per seed, the difficulties taking turns.
fn run_monkey(args: &RunArgs) -> Result<ExitCode, String> {
    let difficulties = args.difficulty.difficulties();
    let mut records = Vec::new();
    for seed in args.seeds.clone() {
        let turn = usize::try_from(seed).unwrap_or(0) % difficulties.len();
        let difficulty = difficulties
            .get(turn)
            .copied()
            .ok_or_else(|| "no difficulty to play".to_owned())?;
        records.push(play_monkey(seed, difficulty, args.steps));
    }
    let report = MonkeyReport::of(
        [*args.seeds.start(), *args.seeds.end()],
        args.steps as u64,
        &records,
    );
    console::line(&monkey_summary(&report));
    for worst in &report.worst {
        console::line(&format!(
            "  seed {} ({}): {:?}",
            worst.seed, worst.difficulty, worst.fault
        ));
    }
    let clean =
        report.panics + report.not_quittable + report.save_faults + report.contract_faults == 0;
    if let Some(out) = &args.out {
        write(out, &Report::Monkey(report))?;
    }
    Ok(if clean {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn monkey_summary(report: &MonkeyReport) -> String {
    format!(
        "monkey: {} games, {} inputs ({} per mille in an intrusion), {} panics, {} that cannot be left, {} save faults, {} contract faults; damaged saves: {} refused, {} taken",
        report.games,
        report.inputs,
        report.intrusion_per_mille,
        report.panics,
        report.not_quittable,
        report.save_faults,
        report.contract_faults,
        report.damaged_refused,
        report.damaged_accepted
    )
}

fn solve(args: &SolveArgs) -> Result<ExitCode, String> {
    let content =
        Content::embedded().map_err(|e| format!("the game content does not load: {e}"))?;
    let difficulty = match args.difficulty {
        Which::All => None,
        other => other.difficulties().first().copied(),
    };
    let loot = match args.loot {
        LootArg::Objective => Loot::Objective,
        LootArg::All => Loot::All,
    };
    let rows = measure(
        &content,
        args.site.as_deref(),
        difficulty,
        loot,
        args.expansions,
    );
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

fn economy(args: &EconomyArgs) -> Result<ExitCode, String> {
    let content =
        Content::embedded().map_err(|e| format!("the game content does not load: {e}"))?;
    let report = measure_economy(&content);
    console::line(&format!(
        "the stall: {} items for {} credits, {} services; start with {} credits",
        report.items.len(),
        report.catalogue_total,
        report.services.len(),
        report.start_credits
    ));
    for tier in &report.tiers {
        console::line(&format!(
            "tier {}: flagship {} credits, main quests pay {}, all quests {}",
            tier.tier, tier.flagship_price, tier.main_credits, tier.all_credits
        ));
    }
    if let Some(out) = &args.out {
        write(out, &Report::Economy(report))?;
    }
    Ok(ExitCode::SUCCESS)
}

fn programs(args: &ProgramsArgs) -> Result<ExitCode, String> {
    let content =
        Content::embedded().map_err(|e| format!("the game content does not load: {e}"))?;
    let report = analyse_programs(&content, args.expansions, args.site.as_deref());
    for row in &report.programs {
        console::line(&format!(
            "{}{}: needed at {}, more margin at {} (+{} in all), idle at {}{}",
            row.program,
            row.price
                .map_or_else(String::new, |p| format!(" ({p} credits)")),
            row.needed_at.len(),
            row.lowers.len(),
            row.total_gain,
            row.idle_at.len(),
            if row.dominated { ", DOMINATED" } else { "" }
        ));
    }
    if let Some(out) = &args.out {
        write(out, &Report::Programs(report))?;
    }
    Ok(ExitCode::SUCCESS)
}

fn bench_command(args: &BenchArgs) -> Result<ExitCode, String> {
    let content =
        Content::embedded().map_err(|e| format!("the game content does not load: {e}"))?;
    let plans = plans(PlanSet::All).map_err(|e| format!("the game content does not load: {e}"))?;
    let targets = Targets::embedded().map_err(|e| format!("the thresholds do not load: {e}"))?;
    let report = bench(&content, &plans, args.games);
    if let Some(out) = &args.out {
        let mut text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        text.push('\n');
        fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    let failed = print_findings(&judge_bench(&targets.t13, &report));
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Prints the verdicts, and tells whether a barrier failed.
fn print_findings(findings: &[Finding]) -> bool {
    let mut failed = false;
    for finding in findings {
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
    failed
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
    let failed = print_findings(&findings);
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
    let intrusions = if report.intrusions.total == 0 {
        String::new()
    } else {
        format!(
            "; {} intrusions, {} per mille burned, longest streak {}",
            report.intrusions.total,
            report.intrusions.burned_per_mille,
            report.intrusions.longest_burn_streak
        )
    };
    format!(
        "{} {}: {} games, {} finished, {} lost, {} gave up, {} blocked, {} stuck{commands}{intrusions}; endings: {}",
        report.bot,
        report.difficulty,
        report.games,
        report.finished,
        report.lost,
        report.gave_up,
        report.blocked,
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
