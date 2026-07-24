//! Signal application shell: window, IPC command handlers, event bridge.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod artwork;
mod autoplay;
mod bridge;
mod commands;
mod logbus;
mod recorder;
mod state;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use signal_core::{AppConfig, EventBus};
use signal_db::DbPool;
use tauri::Manager;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

use crate::state::AppState;

fn main() {
    let events = EventBus::default();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(logbus::BusLayer::new(events.clone()))
        .init();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
            let config = AppConfig::new(data_dir, cache_dir);

            let db = tauri::async_runtime::block_on(DbPool::connect(&config.db_path))?;
            bridge::spawn(app.handle().clone(), &events);
            autoplay::spawn(app.handle().clone(), &events);
            recorder::spawn(app.handle().clone(), &events);
            let player = signal_player::Player::new(events.clone())?;

            app.manage(AppState {
                config,
                events,
                db,
                player,
                scanning: Arc::new(AtomicBool::new(false)),
                watcher: Mutex::new(None),
                play_context: Mutex::new(state::PlayContext::default()),
            });

            // watch the stored library root, if any
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                match state.db.settings().get("library.root").await {
                    Ok(Some(root)) => state.start_watcher(std::path::Path::new(&root)),
                    Ok(None) => {}
                    Err(err) => tracing::warn!("library.root read failed: {err}"),
                }
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
            commands::library::library_get_artist,
            commands::library::library_reset_and_rescan,
            commands::player::player_play,
            commands::player::player_play_context,
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
            commands::player::player_next,
            commands::player::player_prev,
            commands::stats::stats_overview,
            commands::device::device_list,
            commands::device::device_select,
            commands::device::player_set_replaygain,
            commands::device::player_set_exclusive,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!("fatal: {err}");
        std::process::exit(1);
    }
}
