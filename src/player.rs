use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    thread,
    time::Duration,
};

use anyhow::Error;
use futures_util::Future;
use rodio::Sink;
use tokio::sync::mpsc;

pub struct Player;

impl Player {
    pub fn new(cmd_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>) -> Self {
        thread::spawn(move || {
            let internal = PlayerInternal::new(cmd_rx);
            futures::executor::block_on(internal);
        });
        Self {}
    }
}

#[derive(Debug)]
pub enum PlayerCommand {
    Play(String),
    PlayOrPause,
    Stop,
}

struct PlayerInternal {
    sink: Sink,
    commands: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
}

impl PlayerInternal {
    fn new(cmd_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>) -> Self {
        let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&handle).unwrap();
        Self {
            sink,
            commands: cmd_rx,
        }
    }

    fn handle_play(&self, url: String) -> Result<(), Error> {
        self.sink.stop();
        let source = rodio::Decoder::new(std::io::Cursor::new(
            reqwest::blocking::get(&url).unwrap().bytes().unwrap(),
        ))
        .unwrap();
        self.sink.append(source);
        self.sink.play();
        Ok(())
    }

    fn handle_play_or_pause(&self) -> Result<(), Error> {
        match self.sink.is_paused() {
            true => self.sink.play(),
            false => self.sink.pause(),
        };
        Ok(())
    }

    fn handle_stop(&self) -> Result<(), Error> {
        self.sink.stop();
        Ok(())
    }

    pub fn handle_command(&self, cmd: PlayerCommand) -> Result<(), Error> {
        match cmd {
            PlayerCommand::Play(url) => self.handle_play(url),
            PlayerCommand::PlayOrPause => self.handle_play_or_pause(),
            PlayerCommand::Stop => self.handle_stop(),
        }
    }
}

impl Future for PlayerInternal {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            // Process commands that have been sent to the player
            let cmd = match self.commands.lock().unwrap().poll_recv(cx) {
                Poll::Ready(None) => return Poll::Ready(()), // client has disconnected - shut down.
                Poll::Ready(Some(cmd)) => Some(cmd),
                _ => None,
            };

            if let Some(cmd) = cmd {
                if let Err(e) = self.handle_command(cmd) {
                    println!("{:?}", e);
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    }
}
