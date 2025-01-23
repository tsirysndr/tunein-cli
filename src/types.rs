use radiobrowser::ApiStation;
use tunein::types::{SearchResult, StationLinkDetails};

#[derive(Debug, Clone)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub codec: String,
    pub bitrate: u32,
    pub stream_url: String,
}

impl From<ApiStation> for Station {
    fn from(station: ApiStation) -> Station {
        Station {
            id: station.stationuuid,
            name: station.name,
            codec: station.codec,
            bitrate: station.bitrate,
            stream_url: station.url_resolved,
        }
    }
}

impl From<SearchResult> for Station {
    fn from(result: SearchResult) -> Station {
        Station {
            id: result.guide_id.unwrap_or_default(),
            name: result.text,
            bitrate: result
                .bitrate
                .unwrap_or("0".to_string())
                .parse()
                .unwrap_or_default(),
            codec: "".to_string(),
            stream_url: "".to_string(),
        }
    }
}

impl From<StationLinkDetails> for Station {
    fn from(details: StationLinkDetails) -> Station {
        Station {
            id: "".to_string(),
            name: "".to_string(),
            bitrate: details.bitrate,
            stream_url: details.url,
            codec: details.media_type.to_uppercase(),
        }
    }
}
