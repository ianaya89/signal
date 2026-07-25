use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use signal_core::{AudioDevice, EventBus, PlayerState, ReplayGainMode};

use crate::engine;

#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("mpv init failed: {0}")]
    Init(String),
    #[error("player thread is gone")]
    Disconnected,
}

#[derive(Debug)]
pub(crate) enum Cmd {
    Load {
        track_id: i64,
        path: PathBuf,
    },
    /// Load paused at a position — session restore.
    LoadAt {
        track_id: i64,
        path: PathBuf,
        position_ms: u64,
    },
    /// Stage/replace the gapless next slot (mpv playlist index 1).
    SetNext {
        track_id: i64,
        path: PathBuf,
    },
    /// Drop the staged next slot, if any.
    ClearNext,
    Toggle,
    Pause,
    Stop,
    SeekMs(u64),
    SetVolume(f64),
    SetReplayGain(ReplayGainMode),
    SetDevice(String),
    SetExclusive(bool),
    /// Reply with the current mpv audio-device-list.
    ListDevices(Sender<Vec<AudioDevice>>),
}

/// Non-blocking handle to the mpv thread. Cheap to clone.
#[derive(Clone)]
pub struct Player {
    tx: Sender<Cmd>,
    state: Arc<RwLock<PlayerState>>,
}

impl Player {
    /// Spawns the mpv thread. Fails fast if libmpv can't be initialized.
    pub fn new(events: EventBus) -> Result<Self, PlayerError> {
        let state = Arc::new(RwLock::new(PlayerState::default()));
        let tx = engine::spawn(events, state.clone())?;
        Ok(Self { tx, state })
    }

    pub fn load_and_play(&self, track_id: i64, path: PathBuf) -> Result<(), PlayerError> {
        self.send(Cmd::Load { track_id, path })
    }

    /// Loads paused at `position_ms` (session restore).
    pub fn load_paused_at(
        &self,
        track_id: i64,
        path: PathBuf,
        position_ms: u64,
    ) -> Result<(), PlayerError> {
        self.send(Cmd::LoadAt {
            track_id,
            path,
            position_ms,
        })
    }

    /// Prefetch `path` as the gapless next track (replaces any staged next).
    pub fn set_next(&self, track_id: i64, path: PathBuf) -> Result<(), PlayerError> {
        self.send(Cmd::SetNext { track_id, path })
    }

    pub fn clear_next(&self) -> Result<(), PlayerError> {
        self.send(Cmd::ClearNext)
    }

    pub fn toggle(&self) -> Result<(), PlayerError> {
        self.send(Cmd::Toggle)
    }

    pub fn pause(&self) -> Result<(), PlayerError> {
        self.send(Cmd::Pause)
    }

    pub fn stop(&self) -> Result<(), PlayerError> {
        self.send(Cmd::Stop)
    }

    pub fn seek_ms(&self, position_ms: u64) -> Result<(), PlayerError> {
        self.send(Cmd::SeekMs(position_ms))
    }

    /// `volume` in 0.0..=100.0 (mpv scale).
    pub fn set_volume(&self, volume: f64) -> Result<(), PlayerError> {
        self.send(Cmd::SetVolume(volume.clamp(0.0, 100.0)))
    }

    pub fn set_replaygain(&self, mode: ReplayGainMode) -> Result<(), PlayerError> {
        self.send(Cmd::SetReplayGain(mode))
    }

    pub fn set_device(&self, device_id: String) -> Result<(), PlayerError> {
        self.send(Cmd::SetDevice(device_id))
    }

    pub fn set_exclusive(&self, exclusive: bool) -> Result<(), PlayerError> {
        self.send(Cmd::SetExclusive(exclusive))
    }

    /// Blocks briefly on the mpv thread for the device list.
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>, PlayerError> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.send(Cmd::ListDevices(tx))?;
        rx.recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| PlayerError::Disconnected)
    }

    #[must_use]
    pub fn state(&self) -> PlayerState {
        self.state.read().map(|s| s.clone()).unwrap_or_default()
    }

    fn send(&self, cmd: Cmd) -> Result<(), PlayerError> {
        self.tx.send(cmd).map_err(|_| PlayerError::Disconnected)
    }
}
