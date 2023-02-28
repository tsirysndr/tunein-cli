use tunein::TuneInClient;
use tunein_cli::api::tunein::v1alpha1::{
    browse_service_server::BrowseService, BrowseCategoryRequest, BrowseCategoryResponse,
    GetCategoriesRequest, GetCategoriesResponse, GetStationDetailsRequest,
    GetStationDetailsResponse,
};

pub struct Browse {
    client: TuneInClient,
}

impl Default for Browse {
    fn default() -> Self {
        Self {
            client: TuneInClient::new(),
        }
    }
}

#[tonic::async_trait]
impl BrowseService for Browse {
    async fn get_categories(
        &self,
        request: tonic::Request<GetCategoriesRequest>,
    ) -> Result<tonic::Response<GetCategoriesResponse>, tonic::Status> {
        Ok(tonic::Response::new(GetCategoriesResponse {}))
    }

    async fn browse_category(
        &self,
        request: tonic::Request<BrowseCategoryRequest>,
    ) -> Result<tonic::Response<BrowseCategoryResponse>, tonic::Status> {
        Ok(tonic::Response::new(BrowseCategoryResponse {}))
    }

    async fn get_station_details(
        &self,
        request: tonic::Request<GetStationDetailsRequest>,
    ) -> Result<tonic::Response<GetStationDetailsResponse>, tonic::Status> {
        Ok(tonic::Response::new(GetStationDetailsResponse {}))
    }
}
