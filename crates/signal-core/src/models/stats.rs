use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaySource {
    Queue,
    Playlist,
    Album,
    Search,
    /// Played through the embedded `OpenSubsonic` server (e.g. Symfonium).
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayEvent {
    pub id: i64,
    pub track_id: i64,
    pub started_at: DateTime<Utc>,
    pub ms_played: u64,
    pub completed: bool,
    pub skipped: bool,
    pub source: PlaySource,
}
