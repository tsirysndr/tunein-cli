use crate::types::Station;

use super::Provider;
use anyhow::Error;
use async_trait::async_trait;
use tunein::types::{
    CategoriesResponse, Category, CategoryDetails, CategoryResponse, CategoryTrait, SearchResponse,
    SearchResult, StationLinkDetails, StationResponse,
};

pub struct Tunein {
    client: reqwest::Client,
    base_url: String,
}

impl Tunein {
    pub fn new() -> Self {
        Self::with_base_url(tunein::BASE_URL)
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

    async fn search_results(&self, query: &str) -> Result<Vec<SearchResult>, Error> {
        let response = self
            .client
            .get(format!("{}/Search.ashx", self.base_url))
            .query(&[("query", query), ("render", "json")])
            .send()
            .await?
            .json::<SearchResponse>()
            .await?;
        Ok(response.body)
    }

    async fn station_links(&self, id: &str) -> Result<Vec<StationLinkDetails>, Error> {
        let response = self
            .client
            .get(format!("{}/Tune.ashx", self.base_url))
            .query(&[("id", id), ("render", "json")])
            .send()
            .await?
            .json::<StationResponse>()
            .await?;
        Ok(response.body)
    }

    async fn browse_categories(
        &self,
        category: Option<Category>,
    ) -> Result<Vec<CategoryDetails>, Error> {
        let mut request = self.client.get(format!("{}/Browse.ashx", self.base_url));
        if let Some(category) = category {
            request = request.query(&[("c", category.to_id())]);
        }
        let response = request
            .query(&[("render", "json")])
            .send()
            .await?
            .json::<CategoriesResponse>()
            .await?;
        Ok(response.body)
    }

    async fn browse_stations(&self, id: &str) -> Result<Vec<tunein::types::Station>, Error> {
        let response = self
            .client
            .get(format!("{}/Browse.ashx", self.base_url))
            .query(&[("id", id), ("render", "json")])
            .send()
            .await?
            .json::<CategoryResponse>()
            .await?;
        Ok(response.body)
    }
}

#[async_trait]
impl Provider for Tunein {
    async fn search(&self, name: String) -> Result<Vec<Station>, Error> {
        let results = self.search_results(&name).await?;
        let stations = results.into_iter().map(|x| Station::from(x)).collect();
        Ok(stations)
    }

    async fn get_station(&self, id: String) -> Result<Option<Station>, Error> {
        let stations = self.station_links(&id).await?;
        match stations.len() {
            0 => {
                let results = self.search(id.clone()).await?;
                let station = results.first().cloned();
                match station {
                    Some(st) => {
                        let stations = self.station_links(&st.id).await?;
                        let mut station = Station::from(stations[0].clone());
                        station.id = st.id.clone();
                        station.name = st.name.clone();
                        station.playing = st.playing.clone();
                        return Ok(Some(station));
                    }
                    None => Ok(None),
                }
            }
            _ => {
                let mut station = Station::from(stations[0].clone());
                // Preserve the original station ID since StationLinkDetails doesn't contain it
                station.id = id;
                Ok(Some(station))
            }
        }
    }

    async fn browse(
        &self,
        category: String,
        _offset: u32,
        _limit: u32,
    ) -> Result<Vec<Station>, Error> {
        let guide_id = category.clone();
        let category = match category.to_lowercase().as_str() {
            "by location" => Some(Category::ByLocation),
            "by language" => Some(Category::ByLanguage),
            "sports" => Some(Category::Sports),
            "talk" => Some(Category::Talk),
            "music" => Some(Category::Music),
            "local radio" => Some(Category::LocalRadio),
            "podcasts" => Some(Category::Podcasts),
            _ => None,
        };

        if category.is_none() {
            let category_stations = self.browse_stations(&guide_id).await?;

            let mut stations = vec![];

            for st in category_stations {
                if let Some(children) = st.clone().children {
                    stations = [stations, vec![Box::new(st.clone())], children].concat();
                }
            }

            let stations = stations.into_iter().map(|x| Station::from(x)).collect();
            return Ok(stations);
        }

        let category_stations = self.browse_categories(category).await?;

        let stations = category_stations
            .clone()
            .into_iter()
            .map(|x| Station::from(x))
            .collect::<Vec<Station>>();

        let mut _stations = vec![];
        for st in category_stations {
            if let Some(children) = st.children {
                _stations = [_stations, children].concat();
            }
        }
        let _stations = _stations
            .into_iter()
            .map(|x| Station::from(x))
            .collect::<Vec<Station>>();

        Ok([stations, _stations].concat())
    }

    async fn categories(&self, _offset: u32, _limit: u32) -> Result<Vec<String>, Error> {
        let categories = self.browse_categories(None).await?;
        let categories = categories.into_iter().map(|x| x.text).collect();
        Ok(categories)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server, ServerGuard};

    fn search_result_json(guide_id: &str, text: &str) -> serde_json::Value {
        serde_json::json!({
            "element": "outline",
            "type": "audio",
            "text": text,
            "URL": format!("http://opml.radiotime.com/Tune.ashx?id={}", guide_id),
            "bitrate": "128",
            "reliability": "98",
            "guide_id": guide_id,
            "subtext": "Now Playing",
            "genre_id": "g52",
            "formats": "mp3",
            "item": "station",
            "image": "https://cdn-profiles.tunein.com/s288303/images/logoq.png"
        })
    }

