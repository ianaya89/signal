//! Playlists: static ones are fully editable; smart playlists are visible
//! but read-only (their rules live in Signal).

use std::collections::HashMap;

use serde_json::json;

use signal_subsonic_types::Child;

use crate::dto::{child_from_track, playlist_attrs, to_value};
use crate::envelope::{ApiError, HandlerResult};
use crate::handlers::name_maps;
use crate::ids::Sid;
use crate::params::Params;
use crate::Ctx;

const READ_ONLY_SMART: &str = "smart playlists are read-only over the server API";

/// `(smart, id)` → `(created_at, updated_at)`.
type Stamps = HashMap<(bool, i64), (String, String)>;

async fn stamps(ctx: &Ctx) -> Result<Stamps, ApiError> {
    Ok(ctx
        .db
        .playlists()
        .timestamps()
        .await
        .map_err(ApiError::db)?
        .into_iter()
        .map(|(id, smart, created, updated)| ((smart, id), (created, updated)))
        .collect())
}

pub(crate) async fn list(ctx: &Ctx) -> HandlerResult {
    let summaries = ctx.db.playlists().list().await.map_err(ApiError::db)?;
    let stamps = stamps(ctx).await?;
    let playlist: Vec<signal_subsonic_types::Playlist> = summaries
        .iter()
        .map(|p| {
            let sid = if p.smart {
                Sid::SmartPlaylist(p.id)
            } else {
                Sid::Playlist(p.id)
            };
            playlist_attrs(
                &sid,
                &p.name,
                p.track_count as usize,
                0,
                stamps.get(&(p.smart, p.id)),
            )
        })
        .collect();
    Ok(Some((
        "playlists",
        json!({ "playlist": to_value(playlist) }),
    )))
}

pub(crate) async fn get(ctx: &Ctx, params: &Params) -> HandlerResult {
    let sid =
        Sid::parse(params.require("id")?).ok_or_else(|| ApiError::not_found("no such playlist"))?;
    let (id, smart) = match sid {
        Sid::Playlist(id) => (id, false),
        Sid::SmartPlaylist(id) => (id, true),
        _ => return Err(ApiError::not_found("no such playlist")),
    };

    let name = ctx
        .db
        .playlists()
        .name(id, smart)
        .await
        .map_err(ApiError::db)?
        .ok_or_else(|| ApiError::not_found("no such playlist"))?;
    let tracks = if smart {
        ctx.db
            .playlists()
            .resolve_smart(id)
            .await
            .map_err(ApiError::db)?
    } else {
        ctx.db.playlists().tracks(id).await.map_err(ApiError::db)?
    };
    let maps = name_maps(ctx).await?;

    let duration_secs = tracks.iter().map(|t| t.duration_ms / 1_000).sum();
    let entry: Vec<Child> = tracks.iter().map(|t| child_from_track(t, &maps)).collect();

    let stamps = stamps(ctx).await?;
    let mut payload = playlist_attrs(
        &sid,
        &name,
        entry.len(),
        duration_secs,
        stamps.get(&(smart, id)),
    );
    payload.entry = entry;
    Ok(Some(("playlist", to_value(payload))))
}

pub(crate) async fn create(ctx: &Ctx, params: &Params) -> HandlerResult {
    let name = params.require("name")?;
    let track_ids: Vec<i64> = params
        .get_all("songId")
        .into_iter()
        .filter_map(|raw| match Sid::parse(raw) {
            Some(Sid::Track(id)) => Some(id),
            _ => None,
        })
        .collect();

    let id = ctx
        .db
        .playlists()
        .create(name)
        .await
        .map_err(ApiError::db)?;
    if !track_ids.is_empty() {
        ctx.db
            .playlists()
            .add_tracks(id, &track_ids)
            .await
            .map_err(ApiError::db)?;
    }

    // 1.14+ answers with the full playlist entity
    let created = Params::parse(&format!("id={}", Sid::Playlist(id)));
    get(ctx, &created).await
}

pub(crate) async fn update(ctx: &Ctx, params: &Params) -> HandlerResult {
    let sid = Sid::parse(params.require("playlistId")?)
        .ok_or_else(|| ApiError::not_found("no such playlist"))?;
    let id = match sid {
        Sid::Playlist(id) => id,
        Sid::SmartPlaylist(_) => return Err(ApiError::not_authorized(READ_ONLY_SMART)),
        _ => return Err(ApiError::not_found("no such playlist")),
    };

    if let Some(name) = params.get("name") {
        ctx.db
            .playlists()
            .rename(id, name)
            .await
            .map_err(ApiError::db)?;
    }

    let to_add: Vec<i64> = params
        .get_all("songIdToAdd")
        .into_iter()
        .filter_map(|raw| match Sid::parse(raw) {
            Some(Sid::Track(track_id)) => Some(track_id),
            _ => None,
        })
        .collect();
    if !to_add.is_empty() {
        ctx.db
            .playlists()
            .add_tracks(id, &to_add)
            .await
            .map_err(ApiError::db)?;
    }

    let mut indices: Vec<usize> = params
        .get_all("songIndexToRemove")
        .into_iter()
        .filter_map(|raw| raw.parse().ok())
        .collect();
    if !indices.is_empty() {
        let current = ctx.db.playlists().tracks(id).await.map_err(ApiError::db)?;
        // playlist entries are unique (add_tracks skips duplicates), so
        // index → track_id is unambiguous; remove back-to-front regardless
        indices.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices {
            if let Some(track) = current.get(index) {
                ctx.db
                    .playlists()
                    .remove_track(id, track.id)
                    .await
                    .map_err(ApiError::db)?;
            }
        }
    }
    Ok(None)
}

pub(crate) async fn delete(ctx: &Ctx, params: &Params) -> HandlerResult {
    match Sid::parse(params.require("id")?) {
        Some(Sid::Playlist(id)) => {
            ctx.db.playlists().delete(id).await.map_err(ApiError::db)?;
            Ok(None)
        }
        Some(Sid::SmartPlaylist(_)) => Err(ApiError::not_authorized(READ_ONLY_SMART)),
        _ => Err(ApiError::not_found("no such playlist")),
    }
}
