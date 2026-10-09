//! Drawing the state of the interface.
//!
//! Every function reads the state and writes cells; the only things it writes back are the
//! size-dependent facts of the last drawing (the tier, the page, a scroll clamped to the
//! length of the log), which only the drawing can know.

use neon_engine::Prompt;
use neon_engine::game::GaugeReading;
use neon_engine::text::Text;
use neon_engine::View;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line as TuiLine, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{
    App, CANDIDATE_ROWS_MAX, COMPACT_SIZE, LogEntry, PANEL_WIDTH, PROMPT_AREA_MAX, Tier, tier,
};
use crate::palette::{Ansi, Hue, Palette, Style as PaletteStyle};
use crate::render::LineKind;
use crate::tui::text::{head, wrap, window};

/// Width of the bar in the panel, without its brackets.
const BAR_WIDTH: usize = 10;

/// Borders made of characters every terminal and every code page has.
const ASCII_BORDER: border::Set<'static> = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

/// One row of the log, as it is drawn.
struct Row {
    text: String,
    style: Style,
}

/// What goes under the log: the menu, the candidates of a completion, the input line.
struct PromptLines {
    lines: Vec<TuiLine<'static>>,
    /// Where the cursor is, from the top left of the area.
    cursor: (u16, u16),
}

impl App<'_> {
    pub(crate) fn draw(&self, frame: &mut Frame<'_>) {
        let area = frame.area();
        let palette = self.renderer.palette();
        // An opaque palette paints its background under every cell, so that its contrast
        // does not depend on the terminal's theme.
        if let Some(background) = palette.background {
            let mut base = Style::default().bg(color_of(background, &palette));
            if let Some(foreground) = palette.foreground {
                base = base.fg(color_of(foreground, &palette));
            }
            frame.render_widget(Block::default().style(base), area);
        }
        let tier = tier(area);
        self.shown_tier.set(tier);
        match tier {
            Tier::TooSmall => self.draw_too_small(frame, area),
            tier => self.draw_main(frame, area, tier == Tier::Full),
        }
    }

    /// The whole screen is one sentence, centred, in the language of the game. The state of
    /// the game is not touched: a larger terminal brings everything back.
    fn draw_too_small(&self, frame: &mut Frame<'_>, area: Rect) {
        let message = Text::new("ui.tui.too_small")
            .with_int("width", i64::from(area.width))
            .with_int("height", i64::from(area.height))
            .with_int("min_width", i64::from(COMPACT_SIZE.0))
            .with_int("min_height", i64::from(COMPACT_SIZE.1));
        let rows = wrap(&self.renderer.text(&message), usize::from(area.width));
        let shown = u16::try_from(rows.len()).unwrap_or(u16::MAX).min(area.height);
        let lines: Vec<TuiLine<'static>> = rows.into_iter().map(TuiLine::from).collect();
        let centred = Rect {
            y: area.y + (area.height - shown) / 2,
            height: shown,
            ..area
        };
        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            centred,
        );
    }

    fn draw_main(&self, frame: &mut Frame<'_>, area: Rect, with_panel: bool) {
        let view = self.game.view();
        let prompt = self.prompt_lines(usize::from(area.width));
        let prompt_height = u16::try_from(prompt.lines.len()).unwrap_or(PROMPT_AREA_MAX);
        let [status_area, body_area, prompt_area, help_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(prompt_height),
            Constraint::Length(1),
        ])
        .areas(area);

        frame.render_widget(
            Paragraph::new(head(&self.status_line(&view), usize::from(area.width)))
                .style(Style::default().add_modifier(Modifier::REVERSED)),
            status_area,
        );

        if with_panel {
            let [log_area, panel_area] =
                Layout::horizontal([Constraint::Min(20), Constraint::Length(PANEL_WIDTH)])
                    .areas(body_area);
            let block = self.frame_block(&Text::new("ui.tui.log_title").with_term("terminal", "terminal"));
            let inner = block.inner(log_area);
            frame.render_widget(block, log_area);
            self.draw_log(frame, inner);
            self.draw_panel(frame, panel_area, &view);
        } else {
            self.draw_log(frame, body_area);
        }
        self.draw_prompt(frame, prompt_area, prompt);
        let help = self.renderer.text(&Text::new("ui.tui.help"));
        frame.render_widget(
            Paragraph::new(head(&help, usize::from(area.width)))
                .style(Style::default().add_modifier(Modifier::DIM)),
            help_area,
        );
    }

    fn style_of(&self, kind: LineKind) -> Style {
        let palette = self.renderer.palette();
        to_style(palette.style(kind), &palette)
    }

    /// A box with a title, in characters the mode allows.
    fn frame_block(&self, title: &Text) -> Block<'static> {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.renderer.text(title));
        if self.renderer.mode.ascii {
            block.border_set(ASCII_BORDER)
        } else {
            block.border_set(border::ROUNDED)
        }
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
    fn gauge_summary(&self, reading: &GaugeReading) -> String {
        format!(
            "{} {}/{} ({})",
            self.renderer.text(&Text::new(reading.gauge.name_key())),
            reading.value,
            reading.max,
            self.renderer.text(&reading.band),
        )
    }

    /// The permanent panel of the full layout: for now the gauges, named and numbered, and
    /// the objectives. This is the one place the panel's content is decided.
    fn draw_panel(&self, frame: &mut Frame<'_>, area: Rect, view: &View) {
        let block = self.frame_block(&Text::new("ui.tui.panel_title").with_term("panel", "panel"));
        let inner = block.inner(area);
        let inner_width = usize::from(inner.width);
        let mut lines: Vec<TuiLine<'static>> = Vec::new();
        for reading in &view.gauges {
            lines.push(TuiLine::from(Span::styled(
                head(
                    &format!(
                        "{} {}/{}",
                        self.renderer.text(&Text::new(reading.gauge.name_key())),
                        reading.value,
                        reading.max
                    ),
                    inner_width,
                ),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for row in wrap(
                &format!(
                    "{} {}",
                    self.bar(reading.value, reading.max),
                    self.renderer.text(&reading.band)
                ),
                inner_width,
            ) {
                lines.push(TuiLine::from(row));
            }
            lines.push(TuiLine::default());
        }
        for objective in &view.objectives {
            for row in wrap(&self.renderer.text(objective), inner_width) {
                lines.push(TuiLine::from(row));
            }
        }
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn bar(&self, value: i32, max: i32) -> String {
        let (filled_char, empty_char) = if self.renderer.mode.ascii {
            ('#', '.')
        } else {
            ('█', '░')
        };
        let max = usize::try_from(max.max(1)).unwrap_or(1);
        let value = usize::try_from(value.max(0)).unwrap_or(0).min(max);
        // At least one cell as soon as there is something to show.
        let filled = (value * BAR_WIDTH / max).max(usize::from(value > 0));
        let mut bar = String::from("[");
        bar.extend(std::iter::repeat_n(filled_char, filled));
        bar.extend(std::iter::repeat_n(empty_char, BAR_WIDTH - filled));
        bar.push(']');
        bar
    }

    // ---- The log -------------------------------------------------------------------------

    /// The rows of one entry at this width. Art is cut rather than re-flowed, so that it
    /// stays a picture.
    fn entry_rows(&self, entry: &LogEntry, width: usize) -> Vec<Row> {
        match entry {
            LogEntry::Event(event) => self
                .renderer
                .event(event)
                .into_iter()
                .flat_map(|line| {
                    let style = self.style_of(line.kind);
                    let texts = if line.kind == LineKind::Decor {
                        vec![head(&line.text, width)]
                    } else {
                        wrap(&line.text, width)
                    };
                    texts.into_iter().map(move |text| Row { text, style })
                })
                .collect(),
            LogEntry::Echo(text) => {
                let style = Style::default().add_modifier(Modifier::DIM);
                wrap(text, width)
                    .into_iter()
                    .map(|text| Row { text, style })
                    .collect()
            }
        }
    }

    /// The last `wanted` rows of the log, or all of them when there are fewer. Only as many
    /// entries as needed are laid out, newest first, so a long history costs nothing until
    /// the player scrolls back into it.
    fn tail_rows(&self, width: usize, wanted: usize) -> Vec<Row> {
        let mut blocks: Vec<Vec<Row>> = Vec::new();
        let mut count = 0;
        if self.finished {
            let hint = self.renderer.text(&Text::new("ui.tui.press_key"));
            let rows: Vec<Row> = wrap(&hint, width)
                .into_iter()
                .map(|text| Row {
                    text,
                    style: Style::default(),
                })
                .collect();
            count += rows.len();
            blocks.push(rows);
        }
        for entry in self.log.iter().rev() {
            if count >= wanted {
                break;
            }
            let rows = self.entry_rows(entry, width);
            count += rows.len();
            blocks.push(rows);
        }
        blocks.into_iter().rev().flatten().collect()
    }

    /// The log, following the game or scrolled back, with a line saying so when it is.
    fn draw_log(&self, frame: &mut Frame<'_>, area: Rect) {
        let (width, height) = (usize::from(area.width), usize::from(area.height));
        let request = self.scroll.get();
        let rows = self.tail_rows(width, height.saturating_add(request));
        let total = rows.len();
        // Scrolled back, the last row of the area is the indicator.
        let (shown, scroll) = if request > 0 && total > height && height > 1 {
            (height - 1, request.min(total - (height - 1)))
        } else {
            (height, 0)
        };
        self.scroll.set(scroll);
        // Scrolled back the indicator takes a row; one row of context stays on a page turn.
        self.page.set(height.saturating_sub(2).max(1));

        let end = total - scroll;
        let start = end.saturating_sub(shown);
        let mut lines: Vec<TuiLine<'static>> = rows
            .into_iter()
            .skip(start)
            .take(end - start)
            .map(|row| TuiLine::from(Span::styled(row.text, row.style)))
            .collect();
        if scroll > 0 {
            let indicator = self.renderer.text(
                &Text::new("ui.tui.scrolled")
                    .with_int("rows", i64::try_from(scroll).unwrap_or(i64::MAX)),
            );
            lines.push(TuiLine::from(Span::styled(
                head(&indicator, width),
                Style::default().add_modifier(Modifier::REVERSED),
            )));
        }
        frame.render_widget(Paragraph::new(lines), area);
    }

    // ---- The prompt ------------------------------------------------------------------------

    /// What goes under the log at this width: the menu of the prompt, the candidates of a
    /// completion, and the input line with the end of the text in view and the cursor in it.
    fn prompt_lines(&self, width: usize) -> PromptLines {
        let width = width.max(1);
        let view = self.renderer.prompt(&self.prompt);
        let mut header: Vec<TuiLine<'static>> = view
            .header
            .iter()
            .flat_map(|line| {
                let style = self.style_of(line.kind);
                wrap(&line.text, width)
                    .into_iter()
                    .map(move |row| TuiLine::from(Span::styled(row, style)))
            })
            .collect();
        let mut candidates: Vec<TuiLine<'static>> = Vec::new();
        if !self.candidates.is_empty() {
            let listed = self.renderer.verbatim(&self.candidates.join("  "));
            candidates = wrap(&listed, width)
                .into_iter()
                .take(CANDIDATE_ROWS_MAX)
                .map(|row| {
                    TuiLine::from(Span::styled(
                        row,
                        Style::default().add_modifier(Modifier::DIM),
                    ))
                })
                .collect();
        }
        // The input line always stays; the menu loses its first lines when it is too tall.
        let room = usize::from(PROMPT_AREA_MAX).saturating_sub(1 + candidates.len());
        let skipped = header.len().saturating_sub(room);
        header.drain(..skipped);

        // The marker and the text are made ASCII together, at the single point; the cursor
        // is measured on the same final text.
        let before = self
            .renderer
            .verbatim(&format!("{}{}", view.marker, self.editor.before_cursor()));
        let typed = self
            .renderer
            .verbatim(&format!("{}{}", view.marker, self.editor.text()));
        let (shown, cursor_x) = window(&typed, crate::tui::text::width(&before), width);

        let mut lines = header;
        lines.extend(candidates);
        let cursor_y = u16::try_from(lines.len()).unwrap_or(0);
        lines.push(TuiLine::from(shown));
        PromptLines {
            lines,
            cursor: (u16::try_from(cursor_x).unwrap_or(0), cursor_y),
        }
    }

    fn draw_prompt(&self, frame: &mut Frame<'_>, area: Rect, prompt: PromptLines) {
        frame.render_widget(Paragraph::new(prompt.lines), area);
        if !self.finished && self.prompt != Prompt::End {
            frame.set_cursor_position(Position::new(
                area.x.saturating_add(prompt.cursor.0),
                area.y.saturating_add(prompt.cursor.1),
            ));
        }
    }
}

/// A palette colour as a terminal colour: exact where the palette and the terminal allow it,
/// the terminal's own named colour otherwise.
fn color_of(hue: Hue, palette: &Palette) -> Color {
    if palette.uses_exact_colors() {
        let (red, green, blue) = hue.rgb;
        return Color::Rgb(red, green, blue);
    }
    match hue.ansi {
        Ansi::Black => Color::Black,
        Ansi::Red => Color::Red,
        Ansi::Green => Color::Green,
        Ansi::Yellow => Color::Yellow,
        Ansi::Magenta => Color::Magenta,
        Ansi::Cyan => Color::Cyan,
        Ansi::White => Color::White,
    }
}

/// A palette style as a widget style. There is no blink in it, and in `mono` no colour.
pub(super) fn to_style(style: PaletteStyle, palette: &Palette) -> Style {
    let mut out = Style::default();
    if let Some(hue) = style.hue {
        out = out.fg(color_of(hue, palette));
    }
    let attributes = [
        (style.bold, Modifier::BOLD),
        (style.reverse, Modifier::REVERSED),
        (style.underline, Modifier::UNDERLINED),
        (style.dim, Modifier::DIM),
    ];
    for (on, modifier) in attributes {
        if on {
            out = out.add_modifier(modifier);
        }
    }
    out
}
