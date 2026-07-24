use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub backend: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplayGainMode {
    Off,
    Track,
    Album,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub status: PlaybackStatus,
    pub track_id: Option<i64>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub device_id: Option<String>,
    pub replaygain: ReplayGainMode,
    pub exclusive: bool,
    pub bit_perfect: bool,
    /// Source vs actual output sample rate, when known.
    pub source_rate_hz: Option<u32>,
    pub output_rate_hz: Option<u32>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Stopped,
            track_id: None,
            position_ms: 0,
            duration_ms: 0,
            volume: 1.0,
            device_id: None,
            replaygain: ReplayGainMode::Off,
            exclusive: false,
            bit_perfect: false,
            source_rate_hz: None,
            output_rate_hz: None,
        }
    }
}
