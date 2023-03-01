#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Station {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub playing: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StationLinkDetails {
    #[prost(uint32, tag = "1")]
    pub bitrate: u32,
    #[prost(string, tag = "2")]
    pub element: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub is_ad_clipped_content_enabled: ::prost::alloc::string::String,
    #[prost(bool, tag = "4")]
    pub is_direct: bool,
    #[prost(string, tag = "5")]
    pub is_hls_advanced: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub live_seek_stream: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub media_type: ::prost::alloc::string::String,
    #[prost(uint32, tag = "8")]
    pub player_height: u32,
    #[prost(uint32, tag = "9")]
    pub player_width: u32,
    #[prost(string, tag = "10")]
    pub playlist_type: ::prost::alloc::string::String,
    #[prost(uint32, tag = "11")]
    pub position: u32,
    #[prost(uint32, tag = "12")]
    pub reliability: u32,
    #[prost(string, tag = "13")]
    pub url: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Category {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub stations: ::prost::alloc::vec::Vec<Station>,
}
