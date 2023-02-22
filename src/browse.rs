use std::str::FromStr;

use anyhow::Error;
use owo_colors::OwoColorize;
use tunein::{types::Category, TuneInClient};

pub async fn exec(category: Option<&str>) -> Result<(), Error> {
    let client = TuneInClient::new();

    if category.is_some() && Category::from_str(category.unwrap_or_default()).is_err() {
        let id = category.unwrap_or_default();
        let results = client
            .browse_by_id(id)
            .await
            .map_err(|e| Error::msg(e.to_string()))?;
        for result in results {
            println!("{}", result.text);
            if let Some(children) = result.children {
                for child in children {
                    match child.playing {
                        Some(playing) => println!("  {} | {}", child.text.magenta(), playing),
                        None => println!("  {} | {}", child.text.magenta(), child.url.unwrap()),
                    }
                }
            }
        }
        return Ok(());
    }

    let results = match category {
        Some(category) => match Category::from_str(category) {
            Ok(category) => client
                .browse(Some(category))
                .await
                .map_err(|e| Error::msg(e.to_string()))?,
            Err(_) => {
                println!("Invalid category");
                return Ok(());
            }
        },
        None => client
            .browse(None)
            .await
            .map_err(|e| Error::msg(e.to_string()))?,
    };

    for result in results {
        match result.guide_id {
            Some(_) => println!(
                "{} | id: {}",
                result.text.magenta(),
                result.guide_id.unwrap()
            ),
            None => println!("{}", result.text),
        }
        if let Some(children) = result.children {
            for child in children {
                match child.playing {
                    Some(playing) => println!(
                        "  {} | {} | id: {}",
                        child.text.magenta(),
                        playing,
                        child.guide_id.unwrap()
                    ),
                    None => println!(
                        "  {} | id: {}",
                        child.text.magenta(),
                        child.guide_id.unwrap()
                    ),
                }
            }
        }
    }
    Ok(())
}
