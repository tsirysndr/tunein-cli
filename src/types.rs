use radiobrowser::ApiStation;
use tunein::types::{SearchResult, StationLinkDetails};

#[derive(Debug, Clone)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub codec: String,
    pub bitrate: u32,
    pub stream_url: String,
    pub playing: Option<String>,
}

impl From<ApiStation> for Station {
    fn from(station: ApiStation) -> Station {
        Station {
            id: station.stationuuid,
            name: station.name,
            codec: station.codec,
            bitrate: station.bitrate,
            stream_url: station.url_resolved,
            playing: None,
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
            codec: Default::default(),
            stream_url: Default::default(),
            playing: result.subtext,
        }
    }
}

impl From<Box<SearchResult>> for Station {
    fn from(result: Box<SearchResult>) -> Station {
        Station {
            id: result.guide_id.unwrap_or_default(),
            name: result.text,
            bitrate: result
                .bitrate
                .unwrap_or("0".to_string())
                .parse()
                .unwrap_or_default(),
            codec: Default::default(),
            stream_url: Default::default(),
            playing: None,
        }
    }
}

impl From<StationLinkDetails> for Station {
    fn from(details: StationLinkDetails) -> Station {
        Station {
            id: Default::default(),
            name: Default::default(),
            bitrate: details.bitrate,
            stream_url: details.url,
            codec: details.media_type.to_uppercase(),
            playing: None,
        }
    }
}

impl From<tunein::types::Station> for Station {
    fn from(st: tunein::types::Station) -> Station {
        Station {
            id: st.guide_id.unwrap_or_default(),
            name: st.text,
            bitrate: st
                .bitrate
                .unwrap_or("0".to_string())
                .parse()
                .unwrap_or_default(),
            stream_url: Default::default(),
            codec: st.formats.unwrap_or_default().to_uppercase(),
            playing: st.playing,
        }
    }
}

impl From<Box<tunein::types::Station>> for Station {
    fn from(st: Box<tunein::types::Station>) -> Station {
        Station {
            id: st.guide_id.unwrap_or_default(),
            name: st.text,
            bitrate: st
                .bitrate
                .unwrap_or("0".to_string())
                .parse()
                .unwrap_or_default(),
            stream_url: Default::default(),
            codec: st.formats.unwrap_or_default().to_uppercase(),
            playing: st.playing,
        }
    }
}

impl From<tunein::types::CategoryDetails> for Station {
    fn from(ct: tunein::types::CategoryDetails) -> Station {
        Station {
            id: ct.guide_id.unwrap_or_default(),
            name: ct.text,
            bitrate: 0,
            stream_url: Default::default(),
            codec: Default::default(),
            playing: None,
        }
    }
}
