use std::time::Duration;

use anyhow::Error;
use app::CurrentDisplayMode;
use clap::{arg, builder::ValueParser, Command};

mod app;
mod audio;
mod browse;
mod cfg;
mod decoder;
mod extract;
mod favorites;
mod input;
mod interactive;
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
        .subcommand(
            Command::new("search")
                .about("Search for a radio station")
                .arg(arg!(<query> "The query to search for")),
        )
        .subcommand(
            Command::new("play")
                .about("Play a radio station")
                .arg(arg!(<station> "The station to play"))
                .arg(arg!(--volume "Set the initial volume (as a percent)").default_value("100"))
                .arg(clap::Arg::new("display-mode").long("display-mode").help("Set the display mode to start with").default_value("Spectroscope"))
                .arg(clap::Arg::new("enable-os-media-controls").long("enable-os-media-controls").help("Should enable OS media controls?").default_value("true").value_parser(ValueParser::bool()))
                .arg(clap::Arg::new("poll-events-every").long("poll-events-every").help("Poll for events every specified milliseconds.").default_value("16"))
                .arg(clap::Arg::new("poll-events-every-while-paused").long("poll-events-every-while-paused").help("Poll for events every specified milliseconds while player is paused.").default_value("100")),
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
    let provider = matches.value_of("provider").unwrap().to_string();

    match matches.subcommand() {
        Some(("search", args)) => {
            let query = args.value_of("query").unwrap();
            search::exec(query, provider.as_str()).await?;
        }
        Some(("play", args)) => {
            let station = args.value_of("station").unwrap();
            let volume = args.value_of("volume").unwrap().parse::<f32>().unwrap();
            let display_mode = args
                .value_of("display-mode")
                .unwrap()
                .parse::<CurrentDisplayMode>()
                .unwrap();
            let enable_os_media_controls = args.get_one("enable-os-media-controls").unwrap();
            let poll_events_every =
                Duration::from_millis(args.value_of("poll-events-every").unwrap().parse().unwrap());
            let poll_events_every_while_paused = Duration::from_millis(
                args.value_of("poll-events-every-while-paused")
                    .unwrap()
                    .parse()
                    .unwrap(),
            );
            play::exec(
                station,
                provider.as_str(),
                volume,
                display_mode,
                *enable_os_media_controls,
                poll_events_every,
                poll_events_every_while_paused,
            )
            .await?;
        }
        Some(("browse", args)) => {
            let category = args.value_of("category");
            let offset = args.value_of("offset").unwrap();
            let limit = args.value_of("limit").unwrap();
            browse::exec(
                category,
                offset.parse::<u32>()?,
                limit.parse::<u32>()?,
                provider.as_str(),
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
        None => {
            interactive::run(provider.as_str()).await?;
        }
        Some((other, _)) => {
            eprintln!(
                "Unknown subcommand '{}'. Use `tunein --help` for available commands.",
                other
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
