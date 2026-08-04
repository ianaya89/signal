//! The whole remote-playback chain, with no UI and no mocks: a real
//! `OpenSubsonic` server, a real client browsing it, and mpv decoding the
//! stream URL that comes out the far end.
//!
//! Every piece between "user adds a server" and "audio comes out" is exercised
//! here except the Tauri command wrappers themselves, which only marshal these
//! same calls.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use signal_core::{EventBus, MediaSource, PlaybackStatus, TrackTechnical};
use signal_db::{DbPool, NewTrack};
use signal_player::Player;
use signal_server::{start, ServerConfig};
use signal_subsonic_client::{ClientConfig, SubsonicClient};

const PASSWORD: &str = "sesame";
const FIXTURE: &str = "../../fixtures/flac/tone-44100-16.flac";

#[tokio::test(flavor = "multi_thread")]
async fn browse_a_remote_server_then_play_a_track_from_it() {
    let Ok(player) = Player::new(EventBus::default()) else {
        eprintln!("SKIP browse_a_remote_server_then_play_a_track_from_it: libmpv unavailable");
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let db = DbPool::connect(&dir.path().join("test.db")).await.unwrap();

    // a real decodable file, so mpv has something to actually play
    let audio = std::fs::canonicalize(FIXTURE).unwrap();
    let size = std::fs::metadata(&audio).unwrap().len();
    let artist = db.artists().get_or_create("Soda Stereo").await.unwrap();
    let album = db
        .albums()
        .upsert("Doble Vida", artist, Some(1988))
        .await
        .unwrap();
    db.tracks()
        .insert(&NewTrack {
            title: "En la Ciudad de la Furia".to_owned(),
            artist_id: artist,
            album_id: Some(album),
            track_no: Some(1),
            disc_no: Some(1),
            year: Some(1988),
            duration_ms: 300,
            genres: vec!["Rock".to_owned()],
            technical: TrackTechnical {
                codec: "FLAC".to_owned(),
                container: "FLAC".to_owned(),
                bitrate_kbps: 1_024,
                bit_depth: Some(16),
                sample_rate_hz: 44_100,
                channels: 2,
                replaygain_track_gain: None,
                replaygain_album_gain: None,
                peak: None,
                dr_score: None,
                encoder: None,
                file_path: audio,
                file_size_bytes: size,
                md5: None,
            },
        })
        .await
        .unwrap();

    let server = start(
        db,
        ServerConfig {
            port: 0,
            password: PASSWORD.to_owned(),
            server_version: "test".to_owned(),
            cover_cache_dir: dir.path().join("covers"),
        },
    )
    .await
    .unwrap();

    // what the user does: point at a server, walk down to a song, hit play
    let base = format!("http://127.0.0.1:{}", server.addr().port());
    let client = SubsonicClient::new(&ClientConfig::new(&base, "ian", PASSWORD)).unwrap();

    client.ping().await.expect("server reachable");
    let artists = client.get_artists().await.unwrap();
    let artist_id = artists.index[0].artist[0].id.clone();
    let albums = client.get_artist(&artist_id).await.unwrap();
    let songs = client.get_album(&albums.album[0].id).await.unwrap();
    let song = &songs.song[0];
    assert_eq!(song.title, "En la Ciudad de la Furia");

    player
        .load_and_play(-1, MediaSource::Url(client.stream_url(&song.id)))
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let state = player.state();
        if state.status == PlaybackStatus::Playing && state.duration_ms > 0 {
            player.stop().unwrap();
            server.stop().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never started playing; last state: {:?}", player.state());
}
