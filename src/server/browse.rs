use tunein_cli::api::{
    objects::v1alpha1::{Category, StationLinkDetails},
    tunein::v1alpha1::{
        browse_service_server::BrowseService, BrowseCategoryRequest, BrowseCategoryResponse,
        GetCategoriesRequest, GetCategoriesResponse, GetStationDetailsRequest,
        GetStationDetailsResponse,
    },
};

use tunein_cli::provider::{tunein::Tunein, Provider};

#[derive(Default)]
pub struct Browse;

#[tonic::async_trait]
impl BrowseService for Browse {
    async fn get_categories(
        &self,
        _request: tonic::Request<GetCategoriesRequest>,
    ) -> Result<tonic::Response<GetCategoriesResponse>, tonic::Status> {
        let client: Box<dyn Provider + Send + Sync> = Box::new(Tunein::new());
        let offset = 0;
        let limit = 100;
        let result = client
            .categories(offset, limit)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        Ok(tonic::Response::new(GetCategoriesResponse {
            categories: result.into_iter().map(Category::from).collect(),
        }))
    }

    async fn browse_category(
        &self,
        request: tonic::Request<BrowseCategoryRequest>,
    ) -> Result<tonic::Response<BrowseCategoryResponse>, tonic::Status> {
        let req = request.into_inner();
        let category_id = req.category_id;

        let offset = 0;
        let limit = 100;

        let client: Box<dyn Provider + Send + Sync> = Box::new(Tunein::new());
        let results = client
            .browse(category_id, offset, limit)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        let categories = results.into_iter().map(Category::from).collect();
        Ok(tonic::Response::new(BrowseCategoryResponse { categories }))
    }

    async fn get_station_details(
        &self,
        request: tonic::Request<GetStationDetailsRequest>,
    ) -> Result<tonic::Response<GetStationDetailsResponse>, tonic::Status> {
        let req = request.into_inner();
        let station_id = req.id;
        let client: Box<dyn Provider + Send + Sync> = Box::new(Tunein::new());
        let result = client
            .get_station(station_id)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        let station = match result {
            Some(station) => station,
            None => return Err(tonic::Status::internal("No station found")),
        };
        Ok(tonic::Response::new(GetStationDetailsResponse {
            station_link_details: vec![StationLinkDetails::from(station)],
        }))
    }
}
