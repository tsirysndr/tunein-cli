use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tunein::TuneInClient;
use tunein_cli::api::tunein::v1alpha1::{
    playback_service_server::PlaybackService, PlayOrPauseRequest, PlayOrPauseResponse, PlayRequest,
    PlayResponse, StopRequest, StopResponse,
};

use crate::{
    extract::extract_stream_url,
    player::{Player, PlayerCommand},
};

pub struct Playback {
    client: TuneInClient,
    player: Player,
    cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
}

impl Default for Playback {
    fn default() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<PlayerCommand>();
        let cmd_rx = Arc::new(Mutex::new(cmd_rx));
        let player = Player::new(cmd_rx);
        Self {
            client: TuneInClient::new(),
            player,
            cmd_tx,
        }
    }
}

#[tonic::async_trait]
impl PlaybackService for Playback {
    async fn play(
        &self,
        request: tonic::Request<PlayRequest>,
    ) -> Result<tonic::Response<PlayResponse>, tonic::Status> {
        let req = request.into_inner();

        let results = self
            .client
            .get_station(&req.station_name_or_id)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;
        let (url, playlist_type, _) = match results.is_empty() {
            true => {
                let results = self
                    .client
                    .search(&req.station_name_or_id)
                    .await
                    .map_err(|e| tonic::Status::internal(e.to_string()))?;
                match results.first() {
                    Some(result) => {
                        if result.r#type != Some("audio".to_string()) {
                            return Err(tonic::Status::internal("No station found"));
                        }
                        let id = result.guide_id.as_ref().unwrap();
                        let station = self
                            .client
                            .get_station(id)
                            .await
                            .map_err(|e| tonic::Status::internal(e.to_string()))?;
                        let station = station.first().unwrap();
                        (
                            station.url.clone(),
                            station.playlist_type.clone(),
                            station.media_type.clone(),
                        )
                    }
                    None => ("".to_string(), None, "".to_string()),
                }
            }
            false => {
                let result = results.first().unwrap();
                (
                    result.url.clone(),
                    result.playlist_type.clone(),
                    result.media_type.clone(),
                )
            }
        };
        let stream_url = extract_stream_url(&url, playlist_type)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;
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
