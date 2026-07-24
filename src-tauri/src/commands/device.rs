use signal_core::{AudioDevice, ReplayGainMode, SignalError};
use tauri::State;

use crate::state::AppState;

trait PlayerResultExt<T> {
    fn player_err(self) -> Result<T, SignalError>;
}

impl<T> PlayerResultExt<T> for Result<T, signal_player::PlayerError> {
    fn player_err(self) -> Result<T, SignalError> {
        self.map_err(|e| SignalError::Player(e.to_string()))
    }
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn device_list(state: State<'_, AppState>) -> Result<Vec<AudioDevice>, SignalError> {
    state.player.list_devices().player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn device_select(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), SignalError> {
    state.player.set_device(device_id).player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_set_replaygain(
    state: State<'_, AppState>,
    mode: ReplayGainMode,
) -> Result<(), SignalError> {
    state.player.set_replaygain(mode).player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_set_exclusive(
    state: State<'_, AppState>,
    exclusive: bool,
) -> Result<(), SignalError> {
    state.player.set_exclusive(exclusive).player_err()
}
