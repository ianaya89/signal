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
