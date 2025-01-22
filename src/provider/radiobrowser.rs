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

    async fn get_station(&self, name: String) -> Result<(), Error> {
        let station = self
            .client
            .get_stations()
            .name(&name)
            .name_exact(true)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(format!("{}", e)))?;
        println!("Station: {:#?}", station);
        Ok(())
    }

    async fn browse(&self, category: String) -> Result<Vec<Station>, Error> {
        let stations = self
            .client
            .get_stations()
            .tag(&category)
            .limit("20")
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
        let name = "alternariveradio.us".to_string();
        provider.get_station(name).await.unwrap();
    }

    #[tokio::test]
    pub async fn test_browse() {
        let provider = Radiobrowser::new().await;
        let stations = provider.browse("music".to_string()).await.unwrap();
        let stations = stations
            .into_iter()
            .map(|x| Station::from(x))
            .collect::<Vec<Station>>();
        assert!(stations.len() == 20)
    }
}
