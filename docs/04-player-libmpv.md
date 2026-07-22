# Player — libmpv Wrapper (`signal-player`)

`signal-player` wraps libmpv as Signal's playback engine: a dedicated thread owns the `mpv_handle`, commands flow in over a channel, and mpv events flow out onto the `signal-core` event bus as `SignalEvent` values. This document covers the FFI approach, the public `Player` API, the event loop, gapless/ReplayGain/exclusive-mode behavior, device enumeration, error handling, and the testing strategy.

## 1. Why libmpv

libmpv is the same battle-tested engine behind mpv/mpc, and it already solves the problems Signal actually needs solved: robust demuxing and decoding across every target format (FLAC, ALAC, WAV, AIFF, AAC, MP3, OGG, Opus, including their messier real-world variants — odd WAV chunk layouts, ALAC-in-MP4 vs ALAC-in-CAF, VBR MP3 gapless trims, Opus pre-skip), gapless playback via its internal playlist and prefetch machinery, sample-accurate seeking, and a client API that exposes fine-grained property observation (playback position, audio format, output device) without polling. Building an equivalent pipeline on symphonia or ffmpeg-rs from scratch would mean re-implementing years of format-edge-case handling and per-OS exclusive-mode output plumbing that libmpv already gets right. Treating it as a black-box engine, driven purely through its command/property/event API, keeps `signal-player`'s own surface area small and testable.

## 2. FFI approach

`signal-player` depends on the `libmpv2` crate (an actively maintained fork of `libmpv-rs` tracking the current libmpv client API) rather than hand-rolling FFI bindings — it already wraps `mpv_create`/`mpv_initialize`, property get/set/observe, and command dispatch in safe Rust types, and correctly tears down the handle on `Drop`. If a needed property or command lags behind in `libmpv2`, `signal-player` falls back to raw `libmpv-sys` calls behind a small internal shim rather than forking the crate.

mpv options are set immediately after `mpv_create`, before `mpv_initialize`:

```rust
let mut mpv = Mpv::with_initializer(|init| {
    init.set_option("vo", "null")?;              // no video output — audio-only app
    init.set_option("video", "no")?;              // skip video decode entirely, even for files with embedded art
    init.set_option("audio-display", "no")?;      // don't treat embedded cover art as a video track
    init.set_option("gapless-audio", "yes")?;     // mpv's internal gapless engine (see §5)
    init.set_option("prefetch-playlist", "yes")?; // start opening the next playlist entry before EOF
    init.set_option("keep-open", "yes")?;         // don't auto-unload/quit at end of playlist — Player decides what's next
    init.set_option("idle", "yes")?;              // keep the core alive with no file loaded
    init.set_option("replaygain", "no")?;         // set dynamically per user preference, see §6
    init.set_option("audio-device", "auto")?;     // overridden by set_device()
    init.set_option("audio-exclusive", "no")?;    // overridden by set_exclusive()
    Ok(())
})?;
```

- `vo=null` / `video=no` / `audio-display=no`: Signal never renders video or album-art-as-video; this also avoids allocating a GPU surface or window on platforms where mpv would otherwise try.
- `gapless-audio=yes` + `prefetch-playlist=yes`: together these are what make the two-item sliding window (§5) actually gapless — mpv pre-decodes and pre-buffers the next playlist entry's start while the current entry's tail is still playing.
- `keep-open=yes`: without it, mpv auto-unloads the file (and eventually quits, if `idle=no`) once playback reaches EOF with nothing queued after it; Signal wants to own that decision (stop, or wait for the queue to gain a new head) instead of racing mpv's own idle/quit logic.
- `idle=yes`: keeps the mpv core resident with no file loaded, used at startup before anything is loaded and after an explicit `stop()`.

## 3. Public API

