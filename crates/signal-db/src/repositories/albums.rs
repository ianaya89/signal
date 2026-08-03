use signal_core::{AlbumSummary, Track};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::{to_u32, track_from_row};

const SUMMARY_SELECT: &str = "SELECT al.id, al.name, al.artist_id, ar.name AS artist_name,
        al.year, al.artwork_path, al.added_at, COUNT(t.id) AS track_count,
        COUNT(DISTINCT t.artist_id) AS artist_count
 FROM albums al
 JOIN artists ar ON ar.id = al.artist_id
 LEFT JOIN tracks t ON t.album_id = al.id";

pub struct AlbumRepo {
    pool: SqlitePool,
}

impl AlbumRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Case-insensitive upsert on (artist, name) — the unique index is NOCASE.
    pub async fn upsert(&self, name: &str, artist_id: i64, year: Option<i32>) -> sqlx::Result<i64> {
        let existing: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM albums WHERE artist_id = ?1 AND name = ?2 COLLATE NOCASE",
        )
        .bind(artist_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(id) = existing {
            if let Some(year) = year {
                sqlx::query("UPDATE albums SET year = COALESCE(year, ?2) WHERE id = ?1")
                    .bind(id)
                    .bind(year)
                    .execute(&self.pool)
                    .await?;
            }
            return Ok(id);
        }
        sqlx::query_scalar(
            "INSERT INTO albums (name, artist_id, year) VALUES (?1, ?2, ?3) RETURNING id",
        )
        .bind(name)
        .bind(artist_id)
        .bind(year)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_by_artist(&self, artist_id: i64) -> sqlx::Result<Vec<AlbumSummary>> {
        let rows = sqlx::query(&format!(
            "{SUMMARY_SELECT}
             WHERE al.artist_id = ?1
             GROUP BY al.id
             HAVING track_count > 0
             ORDER BY al.year, al.name COLLATE NOCASE"
        ))
        .bind(artist_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(summary_from_row).collect()
    }

    pub async fn list(&self) -> sqlx::Result<Vec<AlbumSummary>> {
        let rows = sqlx::query(&format!(
            "{SUMMARY_SELECT}
             GROUP BY al.id
             HAVING track_count > 0
             ORDER BY ar.name COLLATE NOCASE, al.year, al.name COLLATE NOCASE"
        ))
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(summary_from_row).collect()
    }

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<AlbumSummary>> {
        let row = sqlx::query(&format!(
            "{SUMMARY_SELECT}
             WHERE al.id = ?1
             GROUP BY al.id"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(summary_from_row).transpose()
    }

    pub async fn tracks(&self, album_id: i64) -> sqlx::Result<Vec<Track>> {
        let rows = sqlx::query(
            "SELECT * FROM tracks
             WHERE album_id = ?1
             ORDER BY disc_no, track_no",
        )
        .bind(album_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(track_from_row).collect()
    }

    /// Renames the album — or, when the same artist already has an album
    /// with the new name (case-insensitive), MERGES into it: tracks
    /// repoint, artwork falls back to the source's when the target lacks
    /// one, the empty album row is deleted. FTS rows re-index in the same
    /// transaction. Returns true when a merge happened.
    pub async fn rename(&self, id: i64, new_name: &str) -> sqlx::Result<bool> {
        let mut tx = self.pool.begin().await?;

        let source: Option<(i64, Option<String>)> =
            sqlx::query_as("SELECT artist_id, artwork_path FROM albums WHERE id = ?1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((artist_id, source_art)) = source else {
            return Ok(false);
        };

        let target: Option<(i64, Option<String>)> = sqlx::query_as(
            "SELECT id, artwork_path FROM albums
             WHERE artist_id = ?1 AND name = ?2 COLLATE NOCASE AND id <> ?3",
        )
        .bind(artist_id)
        .bind(new_name)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        let affected: Vec<i64> = sqlx::query_scalar("SELECT id FROM tracks WHERE album_id = ?1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;

        let merged = if let Some((keep_id, keep_art)) = target {
            sqlx::query("UPDATE tracks SET album_id = ?2 WHERE album_id = ?1")
                .bind(id)
                .bind(keep_id)
                .execute(&mut *tx)
                .await?;
            if keep_art.is_none() {
                if let Some(art) = source_art {
                    sqlx::query("UPDATE albums SET artwork_path = ?2 WHERE id = ?1")
                        .bind(keep_id)
                        .bind(art)
                        .execute(&mut *tx)
                        .await?;
                }
            }
            sqlx::query("DELETE FROM albums WHERE id = ?1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            true
        } else {
            sqlx::query("UPDATE albums SET name = ?2 WHERE id = ?1")
                .bind(id)
                .bind(new_name)
                .execute(&mut *tx)
                .await?;
            false
        };

        for track_id in affected {
            crate::row::refresh_fts_row(&mut tx, track_id).await?;
        }

        tx.commit().await?;
        Ok(merged)
    }

    /// Full album edit: rename, move to another artist (find-or-create by
    /// name, case-insensitive), set year. Colliding with an existing album
    /// under the target artist merges into it (same semantics as `rename`).
    /// The album's tracks follow to the target artist; an orphaned source
    /// artist is swept. Returns true when a merge happened.
    #[allow(clippy::too_many_lines)] // linear sequence; splitting hides the tx flow
    pub async fn update_info(
        &self,
        id: i64,
        name: &str,
        artist_name: &str,
        year: Option<i64>,
    ) -> sqlx::Result<bool> {
        let mut tx = self.pool.begin().await?;

        let source: Option<(i64, Option<String>)> =
            sqlx::query_as("SELECT artist_id, artwork_path FROM albums WHERE id = ?1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((old_artist_id, source_art)) = source else {
            return Ok(false);
        };

        let artist_id: i64 =
            match sqlx::query_scalar("SELECT id FROM artists WHERE name = ?1 COLLATE NOCASE")
                .bind(artist_name)
                .fetch_optional(&mut *tx)
                .await?
            {
                Some(found) => found,
                None => {
                    sqlx::query_scalar("INSERT INTO artists (name) VALUES (?1) RETURNING id")
                        .bind(artist_name)
                        .fetch_one(&mut *tx)
                        .await?
                }
            };

        let target: Option<(i64, Option<String>)> = sqlx::query_as(
            "SELECT id, artwork_path FROM albums
             WHERE artist_id = ?1 AND name = ?2 COLLATE NOCASE AND id <> ?3",
        )
        .bind(artist_id)
        .bind(name)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        let affected: Vec<i64> = sqlx::query_scalar("SELECT id FROM tracks WHERE album_id = ?1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;

        let merged = if let Some((keep_id, keep_art)) = target {
            sqlx::query("UPDATE tracks SET album_id = ?2, artist_id = ?3 WHERE album_id = ?1")
                .bind(id)
                .bind(keep_id)
                .bind(artist_id)
                .execute(&mut *tx)
                .await?;
            if keep_art.is_none() {
                if let Some(art) = source_art {
                    sqlx::query("UPDATE albums SET artwork_path = ?2 WHERE id = ?1")
                        .bind(keep_id)
                        .bind(art)
                        .execute(&mut *tx)
                        .await?;
                }
            }
            if year.is_some() {
                sqlx::query("UPDATE albums SET year = ?2 WHERE id = ?1")
                    .bind(keep_id)
                    .bind(year)
                    .execute(&mut *tx)
                    .await?;
            }
            sqlx::query("DELETE FROM albums WHERE id = ?1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            true
        } else {
            sqlx::query("UPDATE albums SET name = ?2, artist_id = ?3, year = ?4 WHERE id = ?1")
                .bind(id)
                .bind(name)
                .bind(artist_id)
                .bind(year)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE tracks SET artist_id = ?2 WHERE album_id = ?1")
                .bind(id)
                .bind(artist_id)
                .execute(&mut *tx)
                .await?;
            false
        };

        if old_artist_id != artist_id {
            sqlx::query(
                "DELETE FROM artists WHERE id = ?1
                 AND NOT EXISTS (SELECT 1 FROM tracks WHERE artist_id = ?1)
                 AND NOT EXISTS (SELECT 1 FROM albums WHERE artist_id = ?1)",
            )
            .bind(old_artist_id)
            .execute(&mut *tx)
            .await?;
        }

        for track_id in affected {
            crate::row::refresh_fts_row(&mut tx, track_id).await?;
        }

        tx.commit().await?;
        Ok(merged)
    }

    /// (id, name, artist name) for albums lacking artwork.
    pub async fn without_artwork(&self, limit: i64) -> sqlx::Result<Vec<(i64, String, String)>> {
        sqlx::query_as(
            "SELECT al.id, al.name, ar.name FROM albums al
             JOIN artists ar ON ar.id = al.artist_id
             WHERE al.artwork_path IS NULL
             ORDER BY al.added_at DESC
             LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn count_without_artwork(&self) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM albums WHERE artwork_path IS NULL")
            .fetch_one(&self.pool)
            .await
    }

    pub async fn set_artwork(&self, id: i64, path: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE albums SET artwork_path = ?2 WHERE id = ?1")
            .bind(id)
            .bind(path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn name_map(&self) -> sqlx::Result<Vec<(i64, String)>> {
        sqlx::query_as("SELECT id, name FROM albums")
            .fetch_all(&self.pool)
            .await
    }

    /// Total content length per album; `OpenSubsonic`'s `AlbumID3.duration`.
    pub async fn durations(&self) -> sqlx::Result<Vec<(i64, i64)>> {
        sqlx::query_as(
            "SELECT album_id, COALESCE(SUM(duration_ms), 0)
             FROM tracks WHERE album_id IS NOT NULL
             GROUP BY album_id",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// `(album_id, total plays, last played)` from the denormalized track
    /// counters — backs the server's `frequent` and `recent` album lists.
    pub async fn play_stats(&self) -> sqlx::Result<Vec<(i64, i64, Option<String>)>> {
        sqlx::query_as(
            "SELECT album_id, COALESCE(SUM(play_count), 0), MAX(last_played_at)
             FROM tracks WHERE album_id IS NOT NULL
             GROUP BY album_id",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn artwork_path(&self, id: i64) -> sqlx::Result<Option<String>> {
        let row: Option<Option<String>> =
            sqlx::query_scalar("SELECT artwork_path FROM albums WHERE id = ?1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.flatten())
    }
}

fn summary_from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<AlbumSummary> {
    Ok(AlbumSummary {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        artist_id: row.try_get("artist_id")?,
        artist_name: row.try_get("artist_name")?,
        year: row
            .try_get::<Option<i64>, _>("year")?
            .map(|y| i32::try_from(y).unwrap_or_default()),
        artwork_path: row.try_get("artwork_path")?,
        track_count: to_u32(row.try_get::<i64, _>("track_count")?),
        added_at: row.try_get("added_at")?,
        artist_count: to_u32(row.try_get::<i64, _>("artist_count")?),
    })
}
