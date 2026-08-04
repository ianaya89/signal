//! Client-side failures. Auth is split out from the generic API error because
//! callers act on it differently — it's the one failure a user can fix by
//! editing the source's credentials.

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("invalid server url: {0}")]
    BaseUrl(String),

    #[error("wrong username or password")]
    Auth,

    /// The remote answered a well-formed Subsonic error envelope.
    #[error("server error {code}: {message}")]
    Api { code: u32, message: String },

    /// Not a Subsonic envelope at all — an HTML error page, a truncated body,
    /// or a payload whose shape doesn't match. Carries a snippet, since the
    /// cause is usually visible in the first line of what came back.
    #[error("unexpected response: {0}")]
    Parse(String),
}

/// Longest response fragment quoted back in a [`ClientError::Parse`] message.
const SNIPPET_LIMIT: usize = 200;

pub(crate) fn snippet(body: &str) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= SNIPPET_LIMIT {
        return flat;
    }
    let truncated: String = flat.chars().take(SNIPPET_LIMIT).collect();
    format!("{truncated}…")
}
