//! Row → domain-type mapping helpers.
//!
//! `SQLite` INTEGER is always `i64`; domain types use unsigned ints that are
//! valid by schema construction (CHECK constraints, NOT NULL defaults), so
//! lossy conversions fall back to 0 rather than failing a whole list query.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use signal_core::{Track, TrackTechnical};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

pub fn to_u32(v: i64) -> u32 {
    u32::try_from(v).unwrap_or_default()
}

pub fn to_u64(v: i64) -> u64 {
    u64::try_from(v).unwrap_or_default()
}

pub fn to_u8(v: i64) -> u8 {
    u8::try_from(v).unwrap_or_default()
}

pub fn track_from_row(row: &SqliteRow) -> sqlx::Result<Track> {
    let rating = to_u8(row.try_get::<i64, _>("rating")?);
    Ok(Track {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        artist_id: row.try_get("artist_id")?,
        album_id: row
            .try_get::<Option<i64>, _>("album_id")?
            .unwrap_or_default(),
        track_no: row.try_get::<Option<i64>, _>("track_no")?.map(to_u32),
        disc_no: row.try_get::<Option<i64>, _>("disc_no")?.map(to_u32),
        year: row
            .try_get::<Option<i64>, _>("year")?
            .map(|y| i32::try_from(y).unwrap_or_default()),
        duration_ms: to_u64(row.try_get::<i64, _>("duration_ms")?),
        rating: (rating > 0).then_some(rating),
        favorite: row.try_get::<i64, _>("favorite")? != 0,
        play_count: to_u32(row.try_get::<i64, _>("play_count")?),
        skip_count: to_u32(row.try_get::<i64, _>("skip_count")?),
        added_at: row.try_get::<DateTime<Utc>, _>("added_at")?,
        modified_at: row.try_get::<DateTime<Utc>, _>("modified_at")?,
        last_played_at: row.try_get::<Option<DateTime<Utc>>, _>("last_played_at")?,
        technical: TrackTechnical {
            codec: row.try_get("codec")?,
            container: row.try_get("container")?,
            bitrate_kbps: row
                .try_get::<Option<i64>, _>("bitrate_kbps")?
                .map(to_u32)
                .unwrap_or_default(),
            bit_depth: row.try_get::<Option<i64>, _>("bit_depth")?.map(to_u8),
            sample_rate_hz: to_u32(row.try_get::<i64, _>("sample_rate_hz")?),
            channels: to_u8(row.try_get::<i64, _>("channels")?),
            replaygain_track_gain: row.try_get("replaygain_track_gain")?,
            replaygain_album_gain: row.try_get("replaygain_album_gain")?,
            peak: row.try_get("peak")?,
            dr_score: row.try_get("dr_score")?,
            encoder: row.try_get("encoder")?,
            file_path: PathBuf::from(row.try_get::<String, _>("file_path")?),
            file_size_bytes: to_u64(row.try_get::<i64, _>("file_size_bytes")?),
            md5: row.try_get("md5")?,
        },
    })
}
