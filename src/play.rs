use std::{sync::Arc, time::Duration};

use anyhow::Error;
use surf::{Client, Config, Url};
use tunein::TuneInClient;

use crate::playback::{
    audio_backend::{self, rodio::RodioSink},
    config::AudioFormat,
    player::{Player, PlayerCommand},
};

pub async fn exec(name_or_id: &str) -> Result<(), Error> {
    let client = TuneInClient::new();
    let results = client
        .get_station(name_or_id)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let (url, playlist_type, media_type) = match results.is_empty() {
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
                    )
                }
                None => ("".to_string(), None, "".to_string()),
            }
        }
        false => {
            let result = results.first().unwrap();
            (
                result.url.clone(),
                result.playlist_type.clone(),
                result.media_type.clone(),
            )
        }
    };
    let stream_url = extract_stream_url(&url, playlist_type).await?;
    println!("{}", stream_url);

    let audio_format = AudioFormat::default();
    let backend = audio_backend::find(Some(RodioSink::NAME.to_string())).unwrap();
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
    let cmd_tx = Arc::new(std::sync::Mutex::new(cmd_tx));
    let cmd_rx = Arc::new(std::sync::Mutex::new(cmd_rx));

    let (player, _) = Player::new(move || backend(None, audio_format), cmd_tx.clone(), cmd_rx);

    cmd_tx
        .lock()
        .unwrap()
        .send(PlayerCommand::Load {
            stream_url,
            format: media_type,
        })
        .unwrap();

    player.await_end_of_track().await;

    Ok(())
}

async fn extract_stream_url(url: &str, playlist_type: Option<String>) -> Result<String, Error> {
    match playlist_type {
        Some(playlist_type) => match playlist_type.as_str() {
            "pls" => {
                let client: Client = Config::new()
                    .set_timeout(Some(Duration::from_secs(5)))
                    .try_into()
                    .unwrap();
                let response = client
                    .get(Url::parse(url)?)
                    .recv_string()
                    .await
                    .map_err(|e| Error::msg(e.to_string()))?;

                let mut response = response.replace("[Playlist]", "[playlist]");
                if !response.contains("NumberOfEntries") {
                    response = format!("{}\nNumberOfEntries=1", response);
                }

                let url = pls::parse(&mut response.as_bytes())
                    .map_err(|e| Error::msg(e.to_string()))?
                    .first()
                    .map(|e| e.path.clone())
                    .unwrap();
                Ok(url.to_string())
            }
            _ => Err(Error::msg(format!(
                "Playlist type {} not supported",
                playlist_type
            ))),
        },
        None => Ok(url.to_string()),
    }
}
