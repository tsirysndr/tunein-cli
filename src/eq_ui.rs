//! Equalizer popup shared by the `play` TUI and interactive mode.
//!
//! Owns only view state (visibility + selected band); the actual EQ values
//! live in [`crate::equalizer::Equalizer::global`], which the audio thread
//! reads, and every change is persisted straight to the settings file.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame,
};

use crate::equalizer::Equalizer;
use crate::settings::EQ_BANDS;
use crate::theme;

/// Gain range in dB the vertical sliders map onto (± this many dB).
const RANGE_DB: i32 = 24;

/// Selectable columns: the 10 EQ bands plus the bass and treble shelves.
const BASS_COL: usize = EQ_BANDS;
const TREBLE_COL: usize = EQ_BANDS + 1;
const TOTAL_COLS: usize = EQ_BANDS + 2;

pub struct EqPopup {
    pub visible: bool,
    selected: usize,
}

impl EqPopup {
    pub fn new() -> Self {
        Self {
            visible: false,
            selected: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Handle a key while the popup is open. Returns `true` when the key
    /// was consumed; Ctrl-modified keys are passed through so global
    /// shortcuts (Ctrl+C) keep working.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.visible || key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }

        let eq = Equalizer::global();
        let coarse = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Esc | KeyCode::Char('e') => self.visible = false,
            KeyCode::Left => self.selected = self.selected.saturating_sub(1),
            KeyCode::Right => self.selected = (self.selected + 1).min(TOTAL_COLS - 1),
            KeyCode::Up | KeyCode::Down => {
                let sign = if key.code == KeyCode::Up { 1 } else { -1 };
                match self.selected {
                    // Bands are in tenths of dB: 0.5 dB fine, 2 dB coarse.
                    0..=9 => {
                        let step = if coarse { 20 } else { 5 };
                        eq.adjust_band_gain(self.selected, sign * step);
                    }
                    // Tone shelves are in whole dB: 1 dB fine, 4 dB coarse.
                    BASS_COL => {
                        eq.adjust_bass(sign * if coarse { 4 } else { 1 });
                    }
                    _ => {
                        eq.adjust_treble(sign * if coarse { 4 } else { 1 });
                    }
                }
                eq.save();
            }
            KeyCode::Char(' ') | KeyCode::Char('t') => {
                eq.set_enabled(!eq.is_enabled());
                eq.save();
            }
            KeyCode::Char('0') => {
                eq.reset_gains();
                eq.save();
            }
            _ => {}
        }

        true
    }

    /// Draw the popup centered over `frame`. Call last so it sits on top.
    pub fn render(&self, frame: &mut Frame) {
        if !self.visible {
            return;
        }

        let eq = Equalizer::global();
        let enabled = eq.is_enabled();

        // Bands are stored in tenths of dB, the tone shelves in whole dB;
        // normalize everything to tenths for the slider columns.
        let mut columns: Vec<SliderColumn> = eq
            .bands()
            .iter()
            .map(|b| SliderColumn {
                gain_tenths: b.gain,
                label: fmt_hz(b.cutoff),
                dimmed: !enabled,
            })
            .collect();
        columns.push(SliderColumn {
            gain_tenths: eq.bass() * 10,
            label: "Bass".to_string(),
            dimmed: eq.bass() == 0,
        });
        columns.push(SliderColumn {
            gain_tenths: eq.treble() * 10,
            label: "Treble".to_string(),
            dimmed: eq.treble() == 0,
        });

        let area = centered_rect(frame.size(), 80, 20);
        frame.render_widget(Clear, area);

        let title = if enabled {
            format!(" Equalizer — ON (±{} dB) ", RANGE_DB)
        } else {
            format!(" Equalizer — OFF (±{} dB) ", RANGE_DB)
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(if enabled {
                Style::default().fg(theme::PRIMARY)
            } else {
                Style::default().fg(Color::DarkGray)
            });
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 6 || inner.width < 30 {
            frame.render_widget(Paragraph::new("Terminal too small"), inner);
            return;
        }

        // Bottom row of the popup: key hints.
        let hints = Rect {
            x: inner.x,
            y: inner.y + inner.height - 1,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "←/→ select  ↑/↓ adjust (shift: coarse)  space eq on/off  0 reset  esc close",
                Style::default().fg(Color::DarkGray),
            )))
            .alignment(Alignment::Center),
            hints,
        );

        let sliders_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height - 1,
        };
        frame.render_widget(
            EqSliders {
                columns: &columns,
                selected: self.selected,
            },
            sliders_area,
        );
    }
}

