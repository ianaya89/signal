//! Listening-history recorder: turns player events into `play_events` rows.
//!
//! A session opens on `TrackChanged(Some)` and closes on the next
//! `TrackEnded` / `TrackChanged` / stop. Completion follows the Last.fm
//! rule (≥50% or ≥4 minutes listened); anything under 30s counts as a skip.

use chrono::{DateTime, Utc};
use signal_core::{EventBus, PlaySource, SignalEvent};
use signal_db::NewPlayEvent;
use tauri::{AppHandle, Manager};

use crate::state::AppState;

const SKIP_THRESHOLD_MS: u64 = 30_000;
const COMPLETED_MS: u64 = 240_000;

struct Session {
    track_id: i64,
    started_at: DateTime<Utc>,
    ms_played: u64,
    duration_ms: u64,
}

impl Session {
    fn close(self, natural_eof: bool) -> NewPlayEvent {
        let completed = natural_eof
            || self.ms_played >= COMPLETED_MS
            || (self.duration_ms > 0 && self.ms_played * 2 >= self.duration_ms);
        NewPlayEvent {
            track_id: self.track_id,
            started_at: self.started_at,
            ms_played: self.ms_played,
            completed,
            skipped: !completed && self.ms_played < SKIP_THRESHOLD_MS,
            source: PlaySource::Queue,
        }
    }
}

pub fn spawn(app: AppHandle, events: &EventBus) {
    let mut rx = events.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut session: Option<Session> = None;
        let mut last_persist = std::time::Instant::now();

        while let Ok(event) = rx.recv().await {
            match event {
                SignalEvent::PlayerProgress {
                    position_ms,
                    duration_ms,
                } => {
                    if let Some(s) = session.as_mut() {
                        s.ms_played = s.ms_played.max(position_ms);
                        if duration_ms > 0 {
                            s.duration_ms = duration_ms;
                        }
                        // persist resume point every ~5s for session restore
                        if last_persist.elapsed().as_secs() >= 5 {
                            last_persist = std::time::Instant::now();
                            let state = app.state::<AppState>();
                            let json = format!(
                                "{{\"trackId\":{},\"positionMs\":{position_ms}}}",
                                s.track_id
                            );
                            if let Err(err) = state.db.settings().set("session.now", &json).await {
                                tracing::warn!("session persist failed: {err}");
                            }
                        }
                    }
                }
                SignalEvent::TrackEnded { track_id } => {
                    if let Some(s) = session.take() {
                        if s.track_id == track_id {
                            log(&app, s.close(true)).await;
                        } else {
                            session = Some(s); // stale event, keep session
                        }
                    }
                }
                SignalEvent::TrackChanged { track_id } => {
                    if let Some(s) = session.take() {
                        // don't double-log: TrackEnded already closed it if
                        // this change came from a natural EOF
                        if Some(s.track_id) != track_id {
                            log(&app, s.close(false)).await;
                        }
                    }
                    session = track_id.map(|id| Session {
                        track_id: id,
                        started_at: Utc::now(),
                        ms_played: 0,
                        duration_ms: 0,
                    });
                }
                _ => {}
            }
        }
    });
}

async fn log(app: &AppHandle, event: NewPlayEvent) {
    // ignore sub-second noise (double-clicks, instant skips)
    if event.ms_played < 1000 && !event.completed {
        return;
    }
    let state = app.state::<AppState>();
    if let Err(err) = state.db.stats().log_play_event(&event).await {
        tracing::error!("play event log failed: {err}");
    } else {
        tracing::debug!(
            track_id = event.track_id,
            ms = event.ms_played,
            completed = event.completed,
            skipped = event.skipped,
            "play event recorded"
        );
    }
}