```rust
// signal-player/src/lib.rs

pub struct Player {
    cmd_tx: mpsc::Sender<PlayerCommand>,
    state: Arc<RwLock<PlayerState>>,
}

#[derive(Debug)]
enum PlayerCommand {
    Load { track: Track, autoplay: bool },
    Play,
    Pause,
    Toggle,
    Stop,
    Seek(Duration),
    SetVolume(f64),
    Next,
    Prev,
    SetReplayGainMode(ReplayGainMode),
    SetDevice(String),
    SetExclusive(bool),
    SyncQueueWindow { current: Option<Track>, next: Option<Track> },
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayGainMode { Off, Track, Album }

impl Player {
    pub fn spawn(events: broadcast::Sender<SignalEvent>) -> Player {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let state = Arc::new(RwLock::new(PlayerState::default()));
        let state_for_thread = state.clone();

        std::thread::Builder::new()
            .name("signal-player-mpv".into())
            .spawn(move || run_mpv_thread(cmd_rx, state_for_thread, events))
            .expect("failed to spawn mpv thread");

        Player { cmd_tx, state }
    }

    pub fn load(&self, track: Track, autoplay: bool) { self.send(PlayerCommand::Load { track, autoplay }) }
    pub fn play(&self)   { self.send(PlayerCommand::Play) }
    pub fn pause(&self)  { self.send(PlayerCommand::Pause) }
    pub fn toggle(&self) { self.send(PlayerCommand::Toggle) }
    pub fn stop(&self)   { self.send(PlayerCommand::Stop) }
    pub fn seek(&self, pos: Duration) { self.send(PlayerCommand::Seek(pos)) }
    pub fn set_volume(&self, vol: f64) { self.send(PlayerCommand::SetVolume(vol.clamp(0.0, 1.0))) }
    pub fn next(&self) { self.send(PlayerCommand::Next) }
    pub fn prev(&self) { self.send(PlayerCommand::Prev) }
    pub fn set_replaygain_mode(&self, mode: ReplayGainMode) { self.send(PlayerCommand::SetReplayGainMode(mode)) }
    pub fn set_device(&self, id: String) { self.send(PlayerCommand::SetDevice(id)) }
    pub fn set_exclusive(&self, on: bool) { self.send(PlayerCommand::SetExclusive(on)) }

    pub fn current_state(&self) -> PlayerState {
        self.state.read().expect("player state lock poisoned").clone()
    }

    fn send(&self, cmd: PlayerCommand) {
        if self.cmd_tx.send(cmd).is_err() {
            tracing::error!("signal-player: mpv thread gone, command dropped");
        }
    }
}
```

Every mutating method is a non-blocking `mpsc::Sender::send` — fire-and-forget from the caller's perspective. `current_state()` is the one synchronous read, and it reads a shared `Arc<RwLock<PlayerState>>` that the mpv thread updates after each relevant event; callers (Tauri commands invoked from the frontend) never block on an mpv IPC round-trip.

## 4. Event loop

A single thread owns the `mpv_handle` and alternates between draining queued commands and pumping `mpv_wait_event`:

```rust
fn run_mpv_thread(
    cmd_rx: mpsc::Receiver<PlayerCommand>,
    state: Arc<RwLock<PlayerState>>,
    events: broadcast::Sender<SignalEvent>,
) {
    let mut mpv = init_mpv().expect("mpv init failed");
    let mut window = QueueWindow::default();

    mpv.observe_property(1, "playback-time", Format::Double).unwrap();
    mpv.observe_property(2, "audio-params", Format::Node).unwrap();
    mpv.observe_property(3, "audio-out-params", Format::Node).unwrap();
    mpv.observe_property(4, "current-ao", Format::String).unwrap();
    mpv.observe_property(5, "path", Format::String).unwrap();
    mpv.observe_property(6, "eof-reached", Format::Flag).unwrap();
    mpv.observe_property(7, "pause", Format::Flag).unwrap();

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            if matches!(cmd, PlayerCommand::Shutdown) {
                mpv.command("quit", &[]).ok();
                return;
            }
            apply_command(&mut mpv, &mut window, cmd);
        }

        match mpv.wait_event(0.05) {
            Some(Ok(Event::PropertyChange { name: "playback-time", change: PropertyValue::Double(t), .. })) => {
                state.write().unwrap().position = Duration::from_secs_f64(t);
                events.send(SignalEvent::PlayerProgress { position_ms: (t * 1000.0) as u64 }).ok();
            }
            Some(Ok(Event::PropertyChange { name: "audio-out-params", change: PropertyValue::Node(node), .. })) => {
                let mut s = state.write().unwrap();
                update_bit_perfect_flag(&mut s, &node);
                events.send(SignalEvent::PlayerStateChanged(s.clone())).ok();
            }
            Some(Ok(Event::PropertyChange { name: "current-ao", change: PropertyValue::Str(ao), .. })) => {
                events.send(SignalEvent::PlayerDeviceChanged { device: ao.to_string() }).ok();
            }
            Some(Ok(Event::PropertyChange { name: "path", change: PropertyValue::Str(path), .. })) => {
                events.send(SignalEvent::PlayerTrackChanged { path: path.to_string() }).ok();
            }
            Some(Ok(Event::PropertyChange { name: "eof-reached", change: PropertyValue::Flag(true), .. })) => {
                handle_track_boundary(&mut mpv, &mut window, &state, &events);
            }
            Some(Ok(Event::StartFile { .. })) => {
                state.write().unwrap().status = PlaybackStatus::Loading;
            }
            Some(Ok(Event::EndFile { reason: EndFileReason::Error, error, .. })) => {
                tracing::warn!(?error, "mpv end-file error");
                let mut s = state.write().unwrap();
                s.status = PlaybackStatus::Error(error.to_string());
                events.send(SignalEvent::PlayerStateChanged(s.clone())).ok();
            }
            Some(Ok(Event::Shutdown)) => return,
            Some(Err(e)) => tracing::error!(?e, "mpv event error"),
            None => {} // 0.05s wait_event timeout tick — loop back and drain commands again
            _ => {}
        }
    }
}
```

