//! Measurements of the spike (run in release):
//!
//! ```text
//! cargo run --release -p neon-spike-solver --example measure -- <section> [cap]
//! ```
//!
//! Sections: `table`, `techniques`, `domination`, `cap`, `preview`, `scaling`, `minimal`,
//! `all`. `cap` is the expansion limit of the exact search (default 2 000 000).
#![allow(clippy::print_stdout)]

use std::time::Instant;

use neon_spike_solver::analysis::{
    Verdict, anytime_plan, bracket, domination_check, preview_each_turn,
};
use neon_spike_solver::brute::brute_force;
use neon_spike_solver::fallback::{beam_plan, greedy_plan};
use neon_spike_solver::missions::{self, minimal_kit};
use neon_spike_solver::model::{Mission, PACKED_BITS, Program, UNLIMITED};
use neon_spike_solver::solver::{Config, Outcome, Report, solvable_within, solve};
use neon_spike_solver::table::TableKind;

fn best(cap: u64) -> Config {
    Config {
        tables: TableKind::Fx,
        macro_turns: false,
        heuristic: true,
        dominance: true,
        close_first: true,
        max_expanded: cap,
        goal: neon_spike_solver::solver::Goal::Optimal,
    }
}

fn mb(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}

fn kit_label(m: &Mission) -> String {
    let n = Program::ALL.iter().filter(|p| m.has(**p)).count();
    let limited = Program::ALL
        .iter()
        .filter(|p| m.kit[p.index()] != 0 && m.kit[p.index()] != UNLIMITED)
        .count();
    format!("{n} ({limited} limited)")
}

fn result_cell(m: &Mission, r: &Report) -> (String, String, String, String) {
    match &r.outcome {
        Outcome::Solved { plan, .. } => (
            format!("{}", plan.cost),
            format!("{}", plan.turns),
            format!("{}", plan.actions.len()),
            format!("{}", i32::from(m.trace_cap) - i32::from(plan.cost)),
        ),
        Outcome::Unsolvable => ("unsolvable".into(), "-".into(), "-".into(), "-".into()),
        Outcome::Unknown { lower_bound, .. } => {
            let ub = beam_plan(m, &m.initial(), 16)
                .or_else(|| greedy_plan(m, &m.initial()))
                .map(|p| p.cost);
            let cell = match ub {
                Some(u) => format!("[{lower_bound}..{u}]"),
                None => format!("[{lower_bound}..?]"),
            };
            let margin = match ub {
                Some(u) => format!(
                    "{}..{}",
                    i32::from(m.trace_cap) - i32::from(u),
                    i32::from(m.trace_cap) - i32::from(*lower_bound)
                ),
                None => "?".into(),
            };
            (format!("UNKNOWN {cell}"), "-".into(), "-".into(), margin)
        }
    }
}

