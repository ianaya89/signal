use serde::Serialize;
use signal_core::{QueueItem, SignalError, SignalEvent, Track};
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueEntryDto {
    pub item: QueueItem,
    pub track: Track,
}

fn notify(state: &State<'_, AppState>) {
    state.events.publish(SignalEvent::QueueChanged);
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_list(state: State<'_, AppState>) -> Result<Vec<QueueEntryDto>, SignalError> {
    let entries = state.db.queue().list().await.db_err()?;
    Ok(entries
        .into_iter()
        .map(|e| QueueEntryDto {
            item: e.item,
            track: e.track,
        })
        .collect())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_add(state: State<'_, AppState>, track_id: i64) -> Result<(), SignalError> {
    state.db.queue().push_back(track_id).await.db_err()?;
    notify(&state);
    Ok(())
}

/// Stages at the head — plays right after the current track.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_add_next(state: State<'_, AppState>, track_id: i64) -> Result<(), SignalError> {
    state.db.queue().push_front(track_id).await.db_err()?;
    notify(&state);
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_remove(
    state: State<'_, AppState>,
    queue_item_id: i64,
) -> Result<(), SignalError> {
    state.db.queue().remove(queue_item_id).await.db_err()?;
    notify(&state);
    Ok(())
}

/// Full-order rewrite: frontend sends queue item ids in the new order.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_move(
    state: State<'_, AppState>,
    ordered_ids: Vec<i64>,
) -> Result<(), SignalError> {
    state.db.queue().reorder(&ordered_ids).await.db_err()?;
    notify(&state);
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_clear(state: State<'_, AppState>) -> Result<(), SignalError> {
    state.db.queue().clear().await.db_err()?;
    notify(&state);
    Ok(())
}

/// Plays the queue head (pops it). No-op if queue empty.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn queue_play_next(state: State<'_, AppState>) -> Result<bool, SignalError> {
    play_queue_head(&state).await
}

pub(crate) async fn play_queue_head(state: &State<'_, AppState>) -> Result<bool, SignalError> {
    let Some(entry) = state.db.queue().pop_front().await.db_err()? else {
        return Ok(false);
    };
    notify(state);

    let path = entry.track.technical.file_path.clone();
    if !path.is_file() {
        tracing::warn!(path = %path.display(), "queued file missing, skipping");
        return Err(SignalError::Player(format!(
            "file missing: {}",
            path.display()
        )));
    }
    state
        .player
        .load_and_play(entry.track.id, path)
        .map_err(|e| SignalError::Player(e.to_string()))?;
    Ok(true)
}
