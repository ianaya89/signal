//! Signal application shell: window, IPC command handlers, event bridge.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod artwork;
mod autoplay;
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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
            let config = AppConfig::new(data_dir, cache_dir);

            let db = tauri::async_runtime::block_on(DbPool::connect(&config.db_path))?;
            let events = EventBus::default();
            bridge::spawn(app.handle().clone(), &events);
            autoplay::spawn(app.handle().clone(), &events);
            let player = signal_player::Player::new(events.clone())?;

            app.manage(AppState {
                config,
                events,
                db,
                player,
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
            commands::library::library_get_track,
            commands::player::player_play,
            commands::player::player_toggle,
            commands::player::player_pause,
            commands::player::player_stop,
            commands::player::player_seek,
            commands::player::player_set_volume,
            commands::player::player_get_state,
            commands::queue::queue_list,
            commands::queue::queue_add,
            commands::queue::queue_remove,
            commands::queue::queue_move,
            commands::queue::queue_clear,
            commands::queue::queue_play_next,
            commands::search::search_query,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!("fatal: {err}");
        std::process::exit(1);
    }
}