fn print_header() {
    println!(
        "| mission | nodes | patrols | kit | cycles | expanded | stored | time | peak MB | B/state | optimum | turns | plan | margin |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
}

fn print_row(m: &Mission, r: &Report) {
    let (opt, turns, len, margin) = result_cell(m, r);
    let patrols = m.sentinels.iter().filter(|s| s.route.len() > 1).count();
    let per_state = if r.stats.stored > 0 {
        r.stats.bytes_tight as f64 / r.stats.stored as f64
    } else {
        0.0
    };
    println!(
        "| {} | {} | {} | {} | {} | {} | {} | {:.1} ms | {:.1} | {:.0} | {} | {} | {} | {} |",
        m.name,
        m.n,
        patrols,
        kit_label(m),
        m.cycles_per_turn,
        r.stats.expanded,
        r.stats.stored,
        r.stats.elapsed.as_secs_f64() * 1000.0,
        mb(r.stats.bytes_capacity),
        per_state,
        opt,
        turns,
        len,
        margin
    );
}

fn table(cap: u64) {
    println!(
        "\n## Results (exact search: fine-grained, A*, dominance, Fx table; cap {cap} expansions)\n"
    );
    print_header();
    let mut set = missions::all();
    set.push(missions::worst_8_full_kit(0));
    let mut extra = Vec::new();
    for m in &set {
        let r = solve(m, &best(cap));
        print_row(m, &r);
        if !m.name.starts_with("unsolvable_deadlock") {
            extra.push(minimal_kit(m));
        }
    }
    println!("\n### Same missions with the tier's minimal kit (unlimited programs only)\n");
    print_header();
    for m in &extra {
        let r = solve(m, &best(cap));
        print_row(m, &r);
    }
    println!("\nbits of the fixed packing: {PACKED_BITS}; mixed-radix information per mission:");
    for m in &set {
        println!("- {}: {:.1} bits", m.name, m.state_bits());
    }
}

fn techniques(cap: u64) {
    println!("\n## Techniques (same mission, same optimum, cap {cap})\n");
    println!(
        "| mission | technique | expanded | stored | mid-turn states | dominated | time | peak MB | optimum |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    let base = Config::baseline();
    let variants: Vec<(&str, Config, bool)> = vec![
        ("fine, no canonical filters", base, false),
        ("fine (baseline)", base, true),
        (
            "fine + BTreeMap",
            Config {
                tables: TableKind::BTree,
                ..base
            },
            true,
        ),
        (
            "end-of-turn merging (macro)",
            Config {
                macro_turns: true,
                ..base
            },
            true,
        ),
        (
            "fine + A*",
            Config {
                heuristic: true,
                ..base
            },
            true,
        ),
        (
            "fine + dominance",
            Config {
                dominance: true,
                ..base
            },
            true,
        ),
        (
            "fine + A* + dominance",
            Config {
                heuristic: true,
                dominance: true,
                ..base
            },
            true,
        ),
        (
            "fine + A* + dominance + close-first",
            Config {
                heuristic: true,
                dominance: true,
                close_first: true,
                ..base
            },
            true,
        ),
        (
            "macro + A* + dominance + close-first",
            Config {
                macro_turns: true,
                heuristic: true,
                dominance: true,
                close_first: true,
                ..base
            },
            true,
        ),
        (
            "macro + A* + dominance + close-first + BTreeMap",
            Config {
                macro_turns: true,
                heuristic: true,
                dominance: true,
                close_first: true,
                tables: TableKind::BTree,
                ..base
            },
            true,
        ),
    ];
    let set = [
        missions::audit_6(1),
        missions::patrol_5(2),
        missions::cipher_6(2),
    ];
    for m in &set {
        for (label, cfg, canonical) in &variants {
            let mut mm = m.clone();
            mm.canonical_actions = *canonical;
            let cfg = Config {
                max_expanded: cap,
                ..*cfg
            };
            let r = solve(&mm, &cfg);
            let opt = match &r.outcome {
                Outcome::Solved { plan, .. } => plan.cost.to_string(),
                Outcome::Unknown { .. } => "UNKNOWN".into(),
                Outcome::Unsolvable => "unsolvable".into(),
            };
            println!(
                "| {} | {} | {} | {} | {} | {} | {:.1} ms | {:.1} | {} |",
                m.name,
                label,
                r.stats.expanded,
                r.stats.stored,
                r.stats.intra_states,
                r.stats.dominated,
                r.stats.elapsed.as_secs_f64() * 1000.0,
                mb(r.stats.bytes_capacity),
                opt
            );
        }
    }
}

fn domination(cap: u64) {
    println!("\n## Domination check (I19), cap {cap}\n");
    let set = vec![
        missions::easy_4(2),
        missions::audit_6(1),
        missions::patrol_5(2),
        missions::cipher_6(3),
        missions::dual_7(neon_spike_solver::model::Difficulty::Normal, 3),
        missions::boss_8(3),
    ];
    let rows = domination_check(&set, &best(cap));
    println!(
        "| program | in kits of | worsens (mission: with -> without) | used by an optimal plan | verdict |"
    );
    println!("|---|---|---|---|---|");
    for row in rows {
        if row.removals.is_empty() {
            continue;
        }
        let worse: Vec<String> = row
            .removals
            .iter()
            .filter(|r| r.hurts())
            .map(|r| format!("{}: {} -> {}", r.mission, fmt_v(r.with), fmt_v(r.without)))
            .collect();
        let verdict = if row.is_needed() {
            "needed"
        } else if row.is_ever_used() {
            "used but replaceable"
        } else {
            "never useful on this set"
        };
        println!(
            "| {} | {} | {} | {} | {} |",
            row.program.name(),
            row.removals.len(),
            if worse.is_empty() {
                "-".to_string()
            } else {
                worse.join("; ")
            },
            row.removals.iter().filter(|r| r.used_in_plan).count(),
            verdict
        );
    }
}

fn fmt_v(v: Verdict) -> String {
    match v {
        Verdict::Cost(c) => c.to_string(),
        Verdict::Unsolvable => "unsolvable".into(),
        Verdict::Unknown => "?".into(),
    }
}

fn cap_study(cap: u64) {
    println!("\n## Bounded search and fallbacks (cap sweep, cap {cap} for the reference run)\n");
    let mut set = missions::all();
    set.push(missions::worst_8_full_kit(0));
    let mut mins: Vec<Mission> = set
        .iter()
        .filter(|m| !m.name.starts_with("unsolvable_deadlock"))
        .map(minimal_kit)
        .collect();
    set.append(&mut mins);
    // Reference runs.
    struct Ref {
        name: String,
        needed: Option<u64>,
        opt: Option<u8>,
        us_per_exp: f64,
    }
    let mut refs = Vec::new();
    for m in &set {
        let r = solve(m, &best(cap));
        let (needed, opt) = match &r.outcome {
            Outcome::Solved { plan, .. } => (Some(r.stats.expanded), Some(plan.cost)),
            Outcome::Unsolvable => (Some(r.stats.expanded), None),
            Outcome::Unknown { .. } => (None, None),
        };
        refs.push(Ref {
            name: m.name.clone(),
            needed,
            opt,
            us_per_exp: r.stats.elapsed.as_secs_f64() * 1e6 / r.stats.expanded.max(1) as f64,
        });
    }
    println!(
        "| cap N | missions decided (of {}) | still unknown | share unknown |",
        refs.len()
    );
    println!("|---|---|---|---|");
    for n in [
        1_000u64, 3_000, 10_000, 30_000, 100_000, 300_000, 1_000_000, 3_000_000,
    ] {
        let decided = refs
            .iter()
            .filter(|r| r.needed.is_some_and(|x| x <= n))
            .count();
        let unknown = refs.len() - decided;
        println!(
            "| {n} | {decided} | {unknown} | {:.0} % |",
            100.0 * unknown as f64 / refs.len() as f64
        );
    }
    println!("\n| mission | expansions needed | optimum | us / expansion |");
    println!("|---|---|---|---|");
    for r in &refs {
        println!(
            "| {} | {} | {} | {:.2} |",
            r.name,
            r.needed.map_or(format!("> {cap}"), |x| x.to_string()),
            r.opt.map_or("-".to_string(), |o| o.to_string()),
            r.us_per_exp
        );
    }
    println!("\n### Fallback quality (cost = final Trace; gap to the optimum when known)\n");
    println!(
        "| mission | optimum | greedy | beam 4 | beam 16 | anytime 20k | t greedy | t beam 16 | t anytime | expansions anytime |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    for (m, r) in set.iter().zip(&refs) {
        let start = m.initial();
        let t = Instant::now();
        let g = greedy_plan(m, &start).map(|p| p.cost);
        let tg = t.elapsed();
        let mut cells = Vec::new();
        let mut t16 = tg;
        for w in [4usize, 16] {
            let t = Instant::now();
            let c = beam_plan(m, &start, w).map(|p| p.cost);
            if w == 16 {
                t16 = t.elapsed();
            }
            cells.push(c.map_or("fail".to_string(), |c| c.to_string()));
        }
        let t = Instant::now();
        let any = anytime_plan(m, &start, 20_000);
        let ta = t.elapsed();
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.2} ms | {:.1} ms | {:.2} ms | {} |",
            m.name,
            r.opt.map_or("?".to_string(), |o| o.to_string()),
            g.map_or("fail".to_string(), |c| c.to_string()),
            cells[0],
            cells[1],
            any.as_ref()
                .map_or("fail".to_string(), |(p, _)| p.cost.to_string()),
            tg.as_secs_f64() * 1000.0,
            t16.as_secs_f64() * 1000.0,
            ta.as_secs_f64() * 1000.0,
            any.as_ref().map_or(0, |(_, u)| *u)
        );
    }
}

