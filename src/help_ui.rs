//! `?` help popup: a centered modal listing every keyboard shortcut of the
//! current UI. Shared by the `play` TUI and interactive mode, which each
//! pass their own shortcut table.

use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// One `(key, description)` row.
pub type Shortcut = (&'static str, &'static str);

pub struct HelpPopup {
    pub visible: bool,
    shortcuts: &'static [Shortcut],
}

impl HelpPopup {
    pub fn new(shortcuts: &'static [Shortcut]) -> Self {
        Self {
            visible: false,
            shortcuts,
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
        if matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
        ) {
            self.visible = false;
        }
        true
    }

    /// Draw the popup centered over `frame`. Call last so it sits on top.
    pub fn render(&self, frame: &mut Frame) {
        if !self.visible {
            return;
        }

        let key_width = self
            .shortcuts
            .iter()
            .map(|(k, _)| k.len())
            .max()
            .unwrap_or(0);
        let desc_width = self
            .shortcuts
            .iter()
            .map(|(_, d)| d.len())
            .max()
            .unwrap_or(0);

        let width = (key_width + desc_width + 7) as u16;
        let height = self.shortcuts.len() as u16 + 4;
        let area = centered_rect(frame.size(), width.max(40), height);
        frame.render_widget(Clear, area);

        let block = Block::new()
            .borders(Borders::ALL)
            .title(" Keyboard Shortcuts ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(theme::PRIMARY));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::with_capacity(self.shortcuts.len());
        for (key, description) in self.shortcuts {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {:>key_width$}  ", key),
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(*description),
            ]));
        }
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "esc / ? close",
            Style::default().fg(Color::DarkGray),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
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
