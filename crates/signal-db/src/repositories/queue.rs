use signal_core::{QueueItem, Track};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

use crate::row::{to_u32, track_from_row};

/// A queue row joined with its track, ready for UI display.
pub struct QueueEntry {
    pub item: QueueItem,
    pub track: Track,
}

pub struct QueueRepo {
    pool: SqlitePool,
}

impl QueueRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> sqlx::Result<Vec<QueueEntry>> {
        let rows = sqlx::query(
            "SELECT q.id AS q_id, q.position AS q_position, q.added_at AS q_added_at, t.*
             FROM queue_items q
             JOIN tracks t ON t.id = q.track_id
             ORDER BY q.position",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                Ok(QueueEntry {
                    item: QueueItem {
                        id: row.try_get("q_id")?,
                        position: to_u32(row.try_get::<i64, _>("q_position")?),
                        track_id: row.try_get("id")?,
                        added_at: row.try_get("q_added_at")?,
                    },
                    track: track_from_row(row)?,
                })
            })
            .collect()
    }

    /// Appends at the end (max position + 1).
    pub async fn push_back(&self, track_id: i64) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "INSERT INTO queue_items (position, track_id)
             VALUES ((SELECT COALESCE(MAX(position), -1) + 1 FROM queue_items), ?1)
             RETURNING id",
        )
        .bind(track_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn remove(&self, queue_item_id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM queue_items WHERE id = ?1")
            .bind(queue_item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Rewrites all positions to match `ordered_ids` (queue item ids).
    /// Two-phase update dodges the UNIQUE(position) constraint mid-shuffle.
    pub async fn reorder(&self, ordered_ids: &[i64]) -> sqlx::Result<()> {
        let mut tx = self.pool.begin().await?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            sqlx::query("UPDATE queue_items SET position = ?1 WHERE id = ?2")
                .bind(-(i64::try_from(idx).unwrap_or_default() + 1))
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("UPDATE queue_items SET position = -position - 1 WHERE position < 0")
            .execute(&mut *tx)
            .await?;
        tx.commit().await
    }

    pub async fn clear(&self) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM queue_items")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// First queue entry at or after `position`, wrapping around to 0.
    pub async fn first(&self) -> sqlx::Result<Option<QueueEntry>> {
        Ok(self.list().await?.into_iter().next())
    }

    /// Pops the head of the queue: returns and removes it.
    pub async fn pop_front(&self) -> sqlx::Result<Option<QueueEntry>> {
        let head = self.first().await?;
        if let Some(entry) = &head {
            self.remove(entry.item.id).await?;
        }
        Ok(head)
    }
}
