//! From an input to the command or the menu that answers it, and back to a prompt.

use std::str::FromStr;

use crate::command::{ArgRef, Availability, CommandSpec, HandledBy, Lookup};
use crate::content::{Content, Fact};
use crate::event::Event;
use crate::game::Game;
use crate::prompt::{Input, Prompt};
use crate::save::SaveRequest;
use crate::text::Text;

use super::CampaignGame;
use super::commands::{COMMANDS, CONTEXT, FRONTEND};
use super::resolver::Lists;
use super::state::{CampaignState, DEFAULT_HANDLE, Flow, HANDLE_MAX_CHARS};
use super::tutorial::Completion;

/// Whether a command is open, or why not: the rules of `unlocks.toml`, and the commands that
/// only the frontend can run (which no frontend hands over yet).
pub(super) fn availability(c: &Content, s: &CampaignState, spec: &CommandSpec) -> Availability {
    if matches!(spec.handled_by, HandledBy::Frontend(_)) {
        return Availability::Locked(Text::new("campaign.unlock.frontend"));
    }
    match c.unlocks.iter().find(|rule| rule.command == spec.name) {
        Some(rule) if !rule.when.eval(c, &s.missions) => {
            Availability::Locked(Text::dynamic(rule.reason.clone()))
        }
        _ => Availability::Open,
    }
}

/// An error line for an input the current prompt cannot use.
pub(super) fn invalid(events: &mut Vec<Event>) {
    events.push(Event::error(Text::new("ui.invalid_input")));
}

/// The typed id of a resolved argument.
pub(super) fn arg_id<T: FromStr>(args: &[ArgRef], position: usize) -> Option<T> {
    args.get(position)?.id()?.parse().ok()
}

impl CampaignGame {
    /// Hands the input to whatever the game is waiting for. Returns the save the answer asks
    /// for explicitly (a slot, a quit); the autosave and the checkpoint are decided by the
    /// caller.
    pub(super) fn advance(
        &mut self,
        input: &Input,
        events: &mut Vec<Event>,
    ) -> Option<SaveRequest> {
        match self.state.flows.last().cloned() {
            None => self.command(input, events),
            Some(Flow::AskHandle) => {
                self.ask_handle(input, events);
                None
            }
            Some(Flow::Prologue { page }) => {
                self.prologue_step(page, input, events);
                None
            }
            Some(Flow::OfferTutorial) => {
                self.offer_step(input, events);
                None
            }
            Some(Flow::Talk { contact }) => {
                self.talk_menu(&contact, input, events);
                None
            }
            Some(Flow::Decision { decision }) => {
                self.decision_menu(&decision, input, events);
                None
            }
            Some(Flow::ConfirmHack { site }) => {
                self.confirm_hack(&site, input, events);
                None
            }
            Some(Flow::ConfirmBuy { item }) => {
                self.confirm_buy(&item, input, events);
                None
            }
            Some(Flow::ConfirmQuit) => self.confirm_quit(input, events),
        }
    }

    /// What the game is waiting for, derived from the state.
    pub(super) fn flow_prompt(&self) -> Prompt {
        match self.state.flows.last() {
            None => Prompt::Command,
            Some(Flow::AskHandle) => Prompt::Text {
                label: Text::new("campaign.prompt.handle").with_term("handle", "handle"),
                max_chars: HANDLE_MAX_CHARS,
                default: Some(DEFAULT_HANDLE.to_owned()),
            },
            Some(Flow::Prologue { .. }) => Prompt::Continue,
            Some(Flow::OfferTutorial) => Prompt::Confirm {
                question: Text::new("tutorial.offer"),
                default: true,
            },
            Some(Flow::Talk { contact }) => Prompt::Choice(self.talk_choice(contact)),
            Some(Flow::Decision { decision }) => Prompt::Choice(self.decision_choice(decision)),
            Some(Flow::ConfirmHack { site }) => Prompt::Confirm {
                question: Text::new("campaign.hack.confirm")
                    .with_term("site", "site")
                    .with_str("name", site.as_str()),
                default: true,
            },
            Some(Flow::ConfirmBuy { item }) => Prompt::Confirm {
                question: self.buy_question(item),
                default: false,
            },
            Some(Flow::ConfirmQuit) => Prompt::Confirm {
                question: Text::new("campaign.quit.confirm").with_term("net", "net"),
                default: false,
            },
        }
    }

