//! search3 — songs ride signal-search's FTS; artist/album matches are
//! in-memory contains over the (small) summary lists.

use serde_json::json;

use crate::dto::{to_value, AlbumID3, ArtistID3, Child};
use crate::envelope::{ApiError, HandlerResult};
use crate::handlers::{durations_map, name_maps};
use crate::params::Params;
use crate::Ctx;

const DEFAULT_COUNT: u32 = 20;
const MAX_COUNT: u32 = 500;

pub(crate) async fn search3(ctx: &Ctx, params: &Params) -> HandlerResult {
    let query = params.require("query")?.trim().trim_matches('"');

    // Symfonium probes with an empty query on full-sync; answer politely
    if query.is_empty() {
        return Ok(Some((
            "searchResult3",
            json!({ "artist": [], "album": [], "song": [] }),
        )));
    }

    let song_count = params
        .get_u32("songCount")
        .unwrap_or(DEFAULT_COUNT)
        .min(MAX_COUNT);
    let artist_count = params
        .get_u32("artistCount")
        .unwrap_or(DEFAULT_COUNT)
        .min(MAX_COUNT) as usize;
    let album_count = params
        .get_u32("albumCount")
        .unwrap_or(DEFAULT_COUNT)
        .min(MAX_COUNT) as usize;
    let artist_offset = params.get_u32("artistOffset").unwrap_or(0) as usize;
    let album_offset = params.get_u32("albumOffset").unwrap_or(0) as usize;

    let tracks = signal_search::search(&ctx.db, query, song_count)
        .await
        .map_err(|err| ApiError::generic(format!("search failed: {err}")))?;
    let (artists_map, albums_map) = name_maps(ctx).await?;
    let song: Vec<Child> = tracks
        .iter()
        .map(|t| Child::from_track(t, &artists_map, &albums_map))
        .collect();

    let needle = query.to_lowercase();
    let artist: Vec<ArtistID3> = ctx
        .db
        .artists()
        .list()
        .await
        .map_err(ApiError::db)?
        .iter()
        .filter(|a| a.name.to_lowercase().contains(&needle))
        .skip(artist_offset)
        .take(artist_count)
        .map(ArtistID3::from_summary)
        .collect();

    let durations = durations_map(ctx).await?;
    let album: Vec<AlbumID3> = ctx
        .db
        .albums()
        .list()
        .await
        .map_err(ApiError::db)?
        .iter()
        .filter(|a| a.name.to_lowercase().contains(&needle))
        .skip(album_offset)
        .take(album_count)
        .map(|a| AlbumID3::from_summary(a, &durations))
        .collect();

    Ok(Some((
        "searchResult3",
        json!({
            "artist": to_value(artist),
            "album": to_value(album),
            "song": to_value(song),
        }),
    )))
}
