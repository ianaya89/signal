//! Library health checks: duplicates, missing metadata, dead files,
//! suspicious quality. Read-only inspection; fixes go through the normal
//! repos.

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::to_u32;

const LIST_CAP: usize = 100;
/// lossy below this smells like a bad rip
const LOW_BITRATE_KBPS: i64 = 160;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRef {
    pub id: i64,
    pub title: String,
    pub artist_name: String,
    pub album_id: i64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    pub title: String,
    pub artist_name: String,
    pub count: u32,
    pub track_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumRef {
    pub id: i64,
    pub name: String,
    pub artist_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub total_tracks: u32,
    pub lossless_pct: u32,
    /// 0-100, penalized by each problem class
    pub score: u32,
    pub missing_files: Vec<TrackRef>,
    pub missing_files_total: u32,
    pub duplicates: Vec<DupGroup>,
    pub duplicates_total: u32,
    pub albums_without_art: Vec<AlbumRef>,
    pub albums_without_art_total: u32,
    pub tracks_without_year: u32,
    pub tracks_without_genre: u32,
    pub low_bitrate: Vec<TrackRef>,
    pub low_bitrate_total: u32,
}

pub struct HealthRepo {
    pool: SqlitePool,
}

impl HealthRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Full report. File-existence checks touch the filesystem — call from
    /// a context where ~1s for very large libraries is acceptable.
    // one linear sequence of independent checks; splitting adds nothing
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    pub async fn report(&self) -> sqlx::Result<HealthReport> {
        let total_tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&self.pool)
            .await?;

        let lossless: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks
             WHERE codec IN ('FLAC', 'ALAC', 'PCM (WAV)', 'PCM (AIFF)')",
        )
        .fetch_one(&self.pool)
        .await?;

        // ---- dead files: path no longer on disk ----
        let paths: Vec<(i64, String, String, String, i64)> = sqlx::query_as(
            "SELECT t.id, t.title, ar.name, t.file_path, t.album_id
             FROM tracks t JOIN artists ar ON ar.id = t.artist_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let missing: Vec<TrackRef> = tokio::task::spawn_blocking(move || {
            paths
                .into_iter()
                .filter(|(_, _, _, path, _)| !std::path::Path::new(path).is_file())
                .map(|(id, title, artist_name, path, album_id)| TrackRef {
                    id,
                    title,
                    artist_name,
                    album_id,
                    detail: path,
                })
                .collect()
        })
        .await
        .unwrap_or_default();

        // ---- probable duplicates: same title+artist, similar duration ----
        let duplicates = self.duplicate_groups().await?;

        // ---- albums without artwork ----
        let art_rows = sqlx::query(
            "SELECT al.id, al.name, ar.name AS artist_name
             FROM albums al JOIN artists ar ON ar.id = al.artist_id
             WHERE al.artwork_path IS NULL
             ORDER BY ar.name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        let albums_without_art: Vec<AlbumRef> = art_rows
            .iter()
            .map(|row| {
                Ok(AlbumRef {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    artist_name: row.try_get("artist_name")?,
                })
            })
            .collect::<sqlx::Result<_>>()?;

        let no_year: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE year IS NULL")
            .fetch_one(&self.pool)
            .await?;
        let no_genre: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks
             WHERE id NOT IN (SELECT track_id FROM track_genres)",
        )
        .fetch_one(&self.pool)
        .await?;

        // ---- suspicious quality: lossy below the floor ----
        let low_rows = sqlx::query(
            "SELECT t.id, t.title, ar.name AS artist_name, t.album_id,
                    t.codec, t.bitrate_kbps
             FROM tracks t JOIN artists ar ON ar.id = t.artist_id
             WHERE t.codec IN ('MP3', 'AAC', 'Opus', 'Vorbis')
               AND t.bitrate_kbps > 0 AND t.bitrate_kbps < ?1
             ORDER BY t.bitrate_kbps",
        )
        .bind(LOW_BITRATE_KBPS)
        .fetch_all(&self.pool)
        .await?;
        let low_bitrate: Vec<TrackRef> = low_rows
            .iter()
            .map(|row| {
                Ok(TrackRef {
                    id: row.try_get("id")?,
                    title: row.try_get("title")?,
                    artist_name: row.try_get("artist_name")?,
                    album_id: row.try_get("album_id")?,
                    detail: format!(
                        "{} @ {} kbps",
                        row.try_get::<String, _>("codec")?,
                        row.try_get::<i64, _>("bitrate_kbps")?
                    ),
                })
            })
            .collect::<sqlx::Result<_>>()?;

        let total = to_u32(total_tracks).max(1);
        let penalty = |count: usize, weight: f64| -> f64 {
            (count as f64 / f64::from(total)) * weight * 100.0
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let score = (100.0
            - penalty(missing.len(), 3.0)
            - penalty(duplicates.len(), 1.0)
            - penalty(albums_without_art.len(), 0.5)
            - penalty(usize::try_from(no_year).unwrap_or_default(), 0.3)
            - penalty(usize::try_from(no_genre).unwrap_or_default(), 0.3)
            - penalty(low_bitrate.len(), 0.5))
        .clamp(0.0, 100.0) as u32;

        Ok(HealthReport {
            total_tracks: to_u32(total_tracks),
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            lossless_pct: ((lossless as f64 / f64::from(total)) * 100.0).round() as u32,
            score,
            missing_files_total: u32::try_from(missing.len()).unwrap_or_default(),
            missing_files: missing.into_iter().take(LIST_CAP).collect(),
            duplicates_total: u32::try_from(duplicates.len()).unwrap_or_default(),
            duplicates: duplicates.into_iter().take(LIST_CAP).collect(),
            albums_without_art_total: u32::try_from(albums_without_art.len()).unwrap_or_default(),
            albums_without_art: albums_without_art.into_iter().take(LIST_CAP).collect(),
            tracks_without_year: to_u32(no_year),
            tracks_without_genre: to_u32(no_genre),
            low_bitrate_total: u32::try_from(low_bitrate.len()).unwrap_or_default(),
            low_bitrate: low_bitrate.into_iter().take(LIST_CAP).collect(),
        })
    }

    /// Probable duplicates: same artist + title (case-insensitive), similar
    /// duration. Shared by the report and the resolver.
    pub async fn duplicate_groups(&self) -> sqlx::Result<Vec<DupGroup>> {
        let dup_rows = sqlx::query(
            "SELECT lower(t.title) AS key_title, ar.name AS artist_name,
                    t.title AS title,
                    COUNT(*) AS cnt, group_concat(t.id) AS ids
             FROM tracks t JOIN artists ar ON ar.id = t.artist_id
             GROUP BY t.artist_id, lower(t.title), t.duration_ms / 3000
             HAVING cnt > 1
             ORDER BY cnt DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        dup_rows
            .iter()
            .map(|row| {
                let ids: Vec<i64> = row
                    .try_get::<String, _>("ids")
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|s| s.parse().ok())
                    .collect();
                Ok(DupGroup {
                    title: row.try_get("title")?,
                    artist_name: row.try_get("artist_name")?,
                    count: to_u32(row.try_get::<i64, _>("cnt")?),
                    track_ids: ids,
                })
            })
            .collect::<sqlx::Result<_>>()
    }

    /// Best-quality track of a duplicate group: lossless first, then bit
    /// depth, sample rate, bitrate; oldest id breaks ties.
    pub async fn pick_best(&self, track_ids: &[i64]) -> sqlx::Result<Option<i64>> {
        if track_ids.is_empty() {
            return Ok(None);
        }
        let placeholders = vec!["?"; track_ids.len()].join(",");
        let sql = format!(
            "SELECT id FROM tracks WHERE id IN ({placeholders})
             ORDER BY (codec IN ('FLAC', 'ALAC', 'PCM (WAV)', 'PCM (AIFF)')) DESC,
                      COALESCE(bit_depth, 0) DESC,
                      sample_rate_hz DESC,
                      COALESCE(bitrate_kbps, 0) DESC,
                      id ASC
             LIMIT 1"
        );
        let mut query = sqlx::query_scalar(&sql);
        for id in track_ids {
            query = query.bind(id);
        }
        query.fetch_optional(&self.pool).await
    }

    /// Removes tracks whose files are gone (cascades clean the rest).
    pub async fn prune_missing(&self, track_ids: &[i64]) -> sqlx::Result<u32> {
        let mut removed = 0u32;
        for id in track_ids {
            let res = sqlx::query("DELETE FROM tracks WHERE id = ?1")
                .bind(id)
                .execute(&self.pool)
                .await?;
            removed += to_u32(i64::try_from(res.rows_affected()).unwrap_or_default());
        }
        Ok(removed)
    }
}
