//! From the facts of the rules to the lines of the game, in the order of
//! `docs/spec/run-screen.md` § 3 and with the importance of § 7.

use crate::event::{Event, Gauge, Importance, Role, Severity};
use crate::run::{
    ActionError, Fact, ForecastTurn, LootKind, Mission, NodeId, Program, RunState, Status,
    TraceBand, VIRUS_TURNS, forecast,
};
use crate::text::Text;

/// What turns the facts of one site into lines: the names of its nodes, and how far the
/// forecast looks.
#[derive(Debug, Clone, Copy)]
pub(super) struct Voice<'a> {
    pub(super) mission: &'a Mission,
    /// The id of the site as it is written in a text key (`corp_server_01`).
    pub(super) site: &'a str,
    /// The id of the site as the player types it (`corp-server-01`).
    pub(super) site_id: &'a str,
    /// Turns the forecast announces: two in Story, one otherwise.
    pub(super) horizon: u8,
}

/// An id as it is written in a key: `-` becomes `_`.
pub(super) fn key_of(id: &str) -> String {
    id.replace('-', "_")
}

/// The glossary term of the band a Trace is in.
pub(super) fn band_term(trace: u8) -> &'static str {
    match TraceBand::of(trace) {
        TraceBand::Calm => "trace_calm",
        TraceBand::Vigilance => "trace_wary",
        TraceBand::Alert => "trace_alert",
        TraceBand::Danger => "trace_danger",
    }
}

/// A line that matters less than the result of the action: `brief` leaves it out.
fn aside(text: Text) -> Event {
    Event::message(Role::System, Importance::Normal, text)
}

fn number(value: impl Into<i64>) -> i64 {
    value.into()
}

