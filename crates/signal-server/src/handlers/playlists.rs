//! Playlists: static ones are fully editable; smart playlists are visible
//! but read-only (their rules live in Signal).

use chrono::{SecondsFormat, Utc};
use serde_json::json;

use crate::dto::{to_value, Child};
use crate::envelope::{ApiError, HandlerResult};
use crate::handlers::name_maps;
use crate::ids::Sid;
use crate::params::Params;
use crate::Ctx;

const READ_ONLY_SMART: &str = "smart playlists are read-only over the server API";

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn playlist_attrs(id: &Sid, name: &str, song_count: usize, duration_secs: u64) -> serde_json::Value {
    // changed = now on every call: forces clients to re-sync rather than
    // trust a staleness signal Signal doesn't track yet
    json!({
        "id": id.to_string(),
        "name": name,
        "songCount": song_count,
        "duration": duration_secs,
        "public": false,
        "owner": "signal",
        "created": now_iso(),
        "changed": now_iso(),
    })
}

pub(crate) async fn list(ctx: &Ctx) -> HandlerResult {
    let summaries = ctx.db.playlists().list().await.map_err(ApiError::db)?;
    let playlist: Vec<serde_json::Value> = summaries
        .iter()
        .map(|p| {
            let sid = if p.smart {
                Sid::SmartPlaylist(p.id)
            } else {
                Sid::Playlist(p.id)
            };
            playlist_attrs(&sid, &p.name, p.track_count as usize, 0)
        })
        .collect();
    Ok(Some(("playlists", json!({ "playlist": playlist }))))
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
    let (artists, albums) = name_maps(ctx).await?;

    let duration_secs = tracks.iter().map(|t| t.duration_ms / 1_000).sum();
    let entry: Vec<Child> = tracks
        .iter()
        .map(|t| Child::from_track(t, &artists, &albums))
        .collect();

    let mut payload = playlist_attrs(&sid, &name, entry.len(), duration_secs);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("entry".into(), to_value(entry));
    }
    Ok(Some(("playlist", payload)))
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
