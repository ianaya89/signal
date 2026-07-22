use signal_core::SignalError;
use tauri::State;

use crate::state::AppState;

// Both commands: async + State<'_> requires a Result return in Tauri v2,
// hence the unnecessary_wraps allows.

#[tauri::command]
#[tracing::instrument(skip(state))]
#[allow(clippy::unnecessary_wraps)]
pub async fn settings_get(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, SignalError> {
    Ok(state.settings.read().await.get(&key).cloned())
}

#[tauri::command]
#[tracing::instrument(skip(state))]
#[allow(clippy::unnecessary_wraps)]
pub async fn settings_set(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), SignalError> {
    state.settings.write().await.insert(key, value);
    Ok(())
}
