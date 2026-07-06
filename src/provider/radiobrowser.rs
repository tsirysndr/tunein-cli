use crate::types::Station;

use super::{is_valid_uuid, Provider};
use anyhow::Error;
use async_trait::async_trait;
use radiobrowser::{ApiStation, ApiTag, RadioBrowserAPI};
use std::process::exit;

pub struct Radiobrowser {
    client: reqwest::Client,
    base_url: String,
}

impl Radiobrowser {
    pub async fn new() -> Self {
        let servers = match RadioBrowserAPI::get_default_servers().await {
            Ok(servers) if !servers.is_empty() => servers,
            _ => {
                eprintln!("Failed to create a RadioBrowserAPI client");
                exit(1);
            }
        };

        Self::with_base_url(format!("https://{}", servers[0]))
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("tunein-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("Failed to create an HTTP client");
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    async fn search_stations(&self, query: &[(&str, &str)]) -> Result<Vec<ApiStation>, Error> {
        let stations = self
            .client
            .get(format!("{}/json/stations/search", self.base_url))
            .query(query)
            .send()
            .await?
            .json::<Vec<ApiStation>>()
            .await?;
        Ok(stations)
    }
}

#[async_trait]
impl Provider for Radiobrowser {
    async fn search(&self, name: String) -> Result<Vec<Station>, Error> {
        let stations = self.search_stations(&[("name", name.as_str())]).await?;
        Ok(stations.into_iter().map(Station::from).collect())
    }

    async fn get_station(&self, name_or_uuid: String) -> Result<Option<Station>, Error> {
        let stations = match is_valid_uuid(&name_or_uuid) {
            true => {
                self.client
                    .get(format!(
                        "{}/json/stations/byuuid/{}",
                        self.base_url, name_or_uuid
                    ))
                    .send()
                    .await?
                    .json::<Vec<ApiStation>>()
                    .await?
            }
            false => {
                self.search_stations(&[("name", name_or_uuid.as_str()), ("nameExact", "true")])
                    .await?
            }
        };

        Ok(stations.into_iter().next().map(Station::from))
    }

    async fn browse(
        &self,
        category: String,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Station>, Error> {
        let offset = offset.to_string();
        let limit = limit.to_string();
        let stations = self
            .search_stations(&[
                ("tag", category.as_str()),
                ("offset", offset.as_str()),
                ("limit", limit.as_str()),
            ])
            .await?;
        Ok(stations.into_iter().map(Station::from).collect())
    }

    async fn categories(&self, offset: u32, limit: u32) -> Result<Vec<String>, Error> {
        let categories = self
            .client
            .get(format!("{}/json/tags", self.base_url))
            .query(&[("offset", offset), ("limit", limit)])
            .send()
            .await?
            .json::<Vec<ApiTag>>()
            .await?;
        Ok(categories.into_iter().map(|x| x.name).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server, ServerGuard};

    fn station_json(uuid: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "changeuuid": "610cafba-71d8-40fc-bf68-1456ec973b9d",
            "stationuuid": uuid,
            "name": name,
            "url": "https://example.com/stream",
            "url_resolved": "https://example.com/stream",
            "homepage": "https://example.com",
            "favicon": "https://example.com/favicon.ico",
            "tags": "music",
            "country": "The United States Of America",
            "countrycode": "US",
            "state": "",
            "language": "english",
            "votes": 10,
            "codec": "MP3",
            "bitrate": 128,
            "hls": 0,
            "lastcheckok": 1,
            "clickcount": 100,
            "clicktrend": 0
        })
    }

    async fn mock_provider() -> (ServerGuard, Radiobrowser) {
        let server = Server::new_async().await;
        let provider = Radiobrowser::with_base_url(server.url());
        (server, provider)
    }

    #[tokio::test]
    pub async fn test_search() {
        let (mut server, provider) = mock_provider().await;
        let name = "alternativeradio";
        let body = serde_json::json!([station_json(
            "964da563-0601-11e8-ae97-52543be04c81",
            "AlternativeRadio.us"
        )]);
        let mock = server
            .mock("GET", "/json/stations/search")
            .match_query(Matcher::UrlEncoded("name".into(), name.into()))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let stations = provider.search(name.to_string()).await.unwrap();

        mock.assert_async().await;
        assert!(stations.len() == 1);
        assert_eq!(stations[0].name, "AlternativeRadio.us");
    }

    #[tokio::test]
    pub async fn test_get_station() {
        let (mut server, provider) = mock_provider().await;
        let name = "AlternativeRadio.us";
        let body = serde_json::json!([station_json(
            "964da563-0601-11e8-ae97-52543be04c81",
            name
        )]);
        let mock = server
            .mock("GET", "/json/stations/search")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("name".into(), name.into()),
                Matcher::UrlEncoded("nameExact".into(), "true".into()),
            ]))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let station = provider.get_station(name.to_string()).await.unwrap();

        mock.assert_async().await;
        assert!(station.is_some());
    }

    #[tokio::test]
    pub async fn test_get_station_by_uuid() {
        let (mut server, provider) = mock_provider().await;
        let uuid = "964da563-0601-11e8-ae97-52543be04c81";
        let body = serde_json::json!([station_json(uuid, "AlternativeRadio.us")]);
        let mock = server
            .mock("GET", &format!("/json/stations/byuuid/{}", uuid)[..])
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let station = provider.get_station(uuid.to_string()).await.unwrap();

        mock.assert_async().await;
        assert!(station.is_some());
        assert_eq!(station.unwrap().id, uuid);
    }

    #[tokio::test]
    pub async fn test_browse() {
        let (mut server, provider) = mock_provider().await;
        let body = serde_json::Value::Array(
            (0..100)
                .map(|i| station_json(&format!("uuid-{}", i), &format!("Station {}", i)))
                .collect(),
        );
        let mock = server
            .mock("GET", "/json/stations/search")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("tag".into(), "music".into()),
                Matcher::UrlEncoded("offset".into(), "0".into()),
                Matcher::UrlEncoded("limit".into(), "100".into()),
            ]))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let stations = provider.browse("music".to_string(), 0, 100).await.unwrap();

        mock.assert_async().await;
        assert!(stations.len() == 100);
    }

    #[tokio::test]
    pub async fn test_categories() {
        let (mut server, provider) = mock_provider().await;
        let body = serde_json::json!([
            { "name": "music", "stationcount": 42 },
            { "name": "news", "stationcount": 7 }
        ]);
        let mock = server
            .mock("GET", "/json/tags")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("offset".into(), "0".into()),
                Matcher::UrlEncoded("limit".into(), "100".into()),
            ]))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let categories = provider.categories(0, 100).await.unwrap();

        mock.assert_async().await;
        assert!(categories.len() > 0);
        assert_eq!(categories, vec!["music", "news"]);
    }
}
