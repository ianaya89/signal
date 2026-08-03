//! Stars, ratings, scrobbles — the phone writing back into Signal.

use crate::envelope::{ApiError, HandlerResult};
use crate::ids::Sid;
use crate::params::Params;
use crate::Ctx;

pub(crate) async fn star(ctx: &Ctx, params: &Params, value: bool) -> HandlerResult {
    let mut ids = params.get_all("id");
    ids.extend(params.get_all("albumId"));
    ids.extend(params.get_all("artistId"));
    if ids.is_empty() {
        return Err(ApiError::missing_param("id"));
    }

    for raw in ids {
        match Sid::parse(raw) {
            Some(Sid::Track(id)) => {
                ctx.db
                    .tracks()
                    .set_favorite(id, value)
                    .await
                    .map_err(ApiError::db)?;
            }
            // album/artist favorites don't exist in Signal; a hard error
            // here would degrade clients that star eagerly — accept + skip
            Some(Sid::Album(_) | Sid::Artist(_)) => {
                tracing::debug!(id = raw, "ignoring album/artist star (unsupported)");
            }
            _ => return Err(ApiError::not_found("no such id")),
        }
    }
    Ok(None)
}

pub(crate) async fn set_rating(ctx: &Ctx, params: &Params) -> HandlerResult {
    let Some(Sid::Track(id)) = Sid::parse(params.require("id")?) else {
        return Err(ApiError::not_found("ratings apply to songs only"));
    };
    let rating = params
        .get_u32("rating")
        .ok_or_else(|| ApiError::missing_param("rating"))?;
    if rating > 5 {
        return Err(ApiError::generic("rating must be 0-5"));
    }
    #[allow(clippy::cast_possible_truncation)]
    ctx.db
        .tracks()
        .set_rating(id, rating as u8)
        .await
        .map_err(ApiError::db)?;
    Ok(None)
}

pub(crate) async fn scrobble(ctx: &Ctx, params: &Params) -> HandlerResult {
    let Some(Sid::Track(id)) = Sid::parse(params.require("id")?) else {
        return Err(ApiError::not_found("no such song"));
    };
    // submission=false is a now-playing hint; Signal has no surface for it
    if params.get("submission") == Some("false") {
        return Ok(None);
    }

    let track = ctx
        .db
        .tracks()
        .get(id)
        .await
        .map_err(ApiError::db)?
        .ok_or_else(|| ApiError::not_found("no such song"))?;
    let started_at = params
        .get_i64("time")
        .and_then(chrono::DateTime::from_timestamp_millis)
        .unwrap_or_else(chrono::Utc::now);

    // same semantics as the desktop recorder: completed play, counters
    // bumped transactionally with the event row
    ctx.db
        .stats()
        .log_play_event(&signal_db::NewPlayEvent {
            track_id: id,
            started_at,
            ms_played: track.duration_ms,
            completed: true,
            skipped: false,
            source: signal_core::models::PlaySource::Remote,
        })
        .await
        .map_err(ApiError::db)?;
    Ok(None)
}
