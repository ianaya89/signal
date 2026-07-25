use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

/// Stores + activates the `ListenBrainz` token (empty string disables).
/// Returns whether the token validated against the API.
#[tauri::command]
#[tracing::instrument(skip(state, token))]
pub async fn plugin_set_listenbrainz(
    state: State<'_, AppState>,
    token: String,
) -> Result<bool, SignalError> {
    let trimmed = token.trim();
    state
        .db
        .settings()
        .set("plugin.listenbrainz.token", trimmed)
        .await
        .db_err()?;

    if trimmed.is_empty() {
        state.plugins.set_listenbrainz_token(None);
        return Ok(false);
    }

    let valid = state
        .plugins
        .validate_listenbrainz(trimmed)
        .await
        .map_err(|e| SignalError::Plugin(format!("listenbrainz: {e}")))?;
    if !valid {
        return Err(SignalError::Plugin("invalid listenbrainz token".into()));
    }
    state
        .plugins
        .set_listenbrainz_token(Some(trimmed.to_owned()));
    Ok(true)
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn plugin_status(state: State<'_, AppState>) -> Result<PluginStatus, SignalError> {
    Ok(PluginStatus {
        listenbrainz: state.plugins.listenbrainz_enabled(),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStatus {
    pub listenbrainz: bool,
}
