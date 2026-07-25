use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

/// Writes an EXTM3U file with absolute paths for the given playlist.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_export_m3u(
    state: State<'_, AppState>,
    playlist_id: i64,
    smart: bool,
    dest_path: String,
) -> Result<u32, SignalError> {
    let tracks = if smart {
        state.db.playlists().resolve_smart(playlist_id).await
    } else {
        state.db.playlists().tracks(playlist_id).await
    }
    .db_err()?;

    if tracks.is_empty() {
        return Err(SignalError::Db("playlist is empty".into()));
    }

    let mut body = String::from("#EXTM3U\n");
    for track in &tracks {
        use std::fmt::Write as _;
        let secs = track.duration_ms / 1000;
        let _ = writeln!(body, "#EXTINF:{secs},{}", track.title);
        let _ = writeln!(body, "{}", track.technical.file_path.display());
    }

    tokio::fs::write(&dest_path, body)
        .await
        .map_err(|e| SignalError::Io(format!("write m3u: {e}")))?;
    Ok(u32::try_from(tracks.len()).unwrap_or_default())
}

/// Consistent snapshot of the database via VACUUM INTO.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_backup(
    state: State<'_, AppState>,
    dest_path: String,
) -> Result<(), SignalError> {
    // VACUUM INTO refuses to overwrite
    if std::path::Path::new(&dest_path).exists() {
        tokio::fs::remove_file(&dest_path)
            .await
            .map_err(|e| SignalError::Io(e.to_string()))?;
    }
    sqlx::query("VACUUM INTO ?1")
        .bind(&dest_path)
        .execute(state.db.inner())
        .await
        .db_err()?;
    tracing::info!(dest = %dest_path, "database backed up");
    Ok(())
}
