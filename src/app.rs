use crossterm::event::{self, Event, KeyCode, KeyModifiers, MediaKeyCode};
use ratatui::{
    prelude::*,
    widgets::{block::*, *},
};
use souvlaki::MediaControlEvent;
use std::{
    io,
    ops::Range,
    sync::{mpsc::Receiver, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tunein_cli::os_media_controls::{self, OsMediaControls};

use crate::{
    extract::get_currently_playing,
    input::stream_to_matrix,
    play::SinkCommand,
    tui,
    visualization::{
        oscilloscope::Oscilloscope, spectroscope::Spectroscope, vectorscope::Vectorscope,
        Dimension, DisplayMode, GraphConfig,
    },
};

#[derive(Debug, Default, Clone)]
pub struct State {
    pub name: String,
    pub now_playing: String,
    pub genre: String,
    pub description: String,
    pub br: String,
    /// [`Volume`].
    pub volume: Volume,
}

/// Volume of the player.
#[derive(Debug, Clone, PartialEq)]
pub struct Volume {
    /// Raw volume stored as percentage.
    raw_volume_percent: f32,
    /// Is muted?
    is_muted: bool,
}

impl Volume {
    /// Create a new [`Volume`].
    pub const fn new(raw_volume_percent: f32, is_muted: bool) -> Self {
        Self {
            raw_volume_percent,
            is_muted,
        }
    }

    /// Get the current volume ratio. Returns `0.0` if muted.
    pub const fn volume_ratio(&self) -> f32 {
        if self.is_muted {
            0.0
        } else {
            self.raw_volume_percent / 100.0
        }
    }

    /// Get the raw volume percent.
    pub const fn raw_volume_percent(&self) -> f32 {
        self.raw_volume_percent
    }

    /// Is volume muted?
    pub const fn is_muted(&self) -> bool {
        self.is_muted
    }

    /// Toggle mute.
    pub const fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
    }

    /// Set the volume to the given volume ratio.
    ///
    /// `1.0` is 100% volume.
    pub const fn set_volume_ratio(&mut self, volume: f32) {
        self.raw_volume_percent = volume * 100.0;
        self.raw_volume_percent = self.raw_volume_percent.max(0.0);
    }

    /// Change the volume by the given step percent.
    ///
    /// To increase the volume, use a positive step. To decrease the
    /// volume, use a negative step.
    pub const fn change_volume(&mut self, step_percent: f32) {
        self.raw_volume_percent += step_percent;
        // limit to 0 volume, no upper bound
        self.raw_volume_percent = self.raw_volume_percent.max(0.0);
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self::new(100.0, false)
    }
}

pub enum CurrentDisplayMode {
    Oscilloscope,
    Vectorscope,
    Spectroscope,
    None,
}

impl std::str::FromStr for CurrentDisplayMode {
    type Err = InvalidDisplayModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Oscilloscope" => Ok(Self::Oscilloscope),
            "Vectorscope" => Ok(Self::Vectorscope),
            "Spectroscope" => Ok(Self::Spectroscope),
            "None" => Ok(Self::None),
            _ => Err(InvalidDisplayModeError),
        }
    }
}

/// Invalid display mode error.
#[derive(Debug)]
pub struct InvalidDisplayModeError;

impl std::fmt::Display for InvalidDisplayModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid display mode")
    }
}

impl std::error::Error for InvalidDisplayModeError {}

pub struct App {
    #[allow(unused)]
    channels: u8,
    graph: GraphConfig,
    oscilloscope: Oscilloscope,
    vectorscope: Vectorscope,
    spectroscope: Spectroscope,
    mode: CurrentDisplayMode,
    frame_rx: Receiver<minimp3::Frame>,
    /// [`OsMediaControls`].
    os_media_controls: Option<OsMediaControls>,
}

