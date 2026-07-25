//! Plugin host. Network-touching integrations live here, opt-in and off by
//! default (no-telemetry principle). First real plugin: ListenBrainz.
//! Design: `docs/08-plugins.md`.

#![allow(clippy::missing_errors_doc)]

mod listenbrainz;

use std::sync::RwLock;

pub use listenbrainz::Listen;

/// Holds enabled plugin state; cheap to share behind the app state.
pub struct PluginHost {
    listenbrainz_token: RwLock<Option<String>>,
    http: reqwest::Client,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self {
            listenbrainz_token: RwLock::new(None),
            http: reqwest::Client::new(),
        }
    }
}

impl PluginHost {
    pub fn set_listenbrainz_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.listenbrainz_token.write() {
            *guard = token.filter(|t| !t.trim().is_empty());
        }
    }

    #[must_use]
    pub fn listenbrainz_enabled(&self) -> bool {
        self.listenbrainz_token
            .read()
            .map(|t| t.is_some())
            .unwrap_or(false)
    }

    /// Submits a completed listen to every enabled scrobbler. Failures are
    /// logged, never bubbled — scrobbling must not affect playback.
    pub async fn scrobble(&self, listen: Listen) {
        let token = self.listenbrainz_token.read().ok().and_then(|t| t.clone());
        if let Some(token) = token {
            if let Err(err) = listenbrainz::submit(&self.http, &token, &listen).await {
                tracing::warn!("listenbrainz submit failed: {err}");
            } else {
                tracing::info!(track = %listen.track, "scrobbled to listenbrainz");
            }
        }
    }

    /// Validates a token against the ListenBrainz API.
    pub async fn validate_listenbrainz(&self, token: &str) -> Result<bool, reqwest::Error> {
        listenbrainz::validate(&self.http, token).await
    }
}
