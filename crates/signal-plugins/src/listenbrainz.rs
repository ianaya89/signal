//! ListenBrainz submission API (https://listenbrainz.readthedocs.io):
//! token auth, no request signing.

use serde::Serialize;

const API: &str = "https://api.listenbrainz.org/1";

#[derive(Debug, Clone)]
pub struct Listen {
    pub artist: String,
    pub track: String,
    pub album: Option<String>,
    /// Unix seconds when playback started.
    pub listened_at: i64,
}

#[derive(Serialize)]
struct SubmitBody<'a> {
    listen_type: &'static str,
    payload: Vec<Payload<'a>>,
}

#[derive(Serialize)]
struct Payload<'a> {
    listened_at: i64,
    track_metadata: TrackMetadata<'a>,
}

#[derive(Serialize)]
struct TrackMetadata<'a> {
    artist_name: &'a str,
    track_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_name: Option<&'a str>,
}

pub async fn submit(
    http: &reqwest::Client,
    token: &str,
    listen: &Listen,
) -> Result<(), reqwest::Error> {
    let body = SubmitBody {
        listen_type: "single",
        payload: vec![Payload {
            listened_at: listen.listened_at,
            track_metadata: TrackMetadata {
                artist_name: &listen.artist,
                track_name: &listen.track,
                release_name: listen.album.as_deref(),
            },
        }],
    };

    http.post(format!("{API}/submit-listens"))
        .header("Authorization", format!("Token {token}"))
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn validate(http: &reqwest::Client, token: &str) -> Result<bool, reqwest::Error> {
    let resp = http
        .get(format!("{API}/validate-token"))
        .header("Authorization", format!("Token {token}"))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Ok(false);
    }
    let body: serde_json::Value = resp.json().await?;
    Ok(body.get("valid").and_then(serde_json::Value::as_bool) == Some(true))
}