impl App {
    pub fn new(
        ui: &crate::cfg::UiOptions,
        source: &crate::cfg::SourceOptions,
        frame_rx: Receiver<minimp3::Frame>,
        mode: CurrentDisplayMode,
        os_media_controls: Option<OsMediaControls>,
    ) -> Self {
        let graph = GraphConfig {
            axis_color: Color::DarkGray,
            labels_color: Color::Cyan,
            palette: vec![Color::Red, Color::Yellow, Color::Green, Color::Magenta],
            scale: ui.scale as f64,
            width: source.buffer, // TODO also make bit depth customizable
            samples: source.buffer,
            sampling_rate: source.sample_rate,
            references: !ui.no_reference,
            show_ui: !ui.no_ui,
            scatter: ui.scatter,
            pause: false,
            marker_type: if ui.no_braille {
                Marker::Dot
            } else {
                Marker::Braille
            },
        };

        let oscilloscope = Oscilloscope::from_args(source);
        let vectorscope = Vectorscope::from_args(source);
        let spectroscope = Spectroscope::from_args(source);

        Self {
            graph,
            oscilloscope,
            vectorscope,
            spectroscope,
            mode,
            channels: source.channels as u8,
            frame_rx,
            os_media_controls,
        }
    }
}

fn render_frame(state: Arc<Mutex<State>>, frame: &mut Frame) {
    let state = state.lock().unwrap();
    let size = frame.size();

    frame.render_widget(
        Block::new()
            .borders(Borders::TOP)
            .title(" TuneIn CLI ")
            .title_alignment(Alignment::Center),
        Rect {
            x: size.x,
            y: size.y,
            width: size.width,
            height: 1,
        },
    );

    render_line(
        "Station ",
        &state.name,
        Rect {
            x: size.x,
            y: size.y + 1,
            width: size.width,
            height: 1,
        },
        frame,
    );

    if !state.now_playing.is_empty() {
        render_line(
            "Now Playing ",
            &state.now_playing,
            Rect {
                x: size.x,
                y: size.y + 2,
                width: size.width,
                height: 1,
            },
            frame,
        );
    }

    render_line(
        "Genre ",
        &state.genre,
        Rect {
            x: size.x,
            y: match state.now_playing.is_empty() {
                true => size.y + 2,
                false => size.y + 3,
            },
            width: size.width,
            height: 1,
        },
        frame,
    );
    render_line(
        "Description ",
        &state.description,
        Rect {
            x: size.x,
            y: match state.now_playing.is_empty() {
                true => size.y + 3,
                false => size.y + 4,
            },
            width: size.width,
            height: 1,
        },
        frame,
    );
    render_line(
        "Bitrate ",
        &match state.br.is_empty() {
            true => "Unknown".to_string(),
            false => format!("{} kbps", &state.br),
        },
        Rect {
            x: size.x,
            y: match state.now_playing.is_empty() {
                true => size.y + 4,
                false => size.y + 5,
            },
            width: size.width,
            height: 1,
        },
        frame,
    );
    render_line(
        "Volume ",
        &if state.volume.is_muted() {
            format!("{}% muted", state.volume.raw_volume_percent())
        } else {
            format!("{}%", state.volume.raw_volume_percent())
        },
        Rect {
            x: size.x,
            y: match state.now_playing.is_empty() {
                true => size.y + 5,
                false => size.y + 6,
            },
            width: size.width,
            height: 1,
        },
        frame,
    )
}

fn render_line(label: &str, value: &str, area: Rect, frame: &mut Frame) {
    let span1 = Span::styled(label, Style::new().fg(Color::LightBlue));
    let span2 = Span::raw(value);

    let line = Line::from(vec![span1, span2]);
    let text: Text = Text::from(vec![line]);

    frame.render_widget(Paragraph::new(text), area);
}

