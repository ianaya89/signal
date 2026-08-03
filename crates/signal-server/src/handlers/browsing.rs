//! ID3 browsing: artists → albums → songs. Signal has no folder hierarchy
//! concept worth exposing, so `getMusicDirectory` stays unimplemented and
//! `getIndexes` serves the same artist buckets as `getArtists`.

use std::collections::BTreeMap;

use serde_json::json;

use crate::dto::{to_value, AlbumID3, ArtistID3, Child};
use crate::envelope::{ApiError, HandlerResult};
use crate::handlers::{durations_map, name_maps};
use crate::ids::Sid;
use crate::params::Params;
use crate::Ctx;

pub(crate) fn music_folders() -> (&'static str, serde_json::Value) {
    (
        "musicFolders",
        json!({ "musicFolder": [{ "id": 1, "name": "Signal Library" }] }),
    )
}

/// Shared by getArtists (`artists`) and getIndexes (`indexes`) — same
/// buckets, different envelope key and extra attrs.
pub(crate) async fn artists(ctx: &Ctx, key: &'static str) -> HandlerResult {
    let list = ctx.db.artists().list().await.map_err(ApiError::db)?;

    let mut buckets: BTreeMap<String, Vec<ArtistID3>> = BTreeMap::new();
    for artist in &list {
        let initial = artist
            .name
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .filter(char::is_ascii_alphabetic)
            .map_or_else(|| "#".to_owned(), String::from);
        buckets
            .entry(initial)
            .or_default()
            .push(ArtistID3::from_summary(artist));
    }
    let index: Vec<serde_json::Value> = buckets
        .into_iter()
        .map(|(name, artists)| json!({ "name": name, "artist": to_value(artists) }))
        .collect();

    let mut payload = json!({ "ignoredArticles": "", "index": index });
    if key == "indexes" {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("lastModified".into(), json!(0));
        }
    }
    Ok(Some((key, payload)))
}

pub(crate) async fn artist(ctx: &Ctx, params: &Params) -> HandlerResult {
    let Some(Sid::Artist(id)) = Sid::parse(params.require("id")?) else {
        return Err(ApiError::not_found("no such artist"));
    };
    let artist = ctx
        .db
        .artists()
        .get(id)
        .await
        .map_err(ApiError::db)?
        .ok_or_else(|| ApiError::not_found("no such artist"))?;
    let albums = ctx
        .db
        .albums()
        .list_by_artist(id)
        .await
        .map_err(ApiError::db)?;
    let durations = durations_map(ctx).await?;

    let mut payload = to_value(ArtistID3::from_summary(&artist));
    if let Some(obj) = payload.as_object_mut() {
        let albums: Vec<AlbumID3> = albums
            .iter()
            .map(|a| AlbumID3::from_summary(a, &durations))
            .collect();
        obj.insert("album".into(), to_value(albums));
    }
    Ok(Some(("artist", payload)))
}

pub(crate) async fn album(ctx: &Ctx, params: &Params) -> HandlerResult {
    let Some(Sid::Album(id)) = Sid::parse(params.require("id")?) else {
        return Err(ApiError::not_found("no such album"));
    };
    let album = ctx
        .db
        .albums()
        .get(id)
        .await
        .map_err(ApiError::db)?
        .ok_or_else(|| ApiError::not_found("no such album"))?;
    let tracks = ctx.db.albums().tracks(id).await.map_err(ApiError::db)?;
    let durations = durations_map(ctx).await?;
    let (artists, albums) = name_maps(ctx).await?;

    let mut payload = to_value(AlbumID3::from_summary(&album, &durations));
    if let Some(obj) = payload.as_object_mut() {
        let songs: Vec<Child> = tracks
            .iter()
            .map(|t| Child::from_track(t, &artists, &albums))
            .collect();
        obj.insert("song".into(), to_value(songs));
    }
    Ok(Some(("album", payload)))
}

pub(crate) async fn song(ctx: &Ctx, params: &Params) -> HandlerResult {
    let Some(Sid::Track(id)) = Sid::parse(params.require("id")?) else {
        return Err(ApiError::not_found("no such song"));
    };
    let track = ctx
        .db
        .tracks()
        .get(id)
        .await
        .map_err(ApiError::db)?
        .ok_or_else(|| ApiError::not_found("no such song"))?;
    let (artists, albums) = name_maps(ctx).await?;
    Ok(Some((
        "song",
        to_value(Child::from_track(&track, &artists, &albums)),
    )))
}

pub(crate) async fn genres(ctx: &Ctx) -> HandlerResult {
    let genres = ctx.db.artists().list_genres().await.map_err(ApiError::db)?;
    let genre: Vec<serde_json::Value> = genres
        .into_iter()
        .map(|(_, name, count)| {
            json!({ "value": name, "songCount": count, "albumCount": 0 })
        })
        .collect();
    Ok(Some(("genres", json!({ "genre": genre }))))
}
