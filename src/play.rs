use std::{thread, time::Duration};

use anyhow::Error;
use hyper::header::HeaderValue;
use tunein::TuneInClient;

use crate::{
    app::{App, State},
    cfg::{SourceOptions, UiOptions},
    decoder::Mp3Decoder,
    extract::{extract_stream_url, get_currently_playing},
    tui,
};

pub async fn exec(name_or_id: &str) -> Result<(), Error> {
    let client = TuneInClient::new();
    let results = client
        .get_station(name_or_id)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let (url, playlist_type, _, id) = match results.is_empty() {
        true => {
            let results = client
                .search(name_or_id)
                .await
                .map_err(|e| Error::msg(e.to_string()))?;
            match results.first() {
                Some(result) => {
                    if result.r#type != Some("audio".to_string()) {
                        return Err(Error::msg("No station found"));
                    }
                    let id = result.guide_id.as_ref().unwrap();
                    let station = client
                        .get_station(id)
                        .await
                        .map_err(|e| Error::msg(e.to_string()))?;
                    let station = station.first().unwrap();
                    (
                        station.url.clone(),
                        station.playlist_type.clone(),
                        station.media_type.clone(),
                        id.clone(),
                    )
                }
                None => ("".to_string(), None, "".to_string(), "".to_string()),
            }
        }
        false => {
            let result = results.first().unwrap();
            (
                result.url.clone(),
                result.playlist_type.clone(),
                result.media_type.clone(),
                name_or_id.to_string(),
            )
        }
    };
    let now_playing = get_currently_playing(&id).await?;
    let stream_url = extract_stream_url(&url, playlist_type).await?;
    println!("{}", stream_url);

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<State>();
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

    let mut app = App::new(&ui, &opts, frame_rx);

    thread::spawn(move || {
        let client = reqwest::blocking::Client::new();

        let response = client.get(stream_url).send().unwrap();

        let headers = response.headers();
        cmd_tx
            .send(State {
                name: headers
                    .get("icy-name")
                    .unwrap_or(&HeaderValue::from_static("Unknown"))
                    .to_str()
                    .unwrap()
                    .to_string(),
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
        let decoder = Mp3Decoder::new(response, frame_tx).unwrap();
        sink.append(decoder);

        loop {
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    let mut terminal = tui::init()?;
    app.run(&mut terminal, cmd_rx, &id).await;
    tui::restore()?;
    Ok(())
}
