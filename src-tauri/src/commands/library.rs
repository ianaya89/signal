use std::path::PathBuf;
use std::sync::atomic::Ordering;

use signal_core::{AlbumDetail, AlbumSummary, ArtistSummary, SignalError};
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

/// Library roots: `library.roots` JSON array, falling back to the legacy
/// single `library.root` key.
pub async fn read_roots(state: &AppState) -> Vec<String> {
    if let Ok(Some(json)) = state.db.settings().get("library.roots").await {
        if let Ok(roots) = serde_json::from_str::<Vec<String>>(&json) {
            return roots;
        }
    }
    match state.db.settings().get("library.root").await {
        Ok(Some(root)) => vec![root],
        _ => Vec::new(),
    }
}

async fn write_roots(state: &AppState, roots: &[String]) {
    match serde_json::to_string(roots) {
        Ok(json) => {
            if let Err(err) = state.db.settings().set("library.roots", &json).await {
                tracing::warn!("could not persist library.roots: {err}");
            }
        }
        Err(err) => tracing::warn!("roots serialize failed: {err}"),
    }
}

fn scan_err(state: &AppState, message: String) -> SignalError {
    state
        .events
        .publish(signal_core::SignalEvent::ScannerError {
            message: message.clone(),
        });
    SignalError::Scanner(message)
}

fn validate_root(state: &AppState, root: &str) -> Result<PathBuf, SignalError> {
    let root = expand_home(root);
    let root = match root.canonicalize() {
        Ok(canonical) => canonical,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(scan_err(state, format!(
                "macOS blocked access to {} — use \"scan folder…\" (picker) or grant access in System Settings → Privacy → Files & Folders",
                root.display()
            )));
        }
        Err(_) => {
            return Err(scan_err(
                state,
                format!("folder not found: {}", root.display()),
            ));
        }
    };
    if !root.is_dir() {
        return Err(scan_err(
            state,
            format!("not a directory: {}", root.display()),
        ));
    }
    Ok(root)
}

/// Kicks off a background scan and returns immediately; progress arrives via
/// `scanner:progress` / `scanner:done` events. Failures are also published
/// as `scanner:error` so they surface regardless of the calling UI. The
/// folder joins the root list (multi-root) and gains a watcher.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_scan(state: State<'_, AppState>, root: String) -> Result<(), SignalError> {
    let root = validate_root(&state, &root)?;

    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err(SignalError::Scanner("a scan is already running".into()));
    }

    let mut roots = read_roots(&state).await;
    let root_str = root.to_string_lossy().into_owned();
    if !roots.contains(&root_str) {
        roots.push(root_str);
        write_roots(&state, &roots).await;
    }
    state.start_watchers(&roots.iter().map(PathBuf::from).collect::<Vec<_>>());

    let scanner = state.scanner();
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
pub async fn library_list_roots(state: State<'_, AppState>) -> Result<Vec<String>, SignalError> {
    Ok(read_roots(&state).await)
}

/// Drops a root from the list (watchers restart without it). With `purge`,
/// also removes every track under it from the database — files stay.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_remove_root(
    state: State<'_, AppState>,
    root: String,
    purge: bool,
) -> Result<u32, SignalError> {
    let mut roots = read_roots(&state).await;
    roots.retain(|r| r != &root);
    write_roots(&state, &roots).await;
    state.start_watchers(&roots.iter().map(PathBuf::from).collect::<Vec<_>>());

    let removed = if purge {
        let removed = state.db.tracks().delete_under_dir(&root).await.db_err()?;
        state.events.publish(signal_core::SignalEvent::QueueChanged);
        removed
    } else {
        0
    };
    Ok(removed)
}

/// Removes every track under a folder from the database (files stay).
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_remove_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<u32, SignalError> {
    let dir = expand_home(&path);
    let removed = state
        .db
        .tracks()
        .delete_under_dir(&dir.to_string_lossy())
        .await
        .db_err()?;
    state.events.publish(signal_core::SignalEvent::QueueChanged);
    tracing::info!(dir = %dir.display(), removed, "folder removed from library");
    Ok(removed)
}

