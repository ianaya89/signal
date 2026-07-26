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
        {
            window
                .set_title_bar_style(tauri::TitleBarStyle::Overlay)
                .map_err(err)?;
            // tao races here: set_decorations applies its style mask async on
            // the main queue, while the overlay style applies sync — the late
            // decorations mask drops FullSizeContentView, leaving an opaque
            // native titlebar band. Re-apply overlay after that mask lands.
            let win = window.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                let _ = win.set_title_bar_style(tauri::TitleBarStyle::Overlay);
            });
        }
    }
    let _ = window.app_handle();
    Ok(())
}