/// One slider column, drawn with vertical block characters: a dB value
/// above the bar, a Hz/kHz (or "Bass"/"Treble") label below, and a
/// highlight on the selected column.
struct SliderColumn {
    /// Gain in tenths of dB.
    gain_tenths: i32,
    /// Text under the bar.
    label: String,
    /// Draw the bar muted (EQ off for bands, 0 dB for the tone shelves).
    dimmed: bool,
}

struct EqSliders<'a> {
    columns: &'a [SliderColumn],
    selected: usize,
}

impl<'a> Widget for EqSliders<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let n = self.columns.len();
        if n == 0 || area.width < 8 || area.height < 5 {
            return;
        }
        let col_w = (area.width / n as u16).max(4);
        let center_row = area.y + area.height / 2;

        // Reserve the top row for the "+N.N dB" text, the bottom row for
        // the label text; every row in between draws the bar.
        let bar_top = area.y + 1;
        let bar_bot = area.y + area.height.saturating_sub(2);
        if bar_bot <= bar_top {
            return;
        }
        let half_h = (bar_bot - bar_top) as i32 / 2;

        for (i, column) in self.columns.iter().enumerate() {
            let col_x = area.x + (i as u16) * col_w;
            let bar_x = col_x + col_w / 2;
            if bar_x >= area.right() {
                break;
            }

            // Separate the tone shelves from the EQ bands visually.
            if i == EQ_BANDS {
                for r in area.y..area.y + area.height {
                    buf.get_mut(col_x, r)
                        .set_char('┆')
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }

            let gain_db = column.gain_tenths as f32 / 10.0;
            let ratio = gain_db.clamp(-(RANGE_DB as f32), RANGE_DB as f32) / RANGE_DB as f32;
            let offset = (ratio * half_h as f32).round() as i32;
            let bar_end_row =
                (center_row as i32 - offset).clamp(bar_top as i32, bar_bot as i32) as u16;

            let is_sel = i == self.selected;
            let bar_color = if column.dimmed {
                Color::DarkGray
            } else if is_sel {
                theme::ACCENT
            } else {
                theme::PRIMARY
            };

            // Zero-dB axis through every column so the user can eyeball
            // who is pushed above / below flat.
            for r in bar_top..=bar_bot {
                let ch = if r == center_row { '─' } else { '│' };
                buf.get_mut(bar_x, r)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }

            // Fill from the zero axis toward the gain position.
            let (from, to) = if bar_end_row < center_row {
                (bar_end_row, center_row)
            } else {
                (center_row, bar_end_row)
            };
            for r in from..=to {
                buf.get_mut(bar_x, r)
                    .set_char('█')
                    .set_style(Style::default().fg(bar_color).add_modifier(Modifier::BOLD));
            }

            let label_style = if is_sel {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else if column.dimmed {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            Paragraph::new(Span::styled(format!("{:+.1}", gain_db), label_style))
                .alignment(Alignment::Center)
                .render(Rect::new(col_x, area.y, col_w, 1), buf);

            let label_row = area.y + area.height - 1;
            Paragraph::new(Span::styled(column.label.clone(), label_style))
                .alignment(Alignment::Center)
                .render(Rect::new(col_x, label_row, col_w, 1), buf);
        }
    }
}

/// Format Hz as `63`, `1k`, or `1.2k` — compact enough to fit under a
/// narrow slider column.
fn fmt_hz(hz: i32) -> String {
    if hz < 1000 {
        format!("{}", hz)
    } else if hz % 1000 == 0 {
        format!("{}k", hz / 1000)
    } else {
        format!("{:.1}k", hz as f32 / 1000.0)
    }
}

/// A rect of at most `width`×`height`, centered in `outer`.
fn centered_rect(outer: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(outer.width);
    let h = height.min(outer.height);
    Rect {
        x: outer.x + (outer.width - w) / 2,
        y: outer.y + (outer.height - h) / 2,
        width: w,
        height: h,
    }
}
