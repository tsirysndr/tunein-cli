use async_trait::async_trait;
use futures_util::{future::FusedFuture, Future, StreamExt};
use hyper_rustls::ConfigBuilderExt;
use log::{error, trace};
use parking_lot::Mutex;
use std::{
    cmp::min,
    collections::HashMap,
    fs::File,
    io::{Cursor, Read, Write},
    mem,
    pin::Pin,
    process::exit,
    sync::Arc,
    task::{Context, Poll},
    thread,
    time::Duration,
};
use symphonia::core::{
    errors::Error,
    io::{MediaSource, MediaSourceStream, ReadOnlySource},
    probe::Hint,
};
use tempfile::NamedTempFile;
use tokio::{
    runtime::{Handle, Runtime},
    sync::mpsc::{self, UnboundedReceiver},
};

use crate::playback::{
    audio_backend::Sink,
    convert::Converter,
    decoder::{symphonia_decoder::SymphoniaDecoder, AudioDecoder},
    dither::{mk_ditherer, TriangularDitherer},
    formatter,
};

const PRELOAD_NEXT_TRACK_BEFORE_END: u64 = 30000;

pub type PlayerResult = Result<(), Error>;

pub enum RepeatState {
    Off,
    One,
    All,
}

#[async_trait]
pub trait PlayerEngine: Send + Sync {
    fn load(&mut self, track_id: &str, format: &str, _start_playing: bool, _position_ms: u32);
    fn preload(&self, _track_id: &str);
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    fn seek(&self, position_ms: u32);
    async fn get_current_track(&self) -> Option<(usize, u32, bool)>;
    async fn wait_for_current_track(
        mut channel: UnboundedReceiver<PlayerEvent>,
    ) -> Option<(usize, u32, bool)>;
}

#[derive(Clone)]
pub struct Player {
    commands: Option<Arc<std::sync::Mutex<mpsc::UnboundedSender<PlayerCommand>>>>,
}

impl Player {
    pub fn new<F>(
        sink_builder: F,
        cmd_tx: Arc<std::sync::Mutex<mpsc::UnboundedSender<PlayerCommand>>>,
        cmd_rx: Arc<std::sync::Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
    ) -> (Player, PlayerEventChannel)
    where
        F: FnOnce() -> Box<dyn Sink> + Send + 'static,
    {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        thread::spawn(move || {
            let internal = PlayerInternal {
                commands: cmd_rx,
                load_handles: Arc::new(Mutex::new(HashMap::new())),
                sink: sink_builder(),
                state: PlayerState::Stopped,
                sink_status: SinkStatus::Closed,
                sink_event_callback: None,
                event_senders: [event_sender].to_vec(),
                position_ms: 0,
            };
            let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            runtime.block_on(internal);
        });
        (
            Player {
                commands: Some(cmd_tx),
            },
            event_receiver,
        )
    }

    fn command(&self, cmd: PlayerCommand) {
        if let Some(commands) = self.commands.as_ref() {
            if let Err(e) = commands.lock().unwrap().send(cmd) {
                error!("Player Commands Error: {}", e);
            }
        }
    }

    pub fn get_player_event_channel(&self) -> PlayerEventChannel {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        self.command(PlayerCommand::AddEventSender(event_sender));
        event_receiver
    }

    pub async fn await_end_of_track(&self) {
        let mut channel = self.get_player_event_channel();
        while let Some(event) = channel.recv().await {
            if matches!(
                event,
                PlayerEvent::EndOfTrack { .. } | PlayerEvent::Stopped { .. }
            ) {
                return;
            }
        }
    }

    pub async fn await_end_of_tracklist(&self) {
        let mut channel = self.get_player_event_channel();
        while let Some(event) = channel.recv().await {
            if matches!(event, PlayerEvent::EndOfTrack { .. })
                && event.get_is_last_track().unwrap_or(false)
            {
                return;
            }
        }
    }
}

#[async_trait]
impl PlayerEngine for Player {
    fn load(&mut self, stream_url: &str, format: &str, _start_playing: bool, _position_ms: u32) {
        self.command(PlayerCommand::Load {
            stream_url: stream_url.to_string(),
            format: format.to_string(),
        });
    }

    fn preload(&self, _track_id: &str) {
        self.command(PlayerCommand::Preload);
    }

    fn play(&self) {
        self.command(PlayerCommand::Play)
    }

    fn pause(&self) {
        self.command(PlayerCommand::Pause)
    }

    fn stop(&self) {
        self.command(PlayerCommand::Stop)
    }

    fn seek(&self, position_ms: u32) {
        self.command(PlayerCommand::Seek(position_ms));
    }

