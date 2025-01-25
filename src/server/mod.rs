use std::net::SocketAddr;

use anyhow::Error;
use owo_colors::OwoColorize;
use tonic::transport::Server;
use tunein_cli::api::tunein::v1alpha1::{
    browse_service_server::BrowseServiceServer, playback_service_server::PlaybackServiceServer,
};
use tunein_cli::api::tunein::FILE_DESCRIPTOR_SET;

use self::{browse::Browse, playback::Playback};

pub mod browse;
pub mod playback;

pub async fn exec(port: u16) -> Result<(), Error> {
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Listening on {}", addr.cyan());
    Server::builder()
        .accept_http1(true)
        .add_service(
            tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
                .build_v1alpha()?,
        )
        .add_service(tonic_web::enable(BrowseServiceServer::new(
            Browse::default(),
        )))
        .add_service(tonic_web::enable(PlaybackServiceServer::new(
            Playback::default(),
        )))
        .serve(addr)
        .await?;
    Ok(())
}
