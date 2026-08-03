//! Embedded `OpenSubsonic` server lifecycle. Settings live in the DB
//! settings table: `server.enabled`, `server.port`, `server.password`.

use signal_core::SignalError;
use tauri::State;

use crate::commands::DbResultExt as _;
use crate::state::AppState;

pub const DEFAULT_PORT: u16 = 4040;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub port: u16,
    /// Lets the UI disable "start" instead of surfacing a backend error.
    pub has_password: bool,
    /// This machine's LAN address, so the UI can show a paste-ready URL.
    pub lan_ip: Option<String>,
}

/// Routing-table trick: connecting a UDP socket sends nothing but resolves
/// which local address would reach the internet. `None` off-network.
fn lan_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

fn take_handle(state: &AppState) -> Option<signal_server::ServerHandle> {
    state.server.lock().ok().and_then(|mut guard| guard.take())
}

pub async fn read_config(state: &AppState) -> Result<signal_server::ServerConfig, SignalError> {
    let port = state
        .db
        .settings()
        .get("server.port")
        .await
        .db_err()?
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let password = state
        .db
        .settings()
        .get("server.password")
        .await
        .db_err()?
        .unwrap_or_default();
    Ok(signal_server::ServerConfig {
        port,
        password,
        server_version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

/// Starts (or restarts) the server on the configured port and remembers the
/// choice for the next launch. Shared by the IPC command and the CLI socket.
pub async fn start_server(state: &AppState) -> Result<ServerStatus, SignalError> {
    let config = read_config(state).await?;
    if config.password.is_empty() {
        return Err(SignalError::InvalidQuery {
            reason: "set a server password first (settings → mobile server)".into(),
        });
    }

    if let Some(previous) = take_handle(state) {
        previous.stop().await;
    }

    let handle = signal_server::start(state.db.clone(), config)
        .await
        .map_err(|err| SignalError::Io(err.to_string()))?;
    let port = handle.addr().port();
    if let Ok(mut guard) = state.server.lock() {
        *guard = Some(handle);
    }
    state.db.settings().set("server.enabled", "true").await.db_err()?;
    Ok(ServerStatus {
        running: true,
        port,
        has_password: true,
        lan_ip: lan_ip(),
    })
}

pub async fn stop_server(state: &AppState) -> Result<(), SignalError> {
    if let Some(handle) = take_handle(state) {
        handle.stop().await;
    }
    state
        .db
        .settings()
        .set("server.enabled", "false")
        .await
        .db_err()?;
    Ok(())
}

pub async fn status_of(state: &AppState) -> Result<ServerStatus, SignalError> {
    let live_port = state
        .server
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|h| h.addr().port()));
    let config = read_config(state).await?;
    Ok(ServerStatus {
        running: live_port.is_some(),
        port: live_port.unwrap_or(config.port),
        has_password: !config.password.is_empty(),
        lan_ip: lan_ip(),
    })
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn server_start(state: State<'_, AppState>) -> Result<ServerStatus, SignalError> {
    start_server(&state).await
}

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn server_stop(state: State<'_, AppState>) -> Result<(), SignalError> {
    stop_server(&state).await
}

#[tauri::command]
pub async fn server_status(state: State<'_, AppState>) -> Result<ServerStatus, SignalError> {
    status_of(&state).await
}
