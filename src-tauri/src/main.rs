//! Signal application shell: window, IPC command handlers, event bridge.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod artwork;
mod bridge;
mod commands;
mod state;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use signal_core::{AppConfig, EventBus};
use signal_db::DbPool;
use tauri::Manager;

use crate::state::AppState;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let result = tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
            let config = AppConfig::new(data_dir, cache_dir);

            let db = tauri::async_runtime::block_on(DbPool::connect(&config.db_path))?;
            let events = EventBus::default();
            bridge::spawn(app.handle().clone(), &events);

            app.manage(AppState {
                config,
                events,
                db,
                scanning: Arc::new(AtomicBool::new(false)),
            });
            tracing::info!("signal started");
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol(artwork::SCHEME, artwork::handle)
        .invoke_handler(tauri::generate_handler![
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::library::library_scan,
            commands::library::library_list_albums,
            commands::library::library_list_artists,
            commands::library::library_get_album,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!("fatal: {err}");
        std::process::exit(1);
    }
}