The 50ms `wait_event` timeout is the responsiveness ceiling for commands issued while mpv is otherwise idle-blocked in the event wait — well under perceptible latency for play/pause/seek, and cheap enough not to matter for CPU usage since `wait_event` blocks efficiently rather than spinning.

## 5. Gapless implementation

Signal's queue (`queue_items`, git-staging-style, independent of playlists — see `docs/03-database-schema.md`) is the single source of truth for playback order. `signal-player` does **not** mirror the whole queue into mpv's playlist. Instead it keeps only a two-slot window: index 0 is the currently-playing entry, index 1 is the prefetch target. Whenever the resolved head of the Signal queue changes, `signal-core` sends a `SyncQueueWindow { current, next }` command with just the next two tracks, resolved from `QueueRepo::list().take(2)`.

```rust
#[derive(Default)]
struct QueueWindow {
    current: Option<TrackId>,
    next: Option<TrackId>,
}

fn sync_window(mpv: &mut Mpv, window: &mut QueueWindow, current: Option<Track>, next: Option<Track>) {
    let want_current = current.as_ref().map(|t| t.id);
    let want_next = next.as_ref().map(|t| t.id);

    if want_current != window.current {
        // The actively playing track itself changed (user picked a different
        // track, or next()/prev() fired) — full reload; a gap here is expected
        // and correct, this is not the gapless-boundary case.
        mpv.command("playlist-clear", &[]).ok();
        if let Some(t) = &current {
            mpv.command("loadfile", &[&t.file_path, "replace"]).ok();
        }
        window.next = None; // force the next-slot branch below to also resync
    }

    if want_next != window.next {
        // Only the prefetch target changed — touch slot 1 only, leaving the
        // actively playing slot 0 (and mpv's gapless engine) undisturbed.
        mpv.command("playlist-remove", &["1"]).ok(); // no-op if slot 1 is empty
        if let Some(t) = &next {
            mpv.command("loadfile", &[&t.file_path, "append"]).ok();
        }
    }

    window.current = want_current;
    window.next = want_next;
}

fn handle_track_boundary(
    mpv: &mut Mpv,
    window: &mut QueueWindow,
    state: &Arc<RwLock<PlayerState>>,
    events: &broadcast::Sender<SignalEvent>,
) {
    // mpv has advanced from slot 0 into slot 1 on its own (gapless-audio handled
    // the transition natively). Shift the window and ask signal-core for the
    // new "next" so slot 1 gets refilled before the following boundary.
    window.current = window.next.take();
    events.send(SignalEvent::QueueAdvanced).ok(); // signal-core resolves the new head/next
}
```

**Resync rules** when the user edits the queue mid-playback:

- **Track added at queue head while nothing is playing** → becomes `current`; loaded and played immediately via the `want_current != window.current` branch.
- **Track added/removed/reordered anywhere beyond position 2** → no-op for `signal-player`; the window only reacts to changes in the *first two resolved* tracks, so `signal-core` doesn't even send a `SyncQueueWindow` unless the resolved pair actually changed.
- **The `next` slot's track is removed from the queue** → the resolved pair's second element shifts to whatever is now second (or `None` if the queue only has one item left); handled by the `want_next != window.next` branch alone, `current` is untouched.
- **The `current` track is removed while playing** (e.g. bulk queue clear) → falls into the `want_current != window.current` branch, which stops the old file and loads whatever is now first (or nothing, transitioning to `PlaybackStatus::Idle`).
- **Reordering** is just a diff: `signal-core` recomputes the resolved `(current, next)` pair after every `queue:changed` event and only emits `SyncQueueWindow` if that pair actually differs from what was last sent — this keeps `signal-player` fully decoupled from queue mechanics (positions, gaps, insert-at-index); it only ever sees "these are the next two tracks."

