//! Queue → player synchronization.
//!
//! The queue is the source of truth; mpv's 2-slot playlist window is a
//! derived cache. This task:
//! - stages the queue head as the gapless next whenever a track starts or
//!   the queue changes (staging PEEKS — the head stays queued),
//! - pops the head once mpv actually advances into it,
//! - falls back to pop+play on EOF when nothing was staged (e.g. the file
//!   appeared in the queue too late to prefetch).

use signal_core::{EventBus, PlaybackStatus, SignalEvent};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

pub fn spawn(app: AppHandle, events: &EventBus) {
    let mut rx = events.subscribe();
    tauri::async_runtime::spawn(async move {
        // track id currently staged in mpv's next slot, if any
        let mut staged_next: Option<i64> = None;

        while let Ok(event) = rx.recv().await {
            let state = app.state::<AppState>();
            match event {
                // a track started by explicit user action — (re)stage next
                SignalEvent::TrackChanged { track_id: Some(_) } => {
                    staged_next = restage(state.inner(), staged_next).await;
                }
                // mpv gapless-advanced into the staged entry: consume the
                // queue head it came from, then stage the following track
                SignalEvent::TrackAutoAdvanced { track_id } => {
                    match state.db.queue().pop_front().await {
                        Ok(Some(entry)) if entry.track.id == track_id => {}
                        Ok(other) => {
                            let found = other.map(|e| e.track.id);
                            tracing::warn!(track_id, ?found, "queue/window desync on advance");
                        }
                        Err(err) => tracing::error!("queue pop failed: {err}"),
                    }
                    state.events.publish(SignalEvent::QueueChanged);
                    staged_next = restage(state.inner(), None).await;
                }
                // EOF with nothing staged: legacy pop+play fallback
                SignalEvent::TrackEnded { .. } if staged_next.is_none() => {
                    match state.db.queue().pop_front().await {
                        Ok(Some(entry)) => {
                            state.events.publish(SignalEvent::QueueChanged);
                            let path = entry.track.technical.file_path.clone();
                            if !path.is_file() {
                                tracing::warn!(path = %path.display(), "queued file missing, skipping");
                                continue;
                            }
                            if let Err(err) = state.player.load_and_play(entry.track.id, path) {
                                tracing::error!("auto-advance failed: {err}");
                            }
                        }
                        Ok(None) => {}
                        Err(err) => tracing::error!("queue read failed: {err}"),
                    }
                }
                // user edited the queue: resync the staged slot
                SignalEvent::QueueChanged
                    if state.player.state().status != PlaybackStatus::Stopped =>
                {
                    staged_next = restage(state.inner(), staged_next).await;
                }
                _ => {}
            }
        }
    });
}

/// Peeks the queue head and syncs it into mpv's next slot. Returns the newly
/// staged track id (None = nothing stageable).
async fn restage(state: &AppState, current: Option<i64>) -> Option<i64> {
    let head = match state.db.queue().first().await {
        Ok(head) => head,
        Err(err) => {
            tracing::error!("queue peek failed: {err}");
            return current;
        }
    };

    let Some(entry) = head else {
        if current.is_some() {
            if let Err(err) = state.player.clear_next() {
                tracing::error!("clear_next failed: {err}");
            }
        }
        return None;
    };

    let path = entry.track.technical.file_path.clone();
    if !path.is_file() {
        tracing::warn!(path = %path.display(), "queue head missing on disk, not staging");
        return None;
    }
    if current == Some(entry.track.id) {
        return current; // already staged
    }
    match state.player.set_next(entry.track.id, path) {
        Ok(()) => Some(entry.track.id),
        Err(err) => {
            tracing::error!("set_next failed: {err}");
            None
        }
    }
}
