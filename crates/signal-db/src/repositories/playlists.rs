use signal_core::Track;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::{to_u32, track_from_row};
use crate::smart::{self, Bind};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    /// true for rule-based (smart) playlists.
    pub smart: bool,
}

pub struct PlaylistRepo {
    pool: SqlitePool,
}

impl PlaylistRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str) -> sqlx::Result<i64> {
        sqlx::query_scalar("INSERT INTO playlists (name) VALUES (?1) RETURNING id")
            .bind(name)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn rename(&self, id: i64, new_name: &str) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE playlists SET name = ?2,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
        )
        .bind(id)
        .bind(new_name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM playlists WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Appends tracks, skipping ones already present.
    pub async fn add_tracks(&self, playlist_id: i64, track_ids: &[i64]) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;
        for track_id in track_ids {
            sqlx::query(
                "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
                 VALUES (?1, ?2,
                         (SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks
                          WHERE playlist_id = ?1))",
            )
            .bind(playlist_id)
            .bind(track_id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "UPDATE playlists SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
        )
        .bind(playlist_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    pub async fn remove_track(&self, playlist_id: i64, track_id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2")
            .bind(playlist_id)
            .bind(track_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Static + smart playlists in one list; the `smart` flag disambiguates
    /// ids, which live in separate tables.
    pub async fn list(&self) -> sqlx::Result<Vec<PlaylistSummary>> {
        let mut out: Vec<PlaylistSummary> = Vec::new();

        let smart_rows = sqlx::query(
            "SELECT id, name FROM smart_playlists ORDER BY sort_order, name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        for row in &smart_rows {
            out.push(PlaylistSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                track_count: 0, // computed on resolve; avoids N compiles per list
                smart: true,
            });
        }

        let rows = sqlx::query(
            "SELECT p.id, p.name, COUNT(pt.track_id) AS track_count
             FROM playlists p
             LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
             GROUP BY p.id
             ORDER BY p.name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?;
        for row in &rows {
            out.push(PlaylistSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                track_count: to_u32(row.try_get::<i64, _>("track_count")?),
                smart: false,
            });
        }

        Ok(out)
    }

    pub async fn name(&self, id: i64, smart: bool) -> sqlx::Result<Option<String>> {
        let table = if smart {
            "smart_playlists"
        } else {
            "playlists"
        };
        sqlx::query_scalar(&format!("SELECT name FROM {table} WHERE id = ?1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn tracks(&self, playlist_id: i64) -> sqlx::Result<Vec<Track>> {
        let rows = sqlx::query(
            "SELECT t.* FROM playlist_tracks pt
             JOIN tracks t ON t.id = pt.track_id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position",
        )
        .bind(playlist_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(track_from_row).collect()
    }

    /// Validates rules by compiling before storing.
    pub async fn create_smart(&self, name: &str, rules_json: &str) -> sqlx::Result<i64> {
        smart::compile(rules_json)
            .map_err(|e| sqlx::Error::Protocol(format!("smart rules: {e}")))?;
        sqlx::query_scalar(
            "INSERT INTO smart_playlists (name, rules, sort_order)
             VALUES (?1, ?2, (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM smart_playlists))
             RETURNING id",
        )
        .bind(name)
        .bind(rules_json)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_smart(&self, id: i64, name: &str, rules_json: &str) -> sqlx::Result<()> {
        smart::compile(rules_json)
            .map_err(|e| sqlx::Error::Protocol(format!("smart rules: {e}")))?;
        sqlx::query(
            "UPDATE smart_playlists SET name = ?2, rules = ?3,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?1",
        )
        .bind(id)
        .bind(name)
        .bind(rules_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_smart(&self, id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM smart_playlists WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn smart_rules(&self, id: i64) -> sqlx::Result<Option<String>> {
        sqlx::query_scalar("SELECT rules FROM smart_playlists WHERE id = ?1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Runs a smart playlist's compiled rules.
    pub async fn resolve_smart(&self, smart_id: i64) -> sqlx::Result<Vec<Track>> {
        let rules: Option<String> =
            sqlx::query_scalar("SELECT rules FROM smart_playlists WHERE id = ?1")
                .bind(smart_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(rules) = rules else {
            return Ok(Vec::new());
        };

        let (where_clause, binds, tail) = smart::compile(&rules)
            .map_err(|e| sqlx::Error::Protocol(format!("smart rules: {e}")))?;

        let sql = format!(
            "SELECT t.* FROM tracks t
             JOIN artists ar ON ar.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             WHERE {where_clause}{tail}"
        );
        let mut query = sqlx::query(&sql);
        for bind in binds {
            query = match bind {
                Bind::Text(s) => query.bind(s),
                Bind::Int(n) => query.bind(n),
                Bind::Real(f) => query.bind(f),
            };
        }
        let rows = query.fetch_all(&self.pool).await?;
        rows.iter().map(track_from_row).collect()
    }
}
