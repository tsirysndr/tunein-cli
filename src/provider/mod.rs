pub mod radiobrowser;
pub mod tunein;

use crate::types::Station;
use anyhow::Error;
use async_trait::async_trait;

#[async_trait]
pub trait Provider {
    async fn search(&self, name: String) -> Result<Vec<Station>, Error>;
    async fn get_station(&self, id: String) -> Result<(), Error>;
    async fn browse(&self, category: String) -> Result<Vec<Station>, Error>;
}