## 6. ReplayGain

mpv's native `replaygain` option reads `REPLAYGAIN_TRACK_GAIN`/`REPLAYGAIN_ALBUM_GAIN` tags directly off the file, which is preferred over applying Signal's own computed gain in software: it works uniformly for files the scanner hasn't (re)analyzed yet and for files tagged by external tools.

```rust
fn apply_replaygain_mode(mpv: &mut Mpv, mode: ReplayGainMode) {
    let value = match mode {
        ReplayGainMode::Off   => "no",
        ReplayGainMode::Track => "track",
        ReplayGainMode::Album => "album",
    };
    mpv.set_property("replaygain", value).ok();
    mpv.set_property("replaygain-preamp", 0.0).ok();
    mpv.set_property("replaygain-clip", "yes").ok(); // hard-limit true-peak clipping from positive gain
}

fn apply_replaygain_fallback(mpv: &mut Mpv, track: &Track) {
    // Only takes effect when the file itself carries no RG tag — covers formats
    // with no standard RG tag field (WAV, AIFF) using signal-scanner's own
    // computed gain, so that scan-time analysis isn't wasted.
    if let Some(gain_db) = track.technical.replaygain_track_gain.or(track.technical.replaygain_album_gain) {
        mpv.set_property("replaygain-fallback", gain_db).ok();
    }
}
```

`apply_replaygain_fallback` is called right before every `loadfile`, using whichever of `replaygain_track_gain`/`replaygain_album_gain` matches the current `ReplayGainMode`, sourced from `TrackTechnical` as already computed and persisted by `signal-scanner`.

## 7. Automatic sample-rate switching + exclusive mode

`audio-exclusive=yes` requests exclusive hardware ownership from the platform audio backend; behavior and the mechanism for automatic sample-rate matching differ per OS:

- **macOS (CoreAudio)**: mpv's `coreaudio` AO engages hog mode (`kAudioDevicePropertyHogMode`) and sets the device's nominal sample rate to match the source (`kAudioDevicePropertyNominalSampleRate`) — this is what actually produces bit-perfect, sample-rate-switched output on macOS. While engaged, the device is unavailable to any other process, including system alert sounds.
- **Windows (WASAPI)**: with `ao=wasapi` and `audio-exclusive=yes`, mpv opens the endpoint in `AUDCLNT_SHAREMODE_EXCLUSIVE`, bypassing the shared-mode session mixer/resampler entirely, and negotiates the closest device-supported mix format to the source sample rate/bit depth.
- **Linux (ALSA)**: with `ao=alsa` and `audio-exclusive=yes`, mpv opens the `hw:` device directly, with no `dmix`/PulseAudio/PipeWire layer resampling in between. This requires `audio-device` to point at real hardware (an `alsa/hw:X,Y`-style id, taken from `audio-device-list`, §8) — pointed at a non-`hw` device, mpv silently uses the default (usually PipeWire-routed, with resampling active) output instead.
- **Common semantics**: `audio-exclusive=yes` means "try exclusive/hog and surface the real negotiated state via property observation" rather than "silently downgrade to shared mode on failure" — `signal-player` always re-checks `current-ao`/`audio-out-params` after the request rather than trusting the option was honored. `set_exclusive(bool)` maps directly to the mpv option and only takes effect on the next `loadfile`, since mpv doesn't support flipping exclusive mode on an already-open device without reopening it.

**Bit-perfect flag** is computed by comparing the source format (`TrackTechnical.sample_rate_hz`/`bit_depth`, from lofty via `signal-scanner`) against mpv's actual negotiated output format, plus confirming no DSP stage is altering samples:

```rust
fn update_bit_perfect_flag(state: &mut PlayerState, out_params: &MpvNode) {
    let out_rate: Option<i64> = out_params.get_int("samplerate");
    let source_rate = state.current_track_technical.sample_rate_hz as i64;

    let rate_match = out_rate == Some(source_rate);
    let no_dsp = state.volume >= 0.999
        && (state.replaygain_mode == ReplayGainMode::Off
            || state.replaygain_applied_gain_db.unwrap_or(0.0).abs() < f64::EPSILON);

    state.bit_perfect = rate_match && no_dsp;
}
```

