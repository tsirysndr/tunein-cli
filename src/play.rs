use anyhow::Error;
use tunein::TuneInClient;

pub async fn exec(_station: &str) -> Result<(), Error> {
    let _client = TuneInClient::new();
    todo!()
}
