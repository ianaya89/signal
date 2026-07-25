//! Queue/context → player synchronization.
//!
//! Advance priority: the queue (explicit staging) always wins; the play
//! context (album/list the current track came from) fills in when the
//! queue is empty. mpv's 2-slot gapless window is a derived cache:
//! - stage the next candidate whenever a track starts or the queue changes
//!   (staging PEEKS — queue head stays queued, context position untouched),
//! - consume from the right source once mpv actually advances,
//! - fall back to load+play on EOF when nothing was staged in time.

use signal_core::{EventBus, PlaybackStatus, SignalEvent};
use tauri::{AppHandle, Manager, State};

use crate::state::AppState;

pub fn spawn(app: AppHandle, events: &EventBus) {
    let mut rx = events.subscribe();
    tauri::async_runtime::spawn(async move {
        // track id currently staged in mpv's next slot, if any
        let mut staged_next: Option<i64> = None;
        // last playing track, to append to history when a new one starts
        let mut last_track: Option<i64> = None;

        while let Ok(event) = rx.recv().await {
            let state = app.state::<AppState>();
            match event {
                // a track started by explicit user action — (re)stage next
                SignalEvent::TrackChanged { track_id: Some(id) } => {
                    if let Some(prev) = last_track.replace(id) {
                        if prev != id {
                            if let Ok(mut history) = state.play_history.lock() {
                                history.push(prev);
                                let overflow =
                                    history.len().saturating_sub(crate::state::HISTORY_CAP);
                                if overflow > 0 {
                                    history.drain(..overflow);
                                }
                            }
                        }
                    }
                    staged_next = restage(&state, staged_next).await;
                }
                SignalEvent::TrackChanged { track_id: None } => {
                    last_track = None;
                }
                // mpv gapless-advanced into the staged entry: consume it
                // from whichever source it came from, then stage the next
                SignalEvent::TrackAutoAdvanced { track_id } => {
                    consume(&state, track_id).await;
                    staged_next = restage(&state, None).await;
                }
                // EOF with nothing staged: load+play fallback
                SignalEvent::TrackEnded { .. } if staged_next.is_none() => {
                    if let Some(track_id) = next_candidate(&state).await {
                        consume(&state, track_id).await;
                        if let Err(err) =
                            crate::commands::player::start_track(&state, track_id).await
                        {
                            tracing::error!("auto-advance failed: {err}");
                        }
                    }
                }
                // user edited the queue: resync the staged slot
                SignalEvent::QueueChanged
                    if state.player.state().status != PlaybackStatus::Stopped =>
                {
                    staged_next = restage(&state, staged_next).await;
                }
                _ => {}
            }
        }
    });
}

/// Next track to play. Priority: repeat-one (current again) → queue head →
/// play context (shuffle/repeat-all aware).
pub async fn next_candidate(state: &State<'_, AppState>) -> Option<i64> {
    let mode = state.play_mode.lock().map(|m| *m).unwrap_or_default();

    if mode.repeat == crate::state::Repeat::One {
        if let Some(current) = state.player.state().track_id {
            return Some(current);
        }
    }

    match state.db.queue().first().await {
        Ok(Some(entry)) => return Some(entry.track.id),
        Ok(None) => {}
        Err(err) => {
            tracing::error!("queue peek failed: {err}");
            return None;
        }
    }
    state.play_context.lock().ok()?.peek_next(mode)
}

/// Marks `track_id` as consumed: pops it from the queue if it is the head,
/// otherwise repositions the play context onto it (covers linear advance,
/// shuffle jumps and repeat-all wraps alike).
pub async fn consume(state: &State<'_, AppState>, track_id: i64) {
    match state.db.queue().first().await {
        Ok(Some(entry)) if entry.track.id == track_id => {
            if let Err(err) = state.db.queue().remove(entry.item.id).await {
                tracing::error!("queue pop failed: {err}");
            }
            state.events.publish(SignalEvent::QueueChanged);
            return;
        }
        Ok(_) => {}
        Err(err) => tracing::error!("queue peek failed: {err}"),
    }
    if let Ok(mut ctx) = state.play_context.lock() {
        if !ctx.jump_to(track_id) {
            tracing::debug!(track_id, "advance outside queue and context");
        }
    }
}

/// Syncs the next candidate into mpv's gapless slot. Returns the staged id.
async fn restage(state: &State<'_, AppState>, current: Option<i64>) -> Option<i64> {
    let Some(next_id) = next_candidate(state).await else {
        if current.is_some() {
            if let Err(err) = state.player.clear_next() {
                tracing::error!("clear_next failed: {err}");
            }
        }
        return None;
    };

    if current == Some(next_id) {
        return current; // already staged
    }

    let track = match state.db.tracks().get(next_id).await {
        Ok(Some(track)) => track,
        Ok(None) => return None,
        Err(err) => {
            tracing::error!("track read failed: {err}");
            return None;
        }
    };
    let path = track.technical.file_path;
    if !path.is_file() {
        tracing::warn!(path = %path.display(), "next track missing on disk, not staging");
        return None;
    }
    match state.player.set_next(next_id, path) {
        Ok(()) => Some(next_id),
        Err(err) => {
            tracing::error!("set_next failed: {err}");
            None
        }
    }
}
