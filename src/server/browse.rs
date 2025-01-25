use tunein_cli::{
    api::{
        objects::v1alpha1::{Category, Station, StationLinkDetails},
        tunein::v1alpha1::{
            browse_service_server::BrowseService, BrowseCategoryRequest, BrowseCategoryResponse,
            GetCategoriesRequest, GetCategoriesResponse, GetStationDetailsRequest,
            GetStationDetailsResponse, SearchRequest, SearchResponse,
        },
    },
    provider::radiobrowser::Radiobrowser,
};

use tunein_cli::provider::{tunein::Tunein, Provider};

#[derive(Default)]
pub struct Browse;

#[tonic::async_trait]
impl BrowseService for Browse {
    async fn get_categories(
        &self,
        request: tonic::Request<GetCategoriesRequest>,
    ) -> Result<tonic::Response<GetCategoriesResponse>, tonic::Status> {
        let req = request.into_inner();
        let provider = req.provider.as_deref();

        let client: Box<dyn Provider + Send + Sync> = match provider {
            Some("tunein") => Box::new(Tunein::new()),
            Some("radiobrowser") => Box::new(Radiobrowser::new().await),
            None => Box::new(Tunein::new()),
            _ => {
                return Err(tonic::Status::internal("Unsupported provider"));
            }
        };

        let offset = req.offset.unwrap_or(0);
        let limit = req.limit.unwrap_or(100);
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

        let offset = req.offset.unwrap_or(0);
        let limit = req.limit.unwrap_or(100);

        let provider = req.provider.as_deref();

        let client: Box<dyn Provider + Send + Sync> = match provider {
            Some("tunein") => Box::new(Tunein::new()),
            Some("radiobrowser") => Box::new(Radiobrowser::new().await),
            None => Box::new(Tunein::new()),
            _ => {
                return Err(tonic::Status::internal("Unsupported provider"));
            }
        };

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

        let provider = req.provider.as_deref();

        let client: Box<dyn Provider + Send + Sync> = match provider {
            Some("tunein") => Box::new(Tunein::new()),
            Some("radiobrowser") => Box::new(Radiobrowser::new().await),
            None => Box::new(Tunein::new()),
            _ => {
                return Err(tonic::Status::internal("Unsupported provider"));
            }
        };

        let result = client
            .get_station(station_id)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        let station = match result {
            Some(station) => station,
            None => return Err(tonic::Status::internal("No station found")),
        };
        Ok(tonic::Response::new(GetStationDetailsResponse {
            station_link_details: Some(StationLinkDetails::from(station)),
        }))
    }

    async fn search(
        &self,
        request: tonic::Request<SearchRequest>,
    ) -> Result<tonic::Response<SearchResponse>, tonic::Status> {
        let req = request.into_inner();
        let provider = req.provider.as_deref();

        let client: Box<dyn Provider + Send + Sync> = match provider {
            Some("tunein") => Box::new(Tunein::new()),
            Some("radiobrowser") => Box::new(Radiobrowser::new().await),
            None => Box::new(Tunein::new()),
            _ => {
                return Err(tonic::Status::internal("Unsupported provider"));
            }
        };

        let results = client
            .search(req.query)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;
        let station = results.into_iter().map(Station::from).collect();
        Ok(tonic::Response::new(SearchResponse { station }))
    }
}
