//! A game that is one intrusion on one site: how the plain and the full-screen interfaces play
//! the intrusion before the campaign hands it over (lot R4.6). The campaign will drive the same
//! [`Intrusion`].

use thiserror::Error;

use crate::campaign::{Difficulty, embedded_content};
use crate::command::{ArgRef, Availability, Capabilities, CommandSpec, Lookup, NoLists};
use crate::content::ids::SiteId;
use crate::event::Event;
use crate::game::{Game, GaugeReading, View};
use crate::prompt::{Input, Prompt, Step};
use crate::run::{Action, NodeId, Program, Utility};
use crate::save::SaveError;
use crate::text::Text;

use super::commands::{COMMANDS, CONTEXT};
use super::gloss::{gloss, gloss_event, gloss_prompt, gloss_view};
use super::lists::Lists;
use super::session::{Intrusion, StartError};

/// Why a game of one intrusion could not be started.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IntrusionError {
    /// The embedded content does not load (the tests of the crate make this impossible in a
    /// build that passes them).
    #[error("the content does not load:\n{0}")]
    Content(String),
    /// The site is not a valid identifier.
    #[error("`{0}` is not a site identifier")]
    BadSite(String),
    /// The intrusion cannot begin.
    #[error(transparent)]
    Start(#[from] StartError),
}

/// What the game waits for besides a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// `jackout` with loot still there.
    ConfirmJackout { left: u8 },
    /// `quit`.
    ConfirmQuit,
}

/// One intrusion, as a frontend plays it.
#[derive(Debug, Clone)]
pub struct IntrusionGame {
    intrusion: Intrusion,
    flow: Option<Flow>,
    /// The game is over: the run ended, or the player left.
    over: bool,
    frontend: Capabilities,
}

impl IntrusionGame {
    /// An intrusion on a site of the embedded content.
    ///
    /// # Errors
    ///
    /// [`IntrusionError`] when the content does not load, the name is not a site identifier,
    /// or the site has no intrusion.
    pub fn new(site: &str, difficulty: Difficulty) -> Result<Self, IntrusionError> {
        let content = embedded_content().map_err(IntrusionError::Content)?;
        let id: SiteId = site
            .parse()
            .map_err(|_| IntrusionError::BadSite(site.to_owned()))?;
        Ok(Self {
            intrusion: Intrusion::new(content, &id, difficulty)?,
            flow: None,
            over: false,
            frontend: Capabilities::PLAIN,
        })
    }

    /// The intrusion, for the tests and tools that read a game.
    #[must_use]
    pub fn intrusion(&self) -> &Intrusion {
        &self.intrusion
    }

    /// The panel of the interface: everything it draws, read from the state.
    #[must_use]
    pub fn run_view(&self) -> super::RunView {
        let mut view = self.intrusion.view();
        gloss_view(&mut view);
        view
    }

    fn step(&self, mut events: Vec<Event>) -> Step {
        events.iter_mut().for_each(gloss_event);
        Step {
            events,
            prompt: self.prompt(),
            save: None,
        }
    }

    fn open(_spec: &CommandSpec) -> Availability {
        Availability::Open
    }

