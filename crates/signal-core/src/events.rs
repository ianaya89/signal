use serde::Serialize;
use tokio::sync::broadcast;

use crate::models::PlayerState;

pub const EVENT_CAPACITY: usize = 256;

/// Every cross-crate event in Signal. Published on the [`EventBus`], bridged
/// to the frontend by `src-tauri` under the channel name from [`SignalEvent::channel`].
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SignalEvent {
    #[serde(rename_all = "camelCase")]
    PlayerState {
        state: PlayerState,
    },
    #[serde(rename_all = "camelCase")]
    PlayerProgress {
        position_ms: u64,
        duration_ms: u64,
    },
    #[serde(rename_all = "camelCase")]
    TrackChanged {
        track_id: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    DeviceChanged {
        device_id: String,
    },
    #[serde(rename_all = "camelCase")]
    ScannerProgress {
        processed: u64,
        total: u64,
        current_path: String,
    },
    #[serde(rename_all = "camelCase")]
    ScannerDone {
        added: u32,
        updated: u32,
        removed: u32,
    },
    QueueChanged,
    #[serde(rename_all = "camelCase")]
    LogLine {
        level: String,
        target: String,
        message: String,
    },
}

impl SignalEvent {
    /// Frontend event channel this variant is emitted on (see `docs/05-ipc-api.md`).
    #[must_use]
    pub fn channel(&self) -> &'static str {
        match self {
            Self::PlayerState { .. } => "player:state",
            Self::PlayerProgress { .. } => "player:progress",
            Self::TrackChanged { .. } => "player:track-changed",
            Self::DeviceChanged { .. } => "player:device-changed",
            Self::ScannerProgress { .. } => "scanner:progress",
            Self::ScannerDone { .. } => "scanner:done",
            Self::QueueChanged => "queue:changed",
            Self::LogLine { .. } => "log:line",
        }
    }
}

/// Cloneable handle over a `tokio::sync::broadcast` channel. Publishing never
/// blocks and never fails; events sent with no subscribers are dropped.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SignalEvent>,
}

impl EventBus {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: SignalEvent) {
        let _ = self.tx.send(event);
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<SignalEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(EVENT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn publish_reaches_subscriber() {
        let bus = EventBus::default();
        let mut rx = bus.subscribe();
        bus.publish(SignalEvent::QueueChanged);
        let event = rx.recv().await.unwrap();
        assert_eq!(event.channel(), "queue:changed");
    }

    #[test]
    fn publish_without_subscribers_is_a_noop() {
        let bus = EventBus::default();
        bus.publish(SignalEvent::QueueChanged);
    }

    #[test]
    fn channel_names_match_ipc_spec() {
        let event = SignalEvent::PlayerProgress {
            position_ms: 0,
            duration_ms: 0,
        };
        assert_eq!(event.channel(), "player:progress");
        assert_eq!(
            SignalEvent::TrackChanged { track_id: None }.channel(),
            "player:track-changed"
        );
    }
}
