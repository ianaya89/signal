//! Auto-advance: when a track ends naturally, play the next queued track.

use signal_core::{EventBus, SignalEvent};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

pub fn spawn(app: AppHandle, events: &EventBus) {
    let mut rx = events.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if !matches!(event, SignalEvent::TrackEnded { .. }) {
                continue;
            }
            let state = app.state::<AppState>();
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
    });
}
