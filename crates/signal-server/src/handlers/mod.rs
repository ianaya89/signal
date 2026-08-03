pub(crate) mod annotation;
pub(crate) mod browsing;
pub(crate) mod lists;
pub(crate) mod media;
pub(crate) mod playlists;
pub(crate) mod search;
pub(crate) mod system;

use std::collections::HashMap;

use crate::envelope::ApiError;
use crate::Ctx;

/// Per-request lookup maps so Child mapping for a 500-song response never
/// fans out into per-track queries.
pub(crate) struct NameMaps {
    pub artists: HashMap<i64, String>,
    pub albums: HashMap<i64, String>,
    pub genres: HashMap<i64, String>,
}

pub(crate) async fn name_maps(ctx: &Ctx) -> Result<NameMaps, ApiError> {
    Ok(NameMaps {
        artists: ctx
            .db
            .artists()
            .name_map()
            .await
            .map_err(ApiError::db)?
            .into_iter()
            .collect(),
        albums: ctx
            .db
            .albums()
            .name_map()
            .await
            .map_err(ApiError::db)?
            .into_iter()
            .collect(),
        genres: ctx
            .db
            .tracks()
            .genre_map()
            .await
            .map_err(ApiError::db)?
            .into_iter()
            .collect(),
    })
}

pub(crate) async fn durations_map(ctx: &Ctx) -> Result<HashMap<i64, i64>, ApiError> {
    Ok(ctx
        .db
        .albums()
        .durations()
        .await
        .map_err(ApiError::db)?
        .into_iter()
        .collect())
}
