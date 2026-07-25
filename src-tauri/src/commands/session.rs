use serde::{Deserialize, Serialize};
use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResume {
    pub track_id: i64,
    pub position_ms: u64,
}

/// Reloads the last playing track paused at its saved position.
/// Returns what was restored, if anything.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn session_restore(
    state: State<'_, AppState>,
) -> Result<Option<SessionResume>, SignalError> {
    let Some(raw) = state.db.settings().get("session.now").await.db_err()? else {
        return Ok(None);
    };
    let Ok(resume) = serde_json::from_str::<SessionResume>(&raw) else {
        return Ok(None);
    };

    let Some(track) = state.db.tracks().get(resume.track_id).await.db_err()? else {
        return Ok(None);
    };
    let path = track.technical.file_path;
    if !path.is_file() {
        return Ok(None);
    }

    state
        .player
        .load_paused_at(resume.track_id, path, resume.position_ms)
        .map_err(|e| SignalError::Player(e.to_string()))?;
    Ok(Some(resume))
}