Crucially this compares against `audio-out-params` (the format actually negotiated with and sent to the device), not `audio-params` (the decoded source format post-filters but pre-output) — `audio-params` is fixed by the file regardless of whether the device actually honored that rate, so it cannot answer "is output bit-perfect."

## 8. Device enumeration

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDevice {
    pub id: String,      // mpv device id, e.g. "coreaudio/AppleUSBAudioEngine:...:2"
    pub name: String,    // human-readable description
    pub backend: String, // "coreaudio" | "wasapi" | "alsa" | "pulse" | ...
}

fn parse_device_list(node: &MpvNode) -> Vec<AudioDevice> {
    node.array_iter()
        .map(|entry| {
            let id = entry.get_str("name").unwrap_or_default().to_string();
            let backend = id.split('/').next().unwrap_or("unknown").to_string();
            AudioDevice {
                id,
                name: entry.get_str("description").unwrap_or_default().to_string(),
                backend,
            }
        })
        .collect()
}
```

mpv's `audio-device-list` property returns a node array of `{name, description}` pairs where `name` is `<backend>/<device-id>`. `signal-player` queries it on startup and re-queries it after any `SetDevice`/`SetExclusive` command settles — mpv doesn't reliably emit a change event for hardware hot-plug on every backend, so this is pull-driven from the Settings UI (via a Tauri command) rather than pushed on the event bus.

## 9. Error and edge-case handling

- **File missing at play time**: `loadfile` triggers `EndFileReason::Error`. `Player` transitions to `PlaybackStatus::Error`, publishes `player:state` with the failure message, and leaves the queue window advanced so `next()` still moves forward to the following track — it does not auto-retry the missing file.
- **Device disappears mid-playback** (e.g. USB DAC unplugged): the active AO reports an error/underrun, surfaced via an empty or changed `current-ao` value. `Player` catches this, falls back to the system default device (`audio-device=auto`), re-applies the configured exclusive-mode setting, and resumes playback via `seek()` to the last known `playback-time` rather than restarting the track. It publishes `player:device-changed` with an `is_fallback: true` flag so the UI can surface a toast rather than silently rerouting audio.
- **Unsupported file** (extension matched the scanner's filter but the content isn't actually decodable, or is corrupt): same `EndFileReason::Error` path, but `Player` inspects the mpv error code to distinguish this from a transient device error and immediately advances via the same path as `next()`, instead of waiting or retrying as it would for a device-related failure.

## 10. Testing strategy

Real mpv playback can't run inside a fast, hermetic `cargo test`, so `signal-player`'s core logic is written against a trait, not the concrete mpv binding:

```rust
pub trait PlayerBackend: Send {
    fn load(&mut self, path: &str) -> Result<(), BackendError>;
    fn play(&mut self) -> Result<(), BackendError>;
    fn pause(&mut self) -> Result<(), BackendError>;
    fn seek(&mut self, pos: Duration) -> Result<(), BackendError>;
    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), BackendError>;
    fn poll_event(&mut self, timeout: Duration) -> Option<BackendEvent>;
}

pub struct MpvBackend {
    mpv: Mpv,
}
impl PlayerBackend for MpvBackend {
    // thin passthrough to libmpv2 calls
}

#[cfg(test)]
pub struct MockBackend {
    pub loaded: Vec<String>,
    pub playing: bool,
    pub scripted_events: VecDeque<BackendEvent>,
}
#[cfg(test)]
impl PlayerBackend for MockBackend {
    // records every call made against it, replays scripted_events from poll_event
}
```

The mpv-thread run loop (`sync_window`, `handle_track_boundary`, command dispatch) is generic over `B: PlayerBackend`, so queue-window logic is unit-tested purely against `MockBackend` — script an `eof-reached` event, assert the resulting `loadfile`/`playlist-remove` calls match the expected resync — without touching real mpv or real audio hardware. This keeps `cargo test` fast and safe to run in any CI environment.

Real-mpv integration tests (does gapless playback actually avoid a click at the boundary, does exclusive mode actually acquire the device, does the bit-perfect flag land `true` for a known-good FLAC at native rate) live in `signal-player/tests/mpv_integration.rs`, gated behind a `mpv-integration` feature flag:

```bash
cargo test -p signal-player --features mpv-integration
```

These run only in a dedicated CI job with a real (or null/dummy ALSA) audio backend available — never as part of the default `cargo test` invocation, since they require the native `libmpv` shared library to be installed and a usable audio device, neither of which is guaranteed in a generic CI container.
