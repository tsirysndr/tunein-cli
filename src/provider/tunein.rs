use crate::types::Station;

use super::Provider;
use anyhow::Error;
use async_trait::async_trait;

pub struct Tunein {}

#[async_trait]
impl Provider for Tunein {
    async fn search(&self, name: String) -> Result<Vec<Station>, Error> {
        Ok(vec![])
    }

    async fn get_station(&self, id: String) -> Result<(), Error> {
        Ok(())
    }

    async fn browse(&self, category: String) -> Result<Vec<Station>, Error> {
        Ok(vec![])
    }
}
