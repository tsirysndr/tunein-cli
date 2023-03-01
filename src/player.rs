use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    thread,
    time::Duration,
};

use anyhow::Error;
use futures_util::Future;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use tokio::sync::mpsc;

use crate::decoder::Mp3Decoder;

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
    stream: OutputStream,
    handle: OutputStreamHandle,
    commands: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
}

impl PlayerInternal {
    fn new(cmd_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>) -> Self {
        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        Self {
            sink: rodio::Sink::try_new(&handle).unwrap(),
            stream,
            handle,
            commands: cmd_rx,
        }
    }

    fn handle_play(&mut self, url: String) -> Result<(), Error> {
        let client = reqwest::blocking::Client::new();

        let response = client.get(url).send().unwrap();

        println!("headers: {:#?}", response.headers());
        let location = response.headers().get("location");

        let response = match location {
            Some(location) => {
                let response = client.get(location.to_str().unwrap()).send().unwrap();
                let location = response.headers().get("location");
                match location {
                    Some(location) => client.get(location.to_str().unwrap()).send().unwrap(),
                    None => response,
                }
            }
            None => response,
        };

        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        self.stream = stream;
        self.sink = rodio::Sink::try_new(&handle).unwrap();
        self.handle = handle;
        let decoder = Mp3Decoder::new(response).unwrap();
        self.sink.append(decoder);
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

    pub fn handle_command(&mut self, cmd: PlayerCommand) -> Result<(), Error> {
        match cmd {
            PlayerCommand::Play(url) => self.handle_play(url),
            PlayerCommand::PlayOrPause => self.handle_play_or_pause(),
            PlayerCommand::Stop => self.handle_stop(),
        }
    }
}

impl Future for PlayerInternal {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
