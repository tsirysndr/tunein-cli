pub mod api {
    #[path = ""]
    pub mod tunein {
        use tunein::types::CategoryDetails;

        use super::objects::v1alpha1::{Category, Station, StationLinkDetails};

        #[path = "tunein.v1alpha1.rs"]
        pub mod v1alpha1;
        impl From<CategoryDetails> for Category {
            fn from(category: CategoryDetails) -> Self {
                Self {
                    id: category.guide_id.unwrap_or_default(),
                    name: category.text,
                    stations: category
                        .children
                        .map(|c| {
                            c.into_iter()
                                .map(|x| Station {
                                    id: x.guide_id.unwrap_or_default(),
                                    name: x.text,
                                    playing: x.playing.unwrap_or_default(),
                                })
                                .collect()
                        })
                        .unwrap_or(vec![]),
                }
            }
        }

        impl From<tunein::types::Station> for Category {
            fn from(s: tunein::types::Station) -> Self {
                Self {
                    id: s.guide_id.unwrap_or_default(),
                    name: s.text,
                    stations: s
                        .children
                        .map(|c| {
                            c.into_iter()
                                .map(|x| Station {
                                    id: x.guide_id.unwrap_or_default(),
                                    name: x.text,
                                    playing: x.playing.unwrap_or_default(),
                                })
                                .collect()
                        })
                        .unwrap_or(vec![]),
                }
            }
        }

        impl From<tunein::types::StationLinkDetails> for StationLinkDetails {
            fn from(s: tunein::types::StationLinkDetails) -> Self {
                Self {
                    bitrate: s.bitrate,
                    element: s.element,
                    is_ad_clipped_content_enabled: s.is_ad_clipped_content_enabled,
                    is_direct: s.is_direct,
                    is_hls_advanced: s.is_hls_advanced,
                    live_seek_stream: s.live_seek_stream,
                    media_type: s.media_type,
                    player_height: s.player_height,
                    player_width: s.player_width,
                    playlist_type: s.playlist_type.unwrap_or_default(),
                    position: s.position,
                    reliability: s.reliability,
                    url: s.url,
                }
            }
        }
    }

    #[path = ""]
    pub mod objects {
        #[path = "objects.v1alpha1.rs"]
        pub mod v1alpha1;
    }
}
