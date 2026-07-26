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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct M3uImportResult {
    pub playlist_id: i64,
    pub name: String,
    pub matched: u32,
    pub total: u32,
}

/// Imports an .m3u/.m3u8 into a new static playlist. Lines resolve
/// against the library by file path (relative entries resolve against the
/// playlist's own folder); unmatched lines are skipped and counted.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn playlist_import_m3u(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<M3uImportResult, SignalError> {
    let source = std::path::PathBuf::from(&source_path);
    let body = tokio::fs::read_to_string(&source)
        .await
        .map_err(|e| SignalError::Io(format!("read m3u: {e}")))?;
    let base = source.parent().map(std::path::Path::to_path_buf);

    let mut total = 0u32;
    let mut ids: Vec<i64> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        total += 1;
        let path = std::path::PathBuf::from(line);
        let resolved = if path.is_absolute() {
            path
        } else if let Some(base) = &base {
            base.join(path)
        } else {
            path
        };
        let canonical = resolved.canonicalize().unwrap_or(resolved);
        if let Some(id) = state
            .db
            .tracks()
            .id_by_path(&canonical.to_string_lossy())
            .await
            .db_err()?
        {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }

    if ids.is_empty() {
        return Err(SignalError::Db(
            "no lines matched tracks in the library — scan their folder first".into(),
        ));
    }

    let name = source.file_stem().map_or_else(
        || "imported".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let playlist_id = state.db.playlists().create(&name).await.db_err()?;
    state
        .db
        .playlists()
        .add_tracks(playlist_id, &ids)
        .await
        .db_err()?;

    tracing::info!(name, matched = ids.len(), total, "m3u imported");
    Ok(M3uImportResult {
        playlist_id,
        name,
        matched: u32::try_from(ids.len()).unwrap_or_default(),
        total,
    })
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
