//! Local control socket: newline-delimited JSON over a Unix domain socket.
//! This is what makes Signal scriptable — the `signal` CLI, Raycast,
//! tmux statuslines and anything else speak this protocol.
//!
//! Request:  {"cmd":"status"} | {"cmd":"play","query":"bocanada"} | …
//! Response: {"ok":true,"data":…} | {"ok":false,"error":"…"}

#![cfg(unix)]

use serde::{Deserialize, Serialize};
use signal_core::PlaybackStatus;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
enum Request {
    Status,
    Toggle,
    Next,
    Prev,
    Stop,
    Play {
        query: String,
    },
    Seek {
        to: String,
    },
    Volume {
        to: String,
    },
    QueueAdd {
        query: String,
    },
    QueueList,
    Search {
        query: String,
    },
    ServerStart,
    ServerStop,
    ServerStatus,
    AnalyzeStart {
        #[serde(default)]
        force: bool,
    },
    AnalyzeStatus,
}

#[derive(Serialize)]
struct StatusPayload {
    state: &'static str,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    position_ms: u64,
    duration_ms: u64,
    volume_pct: u32,
    shuffle: bool,
    repeat: String,
    bit_perfect: bool,
}

pub fn socket_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("signal.sock"))
}

pub fn spawn(app: AppHandle) {
    let Some(path) = socket_path(&app) else {
        return;
    };
    // stale socket from a previous run
    let _ = std::fs::remove_file(&path);

    // bind inside the task: tokio's UnixListener needs the reactor
    tauri::async_runtime::spawn(async move {
        let listener = match UnixListener::bind(&path) {
            Ok(listener) => listener,
            Err(err) => {
                tracing::warn!("control socket bind failed: {err}");
                return;
            }
        };
        tracing::info!(path = %path.display(), "control socket listening");

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        handle_conn(stream, app).await;
                    });
                }
                Err(err) => {
                    tracing::warn!("socket accept failed: {err}");
                    break;
                }
            }
        }
    });
}

async fn handle_conn(stream: UnixStream, app: AppHandle) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => match dispatch(&app, request).await {
                Ok(data) => serde_json::json!({ "ok": true, "data": data }),
                Err(error) => serde_json::json!({ "ok": false, "error": error }),
            },
            Err(err) => serde_json::json!({ "ok": false, "error": format!("bad request: {err}") }),
        };
        let mut out = response.to_string();
        out.push('\n');
        if writer.write_all(out.as_bytes()).await.is_err() {
            break;
        }
    }
}

