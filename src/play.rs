use anyhow::Error;
use tunein::TuneInClient;

pub async fn exec(name_or_id: &str) -> Result<(), Error> {
    let client = TuneInClient::new();
    let results = client
        .get_station(name_or_id)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let (url, playlist_type) = match results.is_empty() {
        true => {
            let results = client
                .search(name_or_id)
                .await
                .map_err(|e| Error::msg(e.to_string()))?;
            match results.first() {
                Some(result) => {
                    if result.r#type != Some("audio".to_string()) {
                        return Err(Error::msg("No station found"));
                    }
                    let id = result.guide_id.as_ref().unwrap();
                    let station = client
                        .get_station(id)
                        .await
                        .map_err(|e| Error::msg(e.to_string()))?;
                    let station = station.first().unwrap();
                    (station.url.clone(), station.playlist_type.clone())
                }
                None => ("".to_string(), None),
            }
        }
        false => {
            let result = results.first().unwrap();
            (result.url.clone(), result.playlist_type.clone())
        }
    };
    println!("{} | {:?}", url, playlist_type);
    Ok(())
}
