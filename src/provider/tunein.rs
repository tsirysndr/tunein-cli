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
            0 => {
                let results = self.search(id.clone()).await?;
                let station = results.first().cloned();
                match station {
                    Some(st) => {
                        let stations = self
                            .client
                            .get_station(&st.id)
                            .await
                            .map_err(|e| Error::msg(e.to_string()))?;
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
            "by location" => Some(tunein::types::Category::ByLocation),
            "by language" => Some(tunein::types::Category::ByLanguage),
            "sports" => Some(tunein::types::Category::Sports),
            "talk" => Some(tunein::types::Category::Talk),
            "music" => Some(tunein::types::Category::Music),
            "local radio" => Some(tunein::types::Category::LocalRadio),
            "podcasts" => Some(tunein::types::Category::Podcasts),
            _ => None,
        };

        if category.is_none() {
            let category_stations = self
                .client
                .browse_by_id(&guide_id)
                .await
                .map_err(|e| Error::msg(e.to_string()))?;

            let mut stations = vec![];

            for st in category_stations {
                if let Some(children) = st.clone().children {
                    stations = [stations, vec![Box::new(st.clone())], children].concat();
                }
            }

            let stations = stations.into_iter().map(|x| Station::from(x)).collect();
            return Ok(stations);
        }

        let category_stations = self
            .client
            .browse(category)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;

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
        let categories = self
            .client
            .browse(None)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        let categories = categories.into_iter().map(|x| x.text).collect();
        Ok(categories)
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
        let stations = provider.browse("music".to_string(), 0, 100).await.unwrap();
        println!("Browse: {:#?}", stations);
        assert!(stations.len() > 0)
    }

    #[tokio::test]
    pub async fn test_browse_by_id() {
        let provider = Tunein::new();
        let stations = provider.browse("c57942".to_string(), 0, 100).await.unwrap();
        println!("Browse by category id: {:#?}", stations);
        assert!(stations.len() > 0)
    }

    #[tokio::test]
    pub async fn test_categories() {
        let provider = Tunein::new();
        let categories = provider.categories(0, 100).await.unwrap();
        println!("Categories: {:#?}", categories);
        assert!(categories.len() > 0)
    }
}
