//! `/` fuzzy finder: a centered, fzf-style modal for searching stations.
//!
//! The popup owns only view state — the query, the candidate corpus it was
//! handed, and the fuzzy-ranked selection. The host ([`crate::interactive`])
//! feeds it fresh candidates from the provider as the query changes and acts
//! on whatever the user submits with `enter`.

use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::theme;
use crate::types::Station;

/// Braille spinner frames shown while a provider search is in flight.
const SPINNER: [char; 8] = ['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];

/// What the host should do after the popup has handled a key.
pub enum FzfOutcome {
    /// The popup was not open; the key belongs to the host.
    Ignored,
    /// The key was handled; nothing else to do.
    Consumed,
    /// The query text changed — the host should (re)run its search.
    QueryChanged,
    /// The user picked a station with `enter`.
    Submit(Station),
    /// The user dismissed the popup with `esc`.
    Close,
}

pub struct FzfPopup {
    visible: bool,
    /// The text typed after the `❯` prompt.
    query: String,
    /// The candidate corpus most recently handed to the popup.
    results: Vec<Station>,
    /// `(index into results, matched char positions in the name)`, best first.
    matches: Vec<(usize, Vec<usize>)>,
    state: ListState,
    /// A provider search is in flight for the current query.
    searching: bool,
    /// Advances every frame the spinner is drawn.
    spinner: usize,
}

impl FzfPopup {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            results: Vec::new(),
            matches: Vec::new(),
            state: ListState::default(),
            searching: false,
            spinner: 0,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// The current query, trimmed of surrounding whitespace.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Open the finder seeded with an initial candidate list (e.g. the
    /// favourites or the list currently on screen) so it is useful before
    /// the first keystroke.
    pub fn open(&mut self, seed: Vec<Station>) {
        self.visible = true;
        self.query.clear();
        self.searching = false;
        self.results = seed;
        self.recompute();
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.searching = false;
    }

    /// Replace the candidate corpus (typically fresh provider results for the
    /// current query) and re-rank.
    pub fn set_results(&mut self, results: Vec<Station>) {
        self.results = results;
        self.recompute();
    }

    pub fn set_searching(&mut self, searching: bool) {
        self.searching = searching;
    }

    /// Handle a key while the popup is open.
    pub fn handle_key(&mut self, key: KeyEvent) -> FzfOutcome {
        if !self.visible {
            return FzfOutcome::Ignored;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                self.close();
                FzfOutcome::Close
            }
            KeyCode::Enter => match self.selected_station() {
                Some(station) => {
                    self.close();
                    FzfOutcome::Submit(station)
                }
                None => FzfOutcome::Consumed,
            },
            KeyCode::Down => {
                self.move_selection(1);
                FzfOutcome::Consumed
            }
            KeyCode::Up => {
                self.move_selection(-1);
                FzfOutcome::Consumed
            }
            // Emacs-style navigation, familiar from fzf itself.
            KeyCode::Char('n') if ctrl => {
                self.move_selection(1);
                FzfOutcome::Consumed
            }
            KeyCode::Char('p') if ctrl => {
                self.move_selection(-1);
                FzfOutcome::Consumed
            }
            KeyCode::Char('u') if ctrl => {
                self.query.clear();
                self.recompute();
                FzfOutcome::QueryChanged
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.recompute();
                FzfOutcome::QueryChanged
            }
            KeyCode::Char(c) if !ctrl => {
                self.query.push(c);
                self.recompute();
                FzfOutcome::QueryChanged
            }
            _ => FzfOutcome::Consumed,
        }
    }

    /// The station under the selection cursor, if any.
    fn selected_station(&self) -> Option<Station> {
        let selected = self.state.selected()?;
        let (index, _) = self.matches.get(selected)?;
        self.results.get(*index).cloned()
    }

    fn move_selection(&mut self, delta: i32) {
        if self.matches.is_empty() {
            self.state.select(None);
            return;
        }
        let len = self.matches.len() as i32;
        let current = self.state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, len - 1) as usize;
        self.state.select(Some(next));
    }

    /// Fuzzy-rank the corpus against the query. An empty query keeps the
    /// corpus in its original order. Selection jumps back to the top match,
    /// like fzf does after every edit.
    fn recompute(&mut self) {
        let mut scored: Vec<(usize, i32, Vec<usize>)> = self
            .results
            .iter()
            .enumerate()
            .filter_map(|(i, station)| {
                fuzzy_match(&self.query, &station.name)
                    .map(|(score, positions)| (i, score, positions))
            })
            .collect();
        // Higher score first; ties keep the corpus order for stability.
        scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        self.matches = scored.into_iter().map(|(i, _, pos)| (i, pos)).collect();

        if self.matches.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }

    /// Draw the popup centered over `frame`. Call last so it sits on top.
    pub fn render(&mut self, frame: &mut Frame) {
        if !self.visible {
            return;
        }

        let size = frame.size();
        let width = ((size.width as u32 * 80) / 100) as u16;
        let height = ((size.height as u32 * 75) / 100) as u16;
        let area = centered_rect(
            size,
            width.clamp(24, size.width),
            height.clamp(6, size.height),
        );
        frame.render_widget(Clear, area);

        let block = Block::new()
            .borders(Borders::ALL)
            .title(" 🔍 Fuzzy Finder ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(theme::PRIMARY));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        if inner.height < 3 || inner.width < 4 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // prompt
                Constraint::Length(1), // counter
                Constraint::Min(1),    // results
            ])
            .split(inner);

        // Prompt line: `❯ query▌`, with a placeholder before the first keystroke.
        let mut prompt = vec![Span::styled(
            "❯ ",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )];
        if self.query.is_empty() {
            prompt.push(Span::styled(
                "Type to search stations…",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            prompt.push(Span::raw(self.query.clone()));
            prompt.push(Span::styled(
                " ",
                Style::default().add_modifier(Modifier::REVERSED),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(prompt)), chunks[0]);

        // Counter line: `⣾ 12/240` — spinner (while searching) and match count.
        let spinner = if self.searching {
            self.spinner = self.spinner.wrapping_add(1);
            SPINNER[self.spinner % SPINNER.len()]
        } else {
            ' '
        };
        let counter = format!(
            "  {} {}/{}",
            spinner,
            self.matches.len(),
            self.results.len()
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                counter,
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[1],
        );

        // Results, with matched characters highlighted.
        let items: Vec<ListItem> = if self.matches.is_empty() {
            let message = if self.searching {
                "Searching…"
            } else if self.query.is_empty() {
                "No stations yet — start typing"
            } else {
                "No matches"
            };
            vec![ListItem::new(Span::styled(
                message,
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            self.matches
                .iter()
                .map(|(index, positions)| {
                    ListItem::new(highlight_line(&self.results[*index], positions))
                })
                .collect()
        };
        let list = List::new(items).highlight_symbol("❯ ").highlight_style(
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, chunks[2], &mut self.state);
    }
}

/// Build the display line for one result, painting the fuzzy-matched
/// characters of the name and appending its "now playing" subtext.
fn highlight_line(station: &Station, positions: &[usize]) -> Line<'static> {
    let matched: HashSet<usize> = positions.iter().copied().collect();
    let mut spans = Vec::new();
    let mut run = String::new();
    let mut run_matched = false;

    for (i, ch) in station.name.chars().enumerate() {
        let is_match = matched.contains(&i);
        if !run.is_empty() && is_match != run_matched {
            spans.push(make_span(&run, run_matched));
            run.clear();
        }
        run.push(ch);
        run_matched = is_match;
    }
    if !run.is_empty() {
        spans.push(make_span(&run, run_matched));
    }

    if let Some(playing) = &station.playing {
        let playing = playing.trim();
        if !playing.is_empty() {
            spans.push(Span::styled(
                format!("  — {}", playing),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    Line::from(spans)
}

fn make_span(text: &str, matched: bool) -> Span<'static> {
    if matched {
        Span::styled(
            text.to_string(),
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(text.to_string())
    }
}

/// Fuzzy subsequence match of `query` against `text`, case-insensitive.
///
/// Returns `None` when `query` is not a subsequence of `text`; otherwise a
/// `(score, matched_char_positions)` pair. The scoring rewards consecutive
/// matches, matches at word boundaries and matches near the start, so the
/// most fzf-like candidate sorts to the top. An empty query matches
/// everything with a neutral score.
pub fn fuzzy_match(query: &str, text: &str) -> Option<(i32, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }

    let text: Vec<char> = text.chars().collect();
    let mut positions = Vec::new();
    let mut score = 0i32;
    let mut cursor = 0usize;
    let mut previous: Option<usize> = None;

    for qc in query.chars() {
        let needle = qc.to_ascii_lowercase();
        let found = loop {
            if cursor >= text.len() {
                return None;
            }
            let matches = text[cursor].to_ascii_lowercase() == needle;
            cursor += 1;
            if matches {
                break cursor - 1;
            }
        };

        let mut char_score = 1;
        match previous {
            Some(prev) if found == prev + 1 => char_score += 5, // adjacent
            Some(prev) => char_score -= ((found - prev - 1).min(10)) as i32, // gap penalty
            None => char_score += 10 - (found.min(10) as i32),  // reward an early first hit
        }
        // Word-boundary bonus: start of string or after a separator.
        let boundary = found == 0
            || matches!(
                text.get(found - 1),
                Some(' ') | Some('-') | Some('_') | Some('/') | Some('.') | Some('(')
            );
        if boundary {
            char_score += 3;
        }

        score += char_score;
        positions.push(found);
        previous = Some(found);
    }

    Some((score, positions))
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
