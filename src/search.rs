use anyhow::Error;
use owo_colors::OwoColorize;

use crate::provider::{radiobrowser::Radiobrowser, tunein::Tunein, Provider};

pub async fn exec(query: &str, provider: &str) -> Result<(), Error> {
    let provider: Box<dyn Provider> = match provider {
        "tunein" => Box::new(Tunein::new()),
        "radiobrowser" => Box::new(Radiobrowser::new().await),
        _ => {
            return Err(anyhow::anyhow!(format!(
                "Unsupported provider '{}'",
                provider
            )))
        }
    };
    let results = provider.search(query.to_string()).await?;
    let query = format!("\"{}\"", query);
    println!("Results for {}:", query.bright_green());

    if results.is_empty() {
        println!("No results found");
        return Ok(());
    }

    for result in results {
        match result.playing {
            Some(playing) => println!(
                "{} | {} | id: {}",
                result.name.magenta(),
                playing,
                result.id
            ),
            None => println!("{} | id: {}", result.name.magenta(), result.id),
        }
    }
    Ok(())
}