impl Voice<'_> {
    /// A node, as it is named in a sentence: its name and its number.
    pub(super) fn node(&self, node: NodeId) -> Text {
        Text::new("run.node.ref")
            .with_text("name", self.node_name(node))
            .with_int("number", number(node.number()))
    }

    /// The name of a node, ten columns at most.
    pub(super) fn node_name(&self, node: NodeId) -> Text {
        Text::dynamic(format!("site.{}.node.{}.name", self.site, node.number()))
    }

    /// The name of a program.
    pub(super) fn program(program: Program) -> Text {
        Text::dynamic(format!("program.{}.name", program.id()))
    }

    /// The name of the band a Trace is in, or `burned` once it is.
    pub(super) fn band(trace: u8, burned: bool) -> Text {
        Text::new("run.band").with_term("band", if burned { "burned" } else { band_term(trace) })
    }

    /// The Trace the interface shows: never above the cap it burns at.
    pub(super) fn shown(&self, trace: u8) -> i64 {
        number(trace.min(self.mission.trace_cap()))
    }

    /// The cycles of a turn as the interface counts them: an Overclock can give more than
    /// the turn does.
    fn cycle_limit(&self, state: &RunState) -> i64 {
        number(self.mission.cycles_per_turn().max(state.cycles()))
    }

    /// The banner of a turn.
    pub(super) fn turn_banner(&self, state: &RunState) -> Event {
        Event::system(
            Text::new("run.turn.start")
                .with_int("number", number(state.turn()))
                .with_int("value", self.shown(state.trace()))
                .with_int("max", number(self.mission.trace_cap()))
                .with_text("band", Self::band(state.trace(), false))
                .with_int("left", number(state.cycles()))
                .with_int("limit", self.cycle_limit(state)),
        )
    }

    /// The Trace moved: the change, and a warning when it enters a worse band.
    fn trace_moved(&self, from: u8, to: u8, burned: bool, events: &mut Vec<Event>) {
        let (from, to) = (
            from.min(self.mission.trace_cap()),
            to.min(self.mission.trace_cap()),
        );
        if from == to {
            return;
        }
        events.push(Event::Changed {
            gauge: Gauge::Trace,
            from: i32::from(from),
            to: i32::from(to),
            band: Self::band(to, burned),
        });
        if burned || to < from {
            return;
        }
        let severity = match TraceBand::of(to) {
            TraceBand::Calm => return,
            _ if TraceBand::of(to) == TraceBand::of(from) => return,
            TraceBand::Vigilance => Severity::Notice,
            TraceBand::Alert => Severity::Warning,
            TraceBand::Danger => Severity::Danger,
        };
        events.push(Event::alert(
            severity,
            Text::new("run.alert.band").with_term("band", band_term(to)),
        ));
    }

    /// The charges a limited program has left, for the line that uses one.
    fn used(&self, after: &RunState, program: Program, key: &'static str, left: &'static str) -> Text {
        match after.supply(self.mission, program) {
            Some(crate::run::Supply::Limited(count)) => {
                Text::new(left).with_int("left", number(count))
            }
            _ => Text::new(key),
        }
    }

    /// What a refused action says.
    pub(super) fn refusal(error: ActionError) -> Event {
        let text = match error {
            ActionError::RunOver => Text::new("run.error.over"),
            ActionError::NotEnoughCycles => Text::new("run.error.cycles"),
            ActionError::NotLinked => Text::new("run.error.not_linked"),
            ActionError::IceBlocks => Text::new("run.error.ice_up"),
            ActionError::NoIce => Text::new("run.error.no_ice"),
            ActionError::NotABreach(program) => {
                Text::new("run.error.not_a_breach").with_text("program", Self::program(program))
            }
            ActionError::NotInDeck(program) => {
                Text::new("run.error.not_in_deck").with_text("program", Self::program(program))
            }
            ActionError::NoCharges(program) => {
                Text::new("run.error.no_charges").with_text("program", Self::program(program))
            }
            ActionError::NeedsIntel => Text::new("run.error.needs_intel"),
            ActionError::NothingToLoot => Text::new("run.error.nothing_to_loot"),
            ActionError::ObjectivesMissing => Text::new("run.error.objectives"),
            ActionError::AlreadyActive(utility) => Text::new("run.error.already_active")
                .with_text("program", Self::program(utility.program())),
            ActionError::NothingToErase => Text::new("run.error.nothing_to_erase"),
        };
        Event::error(text)
    }

    /// What the player's action did, in the order of § 3.1: the result, the node that opens,
    /// the Trace and its band, the projection when it changed, the cycles left.
    pub(super) fn after_action(
        &self,
        before: &RunState,
        after: &RunState,
        facts: &[Fact],
    ) -> Vec<Event> {
        let mut events = Vec::new();
        let mut trace = before.trace();
        for fact in facts {
            match *fact {
                Fact::Moved { to } => events.push(Event::system(
                    Text::new("run.move").with_text("at", self.node(to)),
                )),
                Fact::Breached {
                    node,
                    program,
                    to,
                    ..
                } => {
                    let strength = self.mission.ice(node).map_or(0, |ice| ice.strength);
                    events.push(Event::system(
                        Text::new("run.breach")
                            .with_text("program", Self::program(program))
                            .with_text("at", self.node(node))
                            .with_int("to", number(to))
                            .with_int("strength", number(strength)),
                    ));
                }
                Fact::Opened { node } => events.push(Event::system(
                    Text::new("run.node.open").with_text("at", self.node(node)),
                )),
                Fact::PatrolDown { patrol } => events.push(Event::system(
                    Text::new("run.patrol_down")
                        .with_int("id", number(patrol.saturating_add(1)))
                        .with_int("turns", number(VIRUS_TURNS)),
                )),
                Fact::CloakOn => events.push(Event::system(self.used(
                    after,
                    Program::Cloak,
                    "run.use.cloak",
                    "run.use.cloak_left",
                ))),
                Fact::GhostOn => events.push(Event::system(self.used(
                    after,
                    Program::Ghost,
                    "run.use.ghost",
                    "run.use.ghost_left",
                ))),
                Fact::Erased { removed } => {
                    events.push(Event::system(self.used(
                        after,
                        Program::Spoof,
                        "run.use.spoof",
                        "run.use.spoof_left",
                    )));
                    let to = trace.saturating_sub(removed);
                    self.trace_moved(trace, to, false, &mut events);
                    trace = to;
                }
                Fact::ExtraCycles { count } => {
                    let text = self
                        .used(after, Program::Overclock, "run.use.overclock", "run.use.overclock_left")
                        .with_int("count", number(count));
                    events.push(Event::system(text));
                }
                Fact::Looted { node } => {
                    let kind = match self.mission.loot(node) {
                        Some(LootKind::Objective) => "run.loot_kind.objective",
                        Some(LootKind::Intel) => "run.loot_kind.intel",
                        _ => "run.loot_kind.extra",
                    };
                    events.push(Event::reward(
                        Text::new("run.loot")
                            .with_text("at", self.node(node))
                            .with_text("kind", Text::new(kind)),
                    ));
                }
                Fact::TraceUp { added } => {
                    let to = trace.saturating_add(added);
                    self.trace_moved(trace, to, after.status() == Status::Burned, &mut events);
                    trace = to;
                }
                Fact::PatrolMoved { .. }
                | Fact::Scanned { .. }
                | Fact::ScanCancelled { .. }
                | Fact::Ambient { .. }
                | Fact::TurnStarted { .. }
                | Fact::Ended { .. } => {}
            }
        }
        if after.is_running() {
            self.announce_change(before, after, &mut events);
            if before.cycles() != after.cycles() {
                events.push(Event::system(self.cycles_left(after)));
            }
        }
        events
    }

    /// The projection, written only when the Trace the end of the turn adds has changed.
    fn announce_change(&self, before: &RunState, after: &RunState, events: &mut Vec<Event>) {
        let was = forecast(self.mission, before, 1);
        let now = forecast(self.mission, after, 1);
        if was.total() == now.total() {
            return;
        }
        if let Some(turn) = now.turns.first() {
            events.push(Event::system(self.projection(after, turn)));
        }
    }

    /// "End of turn now: Trace +2 (0 → 2, calm)."
    pub(super) fn projection(&self, state: &RunState, turn: &ForecastTurn) -> Text {
        let burned = turn.after.status() == Status::Burned;
        Text::new("run.projection")
            .with_int("total", number(turn.total))
            .with_int("from", self.shown(state.trace()))
            .with_int("to", self.shown(turn.after.trace()))
            .with_text("band", Self::band(turn.after.trace(), burned))
    }

    /// The cycles left, and what to do at zero.
    pub(super) fn cycles_left(&self, state: &RunState) -> Text {
        let key = if state.cycles() == 0 {
            "run.cycles.none"
        } else {
            "run.cycles"
        };
        Text::new(key)
            .with_int("left", number(state.cycles()))
            .with_int("limit", self.cycle_limit(state))
    }

    /// The end of the turn, in the order of § 3.2: the patrols move, they scan, the ambient
    /// Trace rises, the Trace and its band, then the next turn and its forecast.
    pub(super) fn after_end(
        &self,
        before: &RunState,
        after: &RunState,
        facts: &[Fact],
    ) -> Vec<Event> {
        let mut events = Vec::new();
        for fact in facts {
            match *fact {
                Fact::PatrolMoved { patrol, to, .. } => events.push(aside(
                    Text::new("run.end.patrol_moves")
                        .with_int("id", number(patrol.saturating_add(1)))
                        .with_text("to", self.node(to)),
                )),
                Fact::Scanned {
                    patrol,
                    target,
                    added,
                    ..
                } => events.push(Event::alert(
                    Severity::Warning,
                    Text::new("run.end.scan")
                        .with_int("id", number(patrol.saturating_add(1)))
                        .with_text("target", self.node(target))
                        .with_int("added", number(added)),
                )),
                Fact::ScanCancelled { patrol, target, .. } => events.push(aside(
                    Text::new("run.end.scan_blocked")
                        .with_int("id", number(patrol.saturating_add(1)))
                        .with_text("target", self.node(target)),
                )),
                Fact::Ambient { added } => events.push(aside(
                    Text::new("run.end.ambient").with_int("added", number(added)),
                )),
                _ => {}
            }
        }
        self.trace_moved(
            before.trace(),
            after.trace(),
            after.status() == Status::Burned,
            &mut events,
        );
        if after.is_running() {
            events.push(Event::Break);
            events.push(self.turn_banner(after));
            events.extend(self.forecast_events(after));
        }
        events
    }

    /// The forecast: the total first, always, then the detail.
    pub(super) fn forecast_events(&self, state: &RunState) -> Vec<Event> {
        let announced = forecast(self.mission, state, self.horizon);
        let mut events = Vec::new();
        for (index, turn) in announced.turns.iter().enumerate() {
            if index == 0 {
                events.push(Event::system(
                    Text::new("run.forecast.total").with_int("total", number(turn.total)),
                ));
            } else {
                events.push(aside(
                    Text::new("run.forecast.next_turn")
                        .with_int("number", number(turn.turn))
                        .with_int("total", number(turn.total)),
                ));
            }
            for fact in &turn.facts {
                if let Some(text) = self.forecast_fact(turn, fact) {
                    events.push(aside(text));
                }
            }
            if turn.after.status() == Status::Burned {
                events.push(Event::alert(
                    Severity::Warning,
                    Text::new("run.forecast.burn").with_int("max", number(self.mission.trace_cap())),
                ));
            }
        }
        events
    }

    fn forecast_fact(&self, turn: &ForecastTurn, fact: &Fact) -> Option<Text> {
        Some(match *fact {
            Fact::PatrolMoved { patrol, to, .. } => Text::new("run.forecast.patrol_moves")
                .with_int("id", number(patrol.saturating_add(1)))
                .with_text("to", self.node(to)),
            Fact::Scanned {
                patrol,
                target,
                added,
                ..
            } => Text::new(if turn.cloak_usable {
                "run.forecast.scan_cloak"
            } else {
                "run.forecast.scan"
            })
            .with_int("id", number(patrol.saturating_add(1)))
            .with_text("target", self.node(target))
            .with_int("added", number(added)),
            Fact::ScanCancelled { patrol, target, .. } => Text::new("run.forecast.scan_blocked")
                .with_int("id", number(patrol.saturating_add(1)))
                .with_text("target", self.node(target)),
            Fact::Ambient { added } => {
                Text::new("run.forecast.ambient").with_int("added", number(added))
            }
            _ => return None,
        })
    }
}
