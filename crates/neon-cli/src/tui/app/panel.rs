//! The side panel: the gauges, then what the game lists below them (quests, net, inventory,
//! what is left to read).
//!
//! In the full layout it stands beside the log; in the compact one it is a drawer over the
//! right of the log, opened with F2 or `panel` and closed the same way (or with Escape). It
//! is made of words first: every row is the text a command also says, and the tone of a row
//! (colour, bold, dim) only adds to it (`GAU-1`, `COL-1`).

use neon_engine::game::{PanelRow, PanelSection, Tone, View};
use neon_engine::text::Text;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as TuiLine, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::draw::to_style;
use super::{App, PANEL_WIDTH, Tier};
use crate::palette::Accent;
use crate::tui::text::{head, width, wrap};

/// Width of the gauge bar, without its brackets.
const BAR_WIDTH: usize = 10;

/// What the player decided about the panel, which the layouts read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PanelState {
    /// The permanent panel of the full layout is shown (it is, until the player hides it).
    pub(super) visible: bool,
    /// The drawer of the compact layout is open (it is closed, until the player opens it).
    pub(super) drawer: bool,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            visible: true,
            drawer: false,
        }
    }
}

impl App<'_> {
    /// Shows the panel if it is hidden and hides it if it is shown: the permanent panel of
    /// the full layout, the drawer of the compact one. F2 and `panel` both come here.
    pub(super) fn toggle_panel(&mut self) {
        if self.shown_tier.get() == Tier::Full {
            self.panel.visible = !self.panel.visible;
        } else {
            self.panel.drawer = !self.panel.drawer;
        }
    }

    /// Whether the panel can be seen now, in the layout the last drawing used.
    pub(super) fn panel_shown(&self) -> bool {
        match self.shown_tier.get() {
            Tier::Full => self.panel.visible,
            Tier::Compact => self.panel.drawer,
            Tier::TooSmall => false,
        }
    }

    /// The drawer of the compact layout: the panel over the right of `body`, taking the room
    /// it needs and no more.
    pub(super) fn draw_drawer(&self, frame: &mut Frame<'_>, body: Rect, view: &View) {
        let width = PANEL_WIDTH.min(body.width);
        let area = Rect {
            x: body.x + (body.width - width),
            width,
            ..body
        };
        self.draw_panel(frame, area, view);
    }

    /// Draws the panel in `area`, cleared first: nothing of what is under it shows through.
    pub(super) fn draw_panel(&self, frame: &mut Frame<'_>, area: Rect, view: &View) {
        frame.render_widget(Clear, area);
        frame.render_widget(Block::default().style(self.base_style()), area);
        let block = self.frame_block(&Text::new("ui.tui.panel_title").with_term("panel", "panel"));
        let inner = block.inner(area);
        let lines = fit(
            self.panel_lines(view, usize::from(inner.width), usize::from(inner.height)),
            usize::from(inner.height),
            |hidden| self.more_line(hidden, usize::from(inner.width)),
        );
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    /// The row that says how many rows were left out.
    fn more_line(&self, hidden: usize, width: usize) -> TuiLine<'static> {
        let more = self.renderer.text(
            &Text::new("ui.tui.panel_more")
                .with_int("rows", i64::try_from(hidden).unwrap_or(i64::MAX)),
        );
        TuiLine::from(Span::styled(
            head(&more, width),
            Style::default().add_modifier(Modifier::DIM),
        ))
    }

    /// Every row of the panel at this width: the gauges, then each section under its heading.
    /// When they do not all fit in `height` rows, the sections share the room that is left
    /// fairly (a short one is whole, a long one is cut and says how much), so that none of
    /// them is pushed off the panel by another.
    fn panel_lines(&self, view: &View, inner_width: usize, height: usize) -> Vec<TuiLine<'static>> {
        let mut lines = self.gauge_lines(view, inner_width);
        let sections: Vec<(TuiLine<'static>, Vec<TuiLine<'static>>)> = view
            .sections
            .iter()
            .map(|section| self.section_lines(section, inner_width))
            .collect();
        // Room for the bodies: what is left of the height after the gauges, and a heading and
        // a blank row for each section (the last one needs no blank row).
        let separators = sections.len().saturating_mul(2).saturating_sub(1);
        let available = height
            .saturating_sub(lines.len())
            .saturating_sub(separators);
        let demands: Vec<usize> = sections.iter().map(|(_, body)| body.len()).collect();
        let shares = if demands.iter().sum::<usize>() <= available {
            demands.clone()
        } else {
            allocate(&demands, available)
        };
        for ((heading, mut body), (share, demand)) in
            sections.into_iter().zip(shares.into_iter().zip(demands))
        {
            if share < demand {
                // The last row of the share says what the others hide.
                body.truncate(share.saturating_sub(1));
                if share > 0 {
                    body.push(self.more_line(demand - share + 1, inner_width));
                }
            }
            lines.push(heading);
            lines.extend(body);
            lines.push(TuiLine::default());
        }
        lines
    }

    /// The gauges: name and number, the bar and the level in words, a blank row.
    fn gauge_lines(&self, view: &View, inner_width: usize) -> Vec<TuiLine<'static>> {
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
            let bar = self.bar(reading.value, reading.max);
            let band = self.renderer.text(&reading.band);
            if width(&format!("{bar} {band}")) <= inner_width {
                // The bar takes the colour of the pressure; the word next to it says the same.
                let palette = self.renderer.palette();
                let pressure = to_style(palette.pressure(reading.value, reading.max), &palette);
                lines.push(TuiLine::from(vec![
                    Span::styled(bar, pressure),
                    Span::raw(format!(" {band}")),
                ]));
            } else {
                for row in wrap(&format!("{bar} {band}"), inner_width) {
                    lines.push(TuiLine::from(row));
                }
            }
            lines.push(TuiLine::default());
        }
        lines
    }

    /// The heading of a section, and its rows wrapped to the width.
    fn section_lines(
        &self,
        section: &PanelSection,
        inner_width: usize,
    ) -> (TuiLine<'static>, Vec<TuiLine<'static>>) {
        let marker = if self.renderer.mode.ascii { ">" } else { "▸" };
        let title = format!("{marker} {}", self.renderer.text(&section.title));
        let heading = TuiLine::from(Span::styled(
            head(&title, inner_width),
            self.accent_style(Accent::Title),
        ));
        let body = section
            .rows
            .iter()
            .flat_map(|row| {
                let style = self.tone_style(row);
                wrap(&self.renderer.text(&row.text), inner_width)
                    .into_iter()
                    .map(move |wrapped| TuiLine::from(Span::styled(wrapped, style)))
            })
            .collect();
        (heading, body)
    }

    /// How a row stands out. The text of the row says the same in words.
    fn tone_style(&self, row: &PanelRow) -> Style {
        match row.tone {
            Tone::Plain => Style::default(),
            Tone::Strong => Style::default().add_modifier(Modifier::BOLD),
            Tone::Good => self.accent_style(Accent::Locked),
            Tone::Warning => self.accent_style(Accent::Threat),
            Tone::Muted => Style::default().add_modifier(Modifier::DIM),
        }
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
}