impl App {
    /// runs the application's main loop until the user quits
    pub async fn run(
        &mut self,
        terminal: &mut tui::Tui,
        mut cmd_rx: UnboundedReceiver<State>,
        mut sink_cmd_tx: UnboundedSender<SinkCommand>,
        id: &str,
    ) {
        let new_state = cmd_rx.recv().await.unwrap();
        let new_state = Arc::new(Mutex::new(new_state));

        let id = id.to_string();
        let new_state_clone = new_state.clone();

        thread::spawn(move || loop {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut new_state = new_state_clone.lock().unwrap();
                // Get current playing if available, otherwise use state's value
                new_state.now_playing = get_currently_playing(&id).await.unwrap_or_default();
                drop(new_state);
                std::thread::sleep(Duration::from_millis(10000));
            });
        });

        let mut fps = 0;
        let mut framerate = 0;
        let mut last_poll = Instant::now();

        loop {
            let channels = (!self.graph.pause)
                .then(|| self.frame_rx.recv().unwrap())
                .map(|audio_frame| {
                    stream_to_matrix(audio_frame.data.iter().cloned(), audio_frame.channels, 1.)
                });

            fps += 1;

            if last_poll.elapsed().as_secs() >= 1 {
                framerate = fps;
                fps = 0;
                last_poll = Instant::now();
            }

            {
                let mut datasets = Vec::new();
                let graph = self.graph.clone(); // TODO cheap fix...
                if self.graph.references {
                    if let Some(current_display) = self.current_display() {
                        datasets.append(&mut current_display.references(&graph));
                    }
                }
                if let Some((current_display, channels)) = self.current_display_mut().zip(channels)
                {
                    datasets.append(&mut current_display.process(&graph, &channels));
                }
                terminal
                    .draw(|f| {
                        let mut size = f.size();
                        render_frame(new_state.clone(), f);
                        if let Some(current_display) = self.current_display() {
                            if self.graph.show_ui {
                                f.render_widget(
                                    make_header(
                                        &self.graph,
                                        &current_display.header(&self.graph),
                                        current_display.mode_str(),
                                        framerate,
                                        self.graph.pause,
                                    ),
                                    Rect {
                                        x: size.x,
                                        y: size.y + 7,
                                        width: size.width,
                                        height: 1,
                                    },
                                );
                                size.height -= 8;
                                size.y += 8;
                            }
                            let chart = Chart::new(datasets.iter().map(|x| x.into()).collect())
                                .x_axis(current_display.axis(&self.graph, Dimension::X)) // TODO allow to have axis sometimes?
                                .y_axis(current_display.axis(&self.graph, Dimension::Y));
                            f.render_widget(chart, size)
                        }
                    })
                    .unwrap();
            }

            while let Some(event) = self
                .os_media_controls
                .as_mut()
                .and_then(|os_media_controls| os_media_controls.try_recv_os_event())
            {
                if self.process_os_media_control_event(event, &new_state, &mut sink_cmd_tx) {
                    return;
                }
            }

            while event::poll(Duration::from_millis(0)).unwrap() {
                // process all enqueued events
                let event = event::read().unwrap();

                if self
                    .process_events(event.clone(), new_state.clone(), &mut sink_cmd_tx)
                    .unwrap()
                {
                    return;
                }
                if let Some(current_display) = self.current_display_mut() {
                    current_display.handle(event);
                }
            }
        }
    }

    fn current_display_mut(&mut self) -> Option<&mut dyn DisplayMode> {
        match self.mode {
            CurrentDisplayMode::Oscilloscope => {
                Some(&mut self.oscilloscope as &mut dyn DisplayMode)
            }
            CurrentDisplayMode::Vectorscope => Some(&mut self.vectorscope as &mut dyn DisplayMode),
            CurrentDisplayMode::Spectroscope => {
                Some(&mut self.spectroscope as &mut dyn DisplayMode)
            }
            CurrentDisplayMode::None => None,
        }
    }

    fn current_display(&self) -> Option<&dyn DisplayMode> {
        match self.mode {
            CurrentDisplayMode::Oscilloscope => Some(&self.oscilloscope as &dyn DisplayMode),
            CurrentDisplayMode::Vectorscope => Some(&self.vectorscope as &dyn DisplayMode),
            CurrentDisplayMode::Spectroscope => Some(&self.spectroscope as &dyn DisplayMode),
            CurrentDisplayMode::None => None,
        }
    }

    fn process_events(
        &mut self,
        event: Event,
        state: Arc<Mutex<State>>,
        sink_cmd_tx: &mut UnboundedSender<SinkCommand>,
    ) -> Result<bool, io::Error> {
        let mut quit = false;

        if let Event::Key(key) = event {
            if let KeyModifiers::CONTROL = key.modifiers {
                match key.code {
                    // mimic other programs shortcuts to quit, for user friendlyness
                    KeyCode::Char('c') | KeyCode::Char('q') | KeyCode::Char('w') => quit = true,
                    _ => {}
                }
            }
            let magnitude = match key.modifiers {
                KeyModifiers::SHIFT => 10.0,
                KeyModifiers::CONTROL => 5.0,
                KeyModifiers::ALT => 0.2,
                _ => 1.0,
            };
            match key.code {
                KeyCode::Up => {
                    // inverted to act as zoom
                    update_value_f(&mut self.graph.scale, 0.01, magnitude, 0.0..10.0);
                    raise_volume(&state, self.os_media_controls.as_mut(), sink_cmd_tx);
                }
                KeyCode::Down => {
                    // inverted to act as zoom
                    update_value_f(&mut self.graph.scale, -0.01, magnitude, 0.0..10.0);
                    lower_volume(&state, self.os_media_controls.as_mut(), sink_cmd_tx);
                }
                KeyCode::Right => update_value_i(
                    &mut self.graph.samples,
                    true,
                    25,
                    magnitude,
                    0..self.graph.width * 2,
                ),
                KeyCode::Left => update_value_i(
                    &mut self.graph.samples,
                    false,
                    25,
                    magnitude,
                    0..self.graph.width * 2,
                ),
                KeyCode::Char('q') => quit = true,
                KeyCode::Char(' ') => toggle_play_pause(
                    &mut self.graph,
                    self.os_media_controls.as_mut(),
                    sink_cmd_tx,
                ),
                KeyCode::Char('s') => self.graph.scatter = !self.graph.scatter,
                KeyCode::Char('h') => self.graph.show_ui = !self.graph.show_ui,
                KeyCode::Char('r') => self.graph.references = !self.graph.references,
                KeyCode::Char('m') => {
                    mute_volume(&state, self.os_media_controls.as_mut(), sink_cmd_tx)
                }
                KeyCode::Esc => {
                    self.graph.samples = self.graph.width;
                    self.graph.scale = 1.;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    quit = true;
                }
                KeyCode::Tab => {
                    // switch modes
                    match self.mode {
                        CurrentDisplayMode::Oscilloscope => {
                            self.mode = CurrentDisplayMode::Vectorscope;
                        }
                        CurrentDisplayMode::Vectorscope => {
                            self.mode = CurrentDisplayMode::Spectroscope;
                        }
                        CurrentDisplayMode::Spectroscope => {
                            self.mode = CurrentDisplayMode::None;
                        }
                        CurrentDisplayMode::None => {
                            self.mode = CurrentDisplayMode::Oscilloscope;
                        }
                    }
                }
                KeyCode::Media(media_key_code) => match media_key_code {
                    MediaKeyCode::Play => play(
                        &mut self.graph,
                        self.os_media_controls.as_mut(),
                        sink_cmd_tx,
                    ),
                    MediaKeyCode::Pause => pause(
                        &mut self.graph,
                        self.os_media_controls.as_mut(),
                        sink_cmd_tx,
                    ),
                    MediaKeyCode::PlayPause => toggle_play_pause(
                        &mut self.graph,
                        self.os_media_controls.as_mut(),
                        sink_cmd_tx,
                    ),
                    MediaKeyCode::Stop => {
                        quit = true;
                    }
                    MediaKeyCode::LowerVolume => {
                        lower_volume(&state, self.os_media_controls.as_mut(), sink_cmd_tx)
                    }
                    MediaKeyCode::RaiseVolume => {
                        raise_volume(&state, self.os_media_controls.as_mut(), sink_cmd_tx)
                    }
                    MediaKeyCode::MuteVolume => {
                        mute_volume(&state, self.os_media_controls.as_mut(), sink_cmd_tx)
                    }
                    MediaKeyCode::TrackNext
                    | MediaKeyCode::TrackPrevious
                    | MediaKeyCode::Reverse
                    | MediaKeyCode::FastForward
                    | MediaKeyCode::Rewind
                    | MediaKeyCode::Record => {}
                },
                _ => {}
            }
        };

        Ok(quit)
    }

    /// Process OS media control event.
    ///
    /// Returns [`true`] if application should quit.
    fn process_os_media_control_event(
        &mut self,
        event: MediaControlEvent,
        state: &Mutex<State>,
        sink_cmd_tx: &mut UnboundedSender<SinkCommand>,
    ) -> bool {
        let mut quit = false;

        match event {
            MediaControlEvent::Play => {
                play(
                    &mut self.graph,
                    self.os_media_controls.as_mut(),
                    sink_cmd_tx,
                );
            }
            MediaControlEvent::Pause => {
                pause(
                    &mut self.graph,
                    self.os_media_controls.as_mut(),
                    sink_cmd_tx,
                );
            }
            MediaControlEvent::Toggle => {
                toggle_play_pause(
                    &mut self.graph,
                    self.os_media_controls.as_mut(),
                    sink_cmd_tx,
                );
            }
            MediaControlEvent::Stop | MediaControlEvent::Quit => {
                quit = true;
            }
            MediaControlEvent::SetVolume(volume) => {
                set_volume_ratio(
                    volume as f32,
                    state,
                    self.os_media_controls.as_mut(),
                    sink_cmd_tx,
                );
            }
            MediaControlEvent::Next
            | MediaControlEvent::Previous
            | MediaControlEvent::Seek(_)
            | MediaControlEvent::SeekBy(_, _)
            | MediaControlEvent::SetPosition(_)
            | MediaControlEvent::OpenUri(_)
            | MediaControlEvent::Raise => {}
        }

        quit
    }
}

