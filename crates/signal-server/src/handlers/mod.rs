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

/// Artist/album id→name maps, loaded once per request — Child mapping for a
/// 500-song response must not fan out into per-track queries.
pub(crate) async fn name_maps(
    ctx: &Ctx,
) -> Result<(HashMap<i64, String>, HashMap<i64, String>), ApiError> {
    let artists = ctx
        .db
        .artists()
        .name_map()
        .await
        .map_err(ApiError::db)?
        .into_iter()
        .collect();
    let albums = ctx
        .db
        .albums()
        .name_map()
        .await
        .map_err(ApiError::db)?
        .into_iter()
        .collect();
    Ok((artists, albums))
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
