//! Library metadata edits. Database-first: a rescan of untouched files
//! skips them (path match) so edits survive; a full reset re-imports tag
//! values. With `[library] write_tags` enabled, full-form track edits are
//! also written back into the file's own tags.

use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

fn valid_name(name: &str) -> Result<&str, SignalError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(SignalError::Db("name cannot be empty".into()));
    }
    Ok(trimmed)
}

/// Returns true when the rename merged into an existing artist.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_rename_artist(
    state: State<'_, AppState>,
    artist_id: i64,
    name: String,
) -> Result<bool, SignalError> {
    let merged = state
        .db
        .artists()
        .rename(artist_id, valid_name(&name)?)
        .await
        .db_err()?;
    if merged {
        tracing::info!(artist_id, name, "artist merged into existing");
    }
    Ok(merged)
}

/// Returns true when the rename merged into an existing album.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_rename_album(
    state: State<'_, AppState>,
    album_id: i64,
    name: String,
) -> Result<bool, SignalError> {
    let merged = state
        .db
        .albums()
        .rename(album_id, valid_name(&name)?)
        .await
        .db_err()?;
    if merged {
        tracing::info!(album_id, name, "album merged into existing");
    }
    Ok(merged)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_rename_track(
    state: State<'_, AppState>,
    track_id: i64,
    title: String,
) -> Result<(), SignalError> {
    state
        .db
        .tracks()
        .rename(track_id, valid_name(&title)?)
        .await
        .db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn track_set_rating(
    state: State<'_, AppState>,
    track_id: i64,
    rating: u8,
) -> Result<(), SignalError> {
    state
        .db
        .tracks()
        .set_rating(track_id, rating)
        .await
        .db_err()
}

/// Toggles and returns the new favorite state.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn track_toggle_favorite(
    state: State<'_, AppState>,
    track_id: i64,
) -> Result<bool, SignalError> {
    let track = state
        .db
        .tracks()
        .get(track_id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Db(format!("track {track_id} not found")))?;
    let next = !track.favorite;
    state
        .db
        .tracks()
        .set_favorite(track_id, next)
        .await
        .db_err()?;
    Ok(next)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackMetaArgs {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i64>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub genre: Option<String>,
}

/// Full metadata edit from the UI form. Database-first; with
/// `[library] write_tags` enabled the file's own tags follow.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn track_update_metadata(
    state: State<'_, AppState>,
    track_id: i64,
    meta: TrackMetaArgs,
) -> Result<(), SignalError> {
    let update = signal_db::TrackMetadataUpdate {
        title: valid_name(&meta.title)?.to_string(),
        artist_name: valid_name(&meta.artist)?.to_string(),
        album_name: meta.album,
        year: meta.year,
        track_no: meta.track_no,
        disc_no: meta.disc_no,
        genre: meta.genre,
    };
    state
        .db
        .tracks()
        .update_metadata(track_id, &update)
        .await
        .db_err()?;

    if state.write_tags.load(std::sync::atomic::Ordering::Relaxed) {
        if let Ok(Some(track)) = state.db.tracks().get(track_id).await {
            let path = track.technical.file_path.clone();
            let meta = signal_scanner::WriteBack {
                title: update.title,
                artist: update.artist_name,
                album: {
                    let album = update.album_name.trim();
                    if album.is_empty() {
                        None
                    } else {
                        Some(album.to_string())
                    }
                },
                year: update.year.and_then(|y| u32::try_from(y).ok()),
                track_no: update.track_no.and_then(|n| u32::try_from(n).ok()),
                disc_no: update.disc_no.and_then(|n| u32::try_from(n).ok()),
                genre: update.genre,
            };
            let result =
                tokio::task::spawn_blocking(move || signal_scanner::write_back(&path, &meta)).await;
            match result {
                Ok(Ok(())) => tracing::info!(track_id, "tags written back to file"),
                Ok(Err(err)) => tracing::warn!(track_id, "tag write-back failed: {err}"),
                Err(err) => tracing::warn!(track_id, "tag write-back join failed: {err}"),
            }
        }
    }
    Ok(())
}

/// Returns true when the edit merged into an existing album.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn album_update_info(
    state: State<'_, AppState>,
    album_id: i64,
    name: String,
    artist: String,
    year: Option<i64>,
) -> Result<bool, SignalError> {
    let merged = state
        .db
        .albums()
        .update_info(album_id, valid_name(&name)?, valid_name(&artist)?, year)
        .await
        .db_err()?;
    if merged {
        tracing::info!(album_id, name, "album edit merged into existing");
    }
    Ok(merged)
}

/// Copies a user-picked image into the artwork cache and points the album
/// at it (original file stays where it is).
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_set_album_artwork(
    state: State<'_, AppState>,
    album_id: i64,
    source_path: String,
) -> Result<(), SignalError> {
    let source = std::path::PathBuf::from(&source_path);
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let ext = match ext.as_deref() {
        Some("jpg" | "jpeg") => "jpg",
        Some("png") => "png",
        _ => {
            return Err(SignalError::Db(
                "artwork must be a .jpg or .png image".into(),
            ))
        }
    };

    let dir = state.config.cache_dir.join("artwork");
    let dest = dir.join(format!("album_{album_id}.{ext}"));
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| SignalError::Io(e.to_string()))?;
    tokio::fs::copy(&source, &dest)
        .await
        .map_err(|e| SignalError::Io(format!("copy artwork: {e}")))?;

    state
        .db
        .albums()
        .set_artwork(album_id, &dest.to_string_lossy())
        .await
        .db_err()
}
