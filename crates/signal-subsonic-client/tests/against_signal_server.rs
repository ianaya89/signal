//! The client tested against Signal's own embedded server.
//!
//! Both halves of the wire format live in this repo, so the happy path needs
//! no mocking at all: `signal_server::start` binds an ephemeral port, the
//! client points at it, and any drift between what the server emits and what
//! the client parses fails here immediately. Mocks are reserved for what
//! Signal's own server can't produce (legacy `p=`-only hosts, malformed
//! bodies, TLS behavior).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use signal_core::TrackTechnical;
use signal_db::{DbPool, NewTrack};
use signal_server::{start, ServerConfig, ServerHandle};
use signal_subsonic_client::{AuthMode, ClientConfig, SearchLimits, SubsonicClient};

const PASSWORD: &str = "sesame";
const FILE_BYTES: &[u8] = b"FLACfLaCdata0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKL0123";

struct Fixture {
    client: SubsonicClient,
    base: String,
    handle: ServerHandle,
    _dir: tempfile::TempDir,
}

async fn setup() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let db = DbPool::connect(&dir.path().join("test.db")).await.unwrap();

    let soda = db.artists().get_or_create("Soda Stereo").await.unwrap();
    let album = db
        .albums()
        .upsert("Doble Vida", soda, Some(1988))
        .await
        .unwrap();
    for (i, title) in ["Picnic en el 4to B", "En la Ciudad de la Furia"]
        .iter()
        .enumerate()
    {
        let file = dir.path().join(format!("track{i}.flac"));
        std::fs::write(&file, FILE_BYTES).unwrap();
        db.tracks()
            .insert(&NewTrack {
                title: (*title).to_owned(),
                artist_id: soda,
                album_id: Some(album),
                track_no: Some(u32::try_from(i).unwrap() + 1),
                disc_no: Some(1),
                year: Some(1988),
                duration_ms: 200_000,
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
                    file_path: file.clone(),
                    file_size_bytes: FILE_BYTES.len() as u64,
                    md5: None,
                },
            })
            .await
            .unwrap();
    }

    let handle = start(
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
    let base = format!("http://127.0.0.1:{}", handle.addr().port());
    let client = SubsonicClient::new(&ClientConfig::new(&base, "tester", PASSWORD)).unwrap();
    Fixture {
        client,
        base,
        handle,
        _dir: dir,
    }
}

#[tokio::test]
async fn ping_reports_the_server_identity() {
    let f = setup().await;
    let ident = f.client.ping().await.unwrap();
    assert_eq!(ident.version, "1.16.1");
    assert_eq!(ident.server_type.as_deref(), Some("signal"));
    assert_eq!(ident.server_version.as_deref(), Some("test"));
    assert!(ident.open_subsonic);
    f.handle.stop().await;
}

#[tokio::test]
async fn browse_chain_artists_to_album_songs() {
    let f = setup().await;

    let artists = f.client.get_artists().await.unwrap();
    let bucket = artists
        .index
        .iter()
        .find(|b| b.name == "S")
        .expect("artists bucketed under S");
    let soda = &bucket.artist[0];
    assert_eq!(soda.name, "Soda Stereo");
    assert_eq!(soda.album_count, 1);

    let artist = f.client.get_artist(&soda.id).await.unwrap();
    assert_eq!(artist.artist.name, "Soda Stereo");
    assert_eq!(artist.album.len(), 1);
    let album_id = artist.album[0].id.clone();
    assert_eq!(artist.album[0].name, "Doble Vida");
    assert_eq!(artist.album[0].year, Some(1988));

    let album = f.client.get_album(&album_id).await.unwrap();
    assert_eq!(album.album.song_count, 2);
    assert_eq!(album.song.len(), 2);
    let song = &album.song[0];
    assert_eq!(song.kind, "music");
    assert_eq!(song.duration, 200);
    assert_eq!(song.suffix, "flac");
    assert_eq!(song.content_type, "audio/flac");
    assert_eq!(song.artist.as_deref(), Some("Soda Stereo"));
    assert!(!song.path.contains(f.base.as_str()));

    f.handle.stop().await;
}