    fn lists(&self) -> Lists<'_> {
        Lists {
            mission: self.intrusion.run().mission(),
            state: self.intrusion.state(),
        }
    }

    /// Ends the game if the run is over, with what closes it.
    fn settle(&mut self, events: &mut Vec<Event>) {
        if self.intrusion.is_over() {
            events.extend(self.intrusion.closing());
            self.over = true;
        }
    }

    fn do_action(&mut self, action: Action, events: &mut Vec<Event>) {
        self.intrusion.act(action, events);
        self.settle(events);
    }

    fn command(&mut self, line: &str, events: &mut Vec<Event>) {
        let lookup = COMMANDS.parse(line, CONTEXT, self.frontend, Self::open, &self.lists());
        match lookup {
            Lookup::Empty => {}
            Lookup::Unknown(word) => events.push(Event::error(
                Text::new("error.unknown_command").with_str("command", word),
            )),
            Lookup::Locked { reason, .. } => events.push(Event::error(
                Text::new("error.locked").with_text("reason", reason),
            )),
            Lookup::WrongContext { reason, .. } => events.push(Event::error(reason)),
            Lookup::BadArg { error, .. } => events.push(Event::error(error.text())),
            Lookup::Found { spec, args } => self.execute(spec, &args, events),
        }
    }

    fn node_arg(args: &[ArgRef], position: usize) -> Option<NodeId> {
        let number: u8 = args.get(position)?.id()?.parse().ok()?;
        NodeId::from_number(number)
    }

    fn program_arg(args: &[ArgRef], position: usize) -> Option<Program> {
        Program::from_id(args.get(position)?.id()?)
    }

    fn execute(&mut self, spec: &CommandSpec, args: &[ArgRef], events: &mut Vec<Event>) {
        match spec.name {
            "help" => self.help(args, events),
            "status" => events.extend(self.intrusion.status()),
            "map" => events.extend(self.intrusion.map()),
            "quit" => self.flow = Some(Flow::ConfirmQuit),
            "move" => {
                if let Some(node) = Self::node_arg(args, 0) {
                    self.do_action(Action::Move(node), events);
                }
            }
            "breach" => {
                if let (Some(node), Some(program)) =
                    (Self::node_arg(args, 0), Self::program_arg(args, 1))
                {
                    self.do_action(Action::Breach { node, program }, events);
                }
            }
            "use" => {
                if let Some(utility) = Self::program_arg(args, 0).and_then(Utility::of) {
                    self.do_action(Action::Use(utility), events);
                }
            }
            "loot" => self.do_action(Action::Loot, events),
            "end" => self.do_action(Action::End, events),
            "jackout" => {
                let left = self.intrusion.loot_left();
                let ready = self
                    .intrusion
                    .state()
                    .objectives_done(self.intrusion.run().mission());
                if ready && left > 0 {
                    self.flow = Some(Flow::ConfirmJackout { left });
                } else {
                    self.do_action(Action::Jackout, events);
                }
            }
            "undo" => self.intrusion.undo(events),
            other => events.push(Event::error(
                Text::new("campaign.frontend.unhandled").with_str("command", other),
            )),
        }
    }

    fn help(&self, args: &[ArgRef], events: &mut Vec<Event>) {
        let Some(name) = args.first().and_then(ArgRef::as_word) else {
            events.push(COMMANDS.help(
                Text::new("run.help.title"),
                &[
                    Text::new("run.help.col_command"),
                    Text::new("run.help.col_effect"),
                ],
                CONTEXT,
                self.frontend,
                Self::open,
            ));
            return;
        };
        if let Lookup::Found { spec, .. } | Lookup::BadArg { spec, .. } =
            COMMANDS.parse(name, CONTEXT, self.frontend, Self::open, &NoLists)
        {
            events.push(Event::system(Text::dynamic(spec.help.to_owned())));
            if !spec.args.is_empty() {
                events.push(Event::system(
                    Text::new("run.help.usage").with_text("usage", Text::dynamic(spec.usage_key())),
                ));
            }
        } else {
            events.push(Event::error(Text::new("ui.invalid_input")));
        }
    }

    fn confirm(&mut self, flow: Flow, input: &Input, events: &mut Vec<Event>) {
        match input {
            Input::Confirm(yes) => {
                self.flow = None;
                match (flow, yes) {
                    (Flow::ConfirmJackout { .. }, true) => {
                        self.do_action(Action::Jackout, events);
                    }
                    (Flow::ConfirmQuit, true) => self.over = true,
                    (_, false) => {}
                }
            }
            Input::Cancel => self.flow = None,
            _ => events.push(Event::error(Text::new("ui.invalid_input"))),
        }
    }
}

impl Game for IntrusionGame {
    fn start(&mut self) -> Step {
        self.step(self.intrusion.opening())
    }

    fn handle(&mut self, input: Input) -> Step {
        if self.over {
            return self.step(Vec::new());
        }
        if input == Input::Eof {
            self.over = true;
            return self.step(vec![Event::system(Text::new("ui.eof"))]);
        }
        let mut events = Vec::new();
        match (self.flow, &input) {
            (Some(flow), _) => self.confirm(flow, &input, &mut events),
            (None, Input::Line(line)) => self.command(line.trim(), &mut events),
            (None, Input::Cancel) => {}
            (None, _) => events.push(Event::error(Text::new("ui.invalid_input"))),
        }
        self.step(events)
    }

    fn prompt(&self) -> Prompt {
        if self.over {
            return Prompt::End;
        }
        let mut prompt = match self.flow {
            None => Prompt::Command,
            Some(Flow::ConfirmJackout { left }) => Prompt::Confirm {
                question: Text::new("run.jackout.confirm").with_int("left", i64::from(left)),
                default: false,
            },
            Some(Flow::ConfirmQuit) => Prompt::Confirm {
                question: Text::new("run.quit.confirm"),
                default: false,
            },
        };
        gloss_prompt(&mut prompt);
        prompt
    }

    fn resume(&self) -> Step {
        self.step(self.intrusion.situation())
    }

    fn snapshot(&self) -> Result<String, SaveError> {
        Err(SaveError::Invalid(
            "an intrusion played on its own is not saved: the campaign saves the intrusion it runs"
                .to_owned(),
        ))
    }

    fn view(&self) -> View {
        let panel = self.intrusion.view();
        let mut band = panel.trace.band.clone();
        gloss(&mut band);
        View {
            player: self.intrusion.site().as_str().to_owned(),
            gauges: vec![GaugeReading {
                band,
                ..panel.trace
            }],
            sections: Vec::new(),
        }
    }

    fn set_capabilities(&mut self, capabilities: Capabilities) {
        self.frontend = capabilities;
    }

    fn complete(&self, line: &str) -> Vec<String> {
        if self.flow.is_some() || self.over {
            return Vec::new();
        }
        if line.contains(char::is_whitespace) {
            COMMANDS.complete_args(line, CONTEXT, self.frontend, Self::open, &self.lists())
        } else {
            COMMANDS.complete(line, CONTEXT, self.frontend, Self::open)
        }
    }
}
