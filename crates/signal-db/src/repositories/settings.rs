use sqlx::sqlite::SqlitePool;

pub struct SettingsRepo {
    pool: SqlitePool,
}

impl SettingsRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> sqlx::Result<Option<String>> {
        sqlx::query_scalar("SELECT value FROM settings WHERE key = ?1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn set(&self, key: &str, value: &str) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
