//! Drawing a staged scene: a window over the log with a bar that fills, what the scene is
//! doing and what it is up against, and a memory dump that settles as the bar passes it.
//!
//! The window only repeats what the game says in words (the messages that come with it, the
//! gauges), so a player who skips it, or who never sees it, loses nothing. It is made of
//! text first: the percentage and the level of the defences are written out, and colour only
//! adds to them (`GAU-1`).

use neon_engine::event::Effect;
use neon_engine::text::Text;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as TuiLine, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::App;
use super::draw::color_of;
use crate::palette::Accent;
use crate::tui::motion::{self, Staging};
use crate::tui::text::{head, wrap};

/// Widest the window gets.
const WINDOW_WIDTH_MAX: u16 = 62;
/// Narrowest window that still reads; below it the scene is not drawn.
const WINDOW_WIDTH_MIN: u16 = 30;
/// The words for the levels of defence, from the lightest.
const SECURITY_KEYS: [&str; 6] = [
    "ui.tui.fx.security.1",
    "ui.tui.fx.security.2",
    "ui.tui.fx.security.3",
    "ui.tui.fx.security.4",
    "ui.tui.fx.security.5",
    "ui.tui.fx.security.6",
];
/// Levels of defence there are words for.
const SECURITY_LEVELS: u8 = 6;
/// What the intrusion is doing, step by step (the steps of `motion::HACK_CURVE`).
const HACK_PHASE_KEYS: [&str; 5] = [
    "ui.tui.fx.hack.phase.1",
    "ui.tui.fx.hack.phase.2",
    "ui.tui.fx.hack.phase.3",
    "ui.tui.fx.hack.phase.4",
    "ui.tui.fx.hack.phase.5",
];
/// What the decryption is doing, step by step.
const DECRYPT_PHASE_KEYS: [&str; 4] = [
    "ui.tui.fx.decrypt.phase.1",
    "ui.tui.fx.decrypt.phase.2",
    "ui.tui.fx.decrypt.phase.3",
    "ui.tui.fx.decrypt.phase.4",
];
/// What the purchase is doing, step by step.
const BUY_PHASE_KEYS: [&str; 3] = [
    "ui.tui.fx.buy.phase.1",
    "ui.tui.fx.buy.phase.2",
    "ui.tui.fx.buy.phase.3",
];
/// What the cover is doing, step by step.
const LAYLOW_PHASE_KEYS: [&str; 3] = [
    "ui.tui.fx.laylow.phase.1",
    "ui.tui.fx.laylow.phase.2",
    "ui.tui.fx.laylow.phase.3",
];
/// Cells the percentage takes at the end of the bar, widest case (`100 %`).
const PERCENT_WIDTH: usize = 5;
/// Rows of memory dump.
const NOISE_ROWS: usize = 2;

/// A row of the window and how much it matters when the room is short: the least important
/// rows go first.
struct Piece {
    line: TuiLine<'static>,
    /// Lower stays longer.
    keep: u8,
}

