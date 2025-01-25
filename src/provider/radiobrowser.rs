use crate::types::Station;

use super::{is_valid_uuid, Provider};
use anyhow::Error;
use async_trait::async_trait;
use radiobrowser::{ApiStation, RadioBrowserAPI};
use std::process::exit;

pub struct Radiobrowser {
    client: RadioBrowserAPI,
}

impl Radiobrowser {
    pub async fn new() -> Self {
        let client = RadioBrowserAPI::new().await;

        if client.is_err() {
            eprintln!("Failed to create a RadioBrowserAPI client");
            exit(1);
        }

        let client = client.unwrap();
        Self { client }
    }
}

#[async_trait]
impl Provider for Radiobrowser {
    async fn search(&self, name: String) -> Result<Vec<Station>, Error> {
        let stations = self
            .client
            .get_stations()
            .name(&name)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
        let stations = stations.into_iter().map(|x| Station::from(x)).collect();
        Ok(stations)
    }

    async fn get_station(&self, name_or_uuid: String) -> Result<Option<Station>, Error> {
        match is_valid_uuid(&name_or_uuid) {
            true => {
                let servers = RadioBrowserAPI::get_default_servers()
                    .await
                    .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;

                if servers.is_empty() {
                    return Ok(None);
                }

                let client = reqwest::Client::new();
                let url = format!(
                    "https://{}/json/stations/byuuid/{}",
                    servers[0], name_or_uuid
                );
                let results = client
                    .get(&url)
                    .send()
                    .await?
                    .json::<Vec<ApiStation>>()
                    .await?;

                Ok(results.into_iter().next().map(|x| Station::from(x)))
            }
            false => {
                let stations = self
                    .client
                    .get_stations()
                    .name(&name_or_uuid)
                    .name_exact(true)
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
                match stations.len() {
                    0 => Ok(None),
                    _ => Ok(Some(Station::from(stations[0].clone()))),
                }
            }
        }
    }

    async fn browse(
        &self,
        category: String,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Station>, Error> {
        let stations = self
            .client
            .get_stations()
            .tag(&category)
            .offset(&format!("{}", offset))
            .limit(&format!("{}", limit))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
        let stations = stations.into_iter().map(|x| Station::from(x)).collect();
        Ok(stations)
    }

    async fn categories(&self, offset: u32, limit: u32) -> Result<Vec<String>, Error> {
        let categories = self
            .client
            .get_tags()
            .offset(&format!("{}", offset))
            .limit(&format!("{}", limit))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
        let categories = categories.into_iter().map(|x| x.name).collect();
        Ok(categories)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    pub async fn test_search() {
        let provider = Radiobrowser::new().await;
        let name = "alternativeradio";
        let stations = provider.search(name.to_string()).await.unwrap();
        assert!(stations.len() == 1)
    }

    #[tokio::test]
    pub async fn test_get_station() {
        let provider = Radiobrowser::new().await;
        let name = "AlternativeRadio.us".to_string();
        let station = provider.get_station(name).await.unwrap();
        assert!(station.is_some())
    }

    #[tokio::test]
    pub async fn test_get_station_by_uuid() {
        let provider = Radiobrowser::new().await;
        let name = "964da563-0601-11e8-ae97-52543be04c81".to_string();
        let station = provider.get_station(name).await.unwrap();
        assert!(station.is_some())
    }

    #[tokio::test]
    pub async fn test_browse() {
        let provider = Radiobrowser::new().await;
        let stations = provider.browse("music".to_string(), 0, 100).await.unwrap();
        let stations = stations
            .into_iter()
            .map(|x| Station::from(x))
            .collect::<Vec<Station>>();
        assert!(stations.len() == 100)
    }

    #[tokio::test]
    pub async fn test_categories() {
        let provider = Radiobrowser::new().await;
        let categories = provider.categories(0, 100).await.unwrap();
        assert!(categories.len() > 0)
    }
}