pub fn update_value_f(val: &mut f64, base: f64, magnitude: f64, range: Range<f64>) {
    let delta = base * magnitude;
    if *val + delta > range.end {
        *val = range.end
    } else if *val + delta < range.start {
        *val = range.start
    } else {
        *val += delta;
    }
}

pub fn update_value_i(val: &mut u32, inc: bool, base: u32, magnitude: f64, range: Range<u32>) {
    let delta = (base as f64 * magnitude) as u32;
    if inc {
        if range.end - delta < *val {
            *val = range.end
        } else {
            *val += delta
        }
    } else if range.start + delta > *val {
        *val = range.start
    } else {
        *val -= delta
    }
}

fn make_header<'a>(
    cfg: &GraphConfig,
    module_header: &'a str,
    kind_o_scope: &'static str,
    fps: usize,
    pause: bool,
) -> Table<'a> {
    Table::new(
        vec![Row::new(vec![
            Cell::from(format!("{}::scope-tui", kind_o_scope)).style(
                Style::default()
                    .fg(*cfg.palette.first().expect("empty palette?"))
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(module_header),
            Cell::from(format!("-{:.2}x+", cfg.scale)),
            Cell::from(format!("{}/{} spf", cfg.samples, cfg.width)),
            Cell::from(format!("{}fps", fps)),
            Cell::from(if cfg.scatter { "***" } else { "---" }),
            Cell::from(if pause { "||" } else { "|>" }),
        ])],
        vec![
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(7),
            Constraint::Percentage(13),
            Constraint::Percentage(6),
            Constraint::Percentage(6),
            Constraint::Percentage(6),
        ],
    )
    .style(Style::default().fg(cfg.labels_color))
}

