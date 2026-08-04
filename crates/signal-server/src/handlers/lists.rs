//! Album lists, random songs, genre browsing, starred.

use std::collections::HashSet;

use serde_json::json;

use signal_subsonic_types::{AlbumID3, Child};

use crate::dto::{album_from_summary, child_from_track, to_value};
use crate::envelope::{ApiError, HandlerResult};
use crate::handlers::{durations_map, name_maps};
use crate::params::Params;
use crate::Ctx;

const DEFAULT_SIZE: u32 = 10;
const MAX_SIZE: u32 = 500;

fn page(size: Option<u32>, offset: Option<u32>) -> (usize, usize) {
    let size = size.unwrap_or(DEFAULT_SIZE).min(MAX_SIZE) as usize;
    let offset = offset.unwrap_or(0) as usize;
    (size, offset)
}

/// In-place Fisher-Yates with a time-seeded LCG — same no-RNG-dependency
/// stance as `PlayContext::peek_next`'s shuffle jitter.
fn shuffle<T>(items: &mut [T]) {
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0x5_DEEC_E66D, |d| u64::from(d.subsec_nanos()) | 1);
    for i in (1..items.len()).rev() {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        #[allow(clippy::cast_possible_truncation)]
        let j = (seed >> 33) as usize % (i + 1);
        items.swap(i, j);
    }
}

pub(crate) async fn album_list2(ctx: &Ctx, params: &Params) -> HandlerResult {
    let list_type = params.require("type")?;
    let mut albums = ctx.db.albums().list().await.map_err(ApiError::db)?;
    let durations = durations_map(ctx).await?;

    match list_type {
        "alphabeticalByName" => {
            albums.sort_by_key(|a| a.name.to_lowercase());
        }
        // repo order is already artist, year, name
        "alphabeticalByArtist" => {}
        "newest" => {
            albums.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        }
        "frequent" | "recent" => {
            let stats: std::collections::HashMap<i64, (i64, Option<String>)> = ctx
                .db
                .albums()
                .play_stats()
                .await
                .map_err(ApiError::db)?
                .into_iter()
                .map(|(id, plays, last)| (id, (plays, last)))
                .collect();
            if list_type == "frequent" {
                albums.retain(|a| stats.get(&a.id).is_some_and(|(plays, _)| *plays > 0));
                albums.sort_by_key(|a| std::cmp::Reverse(stats.get(&a.id).map_or(0, |s| s.0)));
            } else {
                albums.retain(|a| stats.get(&a.id).is_some_and(|(_, last)| last.is_some()));
                albums.sort_by(|a, b| {
                    let last =
                        |x: &signal_core::AlbumSummary| stats.get(&x.id).and_then(|s| s.1.clone());
                    last(b).cmp(&last(a))
                });
            }
        }
        "random" => shuffle(&mut albums),
        "byYear" => {
            let from = params
                .get_i64("fromYear")
                .ok_or_else(|| ApiError::missing_param("fromYear"))?;
            let to = params
                .get_i64("toYear")
                .ok_or_else(|| ApiError::missing_param("toYear"))?;
            let (lo, hi) = (from.min(to), from.max(to));
            albums.retain(|a| {
                a.year
                    .is_some_and(|y| i64::from(y) >= lo && i64::from(y) <= hi)
            });
            albums.sort_by_key(|a| a.year);
            if from > to {
                albums.reverse();
            }
        }
        "byGenre" => {
            let wanted = params.require("genre")?;
            let genre_id = ctx
                .db
                .artists()
                .list_genres()
                .await
                .map_err(ApiError::db)?
                .into_iter()
                .find(|(_, name, _)| name.eq_ignore_ascii_case(wanted))
                .map(|(id, _, _)| id)
                .ok_or_else(|| ApiError::not_found("no such genre"))?;
            let track_albums: HashSet<i64> = ctx
                .db
                .tracks()
                .list_by_genre(genre_id)
                .await
                .map_err(ApiError::db)?
                .iter()
                .map(|t| t.album_id)
                .collect();
            albums.retain(|a| track_albums.contains(&a.id));
        }
        "starred" => albums.clear(), // album favorites don't exist in Signal
        other => {
            return Err(ApiError::generic(format!(
                "unsupported album list type '{other}'"
            )))
        }
    }

    let (size, offset) = page(params.get_u32("size"), params.get_u32("offset"));
    let album: Vec<AlbumID3> = albums
        .iter()
        .skip(offset)
        .take(size)
        .map(|a| album_from_summary(a, &durations))
        .collect();
    Ok(Some(("albumList2", json!({ "album": to_value(album) }))))
}

pub(crate) async fn random_songs(ctx: &Ctx, params: &Params) -> HandlerResult {
    let size = params.get_u32("size").unwrap_or(DEFAULT_SIZE).min(MAX_SIZE);
    let tracks = ctx.db.tracks().random(size).await.map_err(ApiError::db)?;
    let maps = name_maps(ctx).await?;
    let song: Vec<Child> = tracks.iter().map(|t| child_from_track(t, &maps)).collect();
    Ok(Some(("randomSongs", json!({ "song": to_value(song) }))))
}

pub(crate) async fn songs_by_genre(ctx: &Ctx, params: &Params) -> HandlerResult {
    let wanted = params.require("genre")?;
    let genre_id = ctx
        .db
        .artists()
        .list_genres()
        .await
        .map_err(ApiError::db)?
        .into_iter()
        .find(|(_, name, _)| name.eq_ignore_ascii_case(wanted))
        .map(|(id, _, _)| id)
        .ok_or_else(|| ApiError::not_found("no such genre"))?;
    let tracks = ctx
        .db
        .tracks()
        .list_by_genre(genre_id)
        .await
        .map_err(ApiError::db)?;
    let maps = name_maps(ctx).await?;

    let (size, offset) = page(params.get_u32("count"), params.get_u32("offset"));
    let song: Vec<Child> = tracks
        .iter()
        .skip(offset)
        .take(size)
        .map(|t| child_from_track(t, &maps))
        .collect();
    Ok(Some(("songsByGenre", json!({ "song": to_value(song) }))))
}

pub(crate) async fn starred2(ctx: &Ctx) -> HandlerResult {
    // strict ♥ favorites only — Subsonic stars map to favorites, and loved
    // additionally includes 4-5★ ratings which have their own field
    let loved = ctx.db.tracks().list_loved().await.map_err(ApiError::db)?;
    let maps = name_maps(ctx).await?;
    let song: Vec<Child> = loved
        .iter()
        .filter(|t| t.favorite)
        .map(|t| child_from_track(t, &maps))
        .collect();
    Ok(Some(("starred2", json!({ "song": to_value(song) }))))
}
