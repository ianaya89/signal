use serde::Serialize;

/// Top-level error type crossing crate and IPC boundaries.
///
/// Serialized form is `{ "kind": "...", "message": ... }` so the frontend can
/// map kinds to toast styles (see `docs/05-ipc-api.md`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum SignalError {
    #[error("database error: {0}")]
    Db(String),
    #[error("player error: {0}")]
    Player(String),
    #[error("scanner error: {0}")]
    Scanner(String),
    #[error("analysis error: {0}")]
    Analysis(String),
    #[error("search error: {0}")]
    Search(String),
    #[error("plugin error: {0}")]
    Plugin(String),
    #[error("remote server error: {0}")]
    Remote(String),
    #[error("invalid query: {reason}")]
    InvalidQuery { reason: String },
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for SignalError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
