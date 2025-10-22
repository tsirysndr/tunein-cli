use std::{process, thread, time::Duration};

use anyhow::Error;
use hyper::header::HeaderValue;
use tunein_cli::os_media_controls::OsMediaControls;

use crate::{
    app::{App, CurrentDisplayMode, State, Volume},
    cfg::{SourceOptions, UiOptions},
    decoder::Mp3Decoder,
    provider::{radiobrowser::Radiobrowser, tunein::Tunein, Provider},
    tui,
};

pub async fn exec(
    name_or_id: &str,
    provider: &str,
    volume: f32,
    display_mode: CurrentDisplayMode,
    enable_os_media_controls: bool,
    poll_events_every: Duration,
    poll_events_every_while_paused: Duration,
) -> Result<(), Error> {
    let _provider = provider;
    let provider: Box<dyn Provider> = match provider {
        "tunein" => Box::new(Tunein::new()),
        "radiobrowser" => Box::new(Radiobrowser::new().await),
        _ => {
            return Err(anyhow::anyhow!(format!(
                "Unsupported provider '{}'",
                provider
            )))
        }
    };
    let station = provider.get_station(name_or_id.to_string()).await?;
    if station.is_none() {
        return Err(Error::msg("No station found"));
    }

    let station = station.unwrap();
    let stream_url = station.stream_url.clone();
    let id = station.id.clone();
    let now_playing = station.playing.clone().unwrap_or_default();

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<State>();
    let (sink_cmd_tx, mut sink_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<SinkCommand>();
    let (frame_tx, frame_rx) = std::sync::mpsc::channel::<minimp3::Frame>();

    let ui = UiOptions {
        scale: 1.0,
        scatter: false,
        no_reference: true,
        no_ui: true,
        no_braille: false,
    };

    let opts = SourceOptions {
        channels: 2,
        buffer: 1152,
        sample_rate: 44100,
        tune: None,
    };

    let os_media_controls = if enable_os_media_controls {
        OsMediaControls::new()
            .inspect_err(|err| {
                eprintln!(
                    "error: failed to initialize os media controls due to `{}`",
                    err
                );
            })
            .ok()
    } else {
        None
    };

    let mut app = App::new(
        &ui,
        &opts,
        frame_rx,
        display_mode,
        os_media_controls,
        poll_events_every,
        poll_events_every_while_paused,
    );
    let station_name = station.name.clone();

    thread::spawn(move || {
        let client = reqwest::blocking::Client::new();

        let response = client.get(stream_url).send().unwrap();

        let headers = response.headers();
        let volume = Volume::new(volume, false);

        cmd_tx
            .send(State {
                name: match headers
                    .get("icy-name")
                    .unwrap_or(&HeaderValue::from_static("Unknown"))
                    .to_str()
                    .unwrap()
                {
                    "Unknown" => station_name,
                    name => name.to_string(),
                },
                now_playing,
                genre: headers
                    .get("icy-genre")
                    .unwrap_or(&HeaderValue::from_static("Unknown"))
                    .to_str()
                    .unwrap()
                    .to_string(),
                description: headers
                    .get("icy-description")
                    .unwrap_or(&HeaderValue::from_static("Unknown"))
                    .to_str()
                    .unwrap()
                    .to_string(),
                br: headers
                    .get("icy-br")
                    .unwrap_or(&HeaderValue::from_static(""))
                    .to_str()
                    .unwrap()
                    .to_string(),
                volume: volume.clone(),
            })
            .unwrap();
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

        let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&handle).unwrap();
        sink.set_volume(volume.volume_ratio());
        let decoder = Mp3Decoder::new(response, Some(frame_tx)).unwrap();
        sink.append(decoder);

        loop {
            while let Ok(sink_cmd) = sink_cmd_rx.try_recv() {
                match sink_cmd {
                    SinkCommand::Play => {
                        sink.play();
                    }
                    SinkCommand::Pause => {
                        sink.pause();
                    }
                    SinkCommand::SetVolume(volume) => {
                        sink.set_volume(volume);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    let mut terminal = tui::init()?;
    app.run(&mut terminal, cmd_rx, sink_cmd_tx, &id).await;
    tui::restore()?;

    process::exit(0);
}

/// Command for a sink.
#[derive(Debug, Clone, PartialEq)]
pub enum SinkCommand {
    /// Play.
    Play,
    /// Pause.
    Pause,
    /// Set the volume.
    SetVolume(f32),
}
