use std::time::Duration;

use anyhow::Error;
use hyper_rustls::ConfigBuilderExt;
use surf::{Client, Config, Url};
use tunein::TuneInClient;

use crate::{decoder::Mp3Decoder, reader::BodyReader};

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

    /*
       let req = hyper::Request::builder()
           .method("GET")
           .uri(stream_url)
           .header("Icy-MetaData", "1")
           .header("Range", "bytes=0-")
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

    */

    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::new();

        let response = client
            .get(stream_url)
            .header("Icy-MetaData", "1")
            .send()
            .unwrap();

        // let res = client.request(req).await.unwrap();

        println!("headers: {:#?}", response.headers());
        let _metaint = response.headers().get("icy-metaint");
        let location = response.headers().get("location");

        let response = match location {
            Some(location) => {
                let response = client
                    .get(location.to_str().unwrap())
                    .header("Icy-MetaData", "1")
                    .send()
                    .unwrap();
                let location = response.headers().get("location");
                match location {
                    Some(location) => client
                        .get(location.to_str().unwrap())
                        .header("Icy-MetaData", "1")
                        .send()
                        .unwrap(),
                    None => response,
                }
            }
            None => response,
        };

        let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&handle).unwrap();
        let decoder = Mp3Decoder::new(response).unwrap();
        sink.append(decoder);
        sink.sleep_until_end();
    })
    .await?;

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
