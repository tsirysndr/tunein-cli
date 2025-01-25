use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tunein_cli::api::tunein::v1alpha1::{
    playback_service_server::PlaybackService, PlayOrPauseRequest, PlayOrPauseResponse, PlayRequest,
    PlayResponse, StopRequest, StopResponse,
};

use crate::{
    player::{Player, PlayerCommand},
    provider::{tunein::Tunein, Provider},
};

pub struct Playback {
    player: Player,
    cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
}

impl Default for Playback {
    fn default() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<PlayerCommand>();
        let cmd_rx = Arc::new(Mutex::new(cmd_rx));
        let player = Player::new(cmd_rx);
        Self { player, cmd_tx }
    }
}

#[tonic::async_trait]
impl PlaybackService for Playback {
    async fn play(
        &self,
        request: tonic::Request<PlayRequest>,
    ) -> Result<tonic::Response<PlayResponse>, tonic::Status> {
        let req = request.into_inner();

        let client: Box<dyn Provider + Send + Sync> = Box::new(Tunein::new());
        let station = client
            .get_station(req.station_name_or_id.clone())
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        if station.is_none() {
            return Err(tonic::Status::internal("No station found"));
        }

        let station = station.unwrap();
        let stream_url = station.stream_url.clone();
        println!("{}", stream_url);

        self.cmd_tx.send(PlayerCommand::Play(stream_url)).unwrap();
        Ok(tonic::Response::new(PlayResponse {}))
    }

    async fn stop(
        &self,
        _request: tonic::Request<StopRequest>,
    ) -> Result<tonic::Response<StopResponse>, tonic::Status> {
        self.cmd_tx.send(PlayerCommand::Stop).unwrap();
        Ok(tonic::Response::new(StopResponse {}))
    }

    async fn play_or_pause(
        &self,
        _request: tonic::Request<PlayOrPauseRequest>,
    ) -> Result<tonic::Response<PlayOrPauseResponse>, tonic::Status> {
        self.cmd_tx.send(PlayerCommand::PlayOrPause).unwrap();
        Ok(tonic::Response::new(PlayOrPauseResponse {}))
    }
}
