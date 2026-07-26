//! Config-as-code: `~/.config/signal/config.toml`, dotfiles-friendly.
//! Loaded at boot and hot-reloaded on change. Values here apply on load;
//! runtime toggles still work (last action wins).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use signal_core::{ReplayGainMode, SignalEvent};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

const TEMPLATE: &str = r"# signal — config as code
# this file is watched: edits apply live. every key is optional;
# runtime toggles (theme switch, RG buttons) still work — last one wins.

[ui]
# theme = 'dark'        # dark | light
# accent = '#8286f5'    # any hex — overrides the theme accent

[playback]
# replaygain = 'off'    # off | track | album
# exclusive = false     # hog the output device

[library]
# path substrings to skip when scanning / watching, e.g.:
# exclude = ['superwhisper', '/Recordings/']
";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileConfig {
    #[serde(default)]
    pub ui: UiSection,
    #[serde(default)]
    pub playback: PlaybackSection,
    #[serde(default)]
    pub library: LibrarySection,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibrarySection {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiSection {
    pub theme: Option<String>,
    pub accent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaybackSection {
    pub replaygain: Option<String>,
    pub exclusive: Option<bool>,
}

pub fn config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("signal")
            .join("config.toml"),
    )
}

/// Ensures the file exists (writing the commented template), loads it,
/// applies it, and watches for changes.
// AppHandle moves into the watcher thread; by-value is the point.
#[allow(clippy::needless_pass_by_value)]
pub fn init(app: AppHandle) {
    let Some(path) = config_path() else {
        return;
    };

    if !path.is_file() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(err) = std::fs::write(&path, TEMPLATE) {
            tracing::warn!("could not write config template: {err}");
            return;
        }
        tracing::info!(path = %path.display(), "config template created");
    }

    load_and_apply(&app, &path);

    // watch the parent dir (editors replace files, inode-level watches break)
    let watch_path = path.clone();
    let handle = app.clone();
    std::thread::spawn(move || {
        use notify::Watcher as _;
        let (tx, rx) = std::sync::mpsc::channel();
        let Ok(mut watcher) = notify::recommended_watcher(tx) else {
            tracing::warn!("config watcher failed to start");
            return;
        };
        let Some(dir) = watch_path.parent() else {
            return;
        };
        if watcher
            .watch(dir, notify::RecursiveMode::NonRecursive)
            .is_err()
        {
            return;
        }
        tracing::info!("config file watcher started");
        let mut last = std::time::Instant::now();
        for event in rx.into_iter().flatten() {
            let touches_config = event
                .paths
                .iter()
                .any(|p| p.file_name().is_some_and(|n| n == "config.toml"));
            // debounce editor multi-writes
            if touches_config && last.elapsed().as_millis() > 300 {
                last = std::time::Instant::now();
                load_and_apply(&handle, &watch_path);
            }
        }
    });
}

fn load_and_apply(app: &AppHandle, path: &std::path::Path) {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("config read failed: {err}");
            return;
        }
    };
    let config: FileConfig = match toml::from_str(&raw) {
        Ok(config) => config,
        Err(err) => {
            tracing::warn!("config.toml invalid: {err}");
            return;
        }
    };

    let state = app.state::<AppState>();

    if let Some(rg) = config.playback.replaygain.as_deref() {
        let mode = match rg {
            "track" => Some(ReplayGainMode::Track),
            "album" => Some(ReplayGainMode::Album),
            "off" => Some(ReplayGainMode::Off),
            other => {
                tracing::warn!("config: unknown replaygain '{other}'");
                None
            }
        };
        if let Some(mode) = mode {
            let _ = state.player.set_replaygain(mode);
        }
    }
    if let Some(exclusive) = config.playback.exclusive {
        let _ = state.player.set_exclusive(exclusive);
    }

    // shared with live scanners/watchers — applies without restart
    if let Ok(mut excludes) = state.excludes.lock() {
        *excludes = config.library.exclude;
    }

    // ui section goes to the frontend as an event
    if let Ok(json) = serde_json::to_string(&config.ui) {
        state
            .events
            .publish(SignalEvent::ConfigChanged { ui: json });
    }

    tracing::info!("config applied");
}
