//! The mpv-owning thread: command handling + event pumping.

use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use libmpv2::events::{Event, PropertyData};
use libmpv2::{mpv_end_file_reason, Format, Mpv};
use signal_core::{EventBus, PlaybackStatus, PlayerState, SignalEvent};

use crate::player::{Cmd, PlayerError};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
const EVENT_TIMEOUT_S: f64 = 0.05;

// mpv reports position/duration as non-negative seconds (f64).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn secs_to_ms(secs: f64) -> u64 {
    (secs.max(0.0) * 1000.0) as u64
}

// volume is clamped to 0..=100 before this is called.
#[allow(clippy::cast_possible_truncation)]
fn volume_frac(volume: f64) -> f32 {
    (volume / 100.0) as f32
}

// track positions are far below f64's 52-bit integer precision limit
#[allow(clippy::cast_precision_loss)]
fn ms_to_secs(ms: u64) -> f64 {
    ms as f64 / 1000.0
}

pub(crate) fn spawn(
    events: EventBus,
    state: Arc<RwLock<PlayerState>>,
) -> Result<Sender<Cmd>, PlayerError> {
    let (tx, rx) = mpsc::channel::<Cmd>();
    let (init_tx, init_rx) = mpsc::channel::<Result<(), PlayerError>>();

    std::thread::Builder::new()
        .name("mpv".into())
        .spawn(move || match init_mpv() {
            Ok(mpv) => {
                let _ = init_tx.send(Ok(()));
                Engine {
                    events,
                    state,
                    current_track: None,
                    duration_ms: 0,
                    last_progress: Instant::now(),
                }
                .run(&mpv, &rx);
            }
            Err(err) => {
                let _ = init_tx.send(Err(err));
            }
        })
        .map_err(|e| PlayerError::Init(e.to_string()))?;

    init_rx
        .recv()
        .map_err(|_| PlayerError::Disconnected)?
        .map(|()| tx)
}

fn init_mpv() -> Result<Mpv, PlayerError> {
    Mpv::with_initializer(|init| {
        init.set_property("video", "no")?;
        init.set_property("audio-display", "no")?;
        init.set_property("gapless-audio", "yes")?;
        init.set_property("idle", "yes")?;
        init.set_property("keep-open", "no")?;
        Ok(())
    })
    .map_err(|e| PlayerError::Init(e.to_string()))
}

struct Engine {
    events: EventBus,
    state: Arc<RwLock<PlayerState>>,
    current_track: Option<i64>,
    duration_ms: u64,
    last_progress: Instant,
}

impl Engine {
    fn run(mut self, mpv: &Mpv, rx: &mpsc::Receiver<Cmd>) {
        for (name, format) in [
            ("time-pos", Format::Double),
            ("duration", Format::Double),
            ("pause", Format::Flag),
        ] {
            if let Err(err) = mpv.observe_property(name, format, 0) {
                tracing::warn!("observe {name} failed: {err}");
            }
        }

        loop {
            match rx.try_recv() {
                Ok(cmd) => {
                    self.apply(mpv, cmd);
                    continue;
                }
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => {}
            }

            match mpv.wait_event(EVENT_TIMEOUT_S) {
                Some(Ok(event)) => self.handle_event(&event),
                Some(Err(err)) => tracing::warn!("mpv event error: {err}"),
                None => {}
            }
        }
        tracing::info!("mpv thread shutting down");
    }

    fn apply(&mut self, mpv: &Mpv, cmd: Cmd) {
        let result = match cmd {
            Cmd::Load { track_id, path } => {
                let path_str = path.to_string_lossy().into_owned();
                self.current_track = Some(track_id);
                self.duration_ms = 0;
                let res = mpv
                    .command("loadfile", &[&path_str, "replace"])
                    .and_then(|()| mpv.set_property("pause", false));
                if res.is_ok() {
                    self.events.publish(SignalEvent::TrackChanged {
                        track_id: Some(track_id),
                    });
                    self.set_state(|s| {
                        s.status = PlaybackStatus::Playing;
                        s.track_id = Some(track_id);
                        s.position_ms = 0;
                    });
                }
                res
            }
            Cmd::Toggle => mpv
                .get_property::<bool>("pause")
                .and_then(|paused| mpv.set_property("pause", !paused)),
            Cmd::Pause => mpv.set_property("pause", true),
            Cmd::Stop => {
                self.current_track = None;
                let res = mpv.command("stop", &[]);
                self.events
                    .publish(SignalEvent::TrackChanged { track_id: None });
                self.set_state(|s| {
                    s.status = PlaybackStatus::Stopped;
                    s.track_id = None;
                    s.position_ms = 0;
                    s.duration_ms = 0;
                });
                res
            }
            Cmd::SeekMs(ms) => mpv.set_property("time-pos", ms_to_secs(ms)),
            Cmd::SetVolume(volume) => {
                let res = mpv.set_property("volume", volume);
                if res.is_ok() {
                    self.set_state(|s| s.volume = volume_frac(volume));
                }
                res
            }
        };

        if let Err(err) = result {
            tracing::warn!("mpv command failed: {err}");
        }
    }

    fn handle_event(&mut self, event: &Event<'_>) {
        match event {
            Event::PropertyChange { name, change, .. } => {
                self.handle_property(name, change);
            }
            // Only natural EOF ends a track; `loadfile replace` and `stop`
            // also emit EndFile (reason Stop/Redirect) and must not.
            Event::EndFile(reason) if *reason == mpv_end_file_reason::Eof => {
                if let Some(track_id) = self.current_track.take() {
                    self.events.publish(SignalEvent::TrackEnded { track_id });
                }
                self.set_state(|s| {
                    s.status = PlaybackStatus::Stopped;
                    s.track_id = None;
                    s.position_ms = 0;
                });
            }
            Event::EndFile(reason) if *reason == mpv_end_file_reason::Error => {
                tracing::warn!("playback ended with error");
                self.current_track = None;
                self.set_state(|s| {
                    s.status = PlaybackStatus::Stopped;
                    s.track_id = None;
                });
            }
            _ => {}
        }
    }

    fn handle_property(&mut self, name: &str, change: &PropertyData<'_>) {
        match (name, change) {
            ("time-pos", PropertyData::Double(pos)) => {
                let position_ms = secs_to_ms(*pos);
                if self.last_progress.elapsed() >= PROGRESS_INTERVAL {
                    self.last_progress = Instant::now();
                    if let Ok(mut s) = self.state.write() {
                        s.position_ms = position_ms;
                    }
                    self.events.publish(SignalEvent::PlayerProgress {
                        position_ms,
                        duration_ms: self.duration_ms,
                    });
                }
            }
            ("duration", PropertyData::Double(duration)) => {
                self.duration_ms = secs_to_ms(*duration);
                let duration_ms = self.duration_ms;
                self.set_state(|s| s.duration_ms = duration_ms);
            }
            ("pause", PropertyData::Flag(paused)) if self.current_track.is_some() => {
                let status = if *paused {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Playing
                };
                self.set_state(|s| s.status = status);
            }
            _ => {}
        }
    }

    fn set_state(&self, update: impl FnOnce(&mut PlayerState)) {
        let snapshot = match self.state.write() {
            Ok(mut s) => {
                update(&mut s);
                s.clone()
            }
            Err(_) => return,
        };
        self.events
            .publish(SignalEvent::PlayerState { state: snapshot });
    }
}
