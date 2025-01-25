use anyhow::Error;
use owo_colors::OwoColorize;

use crate::provider::radiobrowser::Radiobrowser;
use crate::provider::tunein::Tunein;
use crate::provider::Provider;

pub async fn exec(
    category: Option<&str>,
    offset: u32,
    limit: u32,
    provider: &str,
) -> Result<(), Error> {
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

    match category {
        Some(category) => {
            let results = provider.browse(category.to_string(), offset, limit).await?;
            for result in results {
                match result.id.is_empty() {
                    false => match result.playing {
                        Some(playing) => println!(
                            "  {} | {} | id: {}",
                            result.name.magenta(),
                            playing,
                            result.id
                        ),
                        None => println!("  {} | id: {}", result.name.magenta(), result.id),
                    },

                    true => println!("{}", result.name),
                }
            }
        }
        None => {
            let results = provider.categories(offset, limit).await?;
            for result in results {
                println!("{}", result.magenta());
            }
        }
    };
    Ok(())
}
