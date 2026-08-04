//! Startup paths and fatal-startup-error reporting.
//!
//! Tauri panics if the setup hook returns `Err`, and that panic crosses the
//! macOS `applicationDidFinishLaunching` FFI boundary, which turns it into an
//! abort with no message. So the setup hook always returns `Ok` and routes
//! failures through [`report_fatal`] instead.

use std::path::PathBuf;

use tauri::{AppHandle, Manager as _};

/// Anything that can stop the app from coming up.
#[derive(Debug)]
pub enum StartupError {
    Path(tauri::Error),
    Dirs(std::io::Error),
    Db { path: PathBuf, source: sqlx::Error },
    Player(signal_player::PlayerError),
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(err) => write!(f, "signal could not resolve its data folder: {err}"),
            Self::Dirs(err) => write!(f, "signal could not create its data folder: {err}"),
            Self::Db { path, source } => match migration_gap(source) {
                Some(version) => write!(
                    f,
                    "This library database was written by a newer version of signal \
                     (migration {version} is not in this build).\n\n\
                     Install the latest signal, or move {} aside to start with an empty library.",
                    path.display()
                ),
                None => write!(
                    f,
                    "signal could not open the library database at {}: {source}",
                    path.display()
                ),
            },
            Self::Player(err) => write!(f, "signal could not start the audio engine: {err}"),
        }
    }
}

/// Migration version the database has applied but this binary does not ship.
fn migration_gap(err: &sqlx::Error) -> Option<i64> {
    match err {
        sqlx::Error::Migrate(err) => match **err {
            sqlx::migrate::MigrateError::VersionMissing(version) => Some(version),
            _ => None,
        },
        _ => None,
    }
}

/// Per-build data directory. Debug builds get their own subfolder.
pub fn data_dir(app: &AppHandle) -> Result<PathBuf, StartupError> {
    app.path()
        .app_data_dir()
        .map(scoped)
        .map_err(StartupError::Path)
}

/// Per-build cache directory. See [`data_dir`].
pub fn cache_dir(app: &AppHandle) -> Result<PathBuf, StartupError> {
    app.path()
        .app_cache_dir()
        .map(scoped)
        .map_err(StartupError::Path)
}

// Dev builds carry the release bundle identifier, so without this split a debug
// run applies unreleased migrations to the installed app's database and leaves
// it unopenable by the released binary.
fn scoped(dir: PathBuf) -> PathBuf {
    if cfg!(debug_assertions) {
        dir.join("dev")
    } else {
        dir
    }
}

/// Shows the failure to the user and quits. Never returns to a usable app.
pub fn report_fatal(app: &AppHandle, err: &StartupError) {
    let message = err.to_string();
    tracing::error!("startup failed: {message}");
    eprintln!("signal cannot start: {message}");

    // the window is already built at this point and has no state behind it
    for (_, window) in app.webview_windows() {
        let _ = window.hide();
    }

    // off the main thread: the alert blocks until the user dismisses it
    std::thread::spawn(move || {
        alert(&message);
        std::process::exit(1);
    });
}

// the tauri dialog plugin returns without rendering this early in startup
#[cfg(target_os = "macos")]
fn alert(message: &str) {
    let script = format!(
        "display alert \"signal cannot start\" message {} as critical",
        applescript_literal(message)
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status();
}

#[cfg(not(target_os = "macos"))]
fn alert(_message: &str) {}

#[cfg(target_os = "macos")]
fn applescript_literal(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        // AppleScript string literals have no newline escape
        .replace('\n', "\" & return & \"");
    format!("\"{escaped}\"")
}
