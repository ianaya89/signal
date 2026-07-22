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
/// `scanner:progress` / `scanner:done` events.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_scan(state: State<'_, AppState>, root: String) -> Result<(), SignalError> {
    let root = expand_home(&root);
    if !root.is_dir() {
        return Err(SignalError::Scanner(format!(
            "not a directory: {}",
            root.display()
        )));
    }

    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err(SignalError::Scanner("a scan is already running".into()));
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
