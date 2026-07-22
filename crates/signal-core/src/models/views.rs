//! Read-model DTOs for list/detail IPC responses (joined + aggregated rows).

use serde::{Deserialize, Serialize};

use crate::models::Track;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSummary {
    pub id: i64,
    pub name: String,
    pub artist_id: i64,
    pub artist_name: String,
    pub year: Option<i32>,
    pub artwork_path: Option<String>,
    pub track_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistSummary {
    pub id: i64,
    pub name: String,
    pub album_count: u32,
    pub track_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub album: AlbumSummary,
    pub tracks: Vec<Track>,
}
