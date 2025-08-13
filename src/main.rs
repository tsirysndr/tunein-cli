use anyhow::Error;
use clap::{arg, Command};

mod app;
mod browse;
mod cfg;
mod decoder;
mod extract;
mod input;
mod music;
mod play;
mod player;
mod provider;
mod search;
mod server;
mod service;
mod tags;
mod tui;
mod types;
mod visualization;

fn cli() -> Command<'static> {
    const VESRION: &str = env!("CARGO_PKG_VERSION");
    Command::new("tunein")
        .version(VESRION)
        .author("Tsiry Sandratraina <tsiry.sndr@fluentci.io>")
        .about(
            r#"
        ______              ____       _______   ____
       /_  __/_ _____  ___ /  _/__    / ___/ /  /  _/
        / / / // / _ \/ -_)/ // _ \  / /__/ /___/ /  
       /_/  \_,_/_//_/\__/___/_//_/  \___/____/___/  
                                                              
A simple CLI to listen to radio stations"#,
        )
        .arg(
            arg!(-p --provider "The radio provider to use, can be 'tunein' or 'radiobrowser'. Default is 'tunein'").default_value("tunein")
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
                .arg(arg!(<station> "The station to play"))
                .arg(arg!(--volume "Set the initial volume (as a percent)").default_value("100")),
        )
        .subcommand(
            Command::new("browse")
                .about("Browse radio stations")
                .arg(arg!([category] "The category (category name or id) to browse"))
                .arg(arg!(--offset "The offset to start from").default_value("0"))
                .arg(arg!(--limit "The number of results to show").default_value("100")),
        )
        .subcommand(
            Command::new("server")
                .about("Start the server")
                .arg(arg!([port] "The port to listen on").default_value("8090")),
        )
        .subcommand(
            Command::new("service")
            .about("Manage systemd service for tunein-cli server")
            .subcommand(
                Command::new("install")
                        .about("Install systemd service for tunein-cli server")
            )
            .subcommand(
                    Command::new("uninstall")
                        .about("Uninstall systemd service for tunein-cli server")   
                )
            .subcommand(
                    Command::new("status")
                    .about("Check status of tunein-cli systemd service")
                )
        )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("search", args)) => {
            let query = args.value_of("query").unwrap();
            let provider = matches.value_of("provider").unwrap();
            search::exec(query, provider).await?;
        }
        Some(("play", args)) => {
            let station = args.value_of("station").unwrap();
            let provider = matches.value_of("provider").unwrap();
            let volume = args.value_of("volume").unwrap().parse::<f32>().unwrap();
            play::exec(station, provider, volume).await?;
        }
        Some(("browse", args)) => {
            let category = args.value_of("category");
            let offset = args.value_of("offset").unwrap();
            let limit = args.value_of("limit").unwrap();
            let provider = matches.value_of("provider").unwrap();
            browse::exec(
                category,
                offset.parse::<u32>()?,
                limit.parse::<u32>()?,
                provider,
            )
            .await?;
        }
        Some(("server", args)) => {
            let port = args.value_of("port").unwrap();
            let port = port.parse::<u16>().unwrap();
            server::exec(port).await?;
        }
        Some(("service", sub_m)) => match sub_m.subcommand() {
            Some(("install", _)) => service::install()?,
            Some(("uninstall", _)) => service::uninstall()?,
            Some(("status", _)) => service::status()?,
            _ => {
                println!("Invalid subcommand. Use `tunein service --help` for more information");
                std::process::exit(1);
            }
        },
        _ => unreachable!(),
    }

    Ok(())
}
