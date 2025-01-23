use crate::types::Station;

use super::Provider;
use anyhow::Error;
use async_trait::async_trait;
use radiobrowser::RadioBrowserAPI;
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

    async fn get_station(&self, name: String) -> Result<Option<Station>, Error> {
        let stations = self
            .client
            .get_stations()
            .name(&name)
            .name_exact(true)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
        match stations.len() {
            0 => Ok(None),
            _ => Ok(Some(Station::from(stations[0].clone()))),
        }
    }

    async fn browse(&self, category: String) -> Result<Vec<Station>, Error> {
        let stations = self
            .client
            .get_stations()
            .tag(&category)
            .limit("100")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
        let stations = stations.into_iter().map(|x| Station::from(x)).collect();
        Ok(stations)
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
    pub async fn test_browse() {
        let provider = Radiobrowser::new().await;
        let stations = provider.browse("music".to_string()).await.unwrap();
        let stations = stations
            .into_iter()
            .map(|x| Station::from(x))
            .collect::<Vec<Station>>();
        assert!(stations.len() == 100)
    }
}
