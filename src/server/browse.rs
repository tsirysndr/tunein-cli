use std::str::FromStr;

use tunein::{types::Category, TuneInClient};
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
        self.client
            .browse(None)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;
        Ok(tonic::Response::new(GetCategoriesResponse {}))
    }

    async fn browse_category(
        &self,
        request: tonic::Request<BrowseCategoryRequest>,
    ) -> Result<tonic::Response<BrowseCategoryResponse>, tonic::Status> {
        let req = request.into_inner();
        let category_id = req.category_id;
        let results = match Category::from_str(&category_id) {
            Ok(category) => self
                .client
                .browse(Some(category))
                .await
                .map_err(|e| tonic::Status::internal(e.to_string()))?,
            Err(e) => return Err(tonic::Status::internal(e.to_string())),
        };
        Ok(tonic::Response::new(BrowseCategoryResponse {}))
    }

    async fn get_station_details(
        &self,
        request: tonic::Request<GetStationDetailsRequest>,
    ) -> Result<tonic::Response<GetStationDetailsResponse>, tonic::Status> {
        Ok(tonic::Response::new(GetStationDetailsResponse {}))
    }
}
