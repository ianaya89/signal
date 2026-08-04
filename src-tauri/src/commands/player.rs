use signal_core::{MediaSource, PlayerState, SignalError};
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

/// Loads the track and starts playback. Standard player behavior: a bare
/// play adopts the track's album as the follow-on context.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_play(state: State<'_, AppState>, track_id: i64) -> Result<(), SignalError> {
    let context = album_context(&state, track_id).await;
    if let Ok(mut ctx) = state.play_context.lock() {
        match context {
            Some((track_ids, position)) => {
                *ctx = crate::state::PlayContext::default();
                ctx.track_ids = track_ids;
                ctx.position = position;
            }
            None => *ctx = crate::state::PlayContext::default(),
        }
    }
    start_track(&state, track_id).await
}

/// The track's album as (ordered ids, index of `track_id`), when it has one.
pub(crate) async fn album_context(
    state: &State<'_, AppState>,
    track_id: i64,
) -> Option<(Vec<i64>, usize)> {
    let track = state.db.tracks().get(track_id).await.ok()??;
    if track.album_id <= 0 {
        return None;
    }
    let tracks = state.db.albums().tracks(track.album_id).await.ok()?;
    let ids: Vec<i64> = tracks.iter().map(|t| t.id).collect();
    let position = ids.iter().position(|&id| id == track_id)?;
    Some((ids, position))
}

/// Plays `track_ids[start_index]` with the whole list as the implicit
/// follow-on order (album/artist/search "play from here").
#[tauri::command]
#[tracing::instrument(skip(state, track_ids), fields(len = track_ids.len(), start_index))]
pub async fn player_play_context(
    state: State<'_, AppState>,
    track_ids: Vec<i64>,
    start_index: usize,
) -> Result<(), SignalError> {
    let Some(&first) = track_ids.get(start_index) else {
        return Err(SignalError::Player("start index out of range".into()));
    };
    if let Ok(mut ctx) = state.play_context.lock() {
        ctx.track_ids = track_ids;
        ctx.position = start_index;
    }
    start_track(&state, first).await
}

pub(crate) async fn start_track(
    state: &State<'_, AppState>,
    track_id: i64,
) -> Result<(), SignalError> {
    if crate::state::is_remote_id(track_id) {
        let url = state
            .remote_tracks
            .lock()
            .ok()
            .and_then(|slab| slab.get(track_id).map(|t| t.url.clone()))
            .ok_or_else(|| SignalError::Player(format!("remote track {track_id} not found")))?;
        return state
            .player
            .load_and_play(track_id, MediaSource::Url(url))
            .player_err();
    }

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

    state
        .player
        .load_and_play(track_id, MediaSource::File(path))
        .player_err()
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

/// Skips forward: queue head first, else next in the play context.
/// Returns false when there is nothing to advance to.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_next(state: State<'_, AppState>) -> Result<bool, SignalError> {
    let Some(track_id) = crate::autoplay::next_candidate(&state).await else {
        return Ok(false);
    };
    crate::autoplay::consume(&state, track_id).await;
    start_track(&state, track_id).await?;
    Ok(true)
}

/// Beyond ~3s into a track, restarts it; near the start, steps back
/// through the play history.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_prev(state: State<'_, AppState>) -> Result<(), SignalError> {
    let position = state.player.state().position_ms;
    if position > 3000 {
        return state.player.seek_ms(0).player_err();
    }

    let previous = state
        .play_history
        .lock()
        .ok()
        .and_then(|mut history| history.pop());
    match previous {
        Some(track_id) => start_track(&state, track_id).await,
        None => state.player.seek_ms(0).player_err(),
    }
}

/// Sets shuffle/repeat; persisted so it survives restarts.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn player_set_mode(
    state: State<'_, AppState>,
    mode: crate::state::PlayMode,
) -> Result<(), SignalError> {
    if let Ok(mut guard) = state.play_mode.lock() {
        *guard = mode;
    }
    let json = serde_json::to_string(&mode).unwrap_or_default();
    state.db.settings().set("player.mode", &json).await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
#[allow(clippy::unnecessary_wraps)]
pub async fn player_get_mode(
    state: State<'_, AppState>,
) -> Result<crate::state::PlayMode, SignalError> {
    Ok(state.play_mode.lock().map(|m| *m).unwrap_or_default())
}
