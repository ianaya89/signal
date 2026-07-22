use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn settings_get(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, SignalError> {
    state.db.settings().get(&key).await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn settings_set(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), SignalError> {
    state.db.settings().set(&key, &value).await.db_err()
}
