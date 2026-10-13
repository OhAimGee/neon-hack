//! The panel of an intrusion for the full-screen interface, written from the [`RunView`]: the
//! turn and its cycles, the nodes, the patrols, the forecast, the deck. The sections ride the
//! same channel as the quests and the net of the hub ([`crate::game::View::sections`]); every row is short
//! (the panel is 30 columns wide) and says in words what a command also says
//! (`docs/spec/run-screen.md` § 6).

use crate::game::{PanelRow, PanelSection, Tone};
use crate::text::Text;

use super::view::{ActiveUtility, Charges, ForecastItem, NodeState, RunView};

fn number(value: impl Into<i64>) -> i64 {
    value.into()
}

/// What the status bar adds to the gauges: the turn and the cycles.
pub(super) fn status_items(view: &RunView) -> Vec<Text> {
    vec![
        Text::new("run.bar.turn").with_int("number", number(view.turn)),
        Text::new("run.bar.cycles")
            .with_int("left", number(view.cycles.left))
            .with_int("limit", number(view.cycles.max)),
    ]
}

/// The sections of the panel.
pub(super) fn sections(view: &RunView) -> Vec<PanelSection> {
    let mut sections = vec![node_section(view)];
    if !view.patrols.is_empty() {
        sections.push(patrol_section(view));
    }
    sections.push(forecast_section(view));
    sections.push(deck_section(view));
    sections
}

fn section(title: &'static str, rows: Vec<PanelRow>) -> PanelSection {
    PanelSection {
        title: Text::new(title),
        rows,
    }
}

fn node_section(view: &RunView) -> PanelSection {
    let rows = view
        .nodes
        .iter()
        .map(|node| {
            let (key, state, tone) = match (node.state, node.defense) {
                (NodeState::Here, _) => (
                    "run.panel.node_here",
                    Text::new("run.panel.state.here"),
                    Tone::Strong,
                ),
                (NodeState::Closed, Some(ice)) => (
                    "run.panel.node",
                    Text::new("run.panel.defense")
                        .with_term("family", ice.family)
                        .with_int("progress", number(ice.progress))
                        .with_int("strength", number(ice.strength)),
                    Tone::Plain,
                ),
                // Open: nothing guards it, or its ICE is open (a closed node always has ICE).
                (NodeState::Open | NodeState::Closed, None) | (NodeState::Open, Some(_)) => (
                    "run.panel.node",
                    Text::new("run.panel.state.open"),
                    Tone::Good,
                ),
            };
            PanelRow::new(
                Text::new(key)
                    .with_int("number", number(node.id.number()))
                    .with_text("name", node.name.clone())
                    .with_text("state", state),
                tone,
            )
        })
        .collect();
    section("run.panel.nodes", rows)
}

fn patrol_section(view: &RunView) -> PanelSection {
    let rows = view
        .patrols
        .iter()
        .map(|patrol| {
            let (key, tone) = if patrol.down_turns > 0 {
                ("run.panel.patrol_down", Tone::Muted)
            } else {
                ("run.panel.patrol", Tone::Plain)
            };
            PanelRow::new(
                Text::new(key)
                    .with_int("id", number(patrol.id))
                    .with_int("at", number(patrol.at.number()))
                    .with_int("next", number(patrol.next.number()))
                    .with_int("turns", number(patrol.down_turns)),
                tone,
            )
        })
        .collect();
    section("run.panel.patrols", rows)
}

fn forecast_section(view: &RunView) -> PanelSection {
    // What is on changes what follows, so it stands at the head of the forecast.
    let mut rows: Vec<PanelRow> = view
        .active
        .iter()
        .map(|utility| {
            let key = match utility {
                ActiveUtility::Cloak => "run.status.cloak",
                ActiveUtility::Ghost => "run.status.ghost",
            };
            PanelRow::new(Text::new(key), Tone::Good)
        })
        .collect();
    rows.push(PanelRow::new(
        Text::new("run.panel.forecast_total").with_int("total", i64::from(view.forecast_total)),
        Tone::Strong,
    ));
    for item in &view.forecast_items {
        let row = match *item {
            ForecastItem::Scan { patrol, added, .. } => Some(PanelRow::new(
                Text::new("run.panel.scan")
                    .with_int("id", number(patrol))
                    .with_int("added", number(added)),
                Tone::Warning,
            )),
            ForecastItem::ScanBlocked { patrol, .. } => Some(PanelRow::new(
                Text::new("run.panel.scan_blocked").with_int("id", number(patrol)),
                Tone::Muted,
            )),
            ForecastItem::Ambient { added } => Some(PanelRow::new(
                Text::new("run.panel.ambient").with_int("added", number(added)),
                Tone::Plain,
            )),
            ForecastItem::PatrolMoves { .. } => None,
        };
        rows.extend(row);
    }
    section("run.panel.forecast", rows)
}

/// A program of the deck as the panel names it: its place in the list `status` shows, its short
/// name, and what is left of it.
fn program_entry(index: usize, program: &super::view::ProgramView) -> (Text, Tone) {
    let short = Text::dynamic(format!("program.{}.short", program.program.id()));
    let position = number(u8::try_from(index + 1).unwrap_or(u8::MAX));
    match program.charges {
        Charges::Unlimited => (
            Text::new("run.panel.program")
                .with_int("number", position)
                .with_text("name", short),
            Tone::Plain,
        ),
        Charges::Left(left) => (
            Text::new("run.panel.program_left")
                .with_int("number", position)
                .with_text("name", short)
                .with_int("left", number(left)),
            if left == 0 { Tone::Muted } else { Tone::Plain },
        ),
    }
}

/// The deck, two programs to a row: an entry is at most 14 columns, so a pair fits the 30 of the
/// panel (a test holds it to that).
fn deck_section(view: &RunView) -> PanelSection {
    let entries: Vec<(Text, Tone)> = view
        .deck
        .iter()
        .enumerate()
        .map(|(index, program)| program_entry(index, program))
        .collect();
    let rows = entries
        .chunks(2)
        .map(|pair| match pair {
            [(first, tone), (second, other)] => PanelRow::new(
                Text::new("run.panel.pair")
                    .with_text("first", first.clone())
                    .with_text("second", second.clone()),
                if tone == other { *tone } else { Tone::Plain },
            ),
            [(only, tone)] => PanelRow::new(only.clone(), *tone),
            _ => PanelRow::new(Text::new("run.panel.pair"), Tone::Plain),
        })
        .collect();
    section("run.panel.deck", rows)
}
