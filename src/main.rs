use anyhow::Error;
use clap::{arg, Command};

mod browse;
mod play;
mod search;

fn cli() -> Command<'static> {
    const VESRION: &str = env!("CARGO_PKG_VERSION");
    Command::new("tunein")
        .version(VESRION)
        .author("Tsiry Sandratraina <tsiry.sndr@aol.com>")
        .about(
            r#"
        ______              ____       _______   ____
       /_  __/_ _____  ___ /  _/__    / ___/ /  /  _/
        / / / // / _ \/ -_)/ // _ \  / /__/ /___/ /  
       /_/  \_,_/_//_/\__/___/_//_/  \___/____/___/  
                                                              
A simple CLI to listen to radio stations"#,
        )
        .subcommand_required(true)
        .subcommand(
            Command::new("search")
                .about("Search for a radio station")
                .arg(arg!(<query> "The query to search for")),
        )
        .subcommand(
            Command::new("play")
                .about("Play a radio station")
                .arg(arg!(<station> "The station to play")),
        )
        .subcommand(
            Command::new("browse")
                .about("Browse radio stations")
                .arg(arg!([category] "The category (category name or id) to browse")),
        )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("search", args)) => {
            let query = args.value_of("query").unwrap();
            search::exec(query).await?;
        }
        Some(("play", args)) => {
            let station = args.value_of("station").unwrap();
            play::exec(station).await?;
        }
        Some(("browse", args)) => {
            let category = args.value_of("category");
            browse::exec(category).await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
