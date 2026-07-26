//! Optional online artwork fetch: MusicBrainz release search feeding the
//! Cover Art Archive. Strictly user-triggered (doctor button) — the app
//! never phones home on its own.

use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

const USER_AGENT: &str = "signal-player/0.1 (https://github.com/ianaya89/signal)";
const BATCH: i64 = 15;

/// Tries to fetch missing covers for the most recently added albums
/// without artwork (capped batch, ~1 req/s — MusicBrainz rate limit).
/// Returns how many albums gained artwork.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_fetch_artwork(state: State<'_, AppState>) -> Result<u32, SignalError> {
    let albums = state.db.albums().without_artwork(BATCH).await.db_err()?;
    if albums.is_empty() {
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| SignalError::Io(e.to_string()))?;

    let mut fetched = 0u32;
    for (album_id, name, artist) in albums {
        match fetch_one(&client, &state, album_id, &name, &artist).await {
            Ok(true) => {
                fetched += 1;
                tracing::info!(album_id, name, "artwork fetched");
            }
            Ok(false) => tracing::info!(album_id, name, "no artwork found"),
            Err(err) => tracing::warn!(album_id, name, "artwork fetch failed: {err}"),
        }
        // MusicBrainz asks for max 1 request/second
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }
    Ok(fetched)
}

async fn fetch_one(
    client: &reqwest::Client,
    state: &State<'_, AppState>,
    album_id: i64,
    album: &str,
    artist: &str,
) -> Result<bool, String> {
    let query = format!("release:\"{album}\" AND artist:\"{artist}\"");
    let search: serde_json::Value = client
        .get("https://musicbrainz.org/ws/2/release/")
        .query(&[("query", query.as_str()), ("fmt", "json"), ("limit", "1")])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let Some(mbid) = search["releases"]
        .get(0)
        .and_then(|r| r["id"].as_str())
        .map(str::to_owned)
    else {
        return Ok(false);
    };

    let image = client
        .get(format!(
            "https://coverartarchive.org/release/{mbid}/front-500"
        ))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !image.status().is_success() {
        return Ok(false);
    }
    let bytes = image.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() < 512 {
        return Ok(false);
    }

    let dir = state.config.cache_dir.join("artwork");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    let dest = dir.join(format!("album_{album_id}.jpg"));
    tokio::fs::write(&dest, &bytes)
        .await
        .map_err(|e| e.to_string())?;

    state
        .db
        .albums()
        .set_artwork(album_id, &dest.to_string_lossy())
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}
