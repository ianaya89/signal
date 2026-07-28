//! Optional online artwork fetch: `MusicBrainz` release search feeding the
//! Cover Art Archive. Strictly user-triggered (doctor button) — the app
//! never phones home on its own.
//!
//! The lookup is slow by protocol (`MusicBrainz` allows ~1 request/second), so
//! every album reports its outcome on `artwork:progress` as it lands and the
//! run can be cancelled mid-batch.

use std::sync::atomic::Ordering;

use signal_core::{SignalError, SignalEvent};
use tauri::State;

use crate::commands::DbResultExt;
use crate::state::AppState;

const USER_AGENT: &str = "signal-player/0.1 (https://github.com/ianaya89/signal)";
const BATCH: i64 = 15;
const REQUEST_TIMEOUT_SECS: u64 = 10;
/// `MusicBrainz` asks for at most one request per second.
const THROTTLE_MS: u64 = 1100;

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkFetchSummary {
    /// albums attempted in this batch
    pub scanned: u32,
    pub fetched: u32,
    pub failed: u32,
    /// albums still without artwork after this batch
    pub remaining: u32,
    pub cancelled: bool,
}

/// Tries to fetch missing covers for the most recently added albums without
/// artwork, one capped batch per call. Progress is emitted per album.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn library_fetch_artwork(
    state: State<'_, AppState>,
) -> Result<ArtworkFetchSummary, SignalError> {
    let albums = state.db.albums().without_artwork(BATCH).await.db_err()?;
    let total = u32::try_from(albums.len()).unwrap_or_default();
    if albums.is_empty() {
        return Ok(ArtworkFetchSummary::default());
    }

    state.artwork_cancel.store(false, Ordering::SeqCst);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| SignalError::Io(e.to_string()))?;

    let mut summary = ArtworkFetchSummary::default();
    let mut processed = 0u32;
    for (album_id, name, artist) in albums {
        if state.artwork_cancel.load(Ordering::SeqCst) {
            summary.cancelled = true;
            break;
        }
        if processed > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(THROTTLE_MS)).await;
        }

        let result = fetch_one(&client, &state, album_id, &name, &artist).await;
        processed += 1;
        summary.scanned = processed;

        let (outcome, detail) = match &result {
            Ok(true) => {
                summary.fetched += 1;
                tracing::info!(album_id, name, "artwork fetched");
                ("found", None)
            }
            Ok(false) => {
                tracing::info!(album_id, name, "no artwork found");
                ("none", Some("no cover on the archive".to_owned()))
            }
            Err(err) => {
                summary.failed += 1;
                tracing::warn!(album_id, name, "artwork fetch failed: {err}");
                ("error", Some(err.clone()))
            }
        };

        state.events.publish(SignalEvent::ArtworkProgress {
            processed,
            total,
            album: name.clone(),
            artist: artist.clone(),
            outcome: outcome.to_owned(),
            detail,
        });
    }

    summary.remaining = u32::try_from(
        state
            .db
            .albums()
            .count_without_artwork()
            .await
            .db_err()?
            .max(0),
    )
    .unwrap_or_default();

    Ok(summary)
}

/// Stops the running batch after the album in flight resolves.
#[tauri::command]
// tauri commands take State by value
#[allow(clippy::needless_pass_by_value)]
pub fn library_fetch_artwork_cancel(state: State<'_, AppState>) {
    state.artwork_cancel.store(true, Ordering::SeqCst);
}

async fn fetch_one(
    client: &reqwest::Client,
    state: &State<'_, AppState>,
    album_id: i64,
    album: &str,
    artist: &str,
) -> Result<bool, String> {
    // strict phrase match first, then a loose one — punctuation and edition
    // suffixes ("… (Remastered)") miss often enough to be worth the retry
    let mut mbid = search_release(client, &lucene_phrase(album, artist)).await?;
    if mbid.is_none() {
        tokio::time::sleep(std::time::Duration::from_millis(THROTTLE_MS)).await;
        mbid = search_release(client, &lucene_loose(album, artist)).await?;
    }
    let Some(mbid) = mbid else {
        return Ok(false);
    };

    let image = client
        .get(format!(
            "https://coverartarchive.org/release/{mbid}/front-500"
        ))
        .send()
        .await
        .map_err(|e| net_error("cover art archive", &e))?;
    if image.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(false);
    }
    if !image.status().is_success() {
        return Err(format!("cover art archive returned {}", image.status()));
    }
    let bytes = image
        .bytes()
        .await
        .map_err(|e| net_error("cover art archive", &e))?;
    if bytes.len() < 512 {
        return Ok(false);
    }

    let dir = state.config.cache_dir.join("artwork");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("cannot write artwork cache: {e}"))?;
    let dest = dir.join(format!("album_{album_id}.jpg"));
    tokio::fs::write(&dest, &bytes)
        .await
        .map_err(|e| format!("cannot write {}: {e}", dest.display()))?;

    state
        .db
        .albums()
        .set_artwork(album_id, &dest.to_string_lossy())
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

async fn search_release(client: &reqwest::Client, query: &str) -> Result<Option<String>, String> {
    let response = client
        .get("https://musicbrainz.org/ws/2/release/")
        .query(&[("query", query), ("fmt", "json"), ("limit", "1")])
        .send()
        .await
        .map_err(|e| net_error("musicbrainz", &e))?;

    let status = response.status();
    if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
        return Err("musicbrainz is rate limiting — try again in a minute".to_owned());
    }
    if !status.is_success() {
        return Err(format!("musicbrainz returned {status}"));
    }

    let search: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("musicbrainz sent an unreadable response: {e}"))?;
    Ok(search["releases"]
        .get(0)
        .and_then(|r| r["id"].as_str())
        .map(str::to_owned))
}

fn lucene_phrase(album: &str, artist: &str) -> String {
    format!(
        "release:\"{}\" AND artist:\"{}\"",
        escape_lucene(album),
        escape_lucene(artist)
    )
}

fn lucene_loose(album: &str, artist: &str) -> String {
    format!(
        "release:({}) AND artist:({})",
        escape_lucene(album),
        escape_lucene(artist)
    )
}

/// A bare title containing `:` or a quote makes `MusicBrainz` reject the whole
/// query with a 400, which used to surface as "no cover found".
fn escape_lucene(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(
            ch,
            '+' | '-'
                | '&'
                | '|'
                | '!'
                | '('
                | ')'
                | '{'
                | '}'
                | '['
                | ']'
                | '^'
                | '"'
                | '~'
                | '*'
                | '?'
                | ':'
                | '\\'
                | '/'
        ) {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out.trim().to_owned()
}

fn net_error(who: &str, err: &reqwest::Error) -> String {
    if err.is_timeout() {
        format!("{who} timed out")
    } else if err.is_connect() {
        format!("cannot reach {who} — check your connection")
    } else {
        format!("{who}: {err}")
    }
}

#[cfg(test)]
mod tests {
    use super::escape_lucene;

    #[test]
    fn strips_lucene_operators() {
        assert_eq!(escape_lucene("A:B (Deluxe)"), "A B  Deluxe");
        assert_eq!(escape_lucene("AC/DC"), "AC DC");
        assert_eq!(escape_lucene("\"quoted\""), "quoted");
    }
}
