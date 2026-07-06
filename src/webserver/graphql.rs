use async_graphql::{EmptySubscription, Error, Object, Result, Schema, SimpleObject, ID};

use crate::favorites::{FavoriteStation, FavoritesStore};
use tunein_cli::extract::get_currently_playing;
use tunein_cli::provider::{radiobrowser::Radiobrowser, tunein::Tunein, Provider};

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema() -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish()
}

#[derive(SimpleObject)]
pub struct Station {
    pub id: ID,
    pub name: String,
    pub codec: String,
    pub bitrate: u32,
    pub stream_url: String,
    pub playing: Option<String>,
}

impl From<tunein_cli::types::Station> for Station {
    fn from(st: tunein_cli::types::Station) -> Self {
        Self {
            id: ID(st.id),
            name: st.name,
            codec: st.codec,
            bitrate: st.bitrate,
            stream_url: st.stream_url,
            playing: st.playing,
        }
    }
}

#[derive(SimpleObject)]
pub struct Favorite {
    pub id: ID,
    pub name: String,
    pub provider: String,
}

impl From<FavoriteStation> for Favorite {
    fn from(fav: FavoriteStation) -> Self {
        Self {
            id: ID(fav.id),
            name: fav.name,
            provider: fav.provider,
        }
    }
}

async fn resolve_provider(name: Option<String>) -> Result<Box<dyn Provider + Send + Sync>> {
    match name.as_deref() {
        Some("tunein") | None => Ok(Box::new(Tunein::new())),
        Some("radiobrowser") => Ok(Box::new(Radiobrowser::new().await)),
        Some(other) => Err(Error::new(format!("Unsupported provider '{}'", other))),
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Search stations by name.
    async fn search(&self, query: String, provider: Option<String>) -> Result<Vec<Station>> {
        let client = resolve_provider(provider).await?;
        let stations = client
            .search(query)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(stations.into_iter().map(Station::from).collect())
    }

    /// Fetch a single station (with its resolved stream url) by id or exact name.
    async fn station(&self, id: ID, provider: Option<String>) -> Result<Option<Station>> {
        let client = resolve_provider(provider).await?;
        let station = client
            .get_station(id.0)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(station.map(Station::from))
    }

    /// Browse stations of a category (category name or id).
    async fn browse(
        &self,
        category: String,
        #[graphql(default = 0)] offset: u32,
        #[graphql(default = 100)] limit: u32,
        provider: Option<String>,
    ) -> Result<Vec<Station>> {
        let client = resolve_provider(provider).await?;
        let stations = client
            .browse(category, offset, limit)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(stations.into_iter().map(Station::from).collect())
    }

    /// List available categories.
    async fn categories(
        &self,
        #[graphql(default = 0)] offset: u32,
        #[graphql(default = 100)] limit: u32,
        provider: Option<String>,
    ) -> Result<Vec<String>> {
        let client = resolve_provider(provider).await?;
        client
            .categories(offset, limit)
            .await
            .map_err(|e| Error::new(e.to_string()))
    }

    /// Currently playing track of a TuneIn station.
    async fn now_playing(&self, station_id: ID) -> Result<String> {
        get_currently_playing(&station_id.0)
            .await
            .map_err(|e| Error::new(e.to_string()))
    }

    /// Favourite stations, shared with the CLI/TUI (favorites.json).
    async fn favorites(&self) -> Result<Vec<Favorite>> {
        let store = FavoritesStore::load().map_err(|e| Error::new(e.to_string()))?;
        Ok(store.all().iter().cloned().map(Favorite::from).collect())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Add a station to favourites (no-op if already present).
    async fn add_favorite(
        &self,
        id: ID,
        name: String,
        provider: Option<String>,
    ) -> Result<Favorite> {
        let provider = provider.unwrap_or_else(|| "tunein".to_string());
        let mut store = FavoritesStore::load().map_err(|e| Error::new(e.to_string()))?;
        store
            .add(FavoriteStation {
                id: id.0.clone(),
                name: name.clone(),
                provider: provider.clone(),
            })
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(Favorite { id, name, provider })
    }

    /// Remove a station from favourites. Returns true when it existed.
    async fn remove_favorite(&self, id: ID, provider: Option<String>) -> Result<bool> {
        let provider = provider.unwrap_or_else(|| "tunein".to_string());
        let mut store = FavoritesStore::load().map_err(|e| Error::new(e.to_string()))?;
        let existed = store.is_favorite(&id.0, &provider);
        store
            .remove(&id.0, &provider)
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(existed)
    }
}
