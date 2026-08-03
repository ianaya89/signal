//! Binary endpoints: stream/download (Range-capable file serving) and
//! cover art. These bypass the envelope and speak plain HTTP.

use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use tower::ServiceExt as _;
use tower_http::services::ServeFile;

use crate::dto::{content_type_of, suffix_of};
use crate::envelope::{self, ApiError, Format};
use crate::ids::Sid;
use crate::params::Params;
use crate::Ctx;

pub(crate) async fn stream(ctx: &Ctx, params: &Params, format: Format, req: Request) -> Response {
    let track = match track_of(ctx, params).await {
        Ok(track) => track,
        Err(err) => return envelope::render(format, &ctx.server_version, Err(err)),
    };

    let path = &track.technical.file_path;
    // ServeFile handles Range (206 + Content-Range), If-Modified-Since and
    // content-type; maxBitRate/format params are ignored (no transcode, v1)
    let served = ServeFile::new(path).oneshot(req).await;
    let mut response = match served {
        Ok(response) => response.map(axum::body::Body::new),
        Err(err) => {
            tracing::warn!(path = %path.display(), "stream failed: {err}");
            return envelope::render(
                format,
                &ctx.server_version,
                Err(ApiError::not_found("file missing on disk")),
            );
        }
    };

    let known_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("audio/"));
    if !known_type && response.status().is_success() {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(content_type_of(&suffix_of(path))),
        );
    }
    response
}

pub(crate) async fn cover_art(ctx: &Ctx, params: &Params, format: Format) -> Response {
    let enveloped_err = |err| envelope::render(format, &ctx.server_version, Err(err));

    let album_id = match params.require("id").map(Sid::parse) {
        Ok(Some(Sid::Album(id))) => id,
        Ok(_) => return enveloped_err(ApiError::not_found("no cover art for this id")),
        Err(err) => return enveloped_err(err),
    };
    let art_path = match ctx.db.albums().artwork_path(album_id).await {
        Ok(Some(path)) => path,
        Ok(None) => return enveloped_err(ApiError::not_found("album has no artwork")),
        Err(err) => return enveloped_err(ApiError::db(err)),
    };

    if let Some(size) = params.get_u32("size").filter(|s| *s > 0) {
        if let Some(bytes) = thumbnail(ctx, album_id, &art_path, size).await {
            return (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "image/jpeg"),
                    (header::CACHE_CONTROL, "max-age=86400"),
                ],
                bytes,
            )
                .into_response();
        }
        // resize failed — serving the original beats failing the request
    }

    let mime = if art_path.to_ascii_lowercase().ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    match tokio::fs::read(&art_path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime),
                (header::CACHE_CONTROL, "max-age=86400"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => enveloped_err(ApiError::not_found("artwork file missing on disk")),
    }
}

/// Bucketed so client whims (300 vs 320 vs 342…) can't grow the cache
/// unboundedly. Anything above the top bucket serves the original.
const THUMB_BUCKETS: &[u32] = &[64, 128, 256, 512, 1024];

/// Scaled JPEG from the on-disk thumbnail cache, generating on miss.
/// `None` on any failure or when the requested size exceeds the buckets.
async fn thumbnail(ctx: &Ctx, album_id: i64, art_path: &str, size: u32) -> Option<Vec<u8>> {
    let bucket = *THUMB_BUCKETS.iter().find(|b| **b >= size)?;
    let thumb_path = ctx.cover_cache_dir.join(format!("al-{album_id}-{bucket}.jpg"));
    let source = std::path::PathBuf::from(art_path);

    tokio::task::spawn_blocking(move || {
        let stale = match (thumb_path.metadata(), source.metadata()) {
            (Ok(thumb), Ok(src)) => match (thumb.modified(), src.modified()) {
                (Ok(t), Ok(s)) => s > t,
                _ => false,
            },
            _ => true,
        };
        if !stale {
            if let Ok(bytes) = std::fs::read(&thumb_path) {
                return Some(bytes);
            }
        }

        let scaled = image::open(&source).ok()?.thumbnail(bucket, bucket);
        scaled.to_rgb8().save(&thumb_path).ok()?;
        std::fs::read(&thumb_path).ok()
    })
    .await
    .ok()
    .flatten()
}

async fn track_of(ctx: &Ctx, params: &Params) -> Result<signal_core::Track, ApiError> {
    let Some(Sid::Track(id)) = Sid::parse(params.require("id")?) else {
        return Err(ApiError::not_found("no such song"));
    };
    ctx.db
        .tracks()
        .get(id)
        .await
        .map_err(ApiError::db)?
        .ok_or_else(|| ApiError::not_found("no such song"))
}