fn utilities(cap: u64) {
    println!("\n## What makes the search grow: utilities, sentinels, kit size (cap {cap})\n");
    println!("| mission | variant | optimum | expanded | stored | time |");
    println!("|---|---|---|---|---|---|");
    let base_set = [missions::patrol_5(2), missions::cipher_6(3)];
    let drops: [(&str, &[Program]); 7] = [
        ("full kit", &[]),
        ("without Spoof", &[Program::Spoof]),
        ("without Cloak", &[Program::Cloak]),
        ("without Ghost", &[Program::Ghost]),
        ("without Spoof, Cloak", &[Program::Spoof, Program::Cloak]),
        (
            "without Spoof, Cloak, Ghost",
            &[Program::Spoof, Program::Cloak, Program::Ghost],
        ),
        (
            "without Spoof, Cloak, Ghost, Exploit, Quantum, Social",
            &[
                Program::Spoof,
                Program::Cloak,
                Program::Ghost,
                Program::Exploit,
                Program::Quantum,
                Program::Social,
            ],
        ),
    ];
    for base in &base_set {
        let mut variants: Vec<(String, Mission)> = Vec::new();
        for (label, drop) in drops {
            let mut m = base.clone();
            for p in drop {
                m = m.without(*p);
            }
            variants.push((label.to_string(), m));
        }
        let mut quiet = base.clone();
        quiet.sentinels.clear();
        quiet.period = 1;
        variants.push(("no sentinel at all".to_string(), quiet));
        for (label, m) in &variants {
            let r = solve(m, &best(cap));
            let opt = r.cost().map_or("UNKNOWN".to_string(), |c| c.to_string());
            println!(
                "| {} | {} | {} | {} | {} | {:.1} ms |",
                base.name,
                label,
                opt,
                r.stats.expanded,
                r.stats.stored,
                r.stats.elapsed.as_secs_f64() * 1000.0
            );
        }
    }
}

