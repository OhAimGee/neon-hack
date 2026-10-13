//! The state of the full-screen interface, and its keys.
//!
//! [`App`] is a plain state machine over a [`Game`]: keys, pastes and sizes go in, the screen
//! is drawn from the state (see `draw`). Nothing here touches the terminal, so it is tested
//! with ratatui's `TestBackend`. Everything the screen shows is also said by the plain
//! frontend: the log uses the very same renderer, and the panel only repeats what `status`
//! would say.

mod draw;
mod fx;
mod modal;
mod panel;

use std::cell::Cell;
use std::collections::VecDeque;
use std::time::Duration;

use neon_engine::event::{Event, Gauge};
use neon_engine::text::Text;
use neon_engine::{Game, Input, Prompt, Step};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use self::modal::Modal;
use self::panel::PanelState;
use super::editor::{LineEditor, MAX_CHARS};
use super::motion::Staging;
use super::text::width;
use super::typing::Typing;
use crate::config::TypewriterChoice;
use crate::input::{InputError, to_input};
use crate::options::Host;
use crate::persist::Persistence;
use crate::render::Renderer;
use crate::transcript::Transcript;

/// Smallest terminal with the permanent side panel.
pub(crate) const FULL_SIZE: (u16, u16) = (100, 28);
/// Smallest terminal at all: below it the interface asks for more room.
pub(crate) const COMPACT_SIZE: (u16, u16) = (64, 20);
/// Width of the side panel, borders included.
const PANEL_WIDTH: u16 = 30;
/// Most rows the area under the log can take (menu, candidates and the input line).
const PROMPT_AREA_MAX: u16 = 10;
/// Most rows used to list the candidates of a completion.
const CANDIDATE_ROWS_MAX: usize = 3;
/// Events and echoes kept in the log; the oldest go first. The transcript (`export`, R3.3)
/// is what keeps everything.
const LOG_CAPACITY: usize = 5000;

/// How much of the interface fits in the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tier {
    /// Log, status bar, input and the permanent side panel.
    Full,
    /// Log, status bar and input; the panel's content is in the status bar.
    Compact,
    /// Not enough room to play: the screen only says so.
    TooSmall,
}

pub(crate) fn tier(area: Rect) -> Tier {
    if area.width >= FULL_SIZE.0 && area.height >= FULL_SIZE.1 {
        Tier::Full
    } else if area.width >= COMPACT_SIZE.0 && area.height >= COMPACT_SIZE.1 {
        Tier::Compact
    } else {
        Tier::TooSmall
    }
}

/// How the interface ends, when it ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    /// The game is over or the player left it: the program ends.
    Quit,
    /// The player asked for the line-by-line display (`Ctrl+P`, `plain`): the game goes on,
    /// in the plain frontend, with nothing lost.
    Plain,
}

/// What the log keeps. Events are kept as events, so that changing the language or the mode
/// re-renders the whole history; echoes of typed lines are the text as it was typed.
enum LogEntry {
    Event(Event),
    Echo(String),
}

/// The interface state.
pub(crate) struct App<'a> {
    game: &'a mut dyn Game,
    renderer: Renderer<'a>,
    persistence: Persistence,
    log: VecDeque<LogEntry>,
    /// Rows the log is scrolled back from its end (0 follows the game). Only a request: the
    /// drawing, which knows the size and the length, clamps it.
    scroll: Cell<usize>,
    /// Rows the log area showed at the last drawing, so that a page is a page.
    page: Cell<usize>,
    /// The tier of the last drawing: keys mean nothing when the screen is too small to see.
    shown_tier: Cell<Tier>,
    prompt: Prompt,
    editor: LineEditor,
    /// What the last TAB could not decide between, listed under the log until the next key.
    candidates: Vec<String>,
    finished: bool,
    /// Set when the interface is done, and how.
    exit: Option<Exit>,
    /// Whether scenes may be staged (`--reduce-motion` and the screen reader say no).
    motion: bool,
    /// The scene being staged over the log, if any. It ends by itself, or at the first key.
    staging: Option<Staging>,
    /// Entries at the end of the log that the staged scene keeps back until it is over, so
    /// that the window reveals the result instead of sitting on top of it. Always 0 when no
    /// scene plays.
    held: usize,
    /// Columns a second the text is typed out at, or `None` when it appears at once.
    typewriter: Option<f64>,
    /// The typing of the last step, if it is still going. It ends by itself, or at the first key.
    typing: Option<Typing>,
    /// Entries at the end of the log that are being typed out; only meaningful while
    /// `typing` is set, and 0 otherwise.
    typed: usize,
    /// The presentation commands (`options`, `verbosity`), when the game has them.
    options: Option<Host<'a>>,
    /// The game lists `panel` and `plain`, which this interface runs.
    frontend_commands: bool,
    /// Whether the side panel is shown, in each layout.
    panel: PanelState,
    /// The list kept open beside the log, if any (see `modal`).
    modal: Option<Modal>,
    /// Everything said and typed, as text, for `export`. The log keeps the most recent
    /// events; this keeps the lines.
    transcript: Transcript,
}

