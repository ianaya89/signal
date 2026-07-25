//! OS media integration via souvlaki: hardware media keys and the system
//! Now Playing surface (`MPNowPlayingInfoCenter` on macOS, MPRIS on Linux,
//! SMTC on Windows). Metadata follows `TrackChanged`; playback state and
//! position follow `PlayerState`.

use std::sync::Mutex;
use std::time::Duration;

use signal_core::{PlaybackStatus, SignalEvent};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

pub fn spawn(app: AppHandle) {
    let controls = match MediaControls::new(PlatformConfig {
        dbus_name: "signal",
        display_name: "Signal",
        hwnd: None,
    }) {
        Ok(controls) => controls,
        Err(err) => {
            tracing::warn!("media controls unavailable: {err:?}");
            return;
        }
    };

    let controls = std::sync::Arc::new(Mutex::new(controls));

    // hardware keys → player commands
    {
        let app = app.clone();
        let result = controls.lock().ok().map(|mut c| {
            c.attach(move |event: MediaControlEvent| {
                let state = app.state::<AppState>();
                let outcome = match event {
                    MediaControlEvent::Play
                    | MediaControlEvent::Pause
                    | MediaControlEvent::Toggle => state.player.toggle(),
                    MediaControlEvent::Next => {
                        // player_next needs async; publish TrackEnded-like skip
                        // via the sync path: seek to end is wrong — spawn a task
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app.state::<AppState>();
                            if let Some(id) = crate::autoplay::next_candidate(&state).await {
                                crate::autoplay::consume(&state, id).await;
                                let _ = crate::commands::player::start_track(&state, id).await;
                            }
                        });
                        Ok(())
                    }
                    MediaControlEvent::Previous => state.player.seek_ms(0),
                    MediaControlEvent::SetPosition(pos) => {
                        let ms = u64::try_from(pos.0.as_millis()).unwrap_or_default();
                        state.player.seek_ms(ms)
                    }
                    MediaControlEvent::Stop => state.player.stop(),
                    _ => Ok(()),
                };
                if let Err(err) = outcome {
                    tracing::warn!("media key handling failed: {err}");
                }
            })
        });
        if let Some(Err(err)) = result {
            tracing::warn!("media controls attach failed: {err:?}");
        }
    }

    // bus → Now Playing metadata/state
    let events = app.state::<AppState>().events.clone();
    let mut rx = events.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                SignalEvent::TrackChanged { track_id: Some(id) } => {
                    let state = app.state::<AppState>();
                    let Ok(Some(track)) = state.db.tracks().get(id).await else {
                        continue;
                    };
                    let album = state.db.albums().get(track.album_id).await.ok().flatten();
                    let cover = album
                        .as_ref()
                        .and_then(|a| a.artwork_path.as_ref())
                        .map(|p| format!("file://{p}"));

                    if let Ok(mut c) = controls.lock() {
                        let _ = c.set_metadata(MediaMetadata {
                            title: Some(&track.title),
                            artist: album.as_ref().map(|a| a.artist_name.as_str()),
                            album: album.as_ref().map(|a| a.name.as_str()),
                            duration: Some(Duration::from_millis(track.duration_ms)),
                            cover_url: cover.as_deref(),
                        });
                    }
                }
                SignalEvent::PlayerState { state: ps } => {
                    let playback = match ps.status {
                        PlaybackStatus::Playing => MediaPlayback::Playing {
                            progress: Some(souvlaki::MediaPosition(Duration::from_millis(
                                ps.position_ms,
                            ))),
                        },
                        PlaybackStatus::Paused => MediaPlayback::Paused {
                            progress: Some(souvlaki::MediaPosition(Duration::from_millis(
                                ps.position_ms,
                            ))),
                        },
                        PlaybackStatus::Stopped => MediaPlayback::Stopped,
                    };
                    if let Ok(mut c) = controls.lock() {
                        let _ = c.set_playback(playback);
                    }
                }
                _ => {}
            }
        }
    });

    tracing::info!("media controls attached");
}