    fn station_json(guide_id: &str, text: &str) -> serde_json::Value {
        serde_json::json!({
            "element": "outline",
            "type": "audio",
            "text": text,
            "URL": format!("http://opml.radiotime.com/Tune.ashx?id={}", guide_id),
            "bitrate": "128",
            "guide_id": guide_id,
            "formats": "mp3"
        })
    }

    async fn mock_provider() -> (ServerGuard, Tunein) {
        let server = Server::new_async().await;
        let provider = Tunein::with_base_url(server.url());
        (server, provider)
    }

    #[tokio::test]
    pub async fn test_search() {
        let (mut server, provider) = mock_provider().await;
        let name = "alternativeradio";
        let body = serde_json::json!({
            "head": { "status": "200", "title": "Search Results: alternativeradio" },
            "body": [search_result_json("s288303", "AlternativeRadio.us")]
        });
        let mock = server
            .mock("GET", "/Search.ashx")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("query".into(), name.into()),
                Matcher::UrlEncoded("render".into(), "json".into()),
            ]))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let stations = provider.search(name.to_string()).await.unwrap();

        mock.assert_async().await;
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].id, "s288303");
        assert_eq!(stations[0].name, "AlternativeRadio.us");
    }

    #[tokio::test]
    pub async fn test_get_station() {
        let (mut server, provider) = mock_provider().await;
        let id = "s288303";
        let body = serde_json::json!({
            "head": { "status": "200" },
            "body": [{
                "element": "audio",
                "bitrate": 128,
                "is_direct": false,
                "is_hls_advanced": "false",
                "live_seek_stream": "false",
                "media_type": "mp3",
                "player_height": 0,
                "player_width": 0,
                "position": 0,
                "reliability": 92,
                "url": "http://stream.example.com/stream.mp3"
            }]
        });
        let mock = server
            .mock("GET", "/Tune.ashx")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), id.into()),
                Matcher::UrlEncoded("render".into(), "json".into()),
            ]))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let station = provider.get_station(id.to_string()).await.unwrap();

        mock.assert_async().await;
        let station = station.unwrap();
        assert_eq!(station.id, id);
        assert_eq!(station.stream_url, "http://stream.example.com/stream.mp3");
        assert_eq!(station.codec, "MP3");
    }

    #[tokio::test]
    pub async fn test_browse() {
        let (mut server, provider) = mock_provider().await;
        let body = serde_json::json!({
            "head": { "status": "200", "title": "Music" },
            "body": [{
                "element": "outline",
                "text": "Top 40 & Pop Music",
                "key": "c57942",
                "guide_id": "c57942",
                "children": [search_result_json("s288303", "AlternativeRadio.us")]
            }]
        });
        let mock = server
            .mock("GET", "/Browse.ashx")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("c".into(), "music".into()),
                Matcher::UrlEncoded("render".into(), "json".into()),
            ]))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let stations = provider.browse("music".to_string(), 0, 100).await.unwrap();

        mock.assert_async().await;
        // the category itself plus its child station
        assert_eq!(stations.len(), 2);
        assert_eq!(stations[0].name, "Top 40 & Pop Music");
        assert_eq!(stations[1].name, "AlternativeRadio.us");
    }

    #[tokio::test]
    pub async fn test_browse_by_id() {
        let (mut server, provider) = mock_provider().await;
        let id = "c57942";
        let body = serde_json::json!({
            "head": { "status": "200", "title": "Top 40 & Pop Music" },
            "body": [{
                "element": "outline",
                "text": "Stations",
                "guide_id": id,
                "children": [station_json("s288303", "AlternativeRadio.us")]
            }]
        });
        let mock = server
            .mock("GET", "/Browse.ashx")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), id.into()),
                Matcher::UrlEncoded("render".into(), "json".into()),
            ]))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let stations = provider.browse(id.to_string(), 0, 100).await.unwrap();

        mock.assert_async().await;
        // the parent outline plus its child station
        assert_eq!(stations.len(), 2);
        assert_eq!(stations[1].id, "s288303");
        assert_eq!(stations[1].name, "AlternativeRadio.us");
    }

    #[tokio::test]
    pub async fn test_categories() {
        let (mut server, provider) = mock_provider().await;
        let body = serde_json::json!({
            "head": { "status": "200", "title": "Browse" },
            "body": [
                { "element": "outline", "text": "Local Radio", "key": "local" },
                { "element": "outline", "text": "Music", "key": "music" },
                { "element": "outline", "text": "Talk", "key": "talk" }
            ]
        });
        let mock = server
            .mock("GET", "/Browse.ashx")
            .match_query(Matcher::UrlEncoded("render".into(), "json".into()))
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let categories = provider.categories(0, 100).await.unwrap();

        mock.assert_async().await;
        assert_eq!(categories, vec!["Local Radio", "Music", "Talk"]);
    }
}
