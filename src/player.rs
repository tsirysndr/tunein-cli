use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    thread,
    time::Duration,
};

use anyhow::Error;
use futures_util::Future;
use reqwest::blocking::Response;
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
    sink: Arc<Mutex<Sink>>,
    stream: OutputStream,
    handle: OutputStreamHandle,
    commands: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
    decoder: Option<Mp3Decoder<Response>>,
}

impl PlayerInternal {
    fn new(cmd_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>) -> Self {
        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        Self {
            sink: Arc::new(Mutex::new(rodio::Sink::try_new(&handle).unwrap())),
            stream,
            handle,
            commands: cmd_rx,
            decoder: None,
        }
    }

    fn handle_play(&mut self, url: String) -> Result<(), Error> {
        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        self.stream = stream;
        self.sink = Arc::new(Mutex::new(rodio::Sink::try_new(&handle).unwrap()));
        self.handle = handle;
        let sink = self.sink.clone();

        thread::spawn(move || {
            let (frame_tx, _frame_rx) = std::sync::mpsc::channel::<minimp3::Frame>();
            let client = reqwest::blocking::Client::new();

            let response = client.get(url.clone()).send().unwrap();

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
            let decoder = Mp3Decoder::new(response, Some(frame_tx)).unwrap();

            {
                let sink = sink.lock().unwrap();
                sink.append(decoder);
                sink.play();
            }

            loop {
                let sink = sink.lock().unwrap();

                if sink.empty() {
                    break;
                }

                drop(sink);

                std::thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(())
    }

    fn handle_play_or_pause(&self) -> Result<(), Error> {
        let sink = self.sink.lock().unwrap();
        match sink.is_paused() {
            true => sink.play(),
            false => sink.pause(),
        };
        Ok(())
    }

    fn handle_stop(&self) -> Result<(), Error> {
        let sink = self.sink.lock().unwrap();
        sink.stop();
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
