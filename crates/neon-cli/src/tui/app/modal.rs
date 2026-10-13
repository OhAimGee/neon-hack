//! The screens kept open beside the log: the shop, the contacts, the inbox, the archives,
//! the journal of quests, the map of the net.
//!
//! A screen is never a mode. It opens when a command shows a list (the same list the log
//! prints, line by line), the input stays live under it, `Escape` closes it first of all,
//! and after each step it is drawn again from the game, so that it follows what the player
//! just did instead of showing what was true when it opened. Nothing in it is missing from
//! the log: it is the same table, laid out in columns when they fit.

use std::cell::Cell;

use neon_engine::event::{Event, ScreenKind, Table};
use neon_engine::text::Text;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line as TuiLine, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::{App, PANEL_WIDTH, Tier};
use crate::palette::Accent;
use crate::tui::text::{head, sanitize, width, wrap};

/// Columns kept between two columns of the table.
const GAP: usize = 2;
/// Columns of padding inside the frame, on each side.
const PADDING: usize = 1;
/// Narrowest area worth opening a screen in; below it the log alone is left.
const AREA_MIN: u16 = 24;

/// A screen being kept open.
pub(super) struct Modal {
    /// Which list it is, to ask the game for it again.
    pub(super) kind: ScreenKind,
    /// The list as the game last drew it.
    pub(super) table: Table,
    /// Rows scrolled from the top. Only a request: the drawing clamps it.
    pub(super) scroll: Cell<usize>,
    /// Rows the body showed at the last drawing, so that a page is a page.
    pub(super) page: Cell<usize>,
    /// Whether the last drawing had to cut the body, so that `PgUp` and `PgDn` mean this
    /// screen and not the log.
    pub(super) overflow: Cell<bool>,
}

/// What a table looks like once laid out for an area.
struct Layout {
    /// The fixed row above the body: the column headings.
    header: Option<String>,
    /// The body, one string per row.
    body: Vec<String>,
    /// Columns the widest row takes.
    width: usize,
}

