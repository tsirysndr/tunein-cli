use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Error};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use tokio::sync::mpsc;
use tunein_cli::os_media_controls::{self, OsMediaControls};

use crate::app::send_os_media_controls_command;
use crate::audio::{AudioController, PlaybackEvent, PlaybackState};
use crate::eq_ui::EqPopup;
use crate::extract::get_currently_playing;
use crate::favorites::{FavoriteStation, FavoritesStore};
use crate::fzf_ui::{FzfOutcome, FzfPopup};
use crate::help_ui::{HelpPopup, Shortcut};
use crate::provider::{radiobrowser::Radiobrowser, tunein::Tunein, Provider};
use crate::tui;
use crate::types::Station;

const MENU_OPTIONS: &[&str] = &[
    "Search Stations",
    "Browse Categories",
    "Play Station",
    "Favourites",
    "Resume Last Station",
    "Quit",
];

/// Shortcut table shown by the `?` popup in interactive mode.
const HUB_SHORTCUTS: &[Shortcut] = &[
    ("↑ / ↓", "Navigate lists"),
    ("enter", "Select menu entry / play highlighted station"),
    ("e", "Open the equalizer"),
    ("f", "Add or remove the highlighted station from favourites"),
    ("d / delete", "Remove favourite (favourites screen)"),
    ("x", "Stop playback"),
    ("+ / -", "Volume up / down"),
    ("/", "Open the fuzzy finder"),
    ("esc", "Back to the menu"),
    ("?", "Show this help"),
    ("ctrl+c", "Quit"),
];

const STATUS_TIMEOUT: Duration = Duration::from_secs(3);
const NOW_PLAYING_POLL_INTERVAL: Duration = Duration::from_secs(10);
/// How long the fuzzy finder waits after the last keystroke before it fires
/// a provider search, so fast typing doesn't hammer the network.
const FZF_DEBOUNCE: Duration = Duration::from_millis(180);

enum HubMessage {
    NowPlaying(String),
}

/// A debounce timer for the fuzzy finder fired: run the provider search for
/// this `(generation, query)` unless a newer keystroke has superseded it.
struct FzfSearchRequest {
    generation: usize,
    query: String,
}

pub async fn run(provider_name: &str) -> Result<(), Error> {
    let provider = resolve_provider(provider_name).await?;
    let (audio, mut audio_events) = AudioController::new()?;
    let favorites = FavoritesStore::load()?;
    let (metadata_tx, mut metadata_rx) = mpsc::unbounded_channel::<HubMessage>();
    let (fzf_tx, mut fzf_rx) = mpsc::unbounded_channel::<FzfSearchRequest>();

    let mut terminal = tui::init()?;

    let (input_tx, mut input_rx) = mpsc::unbounded_channel();
    spawn_input_thread(input_tx.clone());

    let os_media_controls = OsMediaControls::new()
        .inspect_err(|err| {
            eprintln!(
                "error: failed to initialize os media controls due to `{}`",
                err
            );
        })
        .ok();

    let mut app = HubApp::new(
        provider_name.to_string(),
        provider,
        audio,
        favorites,
        metadata_tx,
        fzf_tx,
        os_media_controls,
    );

    let result = loop {
        terminal.draw(|frame| app.render(frame))?;

        tokio::select! {
            Some(event) = input_rx.recv() => {
                match app.handle_event(event).await? {
                    Action::Quit => break Ok(()),
                    Action::Task(task) => app.perform_task(task).await?,
                    Action::None => {}
                }
            }
            Some(event) = audio_events.recv() => {
                app.handle_playback_event(event);
            }
            Some(message) = metadata_rx.recv() => {
                app.handle_metadata(message);
            }
            Some(request) = fzf_rx.recv() => {
                app.run_fzf_search(request).await;
            }
        }

        app.tick();
    };

    tui::restore()?;

    result
}

fn spawn_input_thread(tx: mpsc::UnboundedSender<Event>) {
    thread::spawn(move || loop {
        if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(event) = crossterm::event::read() {
                if tx.send(event).is_err() {
                    break;
                }
            }
        }
    });
}

struct HubApp {
    provider_name: String,
    provider: Box<dyn Provider>,
    audio: AudioController,
    favorites: FavoritesStore,
    ui: UiState,
    current_station: Option<StationRecord>,
    current_playback: Option<PlaybackState>,
    last_station: Option<StationRecord>,
    volume: f32,
    status: Option<StatusMessage>,
    metadata_tx: mpsc::UnboundedSender<HubMessage>,
    now_playing_station_id: Option<String>,
    next_now_playing_poll: Instant,
    os_media_controls: Option<OsMediaControls>,
    eq_popup: EqPopup,
    help_popup: HelpPopup,
    fzf_popup: FzfPopup,
    fzf_tx: mpsc::UnboundedSender<FzfSearchRequest>,
    /// Monotonic tag for the newest fuzzy-finder search; lets in-flight
    /// searches for stale queries cancel themselves and be discarded.
    fzf_generation: Arc<AtomicUsize>,
}

