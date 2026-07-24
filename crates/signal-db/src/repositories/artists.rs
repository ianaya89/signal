use signal_core::ArtistSummary;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::to_u32;

pub struct ArtistRepo {
    pool: SqlitePool,
}

impl ArtistRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Case-insensitive get-or-create (the unique index is NOCASE). The
    /// first-seen spelling wins; later case variants map onto it.
    pub async fn get_or_create(&self, name: &str) -> sqlx::Result<i64> {
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM artists WHERE name = ?1 COLLATE NOCASE")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;
        if let Some(id) = existing {
            return Ok(id);
        }
        sqlx::query_scalar("INSERT INTO artists (name) VALUES (?1) RETURNING id")
            .bind(name)
            .fetch_one(&self.pool)
            .await
    }

    /// Album artists only — track-level "feat." credits don't clutter the
    /// artist list; their tracks are reachable through the album.
    pub async fn list(&self) -> sqlx::Result<Vec<ArtistSummary>> {
        let rows = sqlx::query(
            "SELECT ar.id, ar.name,
                    COUNT(DISTINCT al.id) AS album_count,
                    COUNT(t.id) AS track_count
             FROM artists ar
             JOIN albums al ON al.artist_id = ar.id
             LEFT JOIN tracks t ON t.album_id = al.id
             GROUP BY ar.id
             HAVING track_count > 0
             ORDER BY ar.name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                Ok(ArtistSummary {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    album_count: to_u32(row.try_get::<i64, _>("album_count")?),
                    track_count: to_u32(row.try_get::<i64, _>("track_count")?),
                })
            })
            .collect()
    }

    /// Renames the artist — or, when another artist already carries the new
    /// name (case-insensitive), MERGES this one into it: albums move over
    /// (colliding album names fuse, keeping the target's artwork when
    /// present), tracks repoint, the empty artist row is deleted. Every
    /// affected FTS row is re-indexed in the same transaction
    /// (delete+reinsert — contentless-delete forbids partial UPDATE).
    /// Returns true when a merge happened.
    pub async fn rename(&self, id: i64, new_name: &str) -> sqlx::Result<bool> {
        let mut tx = self.pool.begin().await?;

        let target: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM artists WHERE name = ?1 COLLATE NOCASE AND id <> ?2",
        )
        .bind(new_name)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        // everything whose FTS row changes: own tracks + tracks inside our albums
        let affected: Vec<i64> = sqlx::query_scalar(
            "SELECT DISTINCT id FROM tracks
             WHERE artist_id = ?1
                OR album_id IN (SELECT id FROM albums WHERE artist_id = ?1)",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;

        let merged = if let Some(target) = target {
            let albums: Vec<(i64, String, Option<String>)> =
                sqlx::query_as("SELECT id, name, artwork_path FROM albums WHERE artist_id = ?1")
                    .bind(id)
                    .fetch_all(&mut *tx)
                    .await?;

            for (album_id, album_name, artwork) in albums {
                let existing: Option<(i64, Option<String>)> = sqlx::query_as(
                    "SELECT id, artwork_path FROM albums
                     WHERE artist_id = ?1 AND name = ?2 COLLATE NOCASE",
                )
                .bind(target)
                .bind(&album_name)
                .fetch_optional(&mut *tx)
                .await?;

                if let Some((keep_id, keep_art)) = existing {
                    sqlx::query("UPDATE tracks SET album_id = ?2 WHERE album_id = ?1")
                        .bind(album_id)
                        .bind(keep_id)
                        .execute(&mut *tx)
                        .await?;
                    if keep_art.is_none() {
                        if let Some(artwork) = artwork {
                            sqlx::query("UPDATE albums SET artwork_path = ?2 WHERE id = ?1")
                                .bind(keep_id)
                                .bind(artwork)
                                .execute(&mut *tx)
                                .await?;
                        }
                    }
                    sqlx::query("DELETE FROM albums WHERE id = ?1")
                        .bind(album_id)
                        .execute(&mut *tx)
                        .await?;
                } else {
                    sqlx::query("UPDATE albums SET artist_id = ?2 WHERE id = ?1")
                        .bind(album_id)
                        .bind(target)
                        .execute(&mut *tx)
                        .await?;
                }
            }

            sqlx::query("UPDATE tracks SET artist_id = ?2 WHERE artist_id = ?1")
                .bind(id)
                .bind(target)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM artists WHERE id = ?1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            true
        } else {
            sqlx::query("UPDATE artists SET name = ?2 WHERE id = ?1")
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

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<ArtistSummary>> {
        let row = sqlx::query(
            "SELECT ar.id, ar.name,
                    COUNT(DISTINCT al.id) AS album_count,
                    COUNT(t.id) AS track_count
             FROM artists ar
             LEFT JOIN albums al ON al.artist_id = ar.id
             LEFT JOIN tracks t ON t.album_id = al.id
             WHERE ar.id = ?1
             GROUP BY ar.id",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            Ok(ArtistSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                album_count: to_u32(row.try_get::<i64, _>("album_count")?),
                track_count: to_u32(row.try_get::<i64, _>("track_count")?),
            })
        })
        .transpose()
    }
}