impl App<'_> {
    /// Keeps open the list the step showed, or draws again the one that is open. Called with
    /// the events of every step.
    pub(super) fn follow_screens(&mut self, events: &[Event]) {
        let shown = events.iter().rev().find_map(|event| match event {
            Event::Screen(table) => table.pin.map(|kind| (kind, table.clone())),
            _ => None,
        });
        if let Some((kind, table)) = shown {
            let same = self.modal.as_ref().is_some_and(|modal| modal.kind == kind);
            let (scroll, page) = self
                .modal
                .as_ref()
                .filter(|_| same)
                .map_or((0, 1), |modal| (modal.scroll.get(), modal.page.get()));
            self.modal = Some(Modal {
                kind,
                table,
                scroll: Cell::new(scroll),
                page: Cell::new(page),
                overflow: Cell::new(false),
            });
        } else if let Some(modal) = &mut self.modal {
            match self.game.screen(modal.kind) {
                Some(table) => modal.table = table,
                None => self.modal = None,
            }
        }
    }

    /// Closes the screen if one is open; true if there was one.
    pub(super) fn close_modal(&mut self) -> bool {
        self.modal.take().is_some()
    }

    /// Turns the screen by `rows` (a page when none are given) when it is cut; false if there
    /// is nothing to turn, and the key or the wheel then belongs to the log.
    pub(super) fn scroll_modal(&self, forward: bool, rows: Option<usize>) -> bool {
        let Some(modal) = self.modal.as_ref().filter(|modal| modal.overflow.get()) else {
            return false;
        };
        let by = rows.unwrap_or_else(|| modal.page.get()).max(1);
        modal.scroll.set(if forward {
            modal.scroll.get().saturating_add(by)
        } else {
            modal.scroll.get().saturating_sub(by)
        });
        true
    }

    /// The part of `body` where a screen can be drawn: all of it, less the drawer if that is
    /// open on top of the log.
    pub(super) fn modal_area(&self, body: Rect) -> Rect {
        if self.shown_tier.get() == Tier::Compact && self.panel.drawer {
            Rect {
                width: body.width.saturating_sub(PANEL_WIDTH),
                ..body
            }
        } else {
            body
        }
    }

    /// Draws the open screen, centred in `area` and cleared first.
    pub(super) fn draw_modal(&self, frame: &mut Frame<'_>, area: Rect) {
        let Some(modal) = &self.modal else {
            return;
        };
        if area.width < AREA_MIN || area.height < 4 {
            return;
        }
        // The catalog dresses a title for the log (`== Journal ==`); the frame needs no more.
        let title = self
            .renderer
            .text(&modal.table.title)
            .trim_matches(|c: char| c == '=' || c.is_whitespace())
            .to_owned();
        // Frame, then the padding on each side.
        let chrome = 2 + 2 * PADDING;
        let room = usize::from(area.width).saturating_sub(chrome);
        let layout = self.lay_out(&modal.table, room);
        let title_width = width(&title);
        let hint_width = width(&self.renderer.text(&Text::new("ui.tui.modal_hint_scroll")));
        let wanted = layout.width.max(title_width).max(hint_width.min(room));
        let window_width = u16::try_from(wanted.saturating_add(chrome))
            .unwrap_or(u16::MAX)
            .min(area.width);
        let fixed = usize::from(layout.header.is_some());
        let window_height = u16::try_from(layout.body.len() + fixed + 2)
            .unwrap_or(u16::MAX)
            .min(area.height);
        let window = Rect {
            x: area.x + (area.width - window_width) / 2,
            y: area.y + (area.height - window_height) / 2,
            width: window_width,
            height: window_height,
        };

        let body_rows = usize::from(window_height).saturating_sub(2 + fixed);
        let overflow = layout.body.len() > body_rows;
        modal.overflow.set(overflow);
        modal.page.set(body_rows.saturating_sub(1).max(1));
        let scroll = modal
            .scroll
            .get()
            .min(layout.body.len().saturating_sub(body_rows));
        modal.scroll.set(scroll);

        let hint = if overflow {
            self.renderer.text(
                &Text::new("ui.tui.modal_hint_scroll")
                    .with_int("first", count(scroll + 1))
                    .with_int("last", count((scroll + body_rows).min(layout.body.len())))
                    .with_int("total", count(layout.body.len())),
            )
        } else {
            self.renderer.text(&Text::new("ui.tui.modal_hint"))
        };
        let inner_width = usize::from(window_width).saturating_sub(2);
        let block = self
            .frame_block_titled(title)
            .title_bottom(head(&format!(" {hint} "), inner_width));

        frame.render_widget(Clear, window);
        frame.render_widget(Block::default().style(self.base_style()), window);
        let lines: Vec<TuiLine<'static>> = layout
            .header
            .iter()
            .map(|header| {
                TuiLine::from(Span::styled(
                    padded(header),
                    self.accent_style(Accent::Title)
                        .add_modifier(Modifier::BOLD),
                ))
            })
            .chain(
                layout
                    .body
                    .iter()
                    .skip(scroll)
                    .take(body_rows)
                    .map(|row| TuiLine::from(Span::raw(padded(row)))),
            )
            .collect();
        frame.render_widget(Paragraph::new(lines).block(block), window);
    }

    /// The table in columns, or as the list the log prints when the columns do not fit in
    /// `room` columns: nothing is ever cut.
    fn lay_out(&self, table: &Table, room: usize) -> Layout {
        let cell = |text: &Text| sanitize(&self.renderer.text(text));
        let mut headings = vec!["#".to_owned()];
        headings.extend(table.columns.iter().map(&cell));
        let rows: Vec<Vec<String>> = table
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let mut cells = vec![(index + 1).to_string()];
                cells.extend(row.iter().map(&cell));
                cells
            })
            .collect();
        let widths: Vec<usize> = (0..headings.len())
            .map(|column| {
                rows.iter()
                    .filter_map(|row| row.get(column))
                    .chain(headings.get(column))
                    .map(|text| width(text))
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let natural = widths.iter().sum::<usize>() + GAP * widths.len().saturating_sub(1);

        if rows.is_empty() {
            let empty = self.renderer.text(&Text::new("ui.tui.modal_empty"));
            return Layout {
                header: None,
                width: width(&empty),
                body: vec![empty],
            };
        }
        if natural <= room {
            let line = |cells: &[String]| {
                cells
                    .iter()
                    .zip(&widths)
                    .map(|(text, wanted)| {
                        format!("{text}{}", " ".repeat(wanted.saturating_sub(width(text))))
                    })
                    .collect::<Vec<_>>()
                    .join(&" ".repeat(GAP))
                    .trim_end()
                    .to_owned()
            };
            return Layout {
                header: Some(line(&headings)),
                body: rows.iter().map(|row| line(row)).collect(),
                width: natural,
            };
        }
        // Too wide for columns: the list of the log, wrapped, without its title.
        let listed = self.renderer.event(&Event::Screen(table.clone()));
        let body: Vec<String> = listed
            .iter()
            .skip(1)
            .flat_map(|line| wrap(&line.text, room))
            .collect();
        Layout {
            header: None,
            width: room,
            body,
        }
    }
}

/// A row with its padding.
fn padded(row: &str) -> String {
    format!("{pad}{row}", pad = " ".repeat(PADDING))
}

/// A count as the `i64` a text argument holds.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}
