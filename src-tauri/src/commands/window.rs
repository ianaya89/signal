use signal_core::SignalError;
use tauri::{Manager, Runtime};

/// Compact modes (mini player / floating dot) drop all window chrome —
/// including the macOS traffic lights that otherwise float over the
/// overlay-style titlebar. Restoring re-applies the overlay style.
#[tauri::command]
#[tracing::instrument(skip(window))]
pub async fn window_set_compact<R: Runtime>(
    window: tauri::Window<R>,
    compact: bool,
) -> Result<(), SignalError> {
    let err = |e: tauri::Error| SignalError::Io(e.to_string());

    if compact {
        window.set_decorations(false).map_err(err)?;
    } else {
        window.set_decorations(true).map_err(err)?;
        #[cfg(target_os = "macos")]
        window
            .set_title_bar_style(tauri::TitleBarStyle::Overlay)
            .map_err(err)?;
    }
    let _ = window.app_handle();
    Ok(())
}
