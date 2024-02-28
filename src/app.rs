use std::{
    io,
    sync::{Arc, Mutex},
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{block::*, *},
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::tui;

#[derive(Debug, Default, Clone)]
pub struct State {
    pub name: String,
    pub now_playing: String,
    pub genre: String,
    pub description: String,
    pub br: String,
}

impl State {
    pub fn update(&mut self, state: State) {
        *self = state;
    }
}

#[derive(Debug, Default)]
pub struct App {
    pub state: Arc<Mutex<State>>,
    exit: bool,
}

impl App {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        Self { state, exit: false }
    }
}

impl App {
    /// runs the application's main loop until the user quits
    pub async fn run(
        &mut self,
        terminal: &mut tui::Tui,
        mut cmd_rx: UnboundedReceiver<State>,
    ) -> anyhow::Result<()> {
        let new_state = cmd_rx.recv().await.unwrap();
        self.state.lock().unwrap().update(new_state);
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        let areas = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ],
        )
        .split(frame.size());

        frame.render_widget(
            Block::new()
                .borders(Borders::TOP)
                .title(" TuneIn CLI ")
                .title_alignment(Alignment::Center),
            areas[0],
        );

        let state = self.state.lock().unwrap();

        self.render_line("Station ", &state.name, areas[1], frame);
        self.render_line("Now Playing ", &state.now_playing, areas[2], frame);
        self.render_line("Genre ", &state.genre, areas[3], frame);
        self.render_line("Description ", &state.description, areas[4], frame);
        self.render_line("Bitrate ", &format!("{} kbps", &state.br), areas[5], frame);
    }

    fn render_line(&self, label: &str, value: &str, area: Rect, frame: &mut Frame) {
        let span1 = Span::styled(label, Style::new().fg(Color::LightBlue));
        let span2 = Span::raw(value);

        let line = Line::from(vec![span1, span2]);
        let text: Text = Text::from(vec![line]);

        frame.render_widget(Paragraph::new(text), area);
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
