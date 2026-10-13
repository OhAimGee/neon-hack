//! The screens of an intrusion: the map, the status, the deck, the report, and the view that a
//! panel draws from. They all say the same thing in different shapes.

use crate::event::{Event, Gauge, Importance, Role, Table};
use crate::game::GaugeReading;
use crate::run::{
    Family, LootKind, NodeId, Program, ProgramSpec, RunState, Status, Supply, Undo, forecast,
};
use crate::text::Text;

use super::speak::Voice;
use super::view::{
    ActiveUtility, Charges, Cycles, DefenseView, LootState, NodeState, NodeView, PatrolView,
    ProgramView, RunView,
};

fn number(value: impl Into<i64>) -> i64 {
    value.into()
}

/// The glossary term of a family.
pub(super) fn family_term(family: Family) -> &'static str {
    match family {
        Family::Network => "family_network",
        Family::Encryption => "family_encryption",
        Family::Human => "family_human",
        Family::AiGuardian => "family_guardian",
    }
}

fn loot_kind(kind: LootKind) -> Text {
    Text::new(match kind {
        LootKind::Objective => "run.loot_kind.objective",
        LootKind::Intel => "run.loot_kind.intel",
        LootKind::Extra => "run.loot_kind.extra",
    })
}

impl Voice<'_> {
    fn node_state(state: &RunState, mission: &crate::run::Mission, node: NodeId) -> NodeState {
        if state.position() == node {
            NodeState::Here
        } else if state.is_open(mission, node) {
            NodeState::Open
        } else {
            NodeState::Closed
        }
    }

    fn loot_state(&self, state: &RunState, node: NodeId) -> LootState {
        match self.mission.loot(node) {
            None => LootState::None,
            Some(kind) if state.is_looted(node) => LootState::Taken(kind),
            Some(kind) => LootState::Visible(kind),
        }
    }

    fn defense(&self, state: &RunState, node: NodeId) -> Option<DefenseView> {
        self.mission.ice(node).map(|ice| DefenseView {
            family: family_term(ice.family),
            strength: ice.strength,
            progress: state.progress(node).min(ice.strength),
        })
    }

    /// How many loots there are, and how many are taken.
    pub(super) fn loot_count(&self, state: &RunState) -> (u8, u8) {
        let mut total = 0u8;
        let mut taken = 0u8;
        for node in self.mission.nodes() {
            if self.mission.loot(node).is_some() {
                total = total.saturating_add(1);
                if state.is_looted(node) {
                    taken = taken.saturating_add(1);
                }
            }
        }
        (taken, total)
    }

    /// The program rows of the deck, in the order of the list `status` shows.
    pub(super) fn deck(&self, state: &RunState) -> Vec<ProgramView> {
        Program::ALL
            .into_iter()
            .filter_map(|program| {
                let supply = state.supply(self.mission, program)?;
                let ProgramSpec {
                    family,
                    power,
                    cycles,
                    noise,
                    ..
                } = program.spec();
                Some(ProgramView {
                    program,
                    name: Self::program(program),
                    family: family.map(family_term),
                    power,
                    cycles,
                    noise: self.mission.gain(noise),
                    charges: match supply {
                        Supply::Unlimited => Charges::Unlimited,
                        Supply::Limited(count) => Charges::Left(count),
                    },
                })
            })
            .collect()
    }

    /// The view of an intrusion: everything a panel draws, nothing it has to work out.
    pub(super) fn view(&self, state: &RunState, undo: Undo) -> RunView {
        let announced = forecast(self.mission, state, 1);
        let lines = self.forecast_events(state);
        let mission = self.mission;
        RunView {
            site: Text::raw(self.site_id),
            turn: state.turn(),
            trace: GaugeReading {
                gauge: Gauge::Trace,
                value: i32::from(state.trace().min(mission.trace_cap())),
                max: i32::from(mission.trace_cap()),
                band: Self::band(state.trace(), state.status() == Status::Burned),
            },
            cycles: Cycles {
                left: state.cycles(),
                max: mission.cycles_per_turn().max(state.cycles()),
            },
            nodes: mission
                .nodes()
                .map(|node| NodeView {
                    id: node,
                    name: self.node_name(node),
                    state: Self::node_state(state, mission, node),
                    defense: self.defense(state, node),
                    links: mission.neighbours(node).collect(),
                    loot: self.loot_state(state, node),
                })
                .collect(),
            patrols: (0..mission.patrol_count())
                .filter_map(|patrol| {
                    Some(PatrolView {
                        id: u8::try_from(patrol).ok()?.saturating_add(1),
                        at: state.patrol_position(mission, patrol)?,
                        next: state.patrol_next(mission, patrol)?,
                        down_turns: state.down_turns(patrol),
                    })
                })
                .collect(),
            deck: self.deck(state),
            active: [
                state.cloak_active().then_some(ActiveUtility::Cloak),
                state.ghost_active().then_some(ActiveUtility::Ghost),
            ]
            .into_iter()
            .flatten()
            .collect(),
            forecast: lines
                .into_iter()
                .filter_map(|event| match event {
                    Event::Message { text, .. } => Some(text),
                    _ => None,
                })
                .collect(),
            forecast_total: i32::from(announced.total()),
            undo_left: undo,
            loot_held: self.loot_count(state),
        }
    }

    /// `map`: the nodes as a table, then a line per patrol.
    pub(super) fn map(&self, state: &RunState, undo: Undo) -> Vec<Event> {
        let view = self.view(state, undo);
        let rows = view
            .nodes
            .iter()
            .map(|node| {
                let here = match node.state {
                    NodeState::Here => "run.map.state.here",
                    NodeState::Open => "run.map.state.open",
                    NodeState::Closed => "run.map.state.closed",
                };
                let defense = match node.defense {
                    None => Text::new("run.map.defense.none"),
                    Some(ice) => Text::new("run.map.defense.ice")
                        .with_term("family", ice.family)
                        .with_int("strength", number(ice.strength))
                        .with_int("progress", number(ice.progress)),
                };
                let links = node
                    .links
                    .iter()
                    .map(|link| link.number().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let loot = match node.loot {
                    LootState::None => Text::new("run.map.loot.none"),
                    LootState::Visible(kind) => {
                        Text::new("run.map.loot.visible").with_text("kind", loot_kind(kind))
                    }
                    LootState::Taken(_) => Text::new("run.map.loot.taken"),
                };
                vec![
                    node.name.clone(),
                    Text::new(here),
                    defense,
                    Text::new("run.map.links").with_str("list", links),
                    loot,
                ]
            })
            .collect();
        let table = Table::new(
            Text::new("run.map.title").with_text("name", view.site.clone()),
            vec![
                Text::new("run.map.col.node"),
                Text::new("run.map.col.state"),
                Text::new("run.map.col.defense"),
                Text::new("run.map.col.links"),
                Text::new("run.map.col.loot"),
            ],
            rows,
        );
        let mut events = vec![Event::Screen(table)];
        events.extend(view.patrols.iter().map(|patrol| {
            let text = if patrol.down_turns > 0 {
                Text::new("run.map.patrol_down").with_int("turns", number(patrol.down_turns))
            } else {
                Text::new("run.map.patrol")
            }
            .with_int("id", number(patrol.id))
            .with_text("at", self.node(patrol.at))
            .with_text("next", self.node(patrol.next));
            Event::message(Role::System, Importance::Essential, text)
        }));
        events
    }

    fn deck_table(&self, state: &RunState) -> Table {
        let rows = self
            .deck(state)
            .into_iter()
            .map(|program| {
                let kind = match program.family {
                    Some(family) => Text::new("run.deck.breach")
                        .with_term("family", family)
                        .with_int("power", number(program.power)),
                    None => Text::new("run.deck.utility"),
                };
                let charges = match program.charges {
                    Charges::Unlimited => Text::new("run.deck.unlimited"),
                    Charges::Left(count) => {
                        Text::new("run.deck.left").with_int("count", number(count))
                    }
                };
                vec![
                    program.name,
                    kind,
                    Text::new("run.deck.cycles").with_int("count", number(program.cycles)),
                    Text::new("run.deck.noise").with_int("value", number(program.noise)),
                    charges,
                ]
            })
            .collect();
        Table::new(
            Text::new("run.deck.title"),
            vec![
                Text::new("run.deck.col.program"),
                Text::new("run.deck.col.kind"),
                Text::new("run.deck.col.cost"),
                Text::new("run.deck.col.noise"),
                Text::new("run.deck.col.charges"),
            ],
            rows,
        )
    }

    /// `status`: the Trace, the cycles, what is on, the deck, the loot, the undos and the
    /// forecast.
    pub(super) fn status(&self, state: &RunState, undo: Undo) -> Vec<Event> {
        let mut events = vec![
            Event::system(
                Text::new("run.status.trace")
                    .with_int("value", self.shown(state.trace()))
                    .with_int("max", number(self.mission.trace_cap()))
                    .with_text("band", Self::band(state.trace(), false)),
            ),
            Event::system(self.cycles_left(state)),
        ];
        if state.cloak_active() {
            events.push(Event::system(Text::new("run.status.cloak")));
        }
        if state.ghost_active() {
            events.push(Event::system(Text::new("run.status.ghost")));
        }
        events.push(Event::Screen(self.deck_table(state)));
        let (taken, total) = self.loot_count(state);
        if total > 0 {
            events.push(Event::system(
                Text::new("run.status.loot")
                    .with_int("taken", number(taken))
                    .with_int("total", number(total)),
            ));
        }
        events.push(Event::system(match undo {
            Undo::Unlimited => Text::new("run.status.undo_unlimited"),
            Undo::Left(0) => Text::new("run.status.undo_none"),
            Undo::Left(count) => Text::new("run.status.undo").with_int("left", number(count)),
        }));
        events.extend(self.forecast_events(state));
        events
    }

    /// The report that closes an intrusion.
    pub(super) fn report(&self, state: &RunState) -> Table {
        let (taken, total) = self.loot_count(state);
        let burned = state.status() == Status::Burned;
        let row = |what: &'static str, value: Text| vec![Text::new(what), value];
        Table::new(
            Text::new("run.report.title"),
            vec![
                Text::new("run.report.col.what"),
                Text::new("run.report.col.value"),
            ],
            vec![
                row("run.report.turns", Text::raw(state.turn().to_string())),
                row(
                    "run.report.trace",
                    Text::new("run.report.trace_value")
                        .with_int("value", self.shown(state.trace()))
                        .with_int("max", number(self.mission.trace_cap()))
                        .with_text("band", Self::band(state.trace(), burned)),
                ),
                row(
                    "run.report.loot",
                    Text::new("run.report.loot_value")
                        .with_int("taken", number(taken))
                        .with_int("total", number(total)),
                ),
            ],
        )
    }
}
