use signal_core::EventBus;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast::error::RecvError;

/// Forwards every `SignalEvent` from the bus to the frontend under its
/// channel name. Runs for the app's lifetime.
pub fn spawn(app: AppHandle, events: &EventBus) {
    let mut rx = events.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Err(err) = app.emit(event.channel(), &event) {
                        tracing::warn!("event bridge emit failed: {err}");
                    }
                }
                Err(RecvError::Lagged(missed)) => {
                    tracing::warn!(missed, "event bridge lagged, events dropped");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}
