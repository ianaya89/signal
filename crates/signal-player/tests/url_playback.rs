//! mpv playing an `http://` URL rather than a file on disk.
//!
//! This is the first thing in Signal that asks mpv to touch the network, so it
//! checks something the type system can't: that the linked libmpv/ffmpeg build
//! actually carries network protocol support (`docs/11-subsonic-client.md`
//! §5, risk 5). A size-trimmed build links and compiles fine, then fails at
//! runtime the first time a URL is loaded.
//!
//! Loopback only — it says nothing about whether *`https`* works, which needs a
//! TLS-capable ffmpeg and is covered by the manual smoke test against a real
//! server.
//!
//! Assertions are made against the events the player publishes rather than
//! against its momentary state. The fixture is 0.3s, so polling for "is it
//! playing right now" is a race decided by how fast the audio backend drains —
//! under `SIGNAL_AO_NULL` it loses that race every time. What actually matters
//! is that the load, the gapless handoff and the EOF happened at all, and each
//! of those is an event.

#![allow(clippy::unwrap_used)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use signal_core::{EventBus, MediaSource, SignalEvent};
use signal_player::Player;

const FIXTURE: &str = "../../fixtures/flac/tone-44100-16.flac";

/// Serializes the whole file. Cargo runs tests in parallel by default, and
/// several libmpv contexts decoding at once in one process is not something
/// these tests need to prove — headless CI segfaulted on exactly that.
fn one_player_at_a_time() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let lock = LOCK.get_or_init(|| Mutex::new(()));
    // a panicking test must not poison the lock for the rest of the file
    lock.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Minimal loopback file server. Deliberately not `signal-server` — the player
/// has no business depending on the Subsonic layer just to be handed a URL.
fn serve_fixture() -> (String, std::thread::JoinHandle<()>) {
    let bytes = std::fs::read(FIXTURE).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/tone.flac", listener.local_addr().unwrap());

    let handle = std::thread::spawn(move || {
        // mpv opens the URL more than once (probe, then decode) and may close
        // early once it has enough; neither is a test failure
        for stream in listener.incoming().take(8) {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: audio/flac\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n\r\n",
                bytes.len()
            );
            if stream.write_all(header.as_bytes()).is_err() {
                continue;
            }
            let _ = stream.write_all(&bytes);
            let _ = stream.flush();
        }
    });
    (url, handle)
}

/// Records everything the player publishes, subscribed before playback starts,
/// so a test can ask what happened instead of trying to catch it happening.
fn record(events: &EventBus) -> Arc<Mutex<Vec<SignalEvent>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let mut rx = events.subscribe();
    std::thread::spawn(move || {
        while let Ok(event) = rx.blocking_recv() {
            if let Ok(mut log) = sink.lock() {
                log.push(event);
            }
        }
    });
    seen
}

/// Waits for a matching event; panics with the whole log on timeout, which is
/// what makes a failure here diagnosable rather than just red.
fn expect_event(
    seen: &Mutex<Vec<SignalEvent>>,
    what: &str,
    matches: impl Fn(&SignalEvent) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(log) = seen.lock() {
            if log.iter().any(&matches) {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let log = seen
        .lock()
        .map_or_else(|_| String::new(), |l| format!("{l:#?}"));
    panic!("never saw {what}; events were:\n{log}");
}

/// mpv could not be initialized at all — an environment gap (no libmpv, no
/// audio stack), not a regression in URL support.
fn skip(test: &str) {
    eprintln!("SKIP {test}: libmpv could not be initialized");
}

#[test]
fn mpv_plays_a_remote_url() {
    let _serial = one_player_at_a_time();
    let events = EventBus::default();
    let seen = record(&events);
    let Ok(player) = Player::new(events) else {
        return skip("mpv_plays_a_remote_url");
    };
    let (url, _server) = serve_fixture();

    player.load_and_play(1, MediaSource::Url(url)).unwrap();

    // reaching EOF is proof mpv opened the URL and decoded it end to end;
    // a build without network support never gets here
    expect_event(&seen, "TrackEnded for the url", |e| {
        matches!(e, SignalEvent::TrackEnded { track_id: 1 })
    });

    player.stop().unwrap();
}

#[test]
fn local_and_remote_sources_are_interchangeable_for_gapless_staging() {
    let _serial = one_player_at_a_time();
    let events = EventBus::default();
    let seen = record(&events);
    let Ok(player) = Player::new(events) else {
        return skip("local_and_remote_sources_are_interchangeable_for_gapless_staging");
    };
    let (url, _server) = serve_fixture();

    // local first, remote staged behind it — the mixed queue the client feature
    // makes possible, and the case mpv's 2-slot window has never seen before
    player
        .load_and_play(1, MediaSource::File(FIXTURE.into()))
        .unwrap();
    player.set_next(2, MediaSource::Url(url)).unwrap();

    // the handoff is what proves a URL can occupy the gapless next slot
    expect_event(&seen, "gapless advance into the staged url", |e| {
        matches!(e, SignalEvent::TrackAutoAdvanced { track_id: 2 })
    });

    player.stop().unwrap();
}

#[test]
fn gapless_advances_from_one_remote_url_to_another() {
    let _serial = one_player_at_a_time();
    let events = EventBus::default();
    let seen = record(&events);
    let Ok(player) = Player::new(events) else {
        return skip("gapless_advances_from_one_remote_url_to_another");
    };
    // two servers, so the handoff crosses connections the way consecutive
    // tracks of a remote album do rather than reusing one open socket
    let (first, _a) = serve_fixture();
    let (second, _b) = serve_fixture();

    player.load_and_play(1, MediaSource::Url(first)).unwrap();
    player.set_next(2, MediaSource::Url(second)).unwrap();

    expect_event(&seen, "gapless advance between two urls", |e| {
        matches!(e, SignalEvent::TrackAutoAdvanced { track_id: 2 })
    });

    player.stop().unwrap();
}

#[test]
fn negative_track_ids_survive_the_player_round_trip() {
    let _serial = one_player_at_a_time();
    let events = EventBus::default();
    let seen = record(&events);
    let Ok(player) = Player::new(events) else {
        return skip("negative_track_ids_survive_the_player_round_trip");
    };
    let (url, _server) = serve_fixture();

    // remote songs are identified by ids from a negative range because they
    // have no `tracks` row; the desktop shell reads that id back off the player
    // to look up what is playing, so the sign has to survive the round trip
    player.load_and_play(-7, MediaSource::Url(url)).unwrap();

    expect_event(&seen, "TrackChanged carrying the negative id", |e| {
        matches!(e, SignalEvent::TrackChanged { track_id: Some(-7) })
    });
    expect_event(&seen, "TrackEnded carrying the negative id", |e| {
        matches!(e, SignalEvent::TrackEnded { track_id: -7 })
    });

    player.stop().unwrap();
}