#[tokio::test]
async fn search3_finds_across_all_three_kinds() {
    let f = setup().await;
    let hits = f
        .client
        .search3("furia", SearchLimits::default())
        .await
        .unwrap();
    assert_eq!(hits.song.len(), 1);
    assert_eq!(hits.song[0].title, "En la Ciudad de la Furia");

    let broad = f
        .client
        .search3("soda", SearchLimits::default())
        .await
        .unwrap();
    assert_eq!(broad.artist.len(), 1);

    // an empty library slot is not an error — the server answers with empties
    let miss = f
        .client
        .search3("nothingmatchesthis", SearchLimits::default())
        .await
        .unwrap();
    assert!(miss.song.is_empty() && miss.album.is_empty() && miss.artist.is_empty());

    f.handle.stop().await;
}

#[tokio::test]
async fn stream_url_is_fetchable_and_ranged() {
    let f = setup().await;
    let album = {
        let artists = f.client.get_artists().await.unwrap();
        let artist_id = artists.index[0].artist[0].id.clone();
        let artist = f.client.get_artist(&artist_id).await.unwrap();
        f.client.get_album(&artist.album[0].id).await.unwrap()
    };
    let url = f.client.stream_url(&album.song[0].id);

    let full = reqwest::get(&url).await.unwrap();
    assert_eq!(full.status(), 200);
    assert_eq!(full.bytes().await.unwrap().as_ref(), FILE_BYTES);

    // mpv seeks with Range on every remote stream; a client-built URL that
    // can't be ranged would make remote playback unseekable
    let partial = reqwest::Client::new()
        .get(&url)
        .header("Range", "bytes=0-9")
        .send()
        .await
        .unwrap();
    assert_eq!(partial.status(), 206);
    assert_eq!(partial.bytes().await.unwrap().len(), 10);

    f.handle.stop().await;
}

#[tokio::test]
async fn legacy_plain_auth_is_accepted_too() {
    let f = setup().await;
    let legacy = f.client.with_auth_mode(AuthMode::LegacyPlain);
    assert!(legacy.ping().await.is_ok());
    f.handle.stop().await;
}

#[tokio::test]
async fn wrong_password_surfaces_as_auth_not_a_generic_error() {
    let f = setup().await;
    let wrong =
        SubsonicClient::new(&ClientConfig::new(&f.base, "tester", "not-the-password")).unwrap();

    let err = wrong.ping().await.unwrap_err();
    assert!(
        matches!(err, signal_subsonic_client::ClientError::Auth),
        "expected Auth, got {err:?}"
    );

    f.handle.stop().await;
}

#[tokio::test]
async fn unknown_id_surfaces_the_servers_error_code() {
    let f = setup().await;
    let err = f.client.get_album("al-999").await.unwrap_err();
    match err {
        signal_subsonic_client::ClientError::Api { code, .. } => assert_eq!(code, 70),
        other => panic!("expected an Api error, got {other:?}"),
    }
    f.handle.stop().await;
}

#[tokio::test]
async fn non_subsonic_body_is_reported_as_a_parse_error() {
    let f = setup().await;
    // the bare root answers plain text for reachability probes, so pointing the
    // client's /rest path at a non-API host is the realistic misconfiguration:
    // a wrong base URL that still answers 200 with HTML
    let bad = SubsonicClient::new(&ClientConfig::new(
        format!("{}/nope", f.base),
        "tester",
        PASSWORD,
    ))
    .unwrap();
    let err = bad.ping().await.unwrap_err();
    assert!(
        matches!(err, signal_subsonic_client::ClientError::Parse(_)),
        "expected Parse, got {err:?}"
    );
    f.handle.stop().await;
}
