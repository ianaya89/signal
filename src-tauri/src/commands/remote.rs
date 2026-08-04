//! Remote `OpenSubsonic` sources: configuration, browsing, and playback.
//!
//! Kept separate from `player`/`queue` rather than widening their `trackId`
//! arguments into a source union — this milestone is additive, and unifying
//! the two paths touches every existing call site (`docs/11-subsonic-client.md`
//! §3, Phase 3).

use signal_core::SignalError;
use signal_db::{RemoteSource, RemoteSourcePatch};
use signal_subsonic_client::types::{
    AlbumWithSongs, ArtistWithAlbums, ArtistsIndex, Child, SearchResult3,
};
use signal_subsonic_client::{AuthMode, ClientConfig, ClientError, SearchLimits, SubsonicClient};
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::{AppState, RemoteTrack};

trait RemoteResultExt<T> {
    fn remote_err(self) -> Result<T, SignalError>;
}

impl<T> RemoteResultExt<T> for Result<T, ClientError> {
    fn remote_err(self) -> Result<T, SignalError> {
        self.map_err(|e| SignalError::Remote(e.to_string()))
    }
}

/// Outcome of a connection test, including which auth form the server took —
/// the UI shows it so a server that silently fell back to plaintext is visible.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub ok: bool,
    pub auth_mode: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: bool,
    pub error: Option<String>,
}

/// Builds (or reuses) the client for `source_id`.
///
/// Cached in `AppState` because a `SubsonicClient` owns a `reqwest::Client`,
/// and rebuilding one per request throws away connection pooling and the TLS
/// session cache. Invalidated whenever the row changes.
async fn client_for(
    state: &State<'_, AppState>,
    source_id: i64,
) -> Result<SubsonicClient, SignalError> {
    if let Ok(cache) = state.remote_clients.lock() {
        if let Some(client) = cache.get(&source_id) {
            return Ok(client.clone());
        }
    }

    let creds = state
        .db
        .remote_sources()
        .credentials(source_id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Remote(format!("no remote source {source_id}")))?;

    let cfg = ClientConfig::new(&creds.base_url, &creds.username, &creds.password)
        .with_auth_mode(AuthMode::from_str_or_default(&creds.auth_mode))
        .with_insecure_tls(creds.allow_insecure_tls);
    let client = SubsonicClient::new(&cfg).remote_err()?;

    if let Ok(mut cache) = state.remote_clients.lock() {
        cache.insert(source_id, client.clone());
    }
    Ok(client)
}