// flat command → action mapping; splitting per-command helpers adds noise
#[allow(clippy::too_many_lines)]
async fn dispatch(app: &AppHandle, request: Request) -> Result<serde_json::Value, String> {
    let state = app.state::<AppState>();
    let err = |e: &dyn std::fmt::Display| e.to_string();

    match request {
        Request::Status => {
            let ps = state.player.state();
            let track = match ps.track_id {
                Some(id) => state.db.tracks().get(id).await.map_err(|e| err(&e))?,
                None => None,
            };
            let album = match &track {
                Some(t) => state
                    .db
                    .albums()
                    .get(t.album_id)
                    .await
                    .map_err(|e| err(&e))?,
                None => None,
            };
            let mode = state.play_mode.lock().map(|m| *m).unwrap_or_default();
            let payload = StatusPayload {
                state: match ps.status {
                    PlaybackStatus::Playing => "playing",
                    PlaybackStatus::Paused => "paused",
                    PlaybackStatus::Stopped => "stopped",
                },
                title: track.as_ref().map(|t| t.title.clone()),
                artist: album.as_ref().map(|a| a.artist_name.clone()),
                album: album.map(|a| a.name),
                position_ms: ps.position_ms,
                duration_ms: ps.duration_ms,
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                volume_pct: (ps.volume * 100.0).round() as u32,
                shuffle: mode.shuffle,
                repeat: format!("{:?}", mode.repeat).to_lowercase(),
                bit_perfect: ps.bit_perfect,
            };
            serde_json::to_value(payload).map_err(|e| err(&e))
        }
        Request::Toggle => state
            .player
            .toggle()
            .map(|()| serde_json::Value::Null)
            .map_err(|e| err(&e)),
        Request::Stop => state
            .player
            .stop()
            .map(|()| serde_json::Value::Null)
            .map_err(|e| err(&e)),
        Request::Next => {
            let Some(id) = crate::autoplay::next_candidate(&state).await else {
                return Err("nothing to advance to".into());
            };
            crate::autoplay::consume(&state, id).await;
            crate::commands::player::start_track(&state, id)
                .await
                .map(|()| serde_json::Value::Null)
                .map_err(|e| err(&e))
        }
        Request::Prev => state
            .player
            .seek_ms(0)
            .map(|()| serde_json::Value::Null)
            .map_err(|e| err(&e)),
        Request::Play { query } => {
            let tracks = signal_search::search(&state.db, &query, 50)
                .await
                .map_err(|e| err(&e))?;
            let Some(first) = tracks.first() else {
                return Err(format!("no match for '{query}'"));
            };
            let title = first.title.clone();
            let id = first.id;
            // standard behavior: the match's album continues; the raw result
            // list is the fallback for album-less tracks
            let context = crate::commands::player::album_context(&state, id).await;
            if let Ok(mut ctx) = state.play_context.lock() {
                *ctx = crate::state::PlayContext::default();
                if let Some((ids, position)) = context {
                    ctx.track_ids = ids;
                    ctx.position = position;
                } else {
                    ctx.track_ids = tracks.iter().map(|t| t.id).collect();
                    ctx.position = 0;
                }
            }
            crate::commands::player::start_track(&state, id)
                .await
                .map_err(|e| err(&e))?;
            Ok(serde_json::json!({ "playing": title }))
        }
        Request::Seek { to } => {
            let current = state.player.state().position_ms;
            let target = parse_relative(&to, current / 1000).ok_or("bad seek value")?;
            state
                .player
                .seek_ms(target * 1000)
                .map(|()| serde_json::Value::Null)
                .map_err(|e| err(&e))
        }
        Request::Volume { to } => {
            let ps = state.player.state();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let current = (ps.volume * 100.0).round() as u64;
            let target = parse_relative(&to, current).ok_or("bad volume value")?;
            #[allow(clippy::cast_precision_loss)]
            state
                .player
                .set_volume((target.min(100)) as f64)
                .map(|()| serde_json::Value::Null)
                .map_err(|e| err(&e))
        }
        Request::QueueAdd { query } => {
            let tracks = signal_search::search(&state.db, &query, 1)
                .await
                .map_err(|e| err(&e))?;
            let Some(track) = tracks.first() else {
                return Err(format!("no match for '{query}'"));
            };
            state
                .db
                .queue()
                .push_back(track.id)
                .await
                .map_err(|e| err(&e))?;
            state.events.publish(signal_core::SignalEvent::QueueChanged);
            Ok(serde_json::json!({ "staged": track.title }))
        }
        Request::QueueList => {
            let entries = state.db.queue().list().await.map_err(|e| err(&e))?;
            let list: Vec<_> = entries
                .iter()
                .map(|e| serde_json::json!({ "id": e.track.id, "title": e.track.title }))
                .collect();
            Ok(serde_json::Value::Array(list))
        }
        Request::ServerStart => {
            let status = crate::commands::server::start_server(&state)
                .await
                .map_err(|e| err(&e))?;
            serde_json::to_value(status).map_err(|e| err(&e))
        }
        Request::ServerStop => {
            crate::commands::server::stop_server(&state)
                .await
                .map_err(|e| err(&e))?;
            Ok(serde_json::json!({ "running": false }))
        }
        Request::ServerStatus => {
            let status = crate::commands::server::status_of(&state)
                .await
                .map_err(|e| err(&e))?;
            serde_json::to_value(status).map_err(|e| err(&e))
        }
        Request::AnalyzeStart { force } => {
            let queued = crate::commands::analysis::start_analysis(&state, force)
                .await
                .map_err(|e| err(&e))?;
            Ok(serde_json::json!({ "queued": queued }))
        }
        Request::AnalyzeStatus => {
            let summary = state.db.analysis().summary().await.map_err(|e| err(&e))?;
            let mut value = serde_json::to_value(summary).map_err(|e| err(&e))?;
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "running".into(),
                    serde_json::Value::Bool(
                        state.analyzing.load(std::sync::atomic::Ordering::SeqCst),
                    ),
                );
            }
            Ok(value)
        }
        Request::Search { query } => {
            let tracks = signal_search::search(&state.db, &query, 25)
                .await
                .map_err(|e| err(&e))?;
            let list: Vec<_> = tracks
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "title": t.title,
                        "codec": t.technical.codec,
                        "durationMs": t.duration_ms,
                    })
                })
                .collect();
            Ok(serde_json::Value::Array(list))
        }
    }
}

/// "50" absolute, "+5"/"-5" relative to `current`.
fn parse_relative(input: &str, current: u64) -> Option<u64> {
    let trimmed = input.trim();
    if let Some(delta) = trimmed.strip_prefix('+') {
        return delta.parse::<u64>().ok().map(|d| current + d);
    }
    if let Some(delta) = trimmed.strip_prefix('-') {
        return delta.parse::<u64>().ok().map(|d| current.saturating_sub(d));
    }
    trimmed.parse().ok()
}