/// How many rows each demand gets out of `available`: a demand below the fair share is met,
/// and what it leaves over goes to the larger ones (the smallest are served first).
fn allocate(demands: &[usize], available: usize) -> Vec<usize> {
    let mut shares = vec![0; demands.len()];
    let mut order: Vec<(usize, usize)> = demands.iter().copied().enumerate().collect();
    order.sort_by_key(|&(_, demand)| demand);
    let mut left = available;
    let mut waiting = order.len();
    for (index, demand) in order {
        let share = demand.min(left / waiting.max(1));
        if let Some(slot) = shares.get_mut(index) {
            *slot = share;
        }
        left -= share;
        waiting -= 1;
    }
    shares
}

/// `lines` cut to `height` rows. When something has to go, the last row says how many rows
/// were left out (`more` draws it), so the panel never pretends to be the whole list.
fn fit(
    mut lines: Vec<TuiLine<'static>>,
    height: usize,
    more: impl FnOnce(usize) -> TuiLine<'static>,
) -> Vec<TuiLine<'static>> {
    // A blank row at the end is only a separator.
    while lines.last().is_some_and(|line| line.spans.is_empty()) {
        lines.pop();
    }
    if lines.len() <= height {
        return lines;
    }
    let kept = height.saturating_sub(1);
    let hidden = lines.len() - kept;
    lines.truncate(kept);
    if height > 0 {
        lines.push(more(hidden));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(count: usize) -> Vec<TuiLine<'static>> {
        (0..count)
            .map(|index| TuiLine::from(format!("row {index}")))
            .collect()
    }

    fn marker(hidden: usize) -> TuiLine<'static> {
        TuiLine::from(format!("+{hidden}"))
    }

    fn text_of(lines: &[TuiLine<'static>]) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn short_demands_are_met_and_the_long_ones_share_the_rest() {
        assert_eq!(allocate(&[9, 1, 2], 6), [3, 1, 2]);
        assert_eq!(allocate(&[9, 9], 7), [3, 4]);
        assert_eq!(allocate(&[2, 2], 10), [2, 2]);
        assert_eq!(allocate(&[5, 5, 5], 0), [0, 0, 0]);
        assert!(allocate(&[], 3).is_empty());
    }

    #[test]
    fn shares_never_exceed_the_room_or_the_demand() {
        for available in 0..30 {
            let demands = [7, 1, 12, 3];
            let shares = allocate(&demands, available);
            assert!(shares.iter().sum::<usize>() <= available);
            assert!(
                shares
                    .iter()
                    .zip(demands)
                    .all(|(share, demand)| *share <= demand)
            );
        }
    }

    #[test]
    fn rows_that_fit_are_all_kept() {
        let kept = fit(rows(4), 4, marker);
        assert_eq!(kept.len(), 4);
        assert!(!text_of(&kept).iter().any(|row| row.starts_with('+')));
    }

    #[test]
    fn a_blank_end_is_not_counted() {
        let mut lines = rows(3);
        lines.push(TuiLine::default());
        assert_eq!(fit(lines, 3, marker).len(), 3);
    }

    #[test]
    fn rows_that_do_not_fit_end_with_how_many_are_left_out() {
        let kept = text_of(&fit(rows(10), 4, marker));
        assert_eq!(kept, ["row 0", "row 1", "row 2", "+7"]);
    }

    #[test]
    fn a_panel_of_one_row_only_says_that_there_is_more() {
        assert_eq!(text_of(&fit(rows(5), 1, marker)), ["+5"]);
    }

    #[test]
    fn a_panel_without_room_draws_nothing() {
        assert!(fit(rows(5), 0, marker).is_empty());
    }
}
