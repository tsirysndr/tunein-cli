use std::str::FromStr;

use tunein::{
    types,
    TuneInClient,
};
use tunein_cli::api::{
    objects::v1alpha1::{Category, StationLinkDetails},
    tunein::v1alpha1::{
        browse_service_server::BrowseService, BrowseCategoryRequest, BrowseCategoryResponse,
        GetCategoriesRequest, GetCategoriesResponse, GetStationDetailsRequest,
        GetStationDetailsResponse,
    },
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
        _request: tonic::Request<GetCategoriesRequest>,
    ) -> Result<tonic::Response<GetCategoriesResponse>, tonic::Status> {
        let result = self
            .client
            .browse(None)
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
        let categories: Vec<Category> = match types::Category::from_str(&category_id) {
            Ok(category) => {
                let results = self
                    .client
                    .browse(Some(category))
                    .await
                    .map_err(|e| tonic::Status::internal(e.to_string()))?;
                results.into_iter().map(Category::from).collect()
            }
            Err(_) => {
                let results = self
                    .client
                    .browse_by_id(&category_id)
                    .await
                    .map_err(|e| tonic::Status::internal(e.to_string()))?;
                results.into_iter().map(Category::from).collect()
            }
        };
        Ok(tonic::Response::new(BrowseCategoryResponse { categories }))
    }

    async fn get_station_details(
        &self,
        request: tonic::Request<GetStationDetailsRequest>,
    ) -> Result<tonic::Response<GetStationDetailsResponse>, tonic::Status> {
        let req = request.into_inner();
        let station_id = req.id;
        let station = self
            .client
            .get_station(&station_id)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

        Ok(tonic::Response::new(GetStationDetailsResponse {
            station_link_details: station.into_iter().map(StationLinkDetails::from).collect(),
        }))
    }
}