fn invalidate(state: &State<'_, AppState>, source_id: i64) {
    if let Ok(mut cache) = state.remote_clients.lock() {
        cache.remove(&source_id);
    }
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_source_list(
    state: State<'_, AppState>,
) -> Result<Vec<RemoteSource>, SignalError> {
    state.db.remote_sources().list().await.db_err()
}

#[tauri::command]
#[tracing::instrument(skip(state, password))]
pub async fn remote_source_add(
    state: State<'_, AppState>,
    name: String,
    base_url: String,
    username: String,
    password: String,
    allow_insecure_tls: bool,
) -> Result<RemoteSource, SignalError> {
    // reject a bad URL here rather than storing a row that can never connect
    let cfg = ClientConfig::new(&base_url, &username, &password);
    SubsonicClient::new(&cfg).remote_err()?;

    let id = state
        .db
        .remote_sources()
        .create(&name, &base_url, &username, &password, allow_insecure_tls)
        .await
        .db_err()?;
    state
        .db
        .remote_sources()
        .get(id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Remote("source vanished after insert".into()))
}

#[tauri::command]
#[tracing::instrument(skip(state, patch))]
pub async fn remote_source_update(
    state: State<'_, AppState>,
    id: i64,
    patch: RemoteSourcePatch,
) -> Result<RemoteSource, SignalError> {
    state
        .db
        .remote_sources()
        .update(id, &patch)
        .await
        .db_err()?;
    invalidate(&state, id);
    state
        .db
        .remote_sources()
        .get(id)
        .await
        .db_err()?
        .ok_or_else(|| SignalError::Remote(format!("no remote source {id}")))
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_source_remove(state: State<'_, AppState>, id: i64) -> Result<(), SignalError> {
    state.db.remote_sources().delete(id).await.db_err()?;
    invalidate(&state, id);
    Ok(())
}

/// Pings the server, falling back to plaintext auth for hosts that reject
/// tokens, and persists whichever form worked so later requests don't reprobe.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_source_test_connection(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ConnectionStatus, SignalError> {
    let client = client_for(&state, id).await?;

    let (mode, result) = match client.ping().await {
        Ok(ident) => (client.auth_mode(), Ok(ident)),
        Err(ClientError::Auth) if client.auth_mode() == AuthMode::Token => {
            let legacy = client.with_auth_mode(AuthMode::LegacyPlain);
            let result = legacy.ping().await;
            (AuthMode::LegacyPlain, result)
        }
        Err(err) => (client.auth_mode(), Err(err)),
    };

    let ok = result.is_ok();
    // only persist a mode that actually worked; a failed probe would otherwise
    // pin the source to plaintext forever
    let recorded = if ok { mode } else { client.auth_mode() };
    state
        .db
        .remote_sources()
        .record_ping(id, ok, recorded.as_str())
        .await
        .db_err()?;
    if recorded != client.auth_mode() {
        invalidate(&state, id);
    }

    Ok(match result {
        Ok(ident) => ConnectionStatus {
            ok: true,
            auth_mode: recorded.as_str().to_owned(),
            server_type: ident.server_type,
            server_version: ident.server_version,
            open_subsonic: ident.open_subsonic,
            error: None,
        },
        Err(err) => ConnectionStatus {
            ok: false,
            auth_mode: recorded.as_str().to_owned(),
            server_type: None,
            server_version: None,
            open_subsonic: false,
            error: Some(err.to_string()),
        },
    })
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_browse_artists(
    state: State<'_, AppState>,
    source_id: i64,
) -> Result<ArtistsIndex, SignalError> {
    client_for(&state, source_id)
        .await?
        .get_artists()
        .await
        .remote_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_browse_artist(
    state: State<'_, AppState>,
    source_id: i64,
    artist_id: String,
) -> Result<ArtistWithAlbums, SignalError> {
    client_for(&state, source_id)
        .await?
        .get_artist(&artist_id)
        .await
        .remote_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_browse_album(
    state: State<'_, AppState>,
    source_id: i64,
    album_id: String,
) -> Result<AlbumWithSongs, SignalError> {
    client_for(&state, source_id)
        .await?
        .get_album(&album_id)
        .await
        .remote_err()
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_search(
    state: State<'_, AppState>,
    source_id: i64,
    query: String,
) -> Result<SearchResult3, SignalError> {
    client_for(&state, source_id)
        .await?
        .search3(&query, SearchLimits::default())
        .await
        .remote_err()
}

/// Registers `songs` in the slab and returns their player ids, in order.
fn register(
    state: &State<'_, AppState>,
    source_id: i64,
    client: &SubsonicClient,
    songs: &[Child],
) -> Result<Vec<i64>, SignalError> {
    let mut slab = state
        .remote_tracks
        .lock()
        .map_err(|_| SignalError::Remote("remote track registry poisoned".into()))?;
    Ok(songs
        .iter()
        .map(|song| {
            slab.register(RemoteTrack {
                source_id,
                remote_id: song.id.clone(),
                url: client.stream_url(&song.id),
                title: song.title.clone(),
                artist: song.artist.clone().unwrap_or_default(),
                album: song.album.clone().unwrap_or_default(),
                // the wire carries whole seconds; the UI works in ms
                duration_ms: song.duration * 1_000,
                suffix: song.suffix.clone(),
                bitrate_kbps: song.bit_rate,
            })
        })
        .collect())
}

/// Plays `songs[start_index]` with the rest as the follow-on order.
///
/// The remote songs are registered under negative ids and dropped into the same
/// `play_context` local tracks use, so gapless staging, next/prev and
/// shuffle/repeat all work on them without a second code path
/// (`docs/11-subsonic-client.md` §2.3). The caller passes the songs it already
/// fetched rather than ids alone, because there is no `tracks` row to recover
/// their titles from later.
#[tauri::command]
#[tracing::instrument(skip(state, songs), fields(len = songs.len(), start_index))]
pub async fn remote_play_context(
    state: State<'_, AppState>,
    source_id: i64,
    songs: Vec<Child>,
    start_index: usize,
) -> Result<(), SignalError> {
    if start_index >= songs.len() {
        return Err(SignalError::Player("start index out of range".into()));
    }
    let client = client_for(&state, source_id).await?;
    let ids = register(&state, source_id, &client, &songs)?;
    let first = ids[start_index];

    if let Ok(mut ctx) = state.play_context.lock() {
        *ctx = crate::state::PlayContext::default();
        ctx.track_ids = ids;
        ctx.position = start_index;
    }
    crate::commands::player::start_track(&state, first).await
}

/// Plays one remote song, with nothing queued to follow it.
#[tauri::command]
#[tracing::instrument(skip(state, song))]
pub async fn remote_play(
    state: State<'_, AppState>,
    source_id: i64,
    song: Child,
) -> Result<(), SignalError> {
    remote_play_context(state, source_id, vec![song], 0).await
}

/// Now-playing metadata for a remote track, shaped like a local one.
///
/// The synthetic `Track` carries `-1` for `artist_id`/`album_id`: the remote
/// server's ids are opaque strings, and any local row those integers pointed at
/// would be the wrong record.
pub fn now_playing(
    state: &State<'_, AppState>,
    track_id: i64,
) -> Result<crate::commands::library::TrackWithContext, SignalError> {
    let remote = state
        .remote_tracks
        .lock()
        .ok()
        .and_then(|slab| slab.get(track_id).cloned())
        .ok_or_else(|| SignalError::Remote(format!("remote track {track_id} not found")))?;

    let now = chrono::Utc::now();
    Ok(crate::commands::library::TrackWithContext {
        artist_name: remote.artist.clone(),
        album_name: remote.album.clone(),
        genre: None,
        track: signal_core::Track {
            id: track_id,
            title: remote.title,
            artist_id: -1,
            album_id: -1,
            track_no: None,
            disc_no: None,
            year: None,
            duration_ms: remote.duration_ms,
            rating: None,
            favorite: false,
            play_count: 0,
            skip_count: 0,
            added_at: now,
            modified_at: now,
            last_played_at: None,
            technical: signal_core::TrackTechnical {
                codec: remote.suffix.to_uppercase(),
                container: remote.suffix.to_uppercase(),
                bitrate_kbps: remote.bitrate_kbps,
                bit_depth: None,
                sample_rate_hz: 0,
                channels: 0,
                replaygain_track_gain: None,
                replaygain_album_gain: None,
                peak: None,
                dr_score: None,
                encoder: None,
                file_path: std::path::PathBuf::new(),
                file_size_bytes: 0,
                md5: None,
            },
        },
    })
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_stream_url(
    state: State<'_, AppState>,
    source_id: i64,
    remote_id: String,
) -> Result<String, SignalError> {
    Ok(client_for(&state, source_id).await?.stream_url(&remote_id))
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn remote_cover_art_url(
    state: State<'_, AppState>,
    source_id: i64,
    remote_id: String,
    size: Option<u32>,
) -> Result<String, SignalError> {
    Ok(client_for(&state, source_id)
        .await?
        .cover_art_url(&remote_id, size))
}
