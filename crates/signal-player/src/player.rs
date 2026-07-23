use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use signal_core::{EventBus, PlayerState};

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

    #[must_use]
    pub fn state(&self) -> PlayerState {
        self.state.read().map(|s| s.clone()).unwrap_or_default()
    }

    fn send(&self, cmd: Cmd) -> Result<(), PlayerError> {
        self.tx.send(cmd).map_err(|_| PlayerError::Disconnected)
    }
}
