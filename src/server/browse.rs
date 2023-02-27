use tunein_cli::api::tunein::v1alpha1::{
    browse_service_server::BrowseService, BrowseCategoryRequest, BrowseCategoryResponse,
    GetCategoriesRequest, GetCategoriesResponse, GetStationDetailsRequest,
    GetStationDetailsResponse,
};

#[derive(Debug, Default)]
pub struct Browse {}


#[tonic::async_trait]
impl BrowseService for Browse {
    async fn get_categories(
        &self,
        request: tonic::Request<GetCategoriesRequest>,
    ) -> Result<tonic::Response<GetCategoriesResponse>, tonic::Status> {
        todo!()
    }

    async fn browse_category(
        &self,
        request: tonic::Request<BrowseCategoryRequest>,
    ) -> Result<tonic::Response<BrowseCategoryResponse>, tonic::Status> {
        todo!()
    }

    async fn get_station_details(
        &self,
        request: tonic::Request<GetStationDetailsRequest>,
    ) -> Result<tonic::Response<GetStationDetailsResponse>, tonic::Status> {
        todo!()
    }
}
