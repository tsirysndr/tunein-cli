use radiobrowser::ApiStation;

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