impl HubApp {
    fn new(
        provider_name: String,
        provider: Box<dyn Provider>,
        audio: AudioController,
        favorites: FavoritesStore,
        metadata_tx: mpsc::UnboundedSender<HubMessage>,
        fzf_tx: mpsc::UnboundedSender<FzfSearchRequest>,
        os_media_controls: Option<OsMediaControls>,
    ) -> Self {
        let mut ui = UiState::default();
        ui.menu_state.select(Some(0));
        Self {
            provider_name,
            provider,
            audio,
            favorites,
            ui,
            current_station: None,
            current_playback: None,
            last_station: None,
            volume: 100.0,
            status: None,
            metadata_tx,
            now_playing_station_id: None,
            next_now_playing_poll: Instant::now(),
            os_media_controls,
            eq_popup: EqPopup::new(),
            help_popup: HelpPopup::new(HUB_SHORTCUTS),
            fzf_popup: FzfPopup::new(),
            fzf_tx,
            fzf_generation: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(8),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(frame.size());

        self.render_header(frame, areas[0]);
        self.render_divider(frame, areas[1]);
        self.render_main(frame, areas[2]);
        frame.render_widget(self.render_footer(), areas[3]);
        self.eq_popup.render(frame);
        self.help_popup.render(frame);
        self.fzf_popup.render(frame);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Block::new()
                .borders(Borders::TOP)
                .title(" TuneIn CLI ")
                .title_alignment(Alignment::Center),
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );

        let mut row = area.y + 1;

        frame.render_widget(
            Paragraph::new(format!("Provider {}", self.provider_name)),
            Rect {
                x: area.x,
                y: row,
                width: area.width,
                height: 1,
            },
        );
        row += 1;

        let station_name = self
            .current_playback
            .as_ref()
            .and_then(|p| {
                let name = p.stream_name.trim();
                if name.is_empty() || name.eq_ignore_ascii_case("unknown") {
                    let fallback = p.station.name.trim();
                    if fallback.is_empty() {
                        None
                    } else {
                        Some(fallback.to_string())
                    }
                } else {
                    Some(name.to_string())
                }
            })
            .or_else(|| {
                self.current_station.as_ref().and_then(|s| {
                    let name = s.station.name.trim();
                    (!name.is_empty()).then_some(name.to_string())
                })
            })
            .unwrap_or_else(|| "Unknown".to_string());
        let station_id = self
            .current_playback
            .as_ref()
            .map(|p| p.station.id.as_str())
            .or_else(|| self.current_station.as_ref().map(|s| s.station.id.as_str()))
            .unwrap_or("N/A");

        self.render_labeled_line(
            frame,
            area,
            row,
            "Station ",
            &format!("{} - {}", station_name, station_id),
        );
        row += 1;

        let now_playing = self
            .current_playback
            .as_ref()
            .and_then(|p| {
                let np = p.now_playing.trim();
                (!np.is_empty()).then_some(np.to_string())
            })
            .or_else(|| {
                self.current_station
                    .as_ref()
                    .and_then(|s| s.station.playing.as_ref())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "—".to_string());
        self.render_labeled_line(frame, area, row, "Now Playing ", &now_playing);
        row += 1;

        let genre = self
            .current_playback
            .as_ref()
            .and_then(|p| {
                let genre = p.genre.trim();
                (!genre.is_empty()).then_some(genre.to_string())
            })
            .unwrap_or_else(|| "Unknown".to_string());
        self.render_labeled_line(frame, area, row, "Genre ", &genre);
        row += 1;

        let description = self
            .current_playback
            .as_ref()
            .and_then(|p| {
                let desc = p.description.trim();
                (!desc.is_empty()).then_some(desc.to_string())
            })
            .unwrap_or_else(|| "Unknown".to_string());
        self.render_labeled_line(frame, area, row, "Description ", &description);
        row += 1;

        let bitrate = self
            .current_playback
            .as_ref()
            .and_then(|p| {
                let br = p.bitrate.trim();
                (!br.is_empty()).then_some(format!("{} kbps", br))
            })
            .or_else(|| {
                self.current_station.as_ref().and_then(|s| {
                    (s.station.bitrate > 0).then_some(format!("{} kbps", s.station.bitrate))
                })
            })
            .unwrap_or_else(|| "Unknown".to_string());
        self.render_labeled_line(frame, area, row, "Bitrate ", &bitrate);
        row += 1;

        let volume_display = format!("{}%", self.volume as u32);
        self.render_labeled_line(frame, area, row, "Volume ", &volume_display);
    }

    fn render_labeled_line(&self, frame: &mut Frame, area: Rect, y: u16, label: &str, value: &str) {
        let span_label = Span::styled(label, Style::default().fg(Color::LightBlue));
        let span_value = Span::raw(value);
        let line = Line::from(vec![span_label, span_value]);
        frame.render_widget(
            Paragraph::new(line),
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            },
        );
    }

