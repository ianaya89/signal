//! Library metadata edits. Database-only: audio file tags are never
//! rewritten — a rescan of untouched files will skip them (path match),
//! so edits survive; a full reset re-imports tag values.

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

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_rename_artist(
    state: State<'_, AppState>,
    artist_id: i64,
    name: String,
) -> Result<(), SignalError> {
    state
        .db
        .artists()
        .rename(artist_id, valid_name(&name)?)
        .await
        .db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_rename_album(
    state: State<'_, AppState>,
    album_id: i64,
    name: String,
) -> Result<(), SignalError> {
    state
        .db
        .albums()
        .rename(album_id, valid_name(&name)?)
        .await
        .db_err()
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
