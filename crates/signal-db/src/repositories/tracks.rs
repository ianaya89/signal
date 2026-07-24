use signal_core::{Track, TrackTechnical};
use sqlx::sqlite::SqlitePool;

use crate::row::track_from_row;

/// Insert payload produced by `signal-scanner`. Timestamps are always set by
/// `SQLite` defaults so the ISO-8601 text format stays uniform in the DB.
#[derive(Debug, Clone)]
pub struct NewTrack {
    pub title: String,
    pub artist_id: i64,
    pub album_id: Option<i64>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<i32>,
    pub duration_ms: u64,
    pub genres: Vec<String>,
    pub technical: TrackTechnical,
}

pub struct TrackRepo {
    pool: SqlitePool,
}

impl TrackRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Inserts the track and links its genres in one transaction. The FTS5
    /// index rows are maintained by triggers (see `migrations/0001_init.sql`).
    pub async fn insert(&self, new: &NewTrack) -> sqlx::Result<i64> {
        let mut tx = self.pool.begin().await?;

        let tech = &new.technical;
        let track_id: i64 = sqlx::query_scalar(
            "INSERT INTO tracks (
                title, artist_id, album_id, track_no, disc_no, year, duration_ms,
                codec, container, bitrate_kbps, bit_depth, sample_rate_hz, channels,
                replaygain_track_gain, replaygain_album_gain, peak, dr_score,
                encoder, file_path, file_size_bytes, md5
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21
             )
             RETURNING id",
        )
        .bind(&new.title)
        .bind(new.artist_id)
        .bind(new.album_id)
        .bind(new.track_no.map(i64::from))
        .bind(new.disc_no.map(i64::from))
        .bind(new.year)
        .bind(i64::try_from(new.duration_ms).unwrap_or_default())
        .bind(&tech.codec)
        .bind(&tech.container)
        .bind(i64::from(tech.bitrate_kbps))
        .bind(tech.bit_depth.map(i64::from))
        .bind(i64::from(tech.sample_rate_hz))
        .bind(i64::from(tech.channels))
        .bind(tech.replaygain_track_gain)
        .bind(tech.replaygain_album_gain)
        .bind(tech.peak)
        .bind(tech.dr_score)
        .bind(&tech.encoder)
        .bind(tech.file_path.to_string_lossy().into_owned())
        .bind(i64::try_from(tech.file_size_bytes).unwrap_or_default())
        .bind(&tech.md5)
        .fetch_one(&mut *tx)
        .await?;

        for genre in &new.genres {
            let genre_id: i64 = sqlx::query_scalar(
                "INSERT INTO genres (name) VALUES (?1)
                 ON CONFLICT(name) DO UPDATE SET name = excluded.name
                 RETURNING id",
            )
            .bind(genre)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query("INSERT OR IGNORE INTO track_genres (track_id, genre_id) VALUES (?1, ?2)")
                .bind(track_id)
                .bind(genre_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(track_id)
    }

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<Track>> {
        let row = sqlx::query("SELECT * FROM tracks WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(track_from_row).transpose()
    }

    pub async fn id_by_path(&self, path: &str) -> sqlx::Result<Option<i64>> {
        sqlx::query_scalar("SELECT id FROM tracks WHERE file_path = ?1")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn count(&self) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&self.pool)
            .await
    }

    /// `rating` 0 clears.
    pub async fn set_rating(&self, id: i64, rating: u8) -> sqlx::Result<()> {
        sqlx::query("UPDATE tracks SET rating = ?2 WHERE id = ?1")
            .bind(id)
            .bind(i64::from(rating.min(5)))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_favorite(&self, id: i64, favorite: bool) -> sqlx::Result<()> {
        sqlx::query("UPDATE tracks SET favorite = ?2 WHERE id = ?1")
            .bind(id)
            .bind(i64::from(favorite))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Retitles a track; the FTS row follows via the update trigger.
    pub async fn rename(&self, id: i64, new_title: &str) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE tracks SET title = ?2,
                    modified_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
        )
        .bind(id)
        .bind(new_title)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Returns true when a row was actually deleted.
    pub async fn delete_by_path(&self, path: &str) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM tracks WHERE file_path = ?1")
            .bind(path)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
