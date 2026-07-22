//! `signal-art://` protocol: serves album artwork straight from disk to the
//! webview, avoiding base64 payloads over IPC.
//!
//! URL shape: `signal-art://localhost/album/<album_id>` (macOS/Linux) or
//! `http://signal-art.localhost/album/<album_id>` (Windows).

use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder, Wry};

use crate::state::AppState;

pub const SCHEME: &str = "signal-art";

// Signature (owned args) is fixed by register_asynchronous_uri_scheme_protocol.
#[allow(clippy::needless_pass_by_value)]
pub fn handle(
    ctx: UriSchemeContext<'_, Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let path = request.uri().path().to_owned();
    tauri::async_runtime::spawn(async move {
        responder.respond(respond(&app, &path).await);
    });
}

async fn respond(app: &tauri::AppHandle, url_path: &str) -> Response<Vec<u8>> {
    match artwork_bytes(app, url_path).await {
        Some((bytes, mime)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, "max-age=86400")
            .body(bytes)
            .unwrap_or_else(|_| Response::new(Vec::new())),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .unwrap_or_else(|_| Response::new(Vec::new())),
    }
}

async fn artwork_bytes(app: &tauri::AppHandle, url_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let album_id: i64 = url_path.strip_prefix("/album/")?.parse().ok()?;
    let state = app.state::<AppState>();
    let art_path = state.db.albums().artwork_path(album_id).await.ok()??;

    let mime = if art_path.to_ascii_lowercase().ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    let bytes = tokio::fs::read(&art_path).await.ok()?;
    Some((bytes, mime))
}