    async fn get_current_track(&self) -> Option<(usize, u32, bool)> {
        let channel = self.get_player_event_channel();
        let handle = thread::spawn(move || {
            Runtime::new()
                .unwrap()
                .block_on(Self::wait_for_current_track(channel))
        });
        self.command(PlayerCommand::GetCurrentTrack);
        handle.join().unwrap()
    }

    async fn wait_for_current_track(
        mut channel: UnboundedReceiver<PlayerEvent>,
    ) -> Option<(usize, u32, bool)> {
        while let Some(event) = channel.recv().await {
            if matches!(event, PlayerEvent::CurrentTrack { .. }) {
                return event.get_current_track();
            }
        }
        None
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SinkStatus {
    Running,
    Closed,
    TemporarilyClosed,
}

pub type SinkEventCallback = Box<dyn Fn(SinkStatus) + Send>;

struct PlayerInternal {
    commands: Arc<std::sync::Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
    load_handles: Arc<Mutex<HashMap<thread::ThreadId, thread::JoinHandle<()>>>>,

    state: PlayerState,
    sink: Box<dyn Sink>,
    sink_status: SinkStatus,
    sink_event_callback: Option<SinkEventCallback>,
    event_senders: Vec<mpsc::UnboundedSender<PlayerEvent>>,
    position_ms: u32,
}

impl Future for PlayerInternal {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            // process commands that were sent to us
            let cmd = match self.commands.lock().unwrap().poll_recv(cx) {
                Poll::Ready(None) => return Poll::Ready(()), // client has disconnected - shut down.
                Poll::Ready(Some(cmd)) => Some(cmd),
                _ => None,
            };

            if let Some(cmd) = cmd {
                if let Err(e) = self.handle_command(cmd) {
                    // error!("Error handling command: {}", e);
                }
            }

            if let PlayerState::Playing { ref mut decoder } = self.state {
                match decoder.next_packet() {
                    Ok(result) => {
                        if let Some((ref packet_position, packet, channels, sample_rate)) = result {
                            match packet.samples() {
                                Ok(_) => {
                                    let mut converter =
                                        Converter::new(Some(mk_ditherer::<TriangularDitherer>));
                                    if let Err(e) = self.sink.write(
                                        packet,
                                        channels,
                                        sample_rate,
                                        &mut converter,
                                    ) {
                                        error!("Error writing to sink: {}", e);
                                        exit(1)
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to decode packet: {}", e);
                                }
                            }
                            self.position_ms = packet_position.position_ms;
                        } else {
                            // end of track
                            self.state = PlayerState::Stopped;
                            self.send_event(PlayerEvent::EndOfTrack {
                                is_last_track: true,
                            });
                        }
                    }
                    Err(e) => {
                        error!("Failed to decode packet: {}", e);
                    }
                };
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl PlayerInternal {
    fn ensure_sink_running(&mut self) {
        if self.sink_status != SinkStatus::Running {
            trace!("== Starting sink ==");
            if let Some(callback) = &mut self.sink_event_callback {
                callback(SinkStatus::Running);
            }
            match self.sink.start() {
                Ok(()) => self.sink_status = SinkStatus::Running,
                Err(e) => {
                    error!("{}", e);
                    exit(1);
                }
            }
        }
    }

    fn ensure_sink_stopped(&mut self, temporarily: bool) {
        match self.sink_status {
            SinkStatus::Running => {
                trace!("== Stopping sink ==");
                match self.sink.stop() {
                    Ok(()) => {
                        self.sink_status = if temporarily {
                            SinkStatus::TemporarilyClosed
                        } else {
                            SinkStatus::Closed
                        };
                        if let Some(callback) = &mut self.sink_event_callback {
                            callback(self.sink_status);
                        }
                    }
                    Err(e) => {
                        error!("{}", e);
                        exit(1);
                    }
                }
            }
            SinkStatus::TemporarilyClosed => {
                if !temporarily {
                    self.sink_status = SinkStatus::Closed;
                    if let Some(callback) = &mut self.sink_event_callback {
                        callback(SinkStatus::Closed);
                    }
                }
            }
            SinkStatus::Closed => (),
        }
    }

    fn handle_command(&mut self, cmd: PlayerCommand) -> PlayerResult {
        match cmd {
            PlayerCommand::Load { stream_url, format } => {
                self.handle_command_load(&stream_url, &format)
            }
            PlayerCommand::Preload => self.handle_command_preload(),
            PlayerCommand::Play => self.handle_play(),
            PlayerCommand::Pause => self.handle_pause(),
            PlayerCommand::Stop => self.handle_player_stop(),
            PlayerCommand::Seek(position_ms) => self.handle_command_seek(),
            PlayerCommand::AddEventSender(sender) => self.event_senders.push(sender),
            PlayerCommand::GetCurrentTrack => self.handle_get_current_track(),
        }
        Ok(())
    }

    fn load_track(&self, song: &str, ext: &str) -> Option<PlayerLoadedTrackData> {
        let handle = Handle::current();
        let song = song.to_string();
        let ext = ext.to_string();
        match thread::spawn(move || handle.block_on(PlayerTrackLoader::load(&song, &ext))).join() {
            Ok(track) => {
                println!("Loaded track");
                return track;
            }
            Err(_) => {
                println!("Failed to load track");
                None
            }
        }
    }

    fn start_playback(&mut self, _track_id: &str, loaded_track: PlayerLoadedTrackData) {
        self.ensure_sink_running();
        self.send_event(PlayerEvent::Playing {});

        self.state = PlayerState::Playing {
            decoder: loaded_track.decoder,
        };
    }

    fn send_event(&mut self, event: PlayerEvent) {
        self.event_senders
            .retain(|sender| sender.send(event.clone()).is_ok());
    }

    fn handle_command_load(&mut self, track_id: &str, ext: &str) {
        formatter::print_format(track_id);
        let loaded_track = self.load_track(track_id, ext);
        match loaded_track {
            Some(loaded_track) => {
                self.start_playback(track_id, loaded_track);
            }
            None => {
                self.send_event(PlayerEvent::Error {
                    track_id: track_id.to_string(),
                    error: "Failed to load track".to_string(),
                });
            }
        }
    }

    fn handle_command_preload(&self) {
        todo!()
    }

    fn handle_play(&mut self) {
        if let PlayerState::Paused { .. } = self.state {
            self.state.paused_to_playing();
            self.send_event(PlayerEvent::Playing);
            self.ensure_sink_running();
        } else {
            error!("Player::play called from invalid state");
        }
    }

    fn handle_player_stop(&mut self) {
        self.ensure_sink_stopped(false);
        self.state = PlayerState::Stopped;
    }

    fn handle_pause(&mut self) {
        if let PlayerState::Playing { .. } = self.state {
            self.state.playing_to_paused();
            self.send_event(PlayerEvent::Paused);
        } else {
            error!("Player::pause called from invalid state");
        }
    }

    fn handle_command_seek(&self) {
        todo!()
    }

    fn handle_get_current_track(&mut self) {
        let is_playing = self.state.is_playing();
        self.send_event(PlayerEvent::CurrentTrack {
            position: 0,
            position_ms: self.position_ms,
            is_playing,
        });
    }
}

struct PlayerLoadedTrackData {
    decoder: Decoder,
}

type Decoder = Box<dyn AudioDecoder + Send>;

enum PlayerState {
    Stopped,
    Loading {
        loader: Pin<Box<dyn FusedFuture<Output = Result<PlayerLoadedTrackData, ()>> + Send>>,
    },
    Paused {
        decoder: Decoder,
    },
    Playing {
        decoder: Decoder,
    },
    EndOfTrack {
        loaded_track: PlayerLoadedTrackData,
    },
    Invalid,
}

impl PlayerState {
    fn is_playing(&self) -> bool {
        use self::PlayerState::*;
        match *self {
            Stopped | EndOfTrack { .. } | Paused { .. } | Loading { .. } => false,
            Playing { .. } => true,
            Invalid => {
                // "PlayerState::is_playing in invalid state"
                exit(1);
            }
        }
    }

    #[allow(dead_code)]
    fn is_stopped(&self) -> bool {
        use self::PlayerState::*;
        matches!(self, Stopped)
    }

    #[allow(dead_code)]
    fn is_loading(&self) -> bool {
        use self::PlayerState::*;
        matches!(self, Loading { .. })
    }

    fn decoder(&mut self) -> Option<&mut Decoder> {
        use self::PlayerState::*;
        match *self {
            Stopped | EndOfTrack { .. } | Loading { .. } => None,
            Paused {
                ref mut decoder, ..
            }
            | Playing {
                ref mut decoder, ..
            } => Some(decoder),
            Invalid => {
                // error!("PlayerState::decoder in invalid state");
                exit(1);
            }
        }
    }

    fn playing_to_paused(&mut self) {
        use self::PlayerState::*;
        let new_state = mem::replace(self, Invalid);
        match new_state {
            Playing { decoder } => {
                *self = Paused { decoder };
            }
            _ => {
                error!("PlayerState::playing_to_paused in invalid state");
                exit(1);
            }
        }
    }

    fn paused_to_playing(&mut self) {
        use self::PlayerState::*;
        let new_state = mem::replace(self, Invalid);
        match new_state {
            Paused { decoder } => {
                *self = Playing { decoder };
            }
            _ => {
                error!("PlayerState::paused_to_playing in invalid state");
                exit(1);
            }
        }
    }
}

pub struct PlayerTrackLoader;

impl PlayerTrackLoader {
    async fn load(song: &str, ext: &str) -> Option<PlayerLoadedTrackData> {
        let symphonia_decoder = |audio_file, format| {
            SymphoniaDecoder::new(audio_file, format).map(|decoder| Box::new(decoder) as Decoder)
        };

        let mut format = Hint::new();
        format.with_extension(ext);

        let req = hyper::Request::builder()
            .method("GET")
            .uri(song)
            .header("Icy-MetaData", "1")
            .body(hyper::Body::empty())
            .unwrap();

        let tls = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_native_roots()
            .with_no_client_auth();
        // Prepare the HTTPS connector
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls)
            .https_or_http()
            .enable_http1()
            .build();

        let client = hyper::client::Client::builder().build(https);
        let res = client.request(req).await.unwrap();
        let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<usize>();
        const MINIMUM_DOWNLOAD_SIZE: usize = 64 * 1024;

        let mut file = NamedTempFile::new().unwrap();
        let file_path = file.path().to_str().unwrap().to_string();

        tokio::spawn(async move {
            let mut downloaded_size = 0;
            let mut ready = false;
            println!("headers: {:#?}", res.headers());
            let metaint = res.headers().get("icy-metaint");
            let location = res.headers().get("location");

            let mut data = vec![];
            let metaint = match metaint {
                Some(metaint) => metaint.to_str().unwrap().parse::<usize>().unwrap(),
                None => 0,
            };

            let body = match location {
                Some(location) => {
                    let req = hyper::Request::builder()
                        .method("GET")
                        .uri(location.to_str().unwrap())
                        .header("Icy-MetaData", "1")
                        .body(hyper::Body::empty())
                        .unwrap();
                    client.request(req).await.unwrap().into_body()
                }
                None => res.into_body(),
            };

            body.for_each(|chunk| {
                let chunk = chunk.unwrap();
                data.extend_from_slice(&chunk);
                // parse metadata
                if metaint != 0 && data.len() >= (metaint + 128) {
                    let metadata = &data[1..metaint + 128];
                    let metadata = &metadata[metaint..];
                    let metadata = String::from_utf8_lossy(metadata);
                    // parse StreamTitle
                    let stream_title = metadata
                        .split(";")
                        .filter(|s| s.starts_with("StreamTitle"))
                        .next()
                        .unwrap();
                    let stream_title = stream_title
                        .split("=")
                        .filter(|s| !s.starts_with("StreamTitle"))
                        .next()
                        .unwrap();
                    let stream_title = stream_title.replace("'", "");
                    println!("metadata: {}", stream_title);
                }

                file.write_all(&chunk).unwrap();
                downloaded_size += chunk.len();
                if downloaded_size > MINIMUM_DOWNLOAD_SIZE && !ready {
                    stream_tx.send(downloaded_size).unwrap();
                    ready = true;
                }
                async move {}
            })
            .await;
        });

        println!("waiting for download to complete ...");
        stream_rx.recv().await;
        println!("download complete ...");

        println!("temporary file_path: {}", file_path);

        let reader = Box::new(File::open(file_path).unwrap());

        let decoder_type = symphonia_decoder(
            MediaSourceStream::new(
                Box::new(ReadOnlySource::new(reader)) as Box<dyn MediaSource>,
                Default::default(),
            ),
            format,
        );

        let decoder = match decoder_type {
            Ok(decoder) => decoder,
            Err(e) => {
                panic!("Failed to create decoder: {}", e);
            }
        };

        println!(">> loaded ...");

        return Some(PlayerLoadedTrackData { decoder });
    }
}

#[derive(Debug)]
pub enum PlayerCommand {
    Load { stream_url: String, format: String },
    Preload,
    Play,
    Pause,
    Stop,
    Seek(u32),
    AddEventSender(mpsc::UnboundedSender<PlayerEvent>),
    GetCurrentTrack,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Stopped,
    Started,
    Loading,
    Preloading,
    Playing,
    Paused,
    TimeToPreloadNextTrack,
    EndOfTrack {
        is_last_track: bool,
    },
    VolumeSet {
        volume: u16,
    },
    Error {
        track_id: String,
        error: String,
    },
    CurrentTrack {
        position: usize,
        position_ms: u32,
        is_playing: bool,
    },
    TrackTimePosition {
        position_ms: u32,
    },
}

impl PlayerEvent {
    pub fn get_is_last_track(&self) -> Option<bool> {
        use PlayerEvent::*;
        match self {
            EndOfTrack { is_last_track, .. } => Some(*is_last_track),
            _ => None,
        }
    }

    pub fn get_current_track(&self) -> Option<(usize, u32, bool)> {
        use PlayerEvent::*;
        match self {
            CurrentTrack {
                position,
                position_ms,
                is_playing,
            } => Some((position.clone(), position_ms.clone(), is_playing.clone())),
            _ => None,
        }
    }
}

pub type PlayerEventChannel = mpsc::UnboundedReceiver<PlayerEvent>;