impl<'a> App<'a> {
    /// Attaches the interface to a game, starting from `first`: the step of a new game or
    /// the current state of a game under way (see [`Game::resume`]).
    pub(crate) fn new(
        game: &'a mut dyn Game,
        renderer: Renderer<'a>,
        first: Step,
        persistence: Persistence,
    ) -> Self {
        let mut app = Self {
            game,
            renderer,
            persistence,
            log: VecDeque::new(),
            scroll: Cell::new(0),
            page: Cell::new(1),
            shown_tier: Cell::new(Tier::Compact),
            prompt: Prompt::Command,
            editor: LineEditor::new(),
            candidates: Vec::new(),
            finished: false,
            exit: None,
            motion: false,
            staging: None,
            held: 0,
            typewriter: None,
            typing: None,
            typed: 0,
            options: None,
            frontend_commands: false,
            panel: PanelState::default(),
            modal: None,
            transcript: Transcript::new(),
        };
        app.apply(first);
        app
    }

    /// Allows (or forbids) staging the scenes the game tells of. Off until asked for, so that
    /// an interface that is not told about motion never moves by itself.
    pub(crate) fn with_motion(mut self, motion: bool) -> Self {
        self.motion = motion;
        self
    }

    /// Types out what the game says at this pace. Off until asked for, like motion, so that an
    /// interface that is not told about it shows everything at once.
    pub(crate) fn with_typewriter(mut self, choice: TypewriterChoice) -> Self {
        self.typewriter = choice.rate();
        self
    }

    /// Lets the player change the presentation in the middle of the game (`options`,
    /// `verbosity`). Without it these lines go to the game, which does not take them.
    pub(crate) fn with_options(mut self, options: Host<'a>) -> Self {
        self.options = Some(options);
        self
    }

    /// Lets the player type the commands that belong to this interface (`panel`, `plain`),
    /// when the game lists them. F2 and Ctrl+P do the same in any case.
    pub(crate) fn with_frontend_commands(mut self) -> Self {
        self.frontend_commands = true;
        self
    }

    /// True while a scene is being staged: the screen then needs to be drawn again at the
    /// pace of [`super::motion::FRAME`] even though nothing was typed. This is the only time
    /// the interface moves by itself.
    pub(crate) fn is_animating(&self) -> bool {
        self.staging.is_some() || self.typing.is_some()
    }

    /// Moves the staged scene, or the typing, forward by the time that passed. When a scene
    /// ends by itself, what it kept back is then typed out.
    pub(crate) fn tick(&mut self, elapsed: Duration) {
        if let Some(staging) = &mut self.staging {
            staging.tick(elapsed);
            if staging.is_over() {
                let kept_back = self.held;
                self.end_staging();
                self.start_typing(kept_back);
            }
        } else if let Some(typing) = &mut self.typing {
            typing.tick(elapsed);
            if typing.is_over() {
                self.end_staging();
            }
        }
    }

    /// Ends the staged scene and the typing, if any: the log shows everything at once.
    fn end_staging(&mut self) {
        self.staging = None;
        self.held = 0;
        self.typing = None;
        self.typed = 0;
    }

    /// Starts typing out the last `count` entries of the log, if the player wants that: the
    /// typewriter is on and motion is allowed.
    fn start_typing(&mut self, count: usize) {
        let count = count.min(self.log.len());
        let Some(rate) = self.typewriter.filter(|_| self.motion) else {
            return;
        };
        let total: usize = self
            .log
            .iter()
            .rev()
            .take(count)
            .map(|entry| match entry {
                LogEntry::Event(event) => self
                    .renderer
                    .event(event)
                    .iter()
                    .map(|line| width(&line.text))
                    .sum(),
                LogEntry::Echo(_) => 0,
            })
            .sum();
        self.typing = Typing::new(total, rate);
        self.typed = if self.typing.is_some() { count } else { 0 };
    }

