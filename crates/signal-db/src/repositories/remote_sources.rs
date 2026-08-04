use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// A configured remote `OpenSubsonic` server.
///
/// The password is deliberately not part of this struct: it's read only when
/// building a client (`credentials`), so listing sources for the UI can't leak
/// it over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSource {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub username: String,
    pub auth_mode: String,
    pub allow_insecure_tls: bool,
    pub enabled: bool,
    pub last_ping_at: Option<String>,
    pub last_ping_ok: Option<bool>,
}

/// Everything needed to build a client, password included.
#[derive(Debug, Clone)]
pub struct RemoteCredentials {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub auth_mode: String,
    pub allow_insecure_tls: bool,
}

/// Fields a caller may change. `None` leaves the stored value alone — which is
/// how the settings UI edits a source without having to re-send the password.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSourcePatch {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub auth_mode: Option<String>,
    pub allow_insecure_tls: Option<bool>,
    pub enabled: Option<bool>,
}

const COLUMNS: &str = "id, name, base_url, username, auth_mode, allow_insecure_tls, enabled, \
                       last_ping_at, last_ping_ok";

pub struct RemoteSourceRepo {
    pool: SqlitePool,
}

impl RemoteSourceRepo {
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> sqlx::Result<Vec<RemoteSource>> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM remote_sources ORDER BY name COLLATE NOCASE"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(source_from_row).collect()
    }

    pub async fn get(&self, id: i64) -> sqlx::Result<Option<RemoteSource>> {
        let row = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM remote_sources WHERE id = ?1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(source_from_row).transpose()
    }

    pub async fn credentials(&self, id: i64) -> sqlx::Result<Option<RemoteCredentials>> {
        let row = sqlx::query(
            "SELECT base_url, username, password, auth_mode, allow_insecure_tls
             FROM remote_sources WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(RemoteCredentials {
                base_url: row.try_get("base_url")?,
                username: row.try_get("username")?,
                password: row.try_get("password")?,
                auth_mode: row.try_get("auth_mode")?,
                allow_insecure_tls: row.try_get::<i64, _>("allow_insecure_tls")? != 0,
            })
        })
        .transpose()
    }

    pub async fn create(
        &self,
        name: &str,
        base_url: &str,
        username: &str,
        password: &str,
        allow_insecure_tls: bool,
    ) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "INSERT INTO remote_sources (name, base_url, username, password, allow_insecure_tls)
             VALUES (?1, ?2, ?3, ?4, ?5)
             RETURNING id",
        )
        .bind(name)
        .bind(base_url)
        .bind(username)
        .bind(password)
        .bind(i64::from(allow_insecure_tls))
        .fetch_one(&self.pool)
        .await
    }

    /// Applies only the populated fields of `patch`. `COALESCE(?, col)` keeps
    /// the update a single statement rather than a read-modify-write that
    /// could race another edit of the same row.
    pub async fn update(&self, id: i64, patch: &RemoteSourcePatch) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE remote_sources SET
                 name               = COALESCE(?2, name),
                 base_url           = COALESCE(?3, base_url),
                 username           = COALESCE(?4, username),
                 password           = COALESCE(?5, password),
                 auth_mode          = COALESCE(?6, auth_mode),
                 allow_insecure_tls = COALESCE(?7, allow_insecure_tls),
                 enabled            = COALESCE(?8, enabled),
                 updated_at         = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
        )
        .bind(id)
        .bind(patch.name.as_deref())
        .bind(patch.base_url.as_deref())
        .bind(patch.username.as_deref())
        .bind(patch.password.as_deref())
        .bind(patch.auth_mode.as_deref())
        .bind(patch.allow_insecure_tls.map(i64::from))
        .bind(patch.enabled.map(i64::from))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM remote_sources WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Records the outcome of a connection test, and the auth mode that worked.
    pub async fn record_ping(&self, id: i64, ok: bool, auth_mode: &str) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE remote_sources SET
                 last_ping_ok = ?2,
                 last_ping_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 auth_mode    = ?3
             WHERE id = ?1",
        )
        .bind(id)
        .bind(i64::from(ok))
        .bind(auth_mode)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn source_from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<RemoteSource> {
    Ok(RemoteSource {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        base_url: row.try_get("base_url")?,
        username: row.try_get("username")?,
        auth_mode: row.try_get("auth_mode")?,
        allow_insecure_tls: row.try_get::<i64, _>("allow_insecure_tls")? != 0,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        last_ping_at: row.try_get("last_ping_at")?,
        last_ping_ok: row
            .try_get::<Option<i64>, _>("last_ping_ok")?
            .map(|v| v != 0),
    })
}
