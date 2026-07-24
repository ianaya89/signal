use serde::Serialize;
use signal_core::{SignalError, Track};
use signal_db::PlaylistSummary;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_list(
    state: State<'_, AppState>,
) -> Result<Vec<PlaylistSummary>, SignalError> {
    state.db.playlists().list().await.db_err()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub id: i64,
    pub name: String,
    pub smart: bool,
    pub tracks: Vec<Track>,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_get(
    state: State<'_, AppState>,
    playlist_id: i64,
    smart: bool,
) -> Result<PlaylistDetail, SignalError> {
    let name = state
        .db
        .playlists()
        .name(playlist_id, smart)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Db(format!("playlist {playlist_id} not found")))?;

    let tracks = if smart {
        state.db.playlists().resolve_smart(playlist_id).await
    } else {
        state.db.playlists().tracks(playlist_id).await
    }
    .db_err()?;

    Ok(PlaylistDetail {
        id: playlist_id,
        name,
        smart,
        tracks,
    })
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_create(state: State<'_, AppState>, name: String) -> Result<i64, SignalError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(SignalError::Db("playlist name is empty".into()));
    }
    state.db.playlists().create(trimmed).await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_delete(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<(), SignalError> {
    state.db.playlists().delete(playlist_id).await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_add_tracks(
    state: State<'_, AppState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<(), SignalError> {
    state
        .db
        .playlists()
        .add_tracks(playlist_id, &track_ids)
        .await
        .db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_remove_track(
    state: State<'_, AppState>,
    playlist_id: i64,
    track_id: i64,
) -> Result<(), SignalError> {
    state
        .db
        .playlists()
        .remove_track(playlist_id, track_id)
        .await
        .db_err()
}

/// Snapshots the current queue into a new playlist (doc 09's `w` — "write").
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_save_as_playlist(
    state: State<'_, AppState>,
    name: String,
) -> Result<i64, SignalError> {
    let entries = state.db.queue().list().await.db_err()?;
    if entries.is_empty() {
        return Err(SignalError::Db("queue is empty".into()));
    }
    let id = playlist_create(state.clone(), name).await?;
    let track_ids: Vec<i64> = entries.iter().map(|e| e.track.id).collect();
    state
        .db
        .playlists()
        .add_tracks(id, &track_ids)
        .await
        .db_err()?;
    Ok(id)
}
