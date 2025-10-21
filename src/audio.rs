use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Error};
use hyper::header::HeaderValue;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use tokio::sync::mpsc;

use crate::decoder::Mp3Decoder;
use crate::types::Station;

/// Commands sent to the audio worker thread.
#[derive(Debug)]
enum AudioCommand {
    Play {
        station: Station,
        volume_percent: f32,
    },
    SetVolume(f32),
    Stop,
}

/// Playback events emitted by the audio worker.
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    Started(PlaybackState),
    Error(String),
    Stopped,
}

/// Public interface for receiving playback events.
pub struct PlaybackEvents {
    rx: mpsc::UnboundedReceiver<PlaybackEvent>,
}

impl PlaybackEvents {
    pub async fn recv(&mut self) -> Option<PlaybackEvent> {
        self.rx.recv().await
    }
}

/// Snapshot of the current playback metadata.
#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub station: Station,
    pub stream_name: String,
    pub now_playing: String,
    pub genre: String,
    pub description: String,
    pub bitrate: String,
}

/// Controller that owns the command channel to the audio worker.
pub struct AudioController {
    cmd_tx: mpsc::UnboundedSender<AudioCommand>,
}

impl AudioController {
    /// Spawn a new audio worker thread and return a controller plus event receiver.
    pub fn new() -> Result<(Self, PlaybackEvents), Error> {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AudioCommand>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<PlaybackEvent>();

        thread::Builder::new()
            .name("tunein-audio-worker".into())
            .spawn({
                let events = event_tx.clone();
                move || {
                    let mut worker = AudioWorker::new(event_tx);
                    if let Err(err) = worker.run(&mut cmd_rx) {
                        let _ = events.send(PlaybackEvent::Error(err.to_string()));
                    }
                }
            })
            .context("failed to spawn audio worker thread")?;

        Ok((Self { cmd_tx }, PlaybackEvents { rx: event_rx }))
    }

    pub fn play(&self, station: Station, volume_percent: f32) -> Result<(), Error> {
        self.cmd_tx
            .send(AudioCommand::Play {
                station,
                volume_percent,
            })
            .map_err(|e| Error::msg(e.to_string()))
    }

    pub fn set_volume(&self, volume_percent: f32) -> Result<(), Error> {
        self.cmd_tx
            .send(AudioCommand::SetVolume(volume_percent))
            .map_err(|e| Error::msg(e.to_string()))
    }

    pub fn stop(&self) -> Result<(), Error> {
        self.cmd_tx
            .send(AudioCommand::Stop)
            .map_err(|e| Error::msg(e.to_string()))
    }
}

struct AudioWorker {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Arc<Sink>>,
    current_volume: f32,
    events: mpsc::UnboundedSender<PlaybackEvent>,
}

impl AudioWorker {
    fn new(events: mpsc::UnboundedSender<PlaybackEvent>) -> Self {
        let (stream, handle) =
            OutputStream::try_default().expect("failed to acquire default audio output device");
        Self {
            _stream: stream,
            handle,
            sink: None,
            current_volume: 100.0,
            events,
        }
    }

    fn run(&mut self, cmd_rx: &mut mpsc::UnboundedReceiver<AudioCommand>) -> Result<(), Error> {
        while let Some(cmd) = cmd_rx.blocking_recv() {
            match cmd {
                AudioCommand::Play {
                    station,
                    volume_percent,
                } => self.handle_play(station, volume_percent)?,
                AudioCommand::SetVolume(volume_percent) => {
                    self.current_volume = volume_percent.max(0.0);
                    if let Some(sink) = &self.sink {
                        sink.set_volume(self.current_volume / 100.0);
                    }
                }
                AudioCommand::Stop => {
                    if let Some(sink) = self.sink.take() {
                        sink.stop();
                    }
                    let _ = self.events.send(PlaybackEvent::Stopped);
                }
            }
        }

        Ok(())
    }

    fn handle_play(&mut self, station: Station, volume_percent: f32) -> Result<(), Error> {
        if let Some(sink) = self.sink.take() {
            sink.stop();
            thread::sleep(Duration::from_millis(50));
        }

        let stream_url = station.stream_url.clone();
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&stream_url)
            .send()
            .with_context(|| format!("failed to open stream {}", stream_url))?;

        let headers = response.headers().clone();
        let now_playing = station.playing.clone().unwrap_or_default();

        let display_name = header_to_string(headers.get("icy-name"))
            .filter(|name| name != "Unknown")
            .unwrap_or_else(|| station.name.clone());
        let genre = header_to_string(headers.get("icy-genre")).unwrap_or_default();
        let description = header_to_string(headers.get("icy-description")).unwrap_or_default();
        let bitrate = header_to_string(headers.get("icy-br")).unwrap_or_default();

        let response = follow_redirects(client, response)?;

        let sink = Arc::new(Sink::try_new(&self.handle)?);
        sink.set_volume(volume_percent.max(0.0) / 100.0);

        let decoder = Mp3Decoder::new(response, None).map_err(|_| {
            Error::msg("stream is not in MP3 format or failed to initialize decoder")
        })?;
        sink.append(decoder);
        sink.play();

        self.current_volume = volume_percent;
        self.sink = Some(sink.clone());

        let state = PlaybackState {
            station,
            stream_name: display_name,
            now_playing,
            genre,
            description,
            bitrate,
        };

        let _ = self.events.send(PlaybackEvent::Started(state));

        Ok(())
    }
}

fn follow_redirects(
    client: reqwest::blocking::Client,
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response, Error> {
    let mut current = response;
    for _ in 0..3 {
        if let Some(location) = current.headers().get("location") {
            let url = location
                .to_str()
                .map_err(|_| Error::msg("invalid redirect location header"))?;
            current = client.get(url).send()?;
        } else {
            return Ok(current);
        }
    }
    Ok(current)
}

fn header_to_string(value: Option<&HeaderValue>) -> Option<String> {
    value
        .and_then(|header| header.to_str().ok())
        .map(|s| s.to_string())
}