/// Play music.
fn play(
    graph: &mut GraphConfig,
    os_media_controls: Option<&mut OsMediaControls>,
    sink_cmd_tx: &UnboundedSender<SinkCommand>,
) {
    graph.pause = false;
    send_os_media_controls_command(os_media_controls, os_media_controls::Command::Play);
    sink_cmd_tx
        .send(SinkCommand::Play)
        .expect("receiver never dropped");
}

/// Pause music.
fn pause(
    graph: &mut GraphConfig,
    os_media_controls: Option<&mut OsMediaControls>,
    sink_cmd_tx: &UnboundedSender<SinkCommand>,
) {
    graph.pause = true;
    send_os_media_controls_command(os_media_controls, os_media_controls::Command::Pause);
    sink_cmd_tx
        .send(SinkCommand::Pause)
        .expect("receiver never dropped");
}

/// Toggle between play and pause.
fn toggle_play_pause(
    graph: &mut GraphConfig,
    os_media_controls: Option<&mut OsMediaControls>,
    sink_cmd_tx: &UnboundedSender<SinkCommand>,
) {
    graph.pause = !graph.pause;
    let (sink_cmd, os_media_controls_command) = if graph.pause {
        (SinkCommand::Pause, os_media_controls::Command::Pause)
    } else {
        (SinkCommand::Play, os_media_controls::Command::Play)
    };
    send_os_media_controls_command(os_media_controls, os_media_controls_command);
    sink_cmd_tx.send(sink_cmd).expect("receiver never dropped");
}