fn preview(cap: u64) {
    println!("\n## Per-turn re-solve (preview) along the optimal plan, cap {cap}\n");
    println!("| mission | turn | expanded | time | cost to go |");
    println!("|---|---|---|---|---|");
    let set = [
        missions::audit_6(1),
        missions::patrol_5(2),
        missions::cipher_6(3),
        missions::dual_7(neon_spike_solver::model::Difficulty::Normal, 3),
        missions::boss_8(3),
    ];
    for m in &set {
        for t in preview_each_turn(m, &best(cap)) {
            println!(
                "| {} | {} | {} | {:.2} ms | {} |",
                m.name,
                t.turn,
                t.expanded,
                t.elapsed.as_secs_f64() * 1000.0,
                t.cost.map_or("?".to_string(), |c| c.to_string())
            );
        }
    }
}

fn scaling(cap: u64) {
    println!("\n## Growth with the strength of the ICE (cap {cap})\n");
    println!("| mission | bonus | optimum | expanded | stored | time |");
    println!("|---|---|---|---|---|---|");
    for name in ["audit_6", "patrol_5", "cipher_6", "dual_7", "boss_8"] {
        for bonus in 0..=3u8 {
            let Some(m) = missions::by_name(name, bonus) else {
                continue;
            };
            let r = solve(&m, &best(cap));
            let opt = r.cost().map_or("UNKNOWN".to_string(), |c| c.to_string());
            println!(
                "| {} | +{} | {} | {} | {} | {:.1} ms |",
                name,
                bonus,
                opt,
                r.stats.expanded,
                r.stats.stored,
                r.stats.elapsed.as_secs_f64() * 1000.0
            );
        }
    }
}

fn minimal(cap: u64) {
    println!(
        "\n## I2: solvable with the minimal kit? (early exit, cap 100, expansion cap {cap})\n"
    );
    println!("| mission | answer | expanded | time |");
    println!("|---|---|---|---|");
    let mut set = missions::all();
    set.push(missions::worst_8_full_kit(0));
    for m in &set {
        for mm in [m.clone(), minimal_kit(m)] {
            if mm.name.starts_with("unsolvable_deadlock (minimal") {
                continue;
            }
            let cfg = Config {
                max_expanded: cap,
                ..best(cap)
            };
            let r = solvable_within(&mm, 100, &cfg);
            let answer = match &r.outcome {
                Outcome::Solved { plan, .. } => format!("solvable (plan Trace {})", plan.cost),
                Outcome::Unsolvable => "UNSOLVABLE (proven)".to_string(),
                Outcome::Unknown { .. } => "unknown (cap)".to_string(),
            };
            println!(
                "| {} | {} | {} | {:.2} ms |",
                mm.name,
                answer,
                r.stats.expanded,
                r.stats.elapsed.as_secs_f64() * 1000.0
            );
        }
    }
}

fn independent_check() {
    println!("\n## Independent brute force vs solver\n");
    println!(
        "| mission | brute optimum | brute turns | brute states | solver optimum | solver turns | agree |"
    );
    println!("|---|---|---|---|---|---|---|");
    for m in [
        missions::tiny_4(0),
        missions::tiny_ghost(),
        missions::easy_4(0),
        missions::audit_6(0),
    ] {
        let b = brute_force(&m);
        let r = solve(
            &m,
            &Config {
                heuristic: false,
                dominance: false,
                close_first: false,
                ..best(0)
            },
        );
        let (c, t) = r
            .plan()
            .map_or((None, None), |p| (Some(p.cost), Some(p.turns)));
        println!(
            "| {} | {:?} | {:?} | {} | {:?} | {:?} | {} |",
            m.name,
            b.best_cost,
            b.best_turns,
            b.states,
            c,
            t,
            if b.best_cost == c && b.best_turns == t {
                "yes"
            } else {
                "NO"
            }
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let section = args.first().map_or("all", String::as_str);
    let cap: u64 = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2_000_000);
    let want = |s: &str| section == "all" || section == s;
    if want("check") {
        independent_check();
    }
    if want("table") {
        table(cap);
    }
    if want("techniques") {
        techniques(cap.min(1_000_000));
    }
    if want("domination") {
        domination(cap.min(1_000_000));
    }
    if want("cap") {
        cap_study(cap);
    }
    if want("preview") {
        preview(cap.min(1_000_000));
    }
    if want("scaling") {
        scaling(cap.min(1_000_000));
    }
    if want("utilities") {
        utilities(cap.min(1_000_000));
    }
    if want("minimal") {
        minimal(cap.min(1_000_000));
    }
    let _ = bracket;
}
