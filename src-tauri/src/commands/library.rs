use std::path::PathBuf;
use std::sync::atomic::Ordering;

use signal_core::{AlbumDetail, AlbumSummary, ArtistSummary, SignalError};
use signal_scanner::Scanner;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

/// Kicks off a background scan and returns immediately; progress arrives via
/// `scanner:progress` / `scanner:done` events. Failures are also published
/// as `scanner:error` so they surface regardless of the calling UI.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_scan(state: State<'_, AppState>, root: String) -> Result<(), SignalError> {
    let scan_err = |message: String| {
        state
            .events
            .publish(signal_core::SignalEvent::ScannerError {
                message: message.clone(),
            });
        SignalError::Scanner(message)
    };

    let root = expand_home(&root);
    let root = match root.canonicalize() {
        Ok(canonical) => canonical,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(scan_err(format!(
                "macOS blocked access to {} — use \"scan folder…\" (picker) or grant access in System Settings → Privacy → Files & Folders",
                root.display()
            )));
        }
        Err(_) => {
            return Err(scan_err(format!("folder not found: {}", root.display())));
        }
    };
    if !root.is_dir() {
        return Err(scan_err(format!("not a directory: {}", root.display())));
    }

    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err(SignalError::Scanner("a scan is already running".into()));
    }

    // remember for `rescan`
    if let Err(err) = state
        .db
        .settings()
        .set("library.root", &root.to_string_lossy())
        .await
    {
        tracing::warn!("could not persist library.root: {err}");
    }

    let scanner = Scanner::new(
        state.db.clone(),
        state.events.clone(),
        state.config.cache_dir.clone(),
    );
    let scanning = state.scanning.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = scanner.scan_full(root).await {
            tracing::error!("scan failed: {err}");
        }
        scanning.store(false, Ordering::SeqCst);
    });

    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_list_albums(
    state: State<'_, AppState>,
) -> Result<Vec<AlbumSummary>, SignalError> {
    state.db.albums().list().await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_list_artists(
    state: State<'_, AppState>,
) -> Result<Vec<ArtistSummary>, SignalError> {
    state.db.artists().list().await.db_err()
}

/// Track + its artist/album display names, for now-playing UI + inspector.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_get_track(
    state: State<'_, AppState>,
    track_id: i64,
) -> Result<TrackWithContext, SignalError> {
    let track = state
        .db
        .tracks()
        .get(track_id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Db(format!("track {track_id} not found")))?;

    let album = state.db.albums().get(track.album_id).await.db_err()?;
    let (album_name, artist_name) = album.map_or_else(
        || (String::new(), String::new()),
        |a| (a.name, a.artist_name),
    );

    Ok(TrackWithContext {
        track,
        artist_name,
        album_name,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackWithContext {
    pub track: signal_core::Track,
    pub artist_name: String,
    pub album_name: String,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_get_album(
    state: State<'_, AppState>,
    album_id: i64,
) -> Result<AlbumDetail, SignalError> {
    let album = state
        .db
        .albums()
        .get(album_id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Db(format!("album {album_id} not found")))?;
    let tracks = state.db.albums().tracks(album_id).await.db_err()?;
    Ok(AlbumDetail { album, tracks })
}