/// Lower the volume.
fn lower_volume(
    state: &Mutex<State>,
    os_media_controls: Option<&mut OsMediaControls>,
    sink_cmd_tx: &UnboundedSender<SinkCommand>,
) {
    let mut state = state.lock().unwrap();
    state.volume.change_volume(-1.0);
    send_os_media_controls_command(
        os_media_controls,
        os_media_controls::Command::SetVolume(state.volume.volume_ratio() as f64),
    );
    sink_cmd_tx
        .send(SinkCommand::SetVolume(state.volume.volume_ratio()))
        .expect("receiver never dropped");
}

/// Raise the volume.
fn raise_volume(
    state: &Mutex<State>,
    os_media_controls: Option<&mut OsMediaControls>,
    sink_cmd_tx: &UnboundedSender<SinkCommand>,
) {
    let mut state = state.lock().unwrap();
    state.volume.change_volume(1.0);
    send_os_media_controls_command(
        os_media_controls,
        os_media_controls::Command::SetVolume(state.volume.volume_ratio() as f64),
    );
    sink_cmd_tx
        .send(SinkCommand::SetVolume(state.volume.volume_ratio()))
        .expect("receiver never dropped");
}

/// Mute the volume.
fn mute_volume(
    state: &Mutex<State>,
    os_media_controls: Option<&mut OsMediaControls>,
    sink_cmd_tx: &UnboundedSender<SinkCommand>,
) {
    let mut state = state.lock().unwrap();
    state.volume.toggle_mute();
    send_os_media_controls_command(
        os_media_controls,
        os_media_controls::Command::SetVolume(state.volume.volume_ratio() as f64),
    );
    sink_cmd_tx
        .send(SinkCommand::SetVolume(state.volume.volume_ratio()))
        .expect("receiver never dropped");
}

/// Set the volume to the given volume ratio.
fn set_volume_ratio(
    volume_ratio: f32,
    state: &Mutex<State>,
    os_media_controls: Option<&mut OsMediaControls>,
    sink_cmd_tx: &UnboundedSender<SinkCommand>,
) {
    let mut state = state.lock().unwrap();
    state.volume.set_volume_ratio(volume_ratio);
    send_os_media_controls_command(
        os_media_controls,
        os_media_controls::Command::SetVolume(state.volume.volume_ratio() as f64),
    );
    sink_cmd_tx
        .send(SinkCommand::SetVolume(state.volume.volume_ratio()))
        .expect("receiver never dropped");
}

/// Send [`os_media_controls::Command`].
fn send_os_media_controls_command(
    os_media_controls: Option<&mut OsMediaControls>,
    command: os_media_controls::Command<'_>,
) {
    if let Some(os_media_controls) = os_media_controls {
        let _ = os_media_controls.send_to_os(command).inspect_err(|err| {
            eprintln!(
                "error: failed to send command to OS media controls due to `{}`",
                err
            );
        });
    }
}
