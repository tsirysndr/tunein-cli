use tunein_cli::api::tunein::v1alpha1::{
    playback_service_server::PlaybackService, PlayOrPauseRequest, PlayOrPauseResponse, PlayRequest,
    PlayResponse, StopRequest, StopResponse,
};

#[derive(Debug, Default)]
pub struct Playback {}

#[tonic::async_trait]
impl PlaybackService for Playback {
    async fn play(
        &self,
        request: tonic::Request<PlayRequest>,
    ) -> Result<tonic::Response<PlayResponse>, tonic::Status> {
        todo!()
    }

    async fn stop(
        &self,
        request: tonic::Request<StopRequest>,
    ) -> Result<tonic::Response<StopResponse>, tonic::Status> {
        todo!()
    }

    async fn play_or_pause(
        &self,
        request: tonic::Request<PlayOrPauseRequest>,
    ) -> Result<tonic::Response<PlayOrPauseResponse>, tonic::Status> {
        todo!()
    }
}
