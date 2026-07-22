use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: i64,
    pub name: String,
    pub artist_id: i64,
    pub year: Option<i32>,
    pub artwork_path: Option<PathBuf>,
    pub added_at: DateTime<Utc>,
}
