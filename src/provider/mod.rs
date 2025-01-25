pub mod radiobrowser;
pub mod tunein;

use crate::types::Station;
use anyhow::Error;
use async_trait::async_trait;
use regex::Regex;

#[async_trait]
pub trait Provider {
    async fn search(&self, name: String) -> Result<Vec<Station>, Error>;
    async fn get_station(&self, id: String) -> Result<Option<Station>, Error>;
    async fn browse(
        &self,
        category: String,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Station>, Error>;
    async fn categories(&self, offset: u32, limit: u32) -> Result<Vec<String>, Error>;
}

pub fn is_valid_uuid(uuid: &str) -> bool {
    let uuid_pattern = Regex::new(
        r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-5][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$"
    ).unwrap();

    uuid_pattern.is_match(uuid)
}
