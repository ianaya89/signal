use signal_core::SignalError;
use signal_db::HealthReport;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_health(state: State<'_, AppState>) -> Result<HealthReport, SignalError> {
    state.db.health().report().await.db_err()
}

/// Removes DB rows for files that no longer exist on disk.
#[tauri::command]
#[tracing::instrument(skip(state, track_ids), fields(count = track_ids.len()))]
pub async fn library_prune_missing(
    state: State<'_, AppState>,
    track_ids: Vec<i64>,
) -> Result<u32, SignalError> {
    let removed = state.db.health().prune_missing(&track_ids).await.db_err()?;
    state.events.publish(signal_core::SignalEvent::QueueChanged);
    Ok(removed)
}

/// Re-links moved files: a dead-path row whose md5 matches a live-path row
/// adopts the new location; the fresh duplicate import is dropped. Stats,
/// playlists and queue membership survive. Returns pairs re-linked.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_relink_missing(state: State<'_, AppState>) -> Result<u32, SignalError> {
    let rows = state.db.tracks().list_paths_md5().await.db_err()?;

    let (dead, alive) = tokio::task::spawn_blocking(move || {
        let mut dead: Vec<(i64, String)> = Vec::new();
        let mut alive: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for (id, path, md5) in rows {
            let Some(md5) = md5 else { continue };
            if std::path::Path::new(&path).is_file() {
                alive.entry(md5).or_insert(id);
            } else {
                dead.push((id, md5));
            }
        }
        (dead, alive)
    })
    .await
    .map_err(|e| SignalError::Io(e.to_string()))?;

    let mut relinked = 0u32;
    for (old_id, md5) in dead {
        if let Some(&new_id) = alive.get(&md5) {
            state.db.tracks().relink(old_id, new_id).await.db_err()?;
            relinked += 1;
        }
    }
    if relinked > 0 {
        state.events.publish(signal_core::SignalEvent::QueueChanged);
    }
    tracing::info!(relinked, "relink pass done");
    Ok(relinked)
}

/// Resolves every duplicate group by keeping the best-quality copy
/// (lossless > bit depth > sample rate > bitrate) and merging the rest
/// into it — stats fold in, playlists repoint. DB-only; files stay.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_resolve_duplicates(state: State<'_, AppState>) -> Result<u32, SignalError> {
    let groups = state.db.health().duplicate_groups().await.db_err()?;
    let mut merged = 0u32;
    for group in groups {
        let Some(keep) = state
            .db
            .health()
            .pick_best(&group.track_ids)
            .await
            .db_err()?
        else {
            continue;
        };
        for drop in group.track_ids.into_iter().filter(|&id| id != keep) {
            state.db.tracks().merge_into(keep, drop).await.db_err()?;
            merged += 1;
        }
    }
    if merged > 0 {
        state.events.publish(signal_core::SignalEvent::QueueChanged);
    }
    tracing::info!(merged, "duplicate resolution done");
    Ok(merged)
}
