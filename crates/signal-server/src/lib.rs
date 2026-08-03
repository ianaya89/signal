//! Embedded `OpenSubsonic`-compatible HTTP server.
//!
//! Exposes the library over the Subsonic 1.16.1 REST API (plus `OpenSubsonic`
//! envelope fields) so mobile clients — `Symfonium`, `DSub`, `Feishin`, `Amperfy` —
//! can browse and stream over LAN. Pure adapter: every read and write goes
//! through the `signal-db` repositories. MVP scope: GET only, no transcode,
//! no TLS, no image resizing.

mod auth;
mod dto;
mod envelope;
mod handlers;
mod ids;
mod params;
mod xml;

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::response::Response;
use signal_db::DbPool;

use crate::envelope::{ApiError, Format};
use crate::params::Params;

pub struct ServerConfig {
    /// 0 binds an ephemeral port (tests).
    pub port: u16,
    pub password: String,
    /// Reported as `serverVersion` in every envelope.
    pub server_version: String,
}

pub struct ServerHandle {
    addr: std::net::SocketAddr,
    shutdown: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl ServerHandle {
    #[must_use]
    pub fn addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    pub async fn stop(self) {
        let _ = self.shutdown.send(());
        let _ = self.task.await;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("bind failed: {0}")]
    Bind(#[from] std::io::Error),
}

pub(crate) struct Ctx {
    pub db: DbPool,
    pub password: String,
    pub server_version: String,
}

/// Binds and serves until [`ServerHandle::stop`].
///
/// # Errors
/// Fails when the port can't be bound (in use, privileged, no permission).
pub async fn start(db: DbPool, cfg: ServerConfig) -> Result<ServerHandle, ServerError> {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", cfg.port)).await?;
    let addr = listener.local_addr()?;

    let ctx = Arc::new(Ctx {
        db,
        password: cfg.password,
        server_version: cfg.server_version,
    });
    let app = axum::Router::new()
        // some clients (Amperfy) GET the bare URL to verify reachability
        // before speaking Subsonic; a 404 there aborts their login
        .route("/", axum::routing::get(|| async { "Signal OpenSubsonic server" }))
        .route("/rest/{endpoint}", axum::routing::get(dispatch))
        .with_state(ctx);

    let (shutdown, rx) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let served = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await;
        if let Err(err) = served {
            tracing::error!("opensubsonic server died: {err}");
        }
    });

    tracing::info!(%addr, "opensubsonic server listening");
    Ok(ServerHandle {
        addr,
        shutdown,
        task,
    })
}

/// One dispatcher instead of per-route handlers: every Subsonic endpoint
/// shares query-string auth, `f=` negotiation, the envelope, and the
/// `.view` suffix quirk — and `star`'s repeated `id=` params rule out
/// axum's `Query<HashMap>` extractor anyway.
async fn dispatch(
    State(ctx): State<Arc<Ctx>>,
    Path(endpoint): Path<String>,
    req: Request,
) -> Response {
    let params = Params::parse(req.uri().query().unwrap_or(""));
    let format = Format::from_param(params.get("f"));

    if let Err(err) = auth::check(&params, &ctx.password) {
        return envelope::render(format, &ctx.server_version, Err(err));
    }

    let name = endpoint.strip_suffix(".view").unwrap_or(&endpoint);

    // binary endpoints bypass the envelope and use real HTTP semantics
    match name {
        "stream" | "download" => return handlers::media::stream(&ctx, &params, format, req).await,
        "getCoverArt" => return handlers::media::cover_art(&ctx, &params, format).await,
        _ => {}
    }

    let result = match name {
        "ping" => Ok(None),
        "getLicense" => Ok(Some(handlers::system::license())),
        "getOpenSubsonicExtensions" => Ok(Some(handlers::system::extensions())),
        "getScanStatus" => handlers::system::scan_status(&ctx).await,
        "getUser" => Ok(Some(handlers::system::user(&params))),
        "getMusicFolders" => Ok(Some(handlers::browsing::music_folders())),
        "getArtists" => handlers::browsing::artists(&ctx, "artists").await,
        "getIndexes" => handlers::browsing::artists(&ctx, "indexes").await,
        "getArtist" => handlers::browsing::artist(&ctx, &params).await,
        "getAlbum" => handlers::browsing::album(&ctx, &params).await,
        "getSong" => handlers::browsing::song(&ctx, &params).await,
        "getGenres" => handlers::browsing::genres(&ctx).await,
        "getArtistInfo2" => Ok(Some(("artistInfo2", serde_json::json!({})))),
        "getAlbumInfo2" => Ok(Some(("albumInfo2", serde_json::json!({})))),
        "getMusicDirectory" => Err(ApiError::not_found(
            "directory browsing not supported — use ID3 endpoints",
        )),
        "getAlbumList2" => handlers::lists::album_list2(&ctx, &params).await,
        "getRandomSongs" => handlers::lists::random_songs(&ctx, &params).await,
        "getSongsByGenre" => handlers::lists::songs_by_genre(&ctx, &params).await,
        "getStarred2" => handlers::lists::starred2(&ctx).await,
        "search3" => handlers::search::search3(&ctx, &params).await,
        "getPlaylists" => handlers::playlists::list(&ctx).await,
        "getPlaylist" => handlers::playlists::get(&ctx, &params).await,
        "createPlaylist" => handlers::playlists::create(&ctx, &params).await,
        "updatePlaylist" => handlers::playlists::update(&ctx, &params).await,
        "deletePlaylist" => handlers::playlists::delete(&ctx, &params).await,
        "star" => handlers::annotation::star(&ctx, &params, true).await,
        "unstar" => handlers::annotation::star(&ctx, &params, false).await,
        "setRating" => handlers::annotation::set_rating(&ctx, &params).await,
        "scrobble" => handlers::annotation::scrobble(&ctx, &params).await,
        other => Err(ApiError::generic(format!(
            "endpoint '{other}' not implemented"
        ))),
    };
    envelope::render(format, &ctx.server_version, result)
}