impl App<'_> {
    /// Draws the scene being staged, centred in `area`, over what is there.
    pub(super) fn draw_staging(&self, frame: &mut Frame<'_>, area: Rect) {
        let Some(staging) = &self.staging else {
            return;
        };
        let width = area.width.min(WINDOW_WIDTH_MAX);
        if width < WINDOW_WIDTH_MIN {
            return;
        }
        let inner_width = usize::from(width - 2);
        let (title, mut pieces) = match staging.effect() {
            Effect::Hack {
                site,
                security,
                taken,
            } => (
                Text::new("ui.tui.fx.hack.title")
                    .with_str("name", site.clone())
                    .with_term("run", "run"),
                self.hack_pieces(staging, *security, *taken, inner_width),
            ),
            Effect::Laylow { cooling } => (
                Text::new("ui.tui.fx.laylow.title"),
                self.laylow_pieces(staging, *cooling, inner_width),
            ),
            Effect::Decrypt { document, fragment } => (
                Text::new("ui.tui.fx.decrypt.title").with_text("name", document.clone()),
                self.decrypt_pieces(staging, *fragment, inner_width),
            ),
            Effect::Buy { item, price, .. } => (
                Text::new("ui.tui.fx.buy.title").with_text("name", item.clone()),
                self.buy_pieces(staging, item, *price, inner_width),
            ),
        };
        // Room for the borders, then the rows that matter most.
        let room = usize::from(area.height.saturating_sub(2));
        while pieces.len() > room {
            let Some(least) = pieces.iter().map(|piece| piece.keep).max() else {
                break;
            };
            // The last of the least important kind, so that the first stay in place.
            if let Some(index) = pieces.iter().rposition(|piece| piece.keep == least) {
                pieces.remove(index);
            }
        }
        // Without its status and its bar the window says nothing.
        if pieces.iter().filter(|piece| piece.keep == 0).count() < 2 {
            return;
        }
        let height = u16::try_from(pieces.len() + 2).unwrap_or(area.height);
        let window = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        // Cleared, then painted again with the base of the palette: no cell is left unpainted
        // on an opaque palette.
        frame.render_widget(Clear, window);
        frame.render_widget(Block::default().style(self.base_style()), window);
        let block = self.frame_block(&title);
        let lines: Vec<TuiLine<'static>> = pieces.into_iter().map(|piece| piece.line).collect();
        frame.render_widget(Paragraph::new(lines).block(block), window);
    }

    fn hack_pieces(&self, staging: &Staging, security: u8, taken: u32, width: usize) -> Vec<Piece> {
        let level = security.clamp(1, SECURITY_LEVELS);
        let status_key = if staging.is_done() {
            "ui.tui.fx.hack.status_done"
        } else {
            "ui.tui.fx.hack.status"
        };
        let status = Text::new(status_key)
            .with_int("percent", i64::from(staging.percent()))
            .with_text(
                "security",
                Text::new(phase_key(&SECURITY_KEYS, usize::from(level))),
            )
            .with_term("site", "site");
        let mut pieces = self.status_pieces(&status, width);
        pieces.push(blank(1));
        pieces.push(self.bar_piece(staging, width));
        pieces.push(self.defences_piece(level));
        let phase = if staging.is_done() {
            if taken == 0 {
                Text::new("ui.tui.fx.hack.done_empty")
            } else {
                Text::new("ui.tui.fx.hack.done").with_int("n", i64::from(taken))
            }
        } else {
            Text::new(phase_key(&HACK_PHASE_KEYS, staging.phase()))
        };
        pieces.push(self.phase_piece(&phase, staging.is_done(), width));
        pieces.extend(self.noise_pieces(staging, width));
        pieces.push(self.hint_piece(width));
        pieces
    }

    fn laylow_pieces(&self, staging: &Staging, cooling: u16, width: usize) -> Vec<Piece> {
        let status_key = if staging.is_done() {
            "ui.tui.fx.laylow.status_done"
        } else {
            "ui.tui.fx.laylow.status"
        };
        let status = Text::new(status_key).with_int("percent", i64::from(staging.percent()));
        let mut pieces = self.status_pieces(&status, width);
        pieces.push(blank(1));
        pieces.push(self.bar_piece(staging, width));
        if let Some(value) = staging.notoriety() {
            let text = Text::new("ui.tui.fx.laylow.notoriety")
                .with_text("gauge", Text::new("gauge.notoriety"))
                .with_int("value", i64::from(value));
            pieces.push(Piece {
                line: TuiLine::from(Span::styled(
                    head(&self.renderer.text(&text), width),
                    self.accent_style(Accent::Threat),
                )),
                keep: 1,
            });
        }
        let phase = if staging.is_done() {
            Text::new("ui.tui.fx.laylow.done")
                .with_term("notoriety", "notoriety")
                .with_int("cooling", i64::from(cooling))
        } else {
            Text::new(phase_key(&LAYLOW_PHASE_KEYS, staging.phase()))
        };
        pieces.push(self.phase_piece(&phase, staging.is_done(), width));
        pieces.extend(self.noise_pieces(staging, width));
        pieces.push(self.hint_piece(width));
        pieces
    }

    fn decrypt_pieces(&self, staging: &Staging, fragment: bool, width: usize) -> Vec<Piece> {
        let status_key = if staging.is_done() {
            "ui.tui.fx.decrypt.status_done"
        } else {
            "ui.tui.fx.decrypt.status"
        };
        let status = Text::new(status_key).with_int("percent", i64::from(staging.percent()));
        let mut pieces = self.status_pieces(&status, width);
        pieces.push(blank(1));
        pieces.push(self.bar_piece(staging, width));
        let phase = if staging.is_done() {
            let key = if fragment {
                "ui.tui.fx.decrypt.done_fragment"
            } else {
                "ui.tui.fx.decrypt.done"
            };
            Text::new(key)
                .with_term("document", "document")
                .with_term("fragment", "fragment")
                .with_term("archives", "archives")
        } else {
            Text::new(phase_key(&DECRYPT_PHASE_KEYS, staging.phase()))
        };
        pieces.push(self.phase_piece(&phase, staging.is_done(), width));
        pieces.extend(self.noise_pieces(staging, width));
        pieces.push(self.hint_piece(width));
        pieces
    }

    fn buy_pieces(&self, staging: &Staging, item: &Text, price: u32, width: usize) -> Vec<Piece> {
        let status_key = if staging.is_done() {
            "ui.tui.fx.buy.status_done"
        } else {
            "ui.tui.fx.buy.status"
        };
        let status = Text::new(status_key).with_int("percent", i64::from(staging.percent()));
        let mut pieces = self.status_pieces(&status, width);
        pieces.push(blank(1));
        pieces.push(self.bar_piece(staging, width));
        let credits = |amount: u32| {
            Text::new("campaign.price")
                .with_int("price", i64::from(amount))
                .with_term("credit", "credit")
        };
        let debit = Text::new("ui.tui.fx.buy.debit").with_text("price", credits(price));
        let mut rows = vec![debit];
        if let Some(balance) = staging.balance() {
            rows.push(Text::new("ui.tui.fx.buy.balance").with_text("balance", credits(balance)));
        }
        for row in rows {
            pieces.push(Piece {
                line: TuiLine::from(Span::styled(
                    head(&self.renderer.text(&row), width),
                    self.accent_style(Accent::Threat),
                )),
                keep: 1,
            });
        }
        let phase = if staging.is_done() {
            Text::new("campaign.buy.done").with_text("name", item.clone())
        } else {
            Text::new(phase_key(&BUY_PHASE_KEYS, staging.phase()))
        };
        pieces.push(self.phase_piece(&phase, staging.is_done(), width));
        pieces.extend(self.noise_pieces(staging, width));
        pieces.push(self.hint_piece(width));
        pieces
    }

    /// The sentence that says what is happening, wrapped to the window.
    fn status_pieces(&self, status: &Text, width: usize) -> Vec<Piece> {
        wrap(&self.renderer.text(status), width)
            .into_iter()
            .map(|row| Piece {
                line: TuiLine::from(Span::styled(row, self.accent_style(Accent::Title))),
                keep: 0,
            })
            .collect()
    }

    /// The bar: a gradient from magenta to cyan where there is colour, bold where there is
    /// none, and the percentage written out at its end.
    fn bar_piece(&self, staging: &Staging, width: usize) -> Piece {
        let palette = self.renderer.palette();
        // Written as the language writes it, right-aligned so that the bar does not shift.
        let figure = self.renderer.text(
            &Text::new("ui.tui.fx.percent").with_int("percent", i64::from(staging.percent())),
        );
        let suffix = format!(" {figure:>PERCENT_WIDTH$}");
        let cells = width.saturating_sub(suffix.chars().count()).max(1);
        let filled = motion::filled(staging.progress(), cells);
        let (full, empty) = if self.renderer.mode.ascii {
            ("#", ".")
        } else {
            ("█", "░")
        };
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(cells + 1);
        for cell in 0..cells {
            if cell < filled {
                let style = palette
                    .gradient(motion::along(cell, cells))
                    .map_or(bold, |hue| {
                        Style::default()
                            .fg(color_of(hue, &palette))
                            .add_modifier(Modifier::BOLD)
                    });
                spans.push(Span::styled(full, style));
            } else {
                spans.push(Span::styled(
                    empty,
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }
        }
        spans.push(Span::styled(suffix, self.accent_style(Accent::Title)));
        Piece {
            line: TuiLine::from(spans),
            keep: 0,
        }
    }

    /// The defences of the site as pips, with their name: the level is also in the sentence.
    fn defences_piece(&self, level: u8) -> Piece {
        let (full, empty) = if self.renderer.mode.ascii {
            ('#', '.')
        } else {
            ('■', '□')
        };
        let pips: String = (1..=SECURITY_LEVELS)
            .map(|pip| if pip <= level { full } else { empty })
            .collect();
        let text = Text::new("ui.tui.fx.defences")
            .with_term("ice", "ice")
            .with_str("pips", pips);
        Piece {
            line: TuiLine::from(Span::styled(
                self.renderer.text(&text),
                self.accent_style(Accent::Threat),
            )),
            keep: 1,
        }
    }

    /// What is being done now, or what came of it.
    fn phase_piece(&self, phase: &Text, done: bool, width: usize) -> Piece {
        let accent = if done { Accent::Locked } else { Accent::Live };
        let marker = if self.renderer.mode.ascii { ">" } else { "▸" };
        Piece {
            line: TuiLine::from(Span::styled(
                head(&format!("{marker} {}", self.renderer.text(phase)), width),
                self.accent_style(accent).add_modifier(Modifier::BOLD),
            )),
            keep: 0,
        }
    }

    /// The memory dump: settled digits behind the bar, running ones ahead of it.
    fn noise_pieces(&self, staging: &Staging, width: usize) -> Vec<Piece> {
        let locked = self.accent_style(Accent::Locked);
        let live = self.accent_style(Accent::Live);
        (0..NOISE_ROWS)
            .map(|row| {
                let noise = staging.noise(row, width);
                Piece {
                    line: TuiLine::from(vec![
                        Span::styled(noise.locked, locked),
                        Span::styled(noise.live, live),
                    ]),
                    keep: 3,
                }
            })
            .collect()
    }

    fn hint_piece(&self, width: usize) -> Piece {
        let hint = self.renderer.text(&Text::new("ui.tui.fx.skip"));
        Piece {
            line: TuiLine::from(Span::styled(
                head(&hint, width),
                Style::default().add_modifier(Modifier::DIM),
            )),
            keep: 2,
        }
    }
}

/// The key of the `number`th text (from 1) of a list; the nearest one if the number is out of
/// the list, so that a curve with more steps than texts still shows something.
fn phase_key(keys: &[&'static str], number: usize) -> &'static str {
    keys.get(number.saturating_sub(1))
        .or(keys.last())
        .copied()
        .unwrap_or_default()
}

fn blank(keep: u8) -> Piece {
    Piece {
        line: TuiLine::default(),
        keep,
    }
}
