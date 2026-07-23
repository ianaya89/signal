use signal_core::{PlayerState, SignalError};
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

trait PlayerResultExt<T> {
    fn player_err(self) -> Result<T, SignalError>;
}

impl<T> PlayerResultExt<T> for Result<T, signal_player::PlayerError> {
    fn player_err(self) -> Result<T, SignalError> {
        self.map_err(|e| SignalError::Player(e.to_string()))
    }
}

/// Loads the track from the library and starts playback.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_play(state: State<'_, AppState>, track_id: i64) -> Result<(), SignalError> {
    let track = state
        .db
        .tracks()
        .get(track_id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Player(format!("track {track_id} not found")))?;

    let path = track.technical.file_path;
    if !path.is_file() {
        return Err(SignalError::Player(format!(
            "file missing: {}",
            path.display()
        )));
    }

    state.player.load_and_play(track_id, path).player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_toggle(state: State<'_, AppState>) -> Result<(), SignalError> {
    state.player.toggle().player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_pause(state: State<'_, AppState>) -> Result<(), SignalError> {
    state.player.pause().player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_stop(state: State<'_, AppState>) -> Result<(), SignalError> {
    state.player.stop().player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_seek(state: State<'_, AppState>, position_ms: u64) -> Result<(), SignalError> {
    state.player.seek_ms(position_ms).player_err()
}

/// `volume` in 0..=100.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_set_volume(state: State<'_, AppState>, volume: f64) -> Result<(), SignalError> {
    state.player.set_volume(volume).player_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
#[allow(clippy::unnecessary_wraps)]
pub async fn player_get_state(state: State<'_, AppState>) -> Result<PlayerState, SignalError> {
    Ok(state.player.state())
}

/// Skips to the queue head. Returns false when the queue is empty.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_next(state: State<'_, AppState>) -> Result<bool, SignalError> {
    crate::commands::queue::play_queue_head(&state).await
}

/// Restarts the current track (no play history yet to go further back).
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_prev(state: State<'_, AppState>) -> Result<(), SignalError> {
    state.player.seek_ms(0).player_err()
}
