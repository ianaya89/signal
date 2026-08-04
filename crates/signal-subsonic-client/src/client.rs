//! The HTTP client. One instance per configured remote source, so per-source
//! TLS settings never leak across sources.

use serde::de::DeserializeOwned;
use signal_subsonic_types::{
    AlbumWithSongs, ArtistWithAlbums, ArtistsIndex, Envelope, ResponseBody, SearchResult3,
};

use crate::auth::{self, AuthMode, SaltGen};
use crate::error::{snippet, ClientError};

/// Protocol version claimed on every request. Matches what `signal-server`
/// reports, and is old enough that no real server refuses it.
pub const API_VERSION: &str = "1.16.1";

/// The `c=` client identifier remote servers log and show in their UI.
const CLIENT_NAME: &str = "signal";

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub auth_mode: AuthMode,
    /// Accept self-signed / unknown-CA certificates for this source only.
    /// Never a global setting: homelab servers are common, but one source
    /// opting out of verification must not weaken the others.
    pub allow_insecure_tls: bool,
}

impl ClientConfig {
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            username: username.into(),
            password: password.into(),
            auth_mode: AuthMode::Token,
            allow_insecure_tls: false,
        }
    }

    #[must_use]
    pub fn with_auth_mode(mut self, mode: AuthMode) -> Self {
        self.auth_mode = mode;
        self
    }

    #[must_use]
    pub fn with_insecure_tls(mut self, allow: bool) -> Self {
        self.allow_insecure_tls = allow;
        self
    }
}

/// What a successful `ping` tells us about the far end.
#[derive(Debug, Clone)]
pub struct ServerIdent {
    pub version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchLimits {
    pub artist_count: u32,
    pub album_count: u32,
    pub song_count: u32,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            artist_count: 20,
            album_count: 20,
            song_count: 20,
        }
    }
}

/// Cheap to clone — the underlying `reqwest::Client` shares one connection
/// pool, which is the point: callers cache a client per source rather than
/// rebuilding the TLS stack per request.
#[derive(Debug)]
pub struct SubsonicClient {
    http: reqwest::Client,
    /// Normalized: scheme present, no trailing slash, no `/rest` suffix.
    base: String,
    username: String,
    password: String,
    auth_mode: AuthMode,
    salt: SaltGen,
}

impl SubsonicClient {
    /// # Errors
    /// Fails when the base URL isn't an absolute http(s) URL, or when the TLS
    /// backend can't be initialized.
    pub fn new(cfg: &ClientConfig) -> Result<Self, ClientError> {
        let base = normalize_base(&cfg.base_url)?;
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(cfg.allow_insecure_tls)
            .build()?;
        Ok(Self {
            http,
            base,
            username: cfg.username.clone(),
            password: cfg.password.clone(),
            auth_mode: cfg.auth_mode,
            salt: SaltGen::default(),
        })
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base
    }

    #[must_use]
    pub fn auth_mode(&self) -> AuthMode {
        self.auth_mode
    }

    /// A copy of this client speaking `mode` instead — used by the connection
    /// probe to retry with `p=` without rebuilding the TLS stack.
    ///
    /// The salt counter restarts at zero in the copy, which is fine: salts also
    /// mix in the clock, so two clients never hand the same server the same one.
    #[must_use]
    pub fn with_auth_mode(&self, mode: AuthMode) -> Self {
        Self {
            http: self.http.clone(),
            base: self.base.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            auth_mode: mode,
            salt: SaltGen::default(),
        }
    }

    /// # Errors
    /// Fails on transport errors, auth rejection, or a non-Subsonic reply.
    ///
    /// A successful ping says the credentials work; it says nothing about
    /// whether the server implements the browse endpoints.
    pub async fn ping(&self) -> Result<ServerIdent, ClientError> {
        let body = self.call("ping", &[]).await?;
        Ok(ServerIdent {
            version: body.version,
            server_type: body.server_type,
            server_version: body.server_version,
            open_subsonic: body.open_subsonic,
        })
    }

