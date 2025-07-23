use std::time::Duration;

use anyhow::Error;
use serde::Deserialize;
use surf::{Client, Config, Url};

#[derive(Deserialize)]
pub struct Header {
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Subtitle")]
    pub subtitle: String,
}

#[derive(Deserialize)]
pub struct NowPlaying {
    #[serde(rename = "Header")]
    pub header: Header,
}

pub async fn extract_stream_url(url: &str, playlist_type: Option<String>) -> Result<String, Error> {
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

pub async fn get_currently_playing(station: &str) -> Result<String, Error> {
    let client = Client::new();
    let url = format!(
        "https://feed.tunein.com/profiles/{}/nowPlaying?partnerId=RadioTime",
        station
    );
    let response: NowPlaying = client
        .get(Url::parse(&url)?)
        .recv_json()
        .await
        .map_err(|e| Error::msg(e.to_string()))?;

    Ok(response.header.subtitle)
}