/// Rescans every stored root sequentially in one background task.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_rescan_all(state: State<'_, AppState>) -> Result<(), SignalError> {
    let roots = read_roots(&state).await;
    if roots.is_empty() {
        return Err(scan_err(
            &state,
            "no library roots stored — scan a folder first".into(),
        ));
    }
    if state.scanning.swap(true, Ordering::SeqCst) {
        return Err(SignalError::Scanner("a scan is already running".into()));
    }

    state.start_watchers(&roots.iter().map(PathBuf::from).collect::<Vec<_>>());
    let scanner = state.scanner();
    let scanning = state.scanning.clone();
    tauri::async_runtime::spawn(async move {
        for root in roots {
            if let Err(err) = scanner.scan_full(PathBuf::from(&root)).await {
                tracing::error!(root, "rescan failed: {err}");
            }
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub artist: ArtistSummary,
    pub albums: Vec<AlbumSummary>,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_get_artist(
    state: State<'_, AppState>,
    artist_id: i64,
) -> Result<ArtistDetail, SignalError> {
    let artist = state
        .db
        .artists()
        .get(artist_id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Db(format!("artist {artist_id} not found")))?;
    let albums = state.db.albums().list_by_artist(artist_id).await.db_err()?;
    Ok(ArtistDetail { artist, albums })
}

/// Wipes library data and rescans the stored root. Fixes libraries imported
/// before album-artist grouping existed.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_reset_and_rescan(state: State<'_, AppState>) -> Result<(), SignalError> {
    if read_roots(&state).await.is_empty() {
        return Err(SignalError::Scanner(
            "no library roots stored — scan first".into(),
        ));
    }
    state.db.reset_library().await.db_err()?;
    state.events.publish(signal_core::SignalEvent::QueueChanged);
    library_rescan_all(state).await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreSummary {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_list_genres(
    state: State<'_, AppState>,
) -> Result<Vec<GenreSummary>, SignalError> {
    let genres = state.db.artists().list_genres().await.db_err()?;
    Ok(genres
        .into_iter()
        .map(|(id, name, track_count)| GenreSummary {
            id,
            name,
            track_count,
        })
        .collect())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_get_genre_tracks(
    state: State<'_, AppState>,
    genre_id: i64,
) -> Result<Vec<signal_core::Track>, SignalError> {
    state.db.tracks().list_by_genre(genre_id).await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_list_loved(
    state: State<'_, AppState>,
) -> Result<Vec<signal_core::Track>, SignalError> {
    state.db.tracks().list_loved().await.db_err()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    pub track_count: u32,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderListing {
    pub root: String,
    pub path: String,
    pub dirs: Vec<FolderEntry>,
    pub tracks: Vec<signal_core::Track>,
}

/// Browses the library by directory structure. `path` empty = library root;
/// anything outside the configured roots is rejected.
///
/// Reads the same root list as everything else. It used to read the legacy
/// single `library.root` key directly, which meant folder browsing broke
/// outright on any install whose roots had only ever been written in the
/// `library.roots` form — the key it was reading simply was not there.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_browse_folder(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<FolderListing, SignalError> {
    let roots = read_roots(&state).await;
    let first = roots
        .first()
        .ok_or_else(|| SignalError::Scanner("no library folders yet — add one in settings".into()))?
        .clone();

    // with several roots there is no single top directory to open, so the top
    // level lists the roots themselves and each one drills down from there
    if roots.len() > 1 && path.as_ref().is_none_or(String::is_empty) {
        let mut dirs = Vec::with_capacity(roots.len());
        for root in &roots {
            let track_count = state.db.tracks().count_under(root).await.db_err()?;
            dirs.push(FolderEntry {
                name: root.clone(),
                path: root.clone(),
                track_count: u32::try_from(track_count).unwrap_or_default(),
            });
        }
        return Ok(FolderListing {
            root: String::new(),
            path: String::new(),
            dirs,
            tracks: Vec::new(),
        });
    }

    let dir = match &path {
        Some(p) if !p.is_empty() => p.clone(),
        _ => first,
    };
    // no escaping the library roots via .. or absolute tricks
    let canonical = std::path::Path::new(&dir)
        .canonicalize()
        .map_err(|_| SignalError::Io(format!("folder not found: {dir}")))?;
    let root = roots
        .iter()
        .find(|r| canonical.starts_with(r))
        .ok_or_else(|| SignalError::Io("path outside library roots".into()))?
        .clone();

    let mut dirs = Vec::new();
    let entries = std::fs::read_dir(&canonical).map_err(|e| SignalError::Io(e.to_string()))?;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if !file_type.is_dir() || name.starts_with('.') {
            continue;
        }
        let sub_path = entry.path().to_string_lossy().into_owned();
        let track_count = state.db.tracks().count_under(&sub_path).await.db_err()?;
        if track_count > 0 {
            dirs.push(FolderEntry {
                name,
                path: sub_path,
                track_count: u32::try_from(track_count).unwrap_or_default(),
            });
        }
    }
    dirs.sort_by_key(|d| d.name.to_lowercase());

    let tracks = state
        .db
        .tracks()
        .list_in_dir(&canonical.to_string_lossy())
        .await
        .db_err()?;

    Ok(FolderListing {
        root,
        path: canonical.to_string_lossy().into_owned(),
        dirs,
        tracks,
    })
}

/// Opens the OS file manager with the file selected.
#[tauri::command]
#[tracing::instrument]
pub async fn reveal_in_file_manager(path: String) -> Result<(), SignalError> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .args(["-R", &path])
        .spawn();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open")
        .arg(
            std::path::Path::new(&path)
                .parent()
                .unwrap_or(std::path::Path::new("/")),
        )
        .spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer")
        .args(["/select,", &path])
        .spawn();

    result.map_err(|e| SignalError::Io(e.to_string()))?;
    Ok(())
}

/// Track + its artist/album display names, for now-playing UI + inspector.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_get_track(
    state: State<'_, AppState>,
    track_id: i64,
) -> Result<TrackWithContext, SignalError> {
    if crate::state::is_remote_id(track_id) {
        return crate::commands::remote::now_playing(&state, track_id);
    }

    let track = state
        .db
        .tracks()
        .get(track_id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Db(format!("track {track_id} not found")))?;

    let album = state.db.albums().get(track.album_id).await.db_err()?;
    let album_name = album.map_or_else(String::new, |a| a.name);
    // the track's own artist, not the album artist — they differ on
    // compilations and after manual edits
    let artist_name = state
        .db
        .artists()
        .get(track.artist_id)
        .await
        .db_err()?
        .map_or_else(String::new, |a| a.name);
    let genres = state.db.tracks().genres_of(track.id).await.db_err()?;
    let genre = if genres.is_empty() {
        None
    } else {
        Some(genres.join(", "))
    };

    Ok(TrackWithContext {
        track,
        artist_name,
        album_name,
        genre,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackWithContext {
    pub track: signal_core::Track,
    pub artist_name: String,
    pub album_name: String,
    pub genre: Option<String>,
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
