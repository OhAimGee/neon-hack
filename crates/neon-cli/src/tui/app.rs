//! The state and the drawing of the full-screen interface.
//!
//! [`App`] is a plain state machine over a [`Game`]: keys go in, the screen is drawn from
//! the state. Nothing here touches the terminal, so it is tested with ratatui's
//! `TestBackend`. Everything the screen shows is also said by the plain frontend: the log
//! uses the very same renderer, and the panel only repeats what `status` would say.

use neon_engine::event::{Event, Severity};
use neon_engine::text::{RenderMode, Text};
use neon_engine::{Game, Input, Prompt, Step, View};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line as TuiLine, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::input::{InputError, to_input};
use crate::persist::Persistence;
use crate::render::{LineKind, Renderer};

/// Smallest terminal with the permanent side panel.
const FULL_SIZE: (u16, u16) = (100, 28);
/// Smallest terminal at all: below it the interface asks for more room.
const COMPACT_SIZE: (u16, u16) = (64, 20);
const PANEL_WIDTH: u16 = 32;
const PROMPT_AREA_MAX: u16 = 10;
const BAR_WIDTH: usize = 10;

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

/// What the log keeps. Events are kept as events, so that changing the language would
/// re-render the whole history; echoes of typed lines are frozen text.
enum LogEntry {
    Event(Event),
    Echo(String),
}