    /// # Errors
    /// Fails on transport errors, auth rejection, or a non-Subsonic reply.
    pub async fn get_artists(&self) -> Result<ArtistsIndex, ClientError> {
        let body = self.call("getArtists", &[]).await?;
        payload_or_default(&body, "artists")
    }

    /// # Errors
    /// Fails as above, plus when the artist id is unknown to the remote.
    pub async fn get_artist(&self, id: &str) -> Result<ArtistWithAlbums, ClientError> {
        let body = self.call("getArtist", &[("id", id.to_owned())]).await?;
        payload_required(&body, "artist")
    }

    /// # Errors
    /// Fails as above, plus when the album id is unknown to the remote.
    pub async fn get_album(&self, id: &str) -> Result<AlbumWithSongs, ClientError> {
        let body = self.call("getAlbum", &[("id", id.to_owned())]).await?;
        payload_required(&body, "album")
    }

    /// # Errors
    /// Fails on transport errors, auth rejection, or a non-Subsonic reply.
    pub async fn search3(
        &self,
        query: &str,
        limits: SearchLimits,
    ) -> Result<SearchResult3, ClientError> {
        let body = self
            .call(
                "search3",
                &[
                    ("query", query.to_owned()),
                    ("artistCount", limits.artist_count.to_string()),
                    ("albumCount", limits.album_count.to_string()),
                    ("songCount", limits.song_count.to_string()),
                ],
            )
            .await?;
        payload_or_default(&body, "searchResult3")
    }

    /// Authenticated stream URL — handed to mpv as-is, never fetched here.
    #[must_use]
    pub fn stream_url(&self, id: &str) -> String {
        self.url_for("stream", &[("id", id.to_owned())])
    }

    /// Authenticated cover-art URL — used directly as an `<img src>`.
    #[must_use]
    pub fn cover_art_url(&self, id: &str, size: Option<u32>) -> String {
        let mut extra = vec![("id", id.to_owned())];
        if let Some(size) = size {
            extra.push(("size", size.to_string()));
        }
        self.url_for("getCoverArt", &extra)
    }

    fn url_for(&self, endpoint: &str, extra: &[(&str, String)]) -> String {
        format!("{}/rest/{endpoint}?{}", self.base, self.query_string(extra))
    }

    fn query_string(&self, extra: &[(&str, String)]) -> String {
        let mut pairs: Vec<(&str, String)> = vec![("u", self.username.clone())];
        match self.auth_mode {
            AuthMode::Token => {
                let salt = self.salt.next_salt();
                pairs.push(("t", auth::token(&self.password, &salt)));
                pairs.push(("s", salt));
            }
            AuthMode::LegacyPlain => pairs.push(("p", self.password.clone())),
        }
        pairs.push(("v", API_VERSION.to_owned()));
        pairs.push(("c", CLIENT_NAME.to_owned()));
        pairs.push(("f", "json".to_owned()));
        pairs.extend(extra.iter().cloned());
        serde_urlencoded::to_string(&pairs).unwrap_or_default()
    }

    async fn call(
        &self,
        endpoint: &str,
        extra: &[(&str, String)],
    ) -> Result<ResponseBody, ClientError> {
        let url = self.url_for(endpoint, extra);
        tracing::debug!(endpoint, base = %self.base, "subsonic request");
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        // Subsonic answers errors inside a 200 envelope, but not every server
        // does — parse first, and only fall back to the HTTP status when the
        // body turns out not to be an envelope at all.
        let envelope: Envelope = match serde_json::from_str(&text) {
            Ok(envelope) => envelope,
            Err(err) => {
                return Err(if status.is_success() {
                    ClientError::Parse(format!("{endpoint}: {err} — body: {}", snippet(&text)))
                } else {
                    ClientError::Parse(format!("{endpoint}: HTTP {status} — {}", snippet(&text)))
                })
            }
        };

        let body = envelope.response;
        if let Some(err) = body.error {
            return Err(if err.is_auth_failure() {
                ClientError::Auth
            } else {
                ClientError::Api {
                    code: err.code,
                    message: err.message,
                }
            });
        }
        if body.status == "failed" {
            return Err(ClientError::Api {
                code: 0,
                message: format!("{endpoint} failed without an error body"),
            });
        }
        Ok(body)
    }
}

