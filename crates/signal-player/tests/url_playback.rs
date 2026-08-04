//! mpv playing an `http://` URL rather than a file on disk.
//!
//! This is the first thing in Signal that asks mpv to touch the network, so it
//! checks something the type system can't: that the vendored libmpv/ffmpeg
//! build actually carries network protocol support (`docs/11-subsonic-client.md`
//! §5, risk 5). A size-trimmed build links and compiles fine, then fails at
//! runtime the first time a URL is loaded.
//!
//! Loopback only — it says nothing about whether *`https`* works, which needs a
//! TLS-capable ffmpeg and is covered by the manual smoke test against a real
//! server.

#![allow(clippy::unwrap_used)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use signal_core::{EventBus, MediaSource, PlaybackStatus};
use signal_player::Player;

const FIXTURE: &str = "../../fixtures/flac/tone-44100-16.flac";

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

fn wait_for(player: &Player, timeout: Duration, done: impl Fn(&signal_core::PlayerState) -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let state = player.state();
        if done(&state) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out; last state: {:?}", player.state());
}

#[test]
fn mpv_plays_a_remote_url() {
    let Ok(player) = Player::new(EventBus::default()) else {
        // headless CI without an audio stack can't init mpv at all; that's an
        // environment gap, not a regression in URL support
        eprintln!("SKIP mpv_plays_a_remote_url: libmpv could not be initialized");
        return;
    };
    let (url, _server) = serve_fixture();

    player
        .load_and_play(1, MediaSource::Url(url.clone()))
        .unwrap();
    wait_for(&player, Duration::from_secs(10), |s| {
        s.status == PlaybackStatus::Playing && s.duration_ms > 0
    });

    let state = player.state();
    assert_eq!(state.track_id, Some(1));
    assert!(
        state.duration_ms > 0,
        "mpv opened the url but never reported a duration: {state:?}"
    );

    player.stop().unwrap();
}

#[test]
fn local_and_remote_sources_are_interchangeable_for_gapless_staging() {
    let Ok(player) = Player::new(EventBus::default()) else {
        eprintln!("SKIP local_and_remote_sources_are_interchangeable_for_gapless_staging: libmpv could not be initialized");
        return;
    };
    let (url, _server) = serve_fixture();

    // local first, remote staged behind it — the mixed queue the client feature
    // makes possible, and the case mpv's 2-slot window has never seen before
    player
        .load_and_play(1, MediaSource::File(FIXTURE.into()))
        .unwrap();
    wait_for(&player, Duration::from_secs(10), |s| {
        s.status == PlaybackStatus::Playing
    });

    player.set_next(2, MediaSource::Url(url)).unwrap();

    // the fixture is 0.3s, so the handoff happens almost immediately: mpv
    // advancing its window to the staged entry is what proves a URL can occupy
    // the gapless next slot at all, and asserting on the resulting track_id
    // beats sleeping for a fixed interval
    wait_for(&player, Duration::from_secs(10), |s| s.track_id == Some(2));

    player.stop().unwrap();
}

#[test]
fn gapless_advances_from_one_remote_url_to_another() {
    let Ok(player) = Player::new(EventBus::default()) else {
        eprintln!(
            "SKIP gapless_advances_from_one_remote_url_to_another: libmpv could not be initialized"
        );
        return;
    };
    // two servers, so the handoff crosses connections the way consecutive
    // tracks of a remote album do rather than reusing one open socket
    let (first, _a) = serve_fixture();
    let (second, _b) = serve_fixture();

    player.load_and_play(1, MediaSource::Url(first)).unwrap();
    wait_for(&player, Duration::from_secs(10), |s| {
        s.status == PlaybackStatus::Playing
    });

    player.set_next(2, MediaSource::Url(second)).unwrap();
    wait_for(&player, Duration::from_secs(10), |s| s.track_id == Some(2));

    player.stop().unwrap();
}

#[test]
fn negative_track_ids_survive_the_player_round_trip() {
    let Ok(player) = Player::new(EventBus::default()) else {
        eprintln!("SKIP negative_track_ids_survive_the_player_round_trip: libmpv unavailable");
        return;
    };
    let (url, _server) = serve_fixture();

    // remote songs are identified by ids from a negative range because they have
    // no `tracks` row; the desktop shell reads that id straight back off the
    // player state to look up what is playing, so the sign has to survive
    player.load_and_play(-7, MediaSource::Url(url)).unwrap();
    wait_for(&player, Duration::from_secs(10), |s| {
        s.status == PlaybackStatus::Playing
    });

    assert_eq!(player.state().track_id, Some(-7));
    player.stop().unwrap();
}