    fn render_main(&mut self, frame: &mut Frame, area: Rect) {
        if matches!(self.ui.screen, Screen::Menu) {
            self.render_menu_area(frame, area);
            return;
        }

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(5),
                ]
                .as_ref(),
            )
            .split(area);

        self.render_non_menu_content(frame, sections[0]);
        self.render_divider(frame, sections[1]);
        self.render_feature_panel(frame, sections[2]);
    }

    fn render_non_menu_content(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.ui.screen {
            Screen::Menu => {}
            Screen::SearchInput => {
                let text = format!(
                    "Search query: {}\n\nPress Enter to submit, Esc to cancel",
                    self.ui.search_input
                );
                let paragraph = Paragraph::new(text)
                    .block(Block::default().title("Search").borders(Borders::ALL));
                frame.render_widget(paragraph, area);
            }
            Screen::PlayInput => {
                let text = format!(
                    "Station name or ID: {}\n\nPress Enter to submit, Esc to cancel",
                    self.ui.play_input
                );
                let paragraph = Paragraph::new(text)
                    .block(Block::default().title("Play Station").borders(Borders::ALL));
                frame.render_widget(paragraph, area);
            }
            Screen::SearchResults => {
                let items = Self::station_items(&self.ui.search_results);
                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(String::from("Search Results"))
                            .borders(Borders::ALL),
                    )
                    .highlight_symbol("➜ ")
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                frame.render_stateful_widget(list, area, &mut self.ui.search_results_state);
            }
            Screen::Categories => {
                let items = Self::category_items(&self.ui.categories);
                let list = List::new(items)
                    .block(Block::default().title("Categories").borders(Borders::ALL))
                    .highlight_symbol("➜ ")
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                frame.render_stateful_widget(list, area, &mut self.ui.categories_state);
            }
            Screen::BrowseStations { category } => {
                let items = Self::station_items(&self.ui.browse_results);
                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(format!("Stations in {}", category))
                            .borders(Borders::ALL),
                    )
                    .highlight_symbol("➜ ")
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                frame.render_stateful_widget(list, area, &mut self.ui.browse_state);
            }
            Screen::Favourites => {
                let items = Self::favourite_items(self.favorites.all());
                let list = List::new(items)
                    .block(Block::default().title("Favourites").borders(Borders::ALL))
                    .highlight_symbol("➜ ")
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                frame.render_stateful_widget(list, area, &mut self.ui.favourites_state);
            }
            Screen::Loading => {
                let message = self
                    .ui
                    .loading_message
                    .as_deref()
                    .unwrap_or("Loading, please wait…");
                let paragraph = Paragraph::new(message)
                    .block(Block::default().title("Loading").borders(Borders::ALL))
                    .alignment(Alignment::Center);
                frame.render_widget(paragraph, area);
            }
        }
    }

    fn render_divider(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let width = area.width as usize;
        if width == 0 {
            return;
        }
        let mut line = String::with_capacity(width + 3);
        while line.len() < width {
            line.push_str("---");
        }
        line.truncate(width);
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_feature_panel(&self, frame: &mut Frame, area: Rect) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let lines = self.feature_panel_lines();
        let text = lines.join("\n");
        let paragraph =
            Paragraph::new(text).block(Block::default().title("Actions").borders(Borders::ALL));
        frame.render_widget(paragraph, area);
    }

    fn render_menu_area(&mut self, frame: &mut Frame, area: Rect) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let disable_resume = self.last_station.is_none();
        let items: Vec<ListItem> = MENU_OPTIONS
            .iter()
            .map(|option| {
                if *option == "Resume Last Station" && disable_resume {
                    ListItem::new(Line::from(Span::styled(
                        *option,
                        Style::default().fg(Color::DarkGray),
                    )))
                } else {
                    ListItem::new(*option)
                }
            })
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Main Menu"))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("➜ ");
        frame.render_stateful_widget(list, area, &mut self.ui.menu_state);
    }

    fn station_items(stations: &[Station]) -> Vec<ListItem<'_>> {
        if stations.is_empty() {
            vec![ListItem::new("No stations found")]
        } else {
            stations
                .iter()
                .map(|station| {
                    let mut line = station.name.clone();
                    if let Some(now) = &station.playing {
                        if !now.is_empty() {
                            line.push_str(&format!(" — {}", now));
                        }
                    }
                    ListItem::new(line)
                })
                .collect()
        }
    }

    fn category_items(categories: &[String]) -> Vec<ListItem<'_>> {
        if categories.is_empty() {
            vec![ListItem::new("No categories available")]
        } else {
            categories
                .iter()
                .map(|category| ListItem::new(category.clone()))
                .collect()
        }
    }

    fn favourite_items(favourites: &[FavoriteStation]) -> Vec<ListItem<'_>> {
        if favourites.is_empty() {
            vec![ListItem::new("No favourites saved yet")]
        } else {
            favourites
                .iter()
                .map(|fav| ListItem::new(format!("{} ({})", fav.name, fav.provider)))
                .collect()
        }
    }

    fn handle_favourite_action(&mut self) -> Result<bool, Error> {
        match self.ui.screen {
            Screen::SearchResults => {
                let Some(index) = self.ui.search_results_state.selected() else {
                    self.set_status("No search result selected");
                    return Ok(true);
                };
                let station = self
                    .ui
                    .search_results
                    .get(index)
                    .cloned()
                    .ok_or_else(|| anyhow!("Search result missing at index {}", index))?;
                self.add_station_to_favourites(station)?;
                Ok(true)
            }
            Screen::BrowseStations { .. } => {
                let Some(index) = self.ui.browse_state.selected() else {
                    self.set_status("No station selected");
                    return Ok(true);
                };
                let station = self
                    .ui
                    .browse_results
                    .get(index)
                    .cloned()
                    .ok_or_else(|| anyhow!("Browse result missing at index {}", index))?;
                self.add_station_to_favourites(station)?;
                Ok(true)
            }
            Screen::Favourites => {
                let Some(index) = self.ui.favourites_state.selected() else {
                    self.set_status("No favourite selected");
                    return Ok(true);
                };
                self.remove_favourite_at(index)?;
                Ok(true)
            }
            _ => {
                self.toggle_current_favourite()?;
                Ok(true)
            }
        }
    }

    fn add_station_to_favourites(&mut self, station: Station) -> Result<(), Error> {
        if station.id.is_empty() {
            self.set_status("Cannot favourite station without an id");
            return Ok(());
        }

        let entry = FavoriteStation {
            id: station.id.clone(),
            name: station.name.clone(),
            provider: self.provider_name.clone(),
        };

        if self.favorites.is_favorite(&entry.id, &entry.provider) {
            self.set_status("Already in favourites");
        } else {
            self.favorites.add(entry)?;
            self.set_status(&format!("Added \"{}\" to favourites", station.name));
        }
        Ok(())
    }

    fn remove_favourite_at(&mut self, index: usize) -> Result<(), Error> {
        let Some(favourite) = self.favorites.all().get(index).cloned() else {
            self.set_status("Favourite not found");
            return Ok(());
        };
        self.favorites.remove(&favourite.id, &favourite.provider)?;
        self.set_status(&format!("Removed \"{}\" from favourites", favourite.name));

        let len = self.favorites.all().len();
        if len == 0 {
            self.ui.favourites_state.select(None);
        } else {
            let new_index = index.min(len - 1);
            self.ui.favourites_state.select(Some(new_index));
        }

        Ok(())
    }

    fn stop_playback(&mut self) -> Result<(), Error> {
        self.audio.stop()?;
        self.set_status("Playback stopped");
        Ok(())
    }

    fn default_footer_hint(&self) -> String {
        match self.ui.screen {
            Screen::SearchResults => {
                "↑/↓ navigate • Enter play • f add to favourites • e equalizer • x stop playback • Esc back • +/- volume"
                    .to_string()
            }
            Screen::Favourites => {
                "↑/↓ navigate • Enter play • f remove favourite • d/Delete remove • x stop playback • Esc back • +/- volume"
                    .to_string()
            }
            Screen::Categories => {
                "↑/↓ navigate • Enter open • x stop playback • Esc back • +/- volume".to_string()
            }
            Screen::BrowseStations { .. } => {
                "↑/↓ navigate • Enter play • f add to favourites • x stop playback • Esc back • +/- volume".to_string()
            }
            Screen::SearchInput | Screen::PlayInput => {
                "Type to edit • Enter submit • x stop playback • Esc cancel • +/- volume".to_string()
            }
            Screen::Loading => "Please wait… • x stop playback • Esc cancel • +/- volume".to_string(),
            Screen::Menu => {
                "↑/↓ navigate • Enter select • / search • e equalizer • x stop • Esc back • +/- volume • ? help"
                    .to_string()
            }
        }
    }

    fn feature_panel_lines(&self) -> Vec<String> {
        let mut lines = match self.ui.screen {
            Screen::SearchResults => vec![
                "Search Results".to_string(),
                "Enter  • Play highlighted station".to_string(),
                "f      • Add highlighted station to favourites".to_string(),
                "Esc    • Return to main menu".to_string(),
            ],
            Screen::Favourites => vec![
                "Favourites".to_string(),
                "Enter  • Play selected favourite".to_string(),
                "f      • Remove highlighted favourite".to_string(),
                "d/Del • Remove highlighted favourite".to_string(),
                "Esc    • Return to main menu".to_string(),
            ],
            Screen::BrowseStations { .. } => vec![
                "Browse Stations".to_string(),
                "Enter  • Play highlighted station".to_string(),
                "f      • Add highlighted station to favourites".to_string(),
                "Esc    • Back to categories".to_string(),
            ],
            Screen::Categories => vec![
                "Categories".to_string(),
                "Enter  • Drill into selected category".to_string(),
                "Esc    • Return to main menu".to_string(),
            ],
            Screen::SearchInput => vec![
                "Search".to_string(),
                "Enter  • Run search".to_string(),
                "Esc    • Cancel".to_string(),
            ],
            Screen::PlayInput => vec![
                "Play Station".to_string(),
                "Enter  • Start playback".to_string(),
                "Esc    • Cancel".to_string(),
            ],
            Screen::Loading => vec!["Loading…".to_string(), "Esc    • Cancel".to_string()],
            Screen::Menu => vec![
                "Main Menu".to_string(),
                "Enter  • Activate highlighted option".to_string(),
                "Esc    • Quit or back".to_string(),
            ],
        };

        if self.current_station.is_some() {
            lines.insert(1, "x      • Stop playback".to_string());
        } else {
            lines.insert(1, "x      • Stop playback (no active stream)".to_string());
        }

        lines
    }

    fn render_footer(&self) -> Paragraph<'_> {
        let hint = self.default_footer_hint();
        let text = if let Some(status) = &self.status {
            format!("{}  •  {}", status.message, hint)
        } else {
            hint
        };
        Paragraph::new(text)
    }

    async fn handle_event(&mut self, event: Event) -> Result<Action, Error> {
        match event {
            Event::Key(key) => self.handle_key_event(key).await,
            Event::Resize(_, _) => Ok(Action::None),
            _ => Ok(Action::None),
        }
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<Action, Error> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(Action::Quit);
        }

        // The fuzzy finder captures the whole keyboard while it is open.
        if self.fzf_popup.is_visible() {
            return self.handle_fzf_key(key);
        }

        // While a popup is open it captures the keyboard.
        if self.eq_popup.handle_key(key) || self.help_popup.handle_key(key) {
            return Ok(Action::None);
        }

        // `/` opens the fuzzy finder from anywhere except a text field, where
        // it is a literal character.
        if key.code == KeyCode::Char('/')
            && !key.modifiers.contains(KeyModifiers::CONTROL)
            && !matches!(self.ui.screen, Screen::SearchInput | Screen::PlayInput)
        {
            self.open_fzf();
            return Ok(Action::None);
        }

        match key.code {
            KeyCode::Char('e')
                if !matches!(self.ui.screen, Screen::SearchInput | Screen::PlayInput) =>
            {
                self.eq_popup.toggle();
                return Ok(Action::None);
            }
            KeyCode::Char('?')
                if !matches!(self.ui.screen, Screen::SearchInput | Screen::PlayInput) =>
            {
                self.help_popup.toggle();
                return Ok(Action::None);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_volume(5.0)?;
                return Ok(Action::None);
            }
            KeyCode::Char('-') => {
                self.adjust_volume(-5.0)?;
                return Ok(Action::None);
            }
            KeyCode::Char('x') => {
                self.stop_playback()?;
                return Ok(Action::None);
            }
            KeyCode::Char('f') => {
                if self.handle_favourite_action()? {
                    return Ok(Action::None);
                }
            }
            KeyCode::Esc if !matches!(self.ui.screen, Screen::Menu) => {
                self.ui.screen = Screen::Menu;
                return Ok(Action::None);
            }
            _ => {}
        }

        match self.ui.screen {
            Screen::Menu => self.handle_menu_keys(key),
            Screen::SearchInput => self.handle_text_input(key, true),
            Screen::PlayInput => self.handle_text_input(key, false),
            Screen::SearchResults => self.handle_station_list_keys(key, ListKind::Search),
            Screen::Categories => self.handle_categories_keys(key),
            Screen::BrowseStations { .. } => self.handle_station_list_keys(key, ListKind::Browse),
            Screen::Favourites => self.handle_favourites_keys(key),
            Screen::Loading => Ok(Action::None),
        }
    }

    /// Open the fuzzy finder, seeding it with whatever list is most relevant
    /// to the current screen so `enter` is useful before any typing.
    fn open_fzf(&mut self) {
        let seed = match &self.ui.screen {
            Screen::SearchResults => self.ui.search_results.clone(),
            Screen::BrowseStations { .. } => self.ui.browse_results.clone(),
            _ => self
                .favorites
                .all()
                .iter()
                .map(|fav| Station {
                    id: fav.id.clone(),
                    name: fav.name.clone(),
                    codec: String::new(),
                    bitrate: 0,
                    stream_url: String::new(),
                    playing: None,
                })
                .collect(),
        };
        self.fzf_popup.open(seed);
    }

    /// Route a key to the fuzzy finder and act on its outcome. Any selected
    /// result — favourite, search hit or browse entry — is played directly.
    fn handle_fzf_key(&mut self, key: KeyEvent) -> Result<Action, Error> {
        match self.fzf_popup.handle_key(key) {
            FzfOutcome::QueryChanged => {
                self.schedule_fzf_search();
                Ok(Action::None)
            }
            FzfOutcome::Submit(station) => {
                Ok(Action::Task(PendingTask::PlayStation(StationRecord {
                    provider: self.provider_name.clone(),
                    station,
                })))
            }
            FzfOutcome::Close | FzfOutcome::Consumed | FzfOutcome::Ignored => Ok(Action::None),
        }
    }

    /// Arm a debounce timer for the finder's current query. When it elapses
    /// (and no newer keystroke has landed) the timer wakes the main loop to
    /// run the actual search — the provider isn't `Send`, so it can't run in
    /// the spawned task itself.
    fn schedule_fzf_search(&mut self) {
        let query = self.fzf_popup.query().trim().to_string();
        if query.is_empty() {
            self.fzf_popup.set_searching(false);
            return;
        }

        let generation = self.fzf_generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.fzf_popup.set_searching(true);

        let tx = self.fzf_tx.clone();
        let latest = self.fzf_generation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(FZF_DEBOUNCE).await;
            // A newer keystroke superseded this one; don't hit the network.
            if latest.load(Ordering::SeqCst) == generation {
                let _ = tx.send(FzfSearchRequest { generation, query });
            }
        });
    }

    /// Run a debounced fuzzy-finder search and feed the results back to the
    /// popup, ignoring stale generations and a finder that has since closed.
    async fn run_fzf_search(&mut self, request: FzfSearchRequest) {
        if !self.fzf_popup.is_visible()
            || request.generation != self.fzf_generation.load(Ordering::SeqCst)
        {
            return;
        }

        let results = self
            .provider
            .search(request.query)
            .await
            .unwrap_or_default();

        // Guard again: the query may have moved on while we awaited the network.
        if self.fzf_popup.is_visible()
            && request.generation == self.fzf_generation.load(Ordering::SeqCst)
        {
            self.fzf_popup.set_results(results);
            self.fzf_popup.set_searching(false);
        }
    }

    fn handle_menu_keys(&mut self, key: KeyEvent) -> Result<Action, Error> {
        let current = self.ui.menu_state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                let new = current.saturating_sub(1);
                self.ui.menu_state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Down => {
                let max = MENU_OPTIONS.len().saturating_sub(1);
                let new = (current + 1).min(max);
                self.ui.menu_state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Enter => match MENU_OPTIONS[current] {
                "Search Stations" => {
                    self.ui.search_input.clear();
                    self.ui.screen = Screen::SearchInput;
                    Ok(Action::None)
                }
                "Browse Categories" => {
                    self.ui.loading_message = Some("Fetching categories…".to_string());
                    self.ui.screen = Screen::Loading;
                    Ok(Action::Task(PendingTask::LoadCategories))
                }
                "Play Station" => {
                    self.ui.play_input.clear();
                    self.ui.screen = Screen::PlayInput;
                    Ok(Action::None)
                }
                "Favourites" => {
                    self.ui.screen = Screen::Favourites;
                    if self.favorites.all().is_empty() {
                        self.ui.favourites_state.select(None);
                    } else {
                        self.ui.favourites_state.select(Some(0));
                    }
                    Ok(Action::None)
                }
                "Resume Last Station" => {
                    if let Some(station) = self.last_station.clone() {
                        Ok(Action::Task(PendingTask::PlayStation(station)))
                    } else {
                        self.set_status("No station played yet to resume");
                        Ok(Action::None)
                    }
                }
                "Quit" => Ok(Action::Quit),
                _ => Ok(Action::None),
            },
            _ => Ok(Action::None),
        }
    }

    fn handle_text_input(&mut self, key: KeyEvent, is_search: bool) -> Result<Action, Error> {
        let buffer = if is_search {
            &mut self.ui.search_input
        } else {
            &mut self.ui.play_input
        };

        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                buffer.push(c);
                Ok(Action::None)
            }
            KeyCode::Backspace => {
                buffer.pop();
                Ok(Action::None)
            }
            KeyCode::Enter => {
                if buffer.trim().is_empty() {
                    self.set_status("Input cannot be empty");
                    return Ok(Action::None);
                }
                let query = buffer.trim().to_string();
                self.ui.loading_message = Some("Searching stations…".to_string());
                self.ui.screen = Screen::Loading;
                if is_search {
                    Ok(Action::Task(PendingTask::Search(query)))
                } else {
                    Ok(Action::Task(PendingTask::PlayDirect(query)))
                }
            }
            _ => Ok(Action::None),
        }
    }

    fn handle_station_list_keys(&mut self, key: KeyEvent, kind: ListKind) -> Result<Action, Error> {
        let (items_len, state) = match kind {
            ListKind::Search => (
                self.ui.search_results.len(),
                &mut self.ui.search_results_state,
            ),
            ListKind::Browse => (self.ui.browse_results.len(), &mut self.ui.browse_state),
        };

        if items_len == 0 {
            if key.code == KeyCode::Esc {
                self.ui.screen = Screen::Menu;
            }
            return Ok(Action::None);
        }

        let current = state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                let new = current.saturating_sub(1);
                state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Down => {
                let max = items_len.saturating_sub(1);
                let new = (current + 1).min(max);
                state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Enter => {
                let station = match kind {
                    ListKind::Search => self.ui.search_results[current].clone(),
                    ListKind::Browse => self.ui.browse_results[current].clone(),
                };
                Ok(Action::Task(PendingTask::PlayStation(StationRecord {
                    provider: self.provider_name.clone(),
                    station,
                })))
            }
            KeyCode::Esc => {
                self.ui.screen = Screen::Menu;
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    fn handle_categories_keys(&mut self, key: KeyEvent) -> Result<Action, Error> {
        let len = self.ui.categories.len();
        if len == 0 {
            if key.code == KeyCode::Esc {
                self.ui.screen = Screen::Menu;
            }
            return Ok(Action::None);
        }

        let current = self.ui.categories_state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                let new = current.saturating_sub(1);
                self.ui.categories_state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Down => {
                let max = len.saturating_sub(1);
                let new = (current + 1).min(max);
                self.ui.categories_state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Enter => {
                let category = self.ui.categories[current].clone();
                self.ui.loading_message = Some(format!("Loading stations for {}…", category));
                self.ui.screen = Screen::Loading;
                Ok(Action::Task(PendingTask::LoadCategoryStations { category }))
            }
            KeyCode::Esc => {
                self.ui.screen = Screen::Menu;
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    fn handle_favourites_keys(&mut self, key: KeyEvent) -> Result<Action, Error> {
        let len = self.favorites.all().len();
        if len == 0 {
            if key.code == KeyCode::Esc {
                self.ui.screen = Screen::Menu;
            }
            return Ok(Action::None);
        }

        let current = self.ui.favourites_state.selected().unwrap_or(0);
        match key.code {
            KeyCode::Up => {
                let new = current.saturating_sub(1);
                self.ui.favourites_state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Down => {
                let max = len.saturating_sub(1);
                let new = (current + 1).min(max);
                self.ui.favourites_state.select(Some(new));
                Ok(Action::None)
            }
            KeyCode::Enter => {
                let favourite = self.favorites.all()[current].clone();
                Ok(Action::Task(PendingTask::PlayFavourite(favourite)))
            }
            KeyCode::Delete | KeyCode::Char('d') | KeyCode::Char('f') => {
                self.remove_favourite_at(current)?;
                Ok(Action::None)
            }
            KeyCode::Esc => {
                self.ui.screen = Screen::Menu;
                Ok(Action::None)
            }
            _ => Ok(Action::None),
        }
    }

    fn adjust_volume(&mut self, delta: f32) -> Result<(), Error> {
        self.volume = (self.volume + delta).clamp(0.0, 150.0);
        self.audio.set_volume(self.volume)?;
        self.set_status(&format!("Volume set to {}%", self.volume as u32));
        Ok(())
    }

    fn toggle_current_favourite(&mut self) -> Result<(), Error> {
        let Some(station) = &self.current_station else {
            self.set_status("No active station to favourite");
            return Ok(());
        };

        if station.station.id.is_empty() {
            self.set_status("Current station cannot be favourited");
            return Ok(());
        }

        let entry = FavoriteStation {
            id: station.station.id.clone(),
            name: station.station.name.clone(),
            provider: station.provider.clone(),
        };
        let added = self.favorites.toggle(entry)?;
        if added {
            self.set_status("Added to favourites");
        } else {
            self.set_status("Removed from favourites");
        }
        Ok(())
    }

    fn handle_playback_event(&mut self, event: PlaybackEvent) {
        match event {
            PlaybackEvent::Started(state) => {
                self.current_playback = Some(state.clone());
                if let Some(station) = self.current_station.as_mut() {
                    station.station.playing = Some(state.now_playing.clone());
                    station.station.id = state.station.id.clone();
                }
                self.set_status(&format!("Now playing {}", state.stream_name));
                self.prepare_now_playing_poll();
            }
            PlaybackEvent::Error(err) => {
                self.current_playback = None;
                self.set_status(&format!("Playback error: {}", err));
                self.now_playing_station_id = None;
            }
            PlaybackEvent::Stopped => {
                self.current_playback = None;
                self.set_status("Playback stopped");
                self.now_playing_station_id = None;
            }
        }
    }

    fn handle_metadata(&mut self, message: HubMessage) {
        match message {
            HubMessage::NowPlaying(now_playing) => {
                if let Some(playback) = self.current_playback.as_mut() {
                    playback.now_playing = now_playing.clone();
                }
                if let Some(station) = self.current_station.as_mut() {
                    station.station.playing = Some(now_playing.clone());
                }
                self.set_status(&format!("Now Playing {}", now_playing));

                let name = self
                    .current_station
                    .as_ref()
                    .map(|s| s.station.name.clone())
                    .unwrap_or_default();

                send_os_media_controls_command(
                    self.os_media_controls.as_mut(),
                    os_media_controls::Command::SetMetadata(souvlaki::MediaMetadata {
                        title: (!now_playing.is_empty()).then_some(now_playing.as_str()),
                        album: (!name.is_empty()).then_some(name.as_str()),
                        artist: None,
                        cover_url: None,
                        duration: None,
                    }),
                );
            }
        }
    }

    async fn perform_task(&mut self, task: PendingTask) -> Result<(), Error> {
        self.ui.loading_message = None;
        match task {
            PendingTask::Search(query) => {
                let results = self.provider.search(query.clone()).await?;
                self.ui.search_results = results;
                self.ui.search_results_state.select(Some(0));
                self.ui.screen = Screen::SearchResults;
                self.set_status(&format!("Search complete for \"{}\"", query));
            }
            PendingTask::LoadCategories => {
                let categories = self.provider.categories(0, 100).await?;
                self.ui.categories = categories;
                self.ui.categories_state.select(Some(0));
                self.ui.screen = Screen::Categories;
                self.set_status("Categories loaded");
            }
            PendingTask::LoadCategoryStations { category } => {
                let stations = self.provider.browse(category.clone(), 0, 100).await?;
                self.ui.browse_results = stations;
                self.ui.browse_state.select(Some(0));
                self.ui.screen = Screen::BrowseStations { category };
                self.set_status("Stations loaded");
            }
            PendingTask::PlayDirect(input) => {
                let provider = resolve_provider(&self.provider_name).await?;
                match provider.get_station(input.clone()).await? {
                    Some(mut station) => {
                        if station.stream_url.is_empty() {
                            station = fetch_station(&self.provider_name, &station.id)
                                .await?
                                .ok_or_else(|| anyhow!("Unable to locate stream for station"))?;
                        }
                        self.play_station(StationRecord {
                            provider: self.provider_name.clone(),
                            station,
                        })
                        .await?;
                    }
                    None => {
                        self.ui.screen = Screen::Menu;
                        self.set_status(&format!("Station \"{}\" not found", input));
                    }
                }
            }
            PendingTask::PlayStation(record) => {
                self.play_station(record).await?;
            }
            PendingTask::PlayFavourite(favourite) => {
                let station = fetch_station(&favourite.provider, &favourite.id)
                    .await?
                    .ok_or_else(|| anyhow!("Failed to load favourite station"))?;
                self.play_station(StationRecord {
                    provider: favourite.provider,
                    station,
                })
                .await?;
            }
        }
        Ok(())
    }

    async fn play_station(&mut self, mut record: StationRecord) -> Result<(), Error> {
        if record.station.stream_url.is_empty() {
            if let Some(enriched) = fetch_station(&record.provider, &record.station.id).await? {
                record.station = enriched;
            } else {
                return Err(anyhow!("Unable to resolve station stream"));
            }
        }

        self.audio.play(record.station.clone(), self.volume)?;
        self.current_station = Some(record.clone());
        self.last_station = Some(record);
        self.prepare_now_playing_poll();
        self.ui.screen = Screen::Menu;
        Ok(())
    }

    fn prepare_now_playing_poll(&mut self) {
        if let Some(station) = &self.current_station {
            if station.provider == "tunein" && !station.station.id.is_empty() {
                self.now_playing_station_id = Some(station.station.id.clone());
                self.next_now_playing_poll = Instant::now();
            } else {
                self.now_playing_station_id = None;
            }
        }
    }

    fn tick(&mut self) {
        if let Some(status) = &self.status {
            if status.expires_at <= Instant::now() {
                self.status = None;
            }
        }
        self.poll_now_playing_if_needed();
    }

    fn poll_now_playing_if_needed(&mut self) {
        let Some(station_id) = self.now_playing_station_id.clone() else {
            return;
        };

        if Instant::now() < self.next_now_playing_poll {
            return;
        }

        let tx = self.metadata_tx.clone();
        tokio::spawn(async move {
            if let Ok(now) = get_currently_playing(&station_id).await {
                let _ = tx.send(HubMessage::NowPlaying(now));
            }
        });

        self.next_now_playing_poll = Instant::now() + NOW_PLAYING_POLL_INTERVAL;
    }

    fn set_status<S: Into<String>>(&mut self, message: S) {
        self.status = Some(StatusMessage {
            message: message.into(),
            expires_at: Instant::now() + STATUS_TIMEOUT,
        });
    }
}

struct UiState {
    screen: Screen,
    menu_state: ListState,
    search_input: String,
    play_input: String,
    search_results: Vec<Station>,
    search_results_state: ListState,
    categories: Vec<String>,
    categories_state: ListState,
    browse_results: Vec<Station>,
    browse_state: ListState,
    favourites_state: ListState,
    loading_message: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::Menu,
            menu_state: ListState::default(),
            search_input: String::new(),
            play_input: String::new(),
            search_results: Vec::new(),
            search_results_state: ListState::default(),
            categories: Vec::new(),
            categories_state: ListState::default(),
            browse_results: Vec::new(),
            browse_state: ListState::default(),
            favourites_state: ListState::default(),
            loading_message: None,
        }
    }
}

#[derive(Clone)]
enum Screen {
    Menu,
    SearchInput,
    PlayInput,
    SearchResults,
    Categories,
    BrowseStations { category: String },
    Favourites,
    Loading,
}

enum ListKind {
    Search,
    Browse,
}

enum PendingTask {
    Search(String),
    LoadCategories,
    LoadCategoryStations { category: String },
    PlayDirect(String),
    PlayStation(StationRecord),
    PlayFavourite(FavoriteStation),
}

enum Action {
    None,
    Quit,
    Task(PendingTask),
}

struct StatusMessage {
    message: String,
    expires_at: Instant,
}

#[derive(Clone)]
struct StationRecord {
    provider: String,
    station: Station,
}

async fn resolve_provider(name: &str) -> Result<Box<dyn Provider>, Error> {
    match name {
        "tunein" => Ok(Box::new(Tunein::new())),
        "radiobrowser" => Ok(Box::new(Radiobrowser::new().await)),
        other => Err(anyhow!("Unsupported provider '{}'", other)),
    }
}

async fn fetch_station(provider_name: &str, id: &str) -> Result<Option<Station>, Error> {
    let provider = resolve_provider(provider_name).await?;
    provider.get_station(id.to_string()).await
}