    /// Whether the interface is done, and how.
    pub(crate) fn exit(&self) -> Option<Exit> {
        self.exit
    }

    /// What is left to the next frontend when this one gives up the game: the game itself,
    /// the renderer as the player's `options` left it, those commands, and the saves.
    pub(crate) fn into_parts(
        self,
    ) -> (
        &'a mut dyn Game,
        Renderer<'a>,
        Option<Host<'a>>,
        Persistence,
    ) {
        (self.game, self.renderer, self.options, self.persistence)
    }

    fn push(&mut self, entry: LogEntry) {
        match &entry {
            LogEntry::Event(event) => self
                .transcript
                .record(self.renderer.event(event).into_iter().map(|line| line.text)),
            LogEntry::Echo(text) => self.transcript.record([self.renderer.verbatim(text)]),
        }
        if self.log.len() >= LOG_CAPACITY {
            self.log.pop_front();
        }
        self.log.push_back(entry);
    }

    fn apply(&mut self, step: Step) {
        let staging = staging_of(&step.events).filter(|_| self.motion);
        self.follow_screens(&step.events);
        let pushed = step.events.len();
        for event in step.events {
            self.push(LogEntry::Event(event));
        }
        if let Some(staging) = staging {
            // The log is capped: never more than what is left of it.
            self.held = pushed.min(self.log.len());
            self.staging = Some(staging);
        } else {
            self.start_typing(pushed);
        }
        // A menu lists its choices above the input line, as the plain interface prints them.
        let header = self.renderer.prompt(&step.prompt).header;
        self.transcript
            .record(header.into_iter().map(|line| line.text));
        self.finished = step.prompt == Prompt::End;
        // A prompt that asks for a short answer says how short.
        let limit = match &step.prompt {
            Prompt::Text { max_chars, .. } => *max_chars,
            _ => MAX_CHARS,
        };
        self.editor.set_limit(limit);
        self.prompt = step.prompt;
    }

    fn submit(&mut self, input: Input) {
        // The player acted: whatever was staged is over.
        self.end_staging();
        let mut step = self.game.handle(input);
        self.persistence.after_step(&*self.game, &mut step);
        // The player acted: show what came of it.
        self.scroll.set(0);
        self.apply(step);
    }

    // ---- Keys ----------------------------------------------------------------------------

