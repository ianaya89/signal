//! ping/license/identity stubs. Symfonium probes these on connect; honest
//! stubs beat error toasts on the device.

use serde_json::json;

use crate::envelope::{ApiError, HandlerResult};
use crate::params::Params;
use crate::Ctx;

type Entry = (&'static str, serde_json::Value);

pub(crate) fn license() -> Entry {
    ("license", json!({ "valid": true }))
}

pub(crate) fn extensions() -> Entry {
    (
        "openSubsonicExtensions",
        json!([{ "name": "formPost", "versions": [1] }]),
    )
}

pub(crate) async fn scan_status(ctx: &Ctx) -> HandlerResult {
    let count = ctx.db.tracks().count().await.map_err(ApiError::db)?;
    Ok(Some((
        "scanStatus",
        json!({ "scanning": false, "count": count }),
    )))
}

pub(crate) fn user(params: &Params) -> Entry {
    let username = params.get("username").or_else(|| params.get("u")).unwrap_or("signal");
    (
        "user",
        json!({
            "username": username,
            "scrobblingEnabled": true,
            "adminRole": false,
            "settingsRole": false,
            "downloadRole": true,
            "uploadRole": false,
            "playlistRole": true,
            "coverArtRole": false,
            "commentRole": false,
            "podcastRole": false,
            "streamRole": true,
            "jukeboxRole": false,
            "shareRole": false,
            "videoConversionRole": false,
            "folder": [1],
        }),
    )
}
