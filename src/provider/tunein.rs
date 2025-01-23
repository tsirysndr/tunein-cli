use crate::types::Station;

use super::Provider;
use anyhow::Error;
use async_trait::async_trait;
use tunein::TuneInClient;

pub struct Tunein {
    client: TuneInClient,
}

impl Tunein {
    pub fn new() -> Self {
        Self {
            client: TuneInClient::new(),
        }
    }
}

#[async_trait]
impl Provider for Tunein {
    async fn search(&self, name: String) -> Result<Vec<Station>, Error> {
        let results = self
            .client
            .search(&name)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        let stations = results.into_iter().map(|x| Station::from(x)).collect();
        Ok(stations)
    }

    async fn get_station(&self, id: String) -> Result<Option<Station>, Error> {
        let stations = self
            .client
            .get_station(&id)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        match stations.len() {
            0 => Ok(None),
            _ => Ok(Some(Station::from(stations[0].clone()))),
        }
    }

    async fn browse(&self, category: String) -> Result<Vec<Station>, Error> {
        let category = match category.as_str() {
            "by location" => Some(tunein::types::Category::ByLocation),
            "by language" => Some(tunein::types::Category::ByLanguage),
            "sports" => Some(tunein::types::Category::Sports),
            "talk" => Some(tunein::types::Category::Talk),
            "music" => Some(tunein::types::Category::Music),
            "local radio" => Some(tunein::types::Category::LocalRadio),
            "podcasts" => Some(tunein::types::Category::Podcasts),
            _ => return Err(Error::msg("Invalid category")),
        };

        let stations = self
            .client
            .browse(category)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    pub async fn test_search() {
        let provider = Tunein::new();
        let name = "alternativeradio";
        let stations = provider.search(name.to_string()).await.unwrap();
        println!("Search: {:#?}", stations);
        assert!(stations.len() > 0)
    }

    #[tokio::test]
    pub async fn test_get_station() {
        let provider = Tunein::new();
        let name = "s288303".to_string();
        let station = provider.get_station(name).await.unwrap();
        println!("Station: {:#?}", station);
        assert!(station.is_some())
    }

    #[tokio::test]
    pub async fn test_browse() {
        let provider = Tunein::new();
        let stations = provider.browse("music".to_string()).await.unwrap();
        let stations = stations
            .into_iter()
            .map(|x| Station::from(x))
            .collect::<Vec<Station>>();
        println!("Browse: {:#?}", stations);
        assert!(stations.len() == 0)
    }
}