impl Clone for SubsonicClient {
    fn clone(&self) -> Self {
        self.with_auth_mode(self.auth_mode)
    }
}

/// Missing key means "the server had nothing to report" — an empty library
/// answers `getArtists` with no `artists` key at all.
fn payload_or_default<T: DeserializeOwned + Default>(
    body: &ResponseBody,
    key: &str,
) -> Result<T, ClientError> {
    body.take::<T>(key)
        .map(Option::unwrap_or_default)
        .map_err(|err| ClientError::Parse(format!("payload '{key}': {err}")))
}

/// Missing key is a protocol violation for endpoints that answer about one
/// specific entity — the server said ok but told us nothing.
fn payload_required<T: DeserializeOwned>(body: &ResponseBody, key: &str) -> Result<T, ClientError> {
    body.take::<T>(key)
        .map_err(|err| ClientError::Parse(format!("payload '{key}': {err}")))?
        .ok_or_else(|| ClientError::Parse(format!("response carried no '{key}' payload")))
}

fn normalize_base(raw: &str) -> Result<String, ClientError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(ClientError::BaseUrl("url is empty".to_owned()));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(ClientError::BaseUrl(format!(
            "expected an http:// or https:// url, got '{raw}'"
        )));
    }
    // people paste the value from another client's config, which often already
    // includes the /rest path this crate appends itself
    let base = trimmed.strip_suffix("/rest").unwrap_or(trimmed);
    Ok(base.trim_end_matches('/').to_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn client(base: &str) -> SubsonicClient {
        SubsonicClient::new(&ClientConfig::new(base, "ian", "sesame")).unwrap()
    }

    #[test]
    fn base_url_normalization() {
        assert_eq!(
            normalize_base("http://nas:4533/").unwrap(),
            "http://nas:4533"
        );
        assert_eq!(
            normalize_base("https://nas.example/rest/").unwrap(),
            "https://nas.example"
        );
        assert_eq!(
            normalize_base("  http://nas:4533  ").unwrap(),
            "http://nas:4533"
        );
        assert!(normalize_base("nas:4533").is_err());
        assert!(normalize_base("").is_err());
    }

    #[test]
    fn stream_url_carries_token_auth_and_the_id() {
        let url = client("http://nas:4533").stream_url("tr-7");
        assert!(url.starts_with("http://nas:4533/rest/stream?"), "{url}");
        assert!(url.contains("u=ian"), "{url}");
        assert!(url.contains("&t=") && url.contains("&s="), "{url}");
        assert!(!url.contains("p=sesame"), "password leaked: {url}");
        assert!(url.contains("id=tr-7"), "{url}");
    }

    #[test]
    fn legacy_mode_sends_the_plain_password_instead() {
        let cfg = ClientConfig::new("http://nas:4533", "ian", "sesame")
            .with_auth_mode(AuthMode::LegacyPlain);
        let url = SubsonicClient::new(&cfg).unwrap().stream_url("tr-7");
        assert!(url.contains("p=sesame"), "{url}");
        assert!(!url.contains("&t="), "{url}");
    }

    #[test]
    fn cover_art_url_omits_size_when_unset() {
        let c = client("http://nas:4533");
        assert!(!c.cover_art_url("al-3", None).contains("size="));
        assert!(c.cover_art_url("al-3", Some(300)).contains("size=300"));
    }

    #[test]
    fn query_params_are_percent_encoded() {
        let cfg = ClientConfig::new("http://nas:4533", "ian@home", "p&w=d");
        let url = SubsonicClient::new(&cfg)
            .unwrap()
            .with_auth_mode(AuthMode::LegacyPlain)
            .stream_url("a b");
        assert!(url.contains("u=ian%40home"), "{url}");
        assert!(url.contains("p=p%26w%3Dd"), "{url}");
        assert!(url.contains("id=a+b"), "{url}");
    }
}