    /// The completions of what is typed, without spoilers: only what could be used now.
    pub(super) fn complete_line(&self, line: &str) -> Vec<String> {
        let open = |spec: &CommandSpec| self.availability(spec);
        match self.prompt() {
            Prompt::Command if !line.contains(char::is_whitespace) => {
                COMMANDS.complete(line, CONTEXT, FRONTEND, open)
            }
            Prompt::Command => {
                let lists = Lists {
                    c: self.content,
                    s: &self.state,
                };
                COMMANDS.complete_args(line, CONTEXT, FRONTEND, open, &lists)
            }
            Prompt::Choice(choice) => {
                let prefix = line.to_ascii_lowercase();
                choice
                    .options
                    .into_iter()
                    .filter(|option| option.available.is_ok())
                    .map(|option| option.id)
                    .filter(|id| id.starts_with(&prefix))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn command(&mut self, input: &Input, events: &mut Vec<Event>) -> Option<SaveRequest> {
        match input {
            Input::Line(line) => self.run_line(line.trim(), events),
            Input::Cancel => None,
            _ => {
                invalid(events);
                None
            }
        }
    }

    fn run_line(&mut self, line: &str, events: &mut Vec<Event>) -> Option<SaveRequest> {
        let lookup = {
            let lists = Lists {
                c: self.content,
                s: &self.state,
            };
            COMMANDS.parse(
                line,
                CONTEXT,
                FRONTEND,
                |spec| self.availability(spec),
                &lists,
            )
        };
        match lookup {
            Lookup::Empty => None,
            Lookup::Unknown(word) => {
                events.push(Event::error(
                    Text::new("error.unknown_command").with_str("command", word),
                ));
                None
            }
            Lookup::Locked { reason, .. } => {
                events.push(Event::error(
                    Text::new("error.locked").with_text("reason", reason),
                ));
                None
            }
            Lookup::WrongContext { reason, .. } => {
                events.push(Event::error(reason));
                None
            }
            Lookup::BadArg { error, .. } => {
                events.push(Event::error(error.text()));
                None
            }
            Lookup::Found { spec, args } => self.execute(spec, &args, events),
        }
    }

    /// Runs an open command whose arguments are resolved.
    fn execute(
        &mut self,
        spec: &CommandSpec,
        args: &[ArgRef],
        events: &mut Vec<Event>,
    ) -> Option<SaveRequest> {
        match spec.name {
            "help" => self.cmd_help(args, events),
            "status" => self.cmd_status(events),
            "quit" => self.cmd_quit(),
            "quests" => self.cmd_quests(args, events),
            "accept" => self.cmd_accept(args, events),
            "contacts" => self.cmd_contacts(events),
            "talk" => self.cmd_talk(args, events),
            "messages" => self.cmd_messages(events),
            "read" => self.cmd_read(args, events),
            "archives" => self.cmd_archives(args, events),
            "decrypt" => self.cmd_decrypt(args, events),
            "net" => self.cmd_net(args, events),
            "hack" => self.cmd_hack(args),
            "shop" => self.cmd_shop(events),
            "buy" => self.cmd_buy(args),
            "laylow" => self.cmd_laylow(args, events),
            "link" => self.cmd_link(args, events),
            "hint" => self.cmd_hint(events),
            "tutorial" => self.cmd_tutorial(args, events),
            "save" => {
                let request = Self::cmd_save(args, events);
                if request.is_some() {
                    self.state.tutorial.observe("save", Completion::Command);
                }
                return request;
            }
            // A command declared for the frontend that its frontend did not handle.
            other => events.push(Event::error(
                Text::new("campaign.frontend.unhandled").with_str("command", other),
            )),
        }
        self.state.tutorial.observe(spec.name, Completion::Command);
        None
    }

    /// Tells the content a command was used (the `use` objectives), once: nothing changes
    /// the second time, so nothing is refreshed.
    pub(super) fn used(&mut self, command: &str, events: &mut Vec<Event>) {
        let Ok(id) = command.parse() else { return };
        if !self.content.has_command(command) || self.state.missions.used.contains(&id) {
            return;
        }
        let at = self.state.next_turn();
        self.apply(vec![Fact::CommandUsed { command: id }], at, events);
    }
}
