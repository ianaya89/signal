use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub artist_id: i64,
    pub album_id: i64,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub duration_ms: u64,
    pub rating: Option<u8>,
    pub favorite: bool,
    pub play_count: u32,
    pub skip_count: u32,
    pub added_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub last_played_at: Option<DateTime<Utc>>,
    pub technical: TrackTechnical,
}

/// Everything the audio inspector shows. Extracted by `signal-scanner` (lofty)
/// and cross-checked against what the player reports at playback time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackTechnical {
    pub codec: String,
    pub container: String,
    pub bitrate_kbps: u32,
    pub bit_depth: Option<u8>,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub replaygain_track_gain: Option<f64>,
    pub replaygain_album_gain: Option<f64>,
    pub peak: Option<f64>,
    pub dr_score: Option<f64>,
    pub encoder: Option<String>,
    pub file_path: PathBuf,
    pub file_size_bytes: u64,
    pub md5: Option<String>,
}