    /// Handles one key press (or auto-repeat; releases are filtered out before).
    pub(crate) fn on_key(&mut self, key: KeyEvent) {
        self.candidates.clear();
        // Any key ends a staged scene, and is then handled as usual: nothing typed is lost.
        self.end_staging();
        if self.finished {
            self.exit = Some(Exit::Quit);
            return;
        }
        let modifiers = key.modifiers;
        let control_only = modifiers.difference(KeyModifiers::SHIFT) == KeyModifiers::CONTROL;
        if self.shown_tier.get() == Tier::TooSmall {
            // Nothing can be seen, so nothing is typed into the game: only a way out, which
            // is to close the input or to play on in the line-by-line display.
            if control_only && matches!(key.code, KeyCode::Char('c' | 'd')) {
                self.submit(Input::Eof);
            } else if control_only && matches!(key.code, KeyCode::Char('p' | 'P')) {
                self.exit = Some(Exit::Plain);
            }
            return;
        }
        // Text is typed with no modifier, with Shift, or with AltGr, which Windows reports as
        // Control+Alt. Anything else is a shortcut or means nothing.
        let types_text = modifiers.difference(KeyModifiers::SHIFT).is_empty()
            || modifiers == (KeyModifiers::CONTROL | KeyModifiers::ALT);
        let word = modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::F(2) => self.toggle_panel(),
            KeyCode::Char(c) if types_text => self.editor.insert(c),
            KeyCode::Char(c) if control_only => self.control(c.to_ascii_lowercase()),
            KeyCode::Enter => self.enter(),
            KeyCode::Esc => self.escape(),
            KeyCode::Backspace => self.editor.backspace(),
            KeyCode::Delete => self.editor.delete(),
            KeyCode::Left if word => self.editor.word_left(),
            KeyCode::Right if word => self.editor.word_right(),
            KeyCode::Left => self.editor.left(),
            KeyCode::Right => self.editor.right(),
            // The history is of commands: a menu or a question has no use for it.
            KeyCode::Up if self.prompt == Prompt::Command => self.editor.history_previous(),
            KeyCode::Down if self.prompt == Prompt::Command => self.editor.history_next(),
            // With nothing typed, Home and End have no cursor to move: they scroll the log.
            KeyCode::Home if word || self.editor.is_empty() => self.scroll.set(usize::MAX),
            KeyCode::End if word || self.editor.is_empty() => self.scroll.set(0),
            KeyCode::Home => self.editor.home(),
            KeyCode::End => self.editor.end(),
            // A screen that is cut turns its own pages; otherwise the keys are the log's.
            KeyCode::PageUp if self.scroll_modal(false) => {}
            KeyCode::PageDown if self.scroll_modal(true) => {}
            KeyCode::PageUp => self.scroll_back(),
            KeyCode::PageDown => self.scroll_forward(),
            KeyCode::Tab => self.complete(),
            _ => {}
        }
    }

    /// Ctrl and a letter.
    fn control(&mut self, letter: char) {
        match letter {
            'a' => self.editor.home(),
            'e' => self.editor.end(),
            'b' => self.editor.left(),
            'f' => self.editor.right(),
            'u' => self.editor.kill_to_start(),
            'k' => self.editor.kill_to_end(),
            'w' => self.editor.kill_word(),
            // What some terminals send for Backspace.
            'h' => self.editor.backspace(),
            // The input closes on an empty line; with text, it is the usual Delete.
            'd' if self.editor.is_empty() => self.submit(Input::Eof),
            'd' => self.editor.delete(),
            'c' => self.interrupt(),
            // The line-by-line display, with the game exactly as it is.
            'p' => self.exit = Some(Exit::Plain),
            _ => {}
        }
    }

    /// Ctrl-C does not kill the process (the terminal would be left in raw mode): it asks to
    /// quit, exactly as typing `quit` does. Where `quit` does not exist (a menu, a question)
    /// it closes the input, like Ctrl-D, which ends the game from any depth.
    fn interrupt(&mut self) {
        if self.prompt == Prompt::Command {
            // What was being typed is dropped, as a shell does on Ctrl-C: it would otherwise
            // be the start of the answer to the question.
            self.editor.clear();
            self.run_line("quit");
        } else {
            self.submit(Input::Eof);
        }
    }

    /// Escape closes the screen, then the drawer, comes back down the log, clears the line,
    /// then backs out of a menu: the most recent thing opened goes first.
    fn escape(&mut self) {
        if self.close_modal() {
            return;
        }
        if self.shown_tier.get() == Tier::Compact && self.panel.drawer {
            self.panel.drawer = false;
        } else if self.scroll.get() > 0 {
            self.scroll.set(0);
        } else if !self.editor.is_empty() {
            self.editor.clear();
        } else if self.prompt != Prompt::Command {
            self.submit(Input::Cancel);
        }
    }

    fn scroll_back(&self) {
        self.scroll
            .set(self.scroll.get().saturating_add(self.page.get()));
    }

    fn scroll_forward(&self) {
        self.scroll
            .set(self.scroll.get().saturating_sub(self.page.get()));
    }

    /// A paste goes into the line, where it can be read and edited: it never runs anything.
    pub(crate) fn on_paste(&mut self, pasted: &str) {
        self.candidates.clear();
        self.end_staging();
        if self.finished {
            self.exit = Some(Exit::Quit);
        } else if self.shown_tier.get() != Tier::TooSmall {
            self.editor.paste(pasted);
        }
    }

    /// Answers the prompt with the line, through the mapping the plain frontend uses.
    fn enter(&mut self) {
        let line = self.editor.text().to_owned();
        let input = to_input(&self.prompt, &line);
        // The line is spent in any case; only commands are worth recalling (never a menu
        // answer, a handle or a yes or no).
        if self.prompt == Prompt::Command {
            self.editor.remember(&line);
        }
        self.editor.clear();
        self.echo(&line);
        if self.prompt == Prompt::Command
            && (self.run_frontend_command(&line) || self.run_presentation(&line))
        {
            return;
        }
        match input {
            Ok(input) => self.submit(input),
            Err(InputError::NotYesOrNo) => {
                self.push(LogEntry::Event(Event::error(Text::new(
                    "ui.invalid_confirm",
                ))));
            }
        }
    }

    /// Runs `plain` or `panel`, if that is the line.
    fn run_frontend_command(&mut self, line: &str) -> bool {
        if !self.frontend_commands {
            return false;
        }
        let word = line.trim();
        if word.eq_ignore_ascii_case("plain") {
            self.exit = Some(Exit::Plain);
            true
        } else if word.eq_ignore_ascii_case("panel") {
            self.run_panel();
            true
        } else {
            false
        }
    }

    /// `panel` shows or hides the side panel, and says which in the log, so that the effect
    /// is also read and not only seen.
    fn run_panel(&mut self) {
        self.toggle_panel();
        let key = if self.panel_shown() {
            "ui.tui.panel_shown"
        } else {
            "ui.tui.panel_hidden"
        };
        self.scroll.set(0);
        self.push(LogEntry::Event(Event::system(
            Text::new(key).with_term("panel", "panel"),
        )));
    }

    /// Runs a line that is one of the presentation commands, if it is: the interface's own,
    /// which the game never hears of. What they say goes to the log like anything else, and
    /// the new settings reach the renderer and the motion.
    fn run_presentation(&mut self, line: &str) -> bool {
        let Some(host) = &mut self.options else {
            return false;
        };
        let Some(events) = host.run(line, &self.transcript) else {
            return false;
        };
        host.apply(&mut self.renderer);
        self.motion = !host.live.reduce_motion;
        self.typewriter = host.live.typewriter.rate();
        self.scroll.set(0);
        for event in events {
            self.push(LogEntry::Event(event));
        }
        true
    }

    /// Submits a line the player did not type (the `quit` of Ctrl-C).
    fn run_line(&mut self, line: &str) {
        if let Ok(input) = to_input(&self.prompt, line) {
            self.echo(line);
            self.submit(input);
        }
    }

    fn echo(&mut self, line: &str) {
        let marker = self.renderer.prompt(&self.prompt).marker;
        // Kept as typed: the mode (ASCII) is applied when it is drawn, so that it can change.
        self.push(LogEntry::Echo(format!("{marker}{line}")));
    }

    // ---- Completion ------------------------------------------------------------------------

    /// TAB, like a shell: the word before the cursor is extended to what every candidate
    /// shares (a lone candidate is completed, with a space after a command), and what is
    /// still ambiguous is listed under the log until the next key. The game proposes only
    /// what the player can use now.
    fn complete(&mut self) {
        let before = self.editor.before_cursor().to_owned();
        let candidates = self.game.complete(&before);
        if candidates.is_empty() {
            return;
        }
        let start = before
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace())
            .map_or(0, |(index, c)| index + c.len_utf8());
        let partial = before.get(start..).unwrap_or_default();
        let shared = common_prefix(&candidates);
        // The word takes the case of the candidate.
        let word = if shared.chars().count() >= partial.chars().count() {
            shared.as_str()
        } else {
            partial
        };
        let after = self
            .editor
            .text()
            .get(before.len()..)
            .unwrap_or_default()
            .to_owned();
        let mut line = format!("{}{word}", before.get(..start).unwrap_or_default());
        let single = candidates.len() == 1;
        if single && self.prompt == Prompt::Command && !after.starts_with(' ') {
            line.push(' ');
        }
        let cursor = line.len();
        line.push_str(&after);
        self.editor.set_at(&line, cursor);
        if !single {
            self.candidates = candidates;
        }
    }
}

/// The scene to stage for what the game just said: the last effect of the step, with the
/// notoriety it moved if the same step told of it.
fn staging_of(events: &[Event]) -> Option<Staging> {
    let effect = events.iter().rev().find_map(|event| match event {
        Event::Effect(effect) => Some(effect.clone()),
        _ => None,
    })?;
    let notoriety = events.iter().find_map(|event| match event {
        Event::Changed {
            gauge: Gauge::Notoriety,
            from,
            to,
            ..
        } => Some((*from, *to)),
        _ => None,
    });
    Some(Staging::new(effect, notoriety))
}

/// The longest start shared by every candidate, without regard to case, in the case of the
/// first one.
fn common_prefix(candidates: &[String]) -> String {
    let Some(first) = candidates.first() else {
        return String::new();
    };
    let length = candidates
        .iter()
        .map(|other| {
            first
                .chars()
                .zip(other.chars())
                .take_while(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
                .count()
        })
        .min()
        .unwrap_or(0);
    first.chars().take(length).collect()
}

#[cfg(test)]
mod tests;
