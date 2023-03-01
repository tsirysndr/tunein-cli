use std::time::Duration;

use anyhow::Error;
use surf::{Client, Config, Url};

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
