use anyhow::Error;
use owo_colors::OwoColorize;
use tunein::TuneInClient;

pub async fn exec(query: &str) -> Result<(), Error> {
    let client = TuneInClient::new();
    let results = client
        .search(query)
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    let query = format!("\"{}\"", query);
    println!("Results for {}:", query.bright_green());

    let results = results
        .into_iter()
        .filter(|r| Some("audio".to_string()) == r.r#type)
        .collect::<Vec<_>>();

    if results.is_empty() {
        println!("No results found");
        return Ok(());
    }

    for result in results {
        if Some("audio".to_string()) == result.r#type {
            println!("{} | {}", result.text, result.subtext.unwrap_or_default());
        }
    }
    Ok(())
}