/// The interface state.
pub(crate) struct App<'a> {
    game: &'a mut dyn Game,
    renderer: Renderer<'a>,
    persistence: Persistence,
    log: Vec<LogEntry>,
    prompt: Prompt,
    input: String,
    finished: bool,
    quit: bool,
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
            log: Vec::new(),
            prompt: Prompt::Command,
            input: String::new(),
            finished: false,
            quit: false,
        };
        app.apply(first);
        app
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.quit
    }

    fn apply(&mut self, step: Step) {
        self.log
            .extend(step.events.into_iter().map(LogEntry::Event));
        self.finished = step.prompt == Prompt::End;
        self.prompt = step.prompt;
    }

    fn submit(&mut self, input: Input) {
        let mut step = self.game.handle(input);
        self.persistence.after_step(&*self.game, &mut step);
        self.apply(step);
    }

    /// Handles one key press.
    pub(crate) fn on_key(&mut self, key: KeyEvent) {
        if self.finished {
            self.quit = true;
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Char('c' | 'd') if ctrl && !alt => self.submit(Input::Eof),
            KeyCode::Enter => self.enter(),
            KeyCode::Esc if self.input.is_empty() && self.prompt != Prompt::Command => {
                self.submit(Input::Cancel);
            }
            KeyCode::Esc => self.input.clear(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Tab => self.complete(),
            // AltGr is reported as Control+Alt on Windows: those characters are text too.
            KeyCode::Char(c) if (!ctrl && !alt) || (ctrl && alt) => self.input.push(c),
            _ => {}
        }
    }

    fn enter(&mut self) {
        let line = std::mem::take(&mut self.input);
        match to_input(&self.prompt, &line) {
            Ok(input) => {
                let marker = self.renderer.prompt(&self.prompt).marker;
                let echo = self.renderer.verbatim(&format!("{marker}{line}"));
                self.log.push(LogEntry::Echo(echo));
                self.submit(input);
            }
            Err(InputError::NotYesOrNo) => {
                self.log.push(LogEntry::Event(Event::error(Text::new(
                    "ui.invalid_confirm",
                ))));
            }
        }
    }

    /// TAB: extends the line to what every candidate shares, like a shell.
    fn complete(&mut self) {
        let candidates = self.game.complete(&self.input);
        let prefix = common_prefix(&candidates);
        if prefix.chars().count() >= self.input.chars().count() && !prefix.is_empty() {
            self.input = prefix;
            if candidates.len() == 1 && self.prompt == Prompt::Command {
                self.input.push(' ');
            }
        }
    }

    // ---- Drawing -------------------------------------------------------------------------

    pub(crate) fn draw(&self, frame: &mut Frame<'_>) {
        let area = frame.area();
        match tier(area) {
            Tier::TooSmall => self.draw_too_small(frame, area),
            tier => self.draw_main(frame, area, tier == Tier::Full),
        }
    }

    fn draw_too_small(&self, frame: &mut Frame<'_>, area: Rect) {
        let message = Text::new("ui.tui.too_small")
            .with_int("width", i64::from(area.width))
            .with_int("height", i64::from(area.height))
            .with_int("min_width", i64::from(COMPACT_SIZE.0))
            .with_int("min_height", i64::from(COMPACT_SIZE.1));
        let paragraph = Paragraph::new(self.renderer.text(&message)).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }

    fn draw_main(&self, frame: &mut Frame<'_>, area: Rect, with_panel: bool) {
        let view = self.game.view();
        let prompt_view = self.renderer.prompt(&self.prompt);
        let header_rows = u16::try_from(prompt_view.header.len()).unwrap_or(PROMPT_AREA_MAX);
        let prompt_height = header_rows.saturating_add(1).min(PROMPT_AREA_MAX);
        let [status_area, body_area, prompt_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(prompt_height),
        ])
        .areas(area);

        frame.render_widget(
            Paragraph::new(self.status_line(&view))
                .style(Style::default().add_modifier(Modifier::REVERSED)),
            status_area,
        );

        let log_area = if with_panel {
            let [log_area, panel_area] =
                Layout::horizontal([Constraint::Min(20), Constraint::Length(PANEL_WIDTH)])
                    .areas(body_area);
            self.draw_panel(frame, panel_area, &view);
            log_area
        } else {
            body_area
        };
        self.draw_log(frame, log_area);
        self.draw_prompt(frame, prompt_area, &prompt_view);
    }

    fn status_line(&self, view: &View) -> String {
        let separator = if self.renderer.mode.ascii { "|" } else { "│" };
        let mut parts = vec![format!(" {}", self.renderer.verbatim(&view.player))];
        parts.extend(
            view.gauges
                .iter()
                .map(|reading| self.gauge_summary(reading)),
        );
        parts.join(&format!(" {separator} "))
    }

    /// `Trace 14/100 (calm)`: the number and the level in words, never colour alone.
    fn gauge_summary(&self, reading: &neon_engine::game::GaugeReading) -> String {
        format!(
            "{} {}/{} ({})",
            self.renderer.text(&Text::new(reading.gauge.name_key())),
            reading.value,
            reading.max,
            self.renderer.text(&reading.band),
        )
    }

    fn draw_panel(&self, frame: &mut Frame<'_>, area: Rect, view: &View) {
        let inner_width = usize::from(area.width.saturating_sub(2));
        let mut lines: Vec<TuiLine<'static>> = Vec::new();
        for reading in &view.gauges {
            lines.push(TuiLine::from(Span::styled(
                format!(
                    "{} {}/{}",
                    self.renderer.text(&Text::new(reading.gauge.name_key())),
                    reading.value,
                    reading.max
                ),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(TuiLine::from(format!(
                "{} {}",
                self.bar(reading.value, reading.max),
                self.renderer.text(&reading.band)
            )));
            lines.push(TuiLine::default());
        }
        for objective in &view.objectives {
            for row in wrap(&self.renderer.text(objective), inner_width) {
                lines.push(TuiLine::from(row));
            }
        }
        let block = Block::default().borders(Borders::LEFT);
        // Box-drawing characters are not ASCII: outside the full mode the border is a plain `|`.
        let block = if self.renderer.mode == RenderMode::FULL {
            block
        } else {
            block.border_set(border::Set {
                vertical_left: "|",
                ..border::PLAIN
            })
        };
        let panel = Paragraph::new(lines).block(block);
        frame.render_widget(panel, area);
    }

    fn bar(&self, value: i32, max: i32) -> String {
        let (filled_char, empty_char) = if self.renderer.mode == RenderMode::FULL {
            ('█', '░')
        } else {
            ('#', '.')
        };
        let max = usize::try_from(max.max(1)).unwrap_or(1);
        let value = usize::try_from(value.max(0)).unwrap_or(0).min(max);
        let filled = value * BAR_WIDTH / max;
        let mut bar = String::from("[");
        bar.extend(std::iter::repeat_n(filled_char, filled));
        bar.extend(std::iter::repeat_n(empty_char, BAR_WIDTH - filled));
        bar.push(']');
        bar
    }

    fn draw_log(&self, frame: &mut Frame<'_>, area: Rect) {
        let width = usize::from(area.width);
        let mut rows: Vec<(String, Style)> = Vec::new();
        for entry in &self.log {
            match entry {
                LogEntry::Event(event) => {
                    for line in self.renderer.event(event) {
                        let style = style_of(line.kind);
                        rows.extend(wrap(&line.text, width).into_iter().map(|row| (row, style)));
                    }
                }
                LogEntry::Echo(text) => {
                    let style = Style::default().add_modifier(Modifier::DIM);
                    rows.extend(wrap(text, width).into_iter().map(|row| (row, style)));
                }
            }
        }
        if self.finished {
            let hint = self.renderer.text(&Text::new("ui.tui.press_key"));
            rows.extend(
                wrap(&hint, width)
                    .into_iter()
                    .map(|row| (row, Style::default())),
            );
        }
        let visible = usize::from(area.height);
        let skipped = rows.len().saturating_sub(visible);
        let lines: Vec<TuiLine<'static>> = rows
            .into_iter()
            .skip(skipped)
            .map(|(text, style)| TuiLine::from(Span::styled(text, style)))
            .collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_prompt(&self, frame: &mut Frame<'_>, area: Rect, view: &crate::render::PromptView) {
        let width = usize::from(area.width);
        let capacity = usize::from(area.height).saturating_sub(1);
        let skipped = view.header.len().saturating_sub(capacity);
        let mut lines: Vec<TuiLine<'static>> = view
            .header
            .iter()
            .skip(skipped)
            .map(|line| TuiLine::from(Span::styled(line.text.clone(), style_of(line.kind))))
            .collect();
        // Keep the end of the line in view, and the cursor right after it.
        let typed = self
            .renderer
            .verbatim(&format!("{}{}", view.marker, self.input));
        let shown = tail(&typed, width.saturating_sub(1));
        let cursor_x = u16::try_from(shown.width()).unwrap_or(0);
        let cursor_y = u16::try_from(lines.len()).unwrap_or(0);
        lines.push(TuiLine::from(shown));
        frame.render_widget(Paragraph::new(lines), area);
        if !self.finished {
            frame.set_cursor_position(Position::new(
                area.x.saturating_add(cursor_x),
                area.y.saturating_add(cursor_y),
            ));
        }
    }
}

fn style_of(kind: LineKind) -> Style {
    match kind {
        LineKind::Narration | LineKind::System | LineKind::Blank => Style::default(),
        LineKind::Dialogue | LineKind::Table => Style::default().fg(Color::Cyan),
        LineKind::Alert(Severity::Danger) => {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        }
        LineKind::Alert(_) => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        LineKind::Reward => Style::default().fg(Color::Green),
        LineKind::Error => Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::UNDERLINED),
        LineKind::Decor => Style::default().fg(Color::Magenta),
        LineKind::Gauge => Style::default().fg(Color::Yellow),
    }
}

/// The longest start shared by every candidate.
fn common_prefix(candidates: &[String]) -> String {
    let Some(first) = candidates.first() else {
        return String::new();
    };
    let mut prefix = first.clone();
    for candidate in candidates {
        let shared: usize = prefix
            .chars()
            .zip(candidate.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a.len_utf8())
            .sum();
        prefix.truncate(shared);
    }
    prefix
}

/// Cuts a text into rows of at most `width` columns, at spaces when it can and inside a
/// word when it must. Width is display width: wide characters take two columns.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = vec![String::new()];
    let mut used = 0;
    for word in text.split(' ') {
        let word_width = word.width();
        let needed = if used == 0 {
            word_width
        } else {
            used + 1 + word_width
        };
        if needed <= width {
            if let Some(row) = rows.last_mut() {
                if used > 0 {
                    row.push(' ');
                }
                row.push_str(word);
            }
            used = needed;
            continue;
        }
        if used > 0 {
            rows.push(String::new());
            used = 0;
        }
        for ch in word.chars() {
            let ch_width = ch.width().unwrap_or(0);
            if used + ch_width > width {
                rows.push(String::new());
                used = 0;
            }
            if let Some(row) = rows.last_mut() {
                row.push(ch);
            }
            used += ch_width;
        }
    }
    rows
}

/// The end of a text that fits in `width` columns.
fn tail(text: &str, width: usize) -> String {
    let mut kept = Vec::new();
    let mut used = 0;
    for ch in text.chars().rev() {
        let ch_width = ch.width().unwrap_or(0);
        if used + ch_width > width {
            break;
        }
        used += ch_width;
        kept.push(ch);
    }
    kept.into_iter().rev().collect()
}

#[cfg(test)]
mod tests;
