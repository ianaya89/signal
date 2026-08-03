#![allow(clippy::unwrap_used)]

use md5::{Digest, Md5};
use signal_core::TrackTechnical;
use signal_db::{DbPool, NewTrack};
use signal_server::{start, ServerConfig, ServerHandle};

const PASSWORD: &str = "sesame";
const FILE_BYTES: &[u8] = b"FLACfLaCdata0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKL0123";

struct TestServer {
    db: DbPool,
    base: String,
    handle: ServerHandle,
    dir: tempfile::TempDir,
}

async fn setup() -> TestServer {
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
        let track_id = db
            .tracks()
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
        if i == 1 {
            db.tracks().set_favorite(track_id, true).await.unwrap();
        }
    }
    let playlist = db.playlists().create("road trip").await.unwrap();
    db.playlists().add_tracks(playlist, &[1]).await.unwrap();

    let handle = start(
        db.clone(),
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
    TestServer {
        db,
        base,
        handle,
        dir,
    }
}

fn auth() -> String {
    format!("u=tester&p={PASSWORD}")
}

async fn get_json(base: &str, path_and_query: &str) -> serde_json::Value {
    let url = format!("{base}/rest/{path_and_query}&f=json&{}", auth());
    let body = reqwest::get(&url).await.unwrap().text().await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    v["subsonic-response"].clone()
}

#[tokio::test]
async fn root_answers_for_reachability_probes() {
    let ts = setup().await;
    let resp = reqwest::get(&ts.base).await.unwrap();
    assert_eq!(resp.status(), 200);
    ts.handle.stop().await;
}

#[tokio::test]
async fn ping_xml_and_json() {
    let ts = setup().await;

    let xml = reqwest::get(format!("{}/rest/ping.view?{}", ts.base, auth()))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(xml.contains("status=\"ok\""), "{xml}");
    assert!(xml.contains("openSubsonic=\"true\""), "{xml}");

    let env = get_json(&ts.base, "ping?").await;
    assert_eq!(env["status"], "ok");
    assert_eq!(env["type"], "signal");

    ts.handle.stop().await;
}

#[tokio::test]
async fn auth_failures() {
    let ts = setup().await;

    // missing username → 10
    let body = reqwest::get(format!("{}/rest/ping?p={PASSWORD}&f=json", ts.base))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["subsonic-response"]["error"]["code"], 10);

    // wrong token → 40
    let mut hasher = Md5::new();
    hasher.update(b"wrongpass");
    hasher.update(b"abc123");
    let token = hex::encode_upper(hasher.finalize()).to_lowercase();
    let body = reqwest::get(format!(
        "{}/rest/ping?u=x&t={token}&s=abc123&f=json",
        ts.base
    ))
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["subsonic-response"]["error"]["code"], 40);

    // correct token → ok
    let mut hasher = Md5::new();
    hasher.update(PASSWORD.as_bytes());
    hasher.update(b"abc123");
    let token = hex::encode(hasher.finalize());
    let body = reqwest::get(format!(
        "{}/rest/ping?u=x&t={token}&s=abc123&f=json",
        ts.base
    ))
    .await
    .unwrap()
    .text()
    .await
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["subsonic-response"]["status"], "ok");

    ts.handle.stop().await;
}

#[tokio::test]
async fn browse_chain_artists_to_album_songs() {
    let ts = setup().await;

    let env = get_json(&ts.base, "getArtists?").await;
    let index = env["artists"]["index"].as_array().unwrap();
    assert_eq!(index[0]["name"], "S");
    let artist_id = index[0]["artist"][0]["id"].as_str().unwrap().to_owned();
    assert_eq!(index[0]["artist"][0]["name"], "Soda Stereo");

    let env = get_json(&ts.base, &format!("getArtist?id={artist_id}")).await;
    let album_id = env["artist"]["album"][0]["id"].as_str().unwrap().to_owned();
    assert_eq!(env["artist"]["album"][0]["songCount"], 2);
    assert_eq!(env["artist"]["album"][0]["duration"], 400);

    let env = get_json(&ts.base, &format!("getAlbum?id={album_id}")).await;
    let songs = env["album"]["song"].as_array().unwrap();
    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0]["artist"], "Soda Stereo");
    assert_eq!(songs[0]["genre"], "Rock");
    assert_eq!(songs[0]["suffix"], "flac");
    assert_eq!(songs[0]["contentType"], "audio/flac");
    assert!(songs[1]["starred"].is_string());

    ts.handle.stop().await;
}

#[tokio::test]
async fn lists_search_playlists_starred() {
    let ts = setup().await;

    let env = get_json(&ts.base, "getAlbumList2?type=newest").await;
    assert_eq!(env["albumList2"]["album"].as_array().unwrap().len(), 1);

    let env = get_json(&ts.base, "getRandomSongs?size=5").await;
    assert_eq!(env["randomSongs"]["song"].as_array().unwrap().len(), 2);

    let env = get_json(&ts.base, "search3?query=furia").await;
    assert_eq!(
        env["searchResult3"]["song"][0]["title"],
        "En la Ciudad de la Furia"
    );

    let env = get_json(&ts.base, "getStarred2?").await;
    assert_eq!(env["starred2"]["song"].as_array().unwrap().len(), 1);

    let env = get_json(&ts.base, "getGenres?").await;
    assert_eq!(env["genres"]["genre"][0]["value"], "Rock");
    assert_eq!(env["genres"]["genre"][0]["songCount"], 2);

    // default smart playlists (migration 0003) ride along with sp- ids
    let env = get_json(&ts.base, "getPlaylists?").await;
    let playlists = env["playlists"]["playlist"].as_array().unwrap();
    let ours = playlists.iter().find(|p| p["name"] == "road trip").unwrap();
    assert!(ours["id"].as_str().unwrap().starts_with("pl-"));
    // real timestamps, not a fresh now() per call
    assert!(ours["changed"].as_str().unwrap().starts_with("20"));
    assert!(playlists
        .iter()
        .any(|p| p["id"].as_str().unwrap().starts_with("sp-")));

    let env = get_json(&ts.base, "createPlaylist?name=from-phone&songId=tr-2").await;
    assert_eq!(env["playlist"]["songCount"], 1);
    let created = ts.db.playlists().list().await.unwrap();
    assert!(created.iter().any(|p| p.name == "from-phone"));

    ts.handle.stop().await;
}

#[tokio::test]
async fn stream_full_and_range() {
    let ts = setup().await;

    let resp = reqwest::get(format!("{}/rest/stream?id=tr-1&{}", ts.base, auth()))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()["content-type"].to_str().unwrap(),
        "audio/flac"
    );
    assert_eq!(resp.bytes().await.unwrap().as_ref(), FILE_BYTES);

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/rest/stream?id=tr-1&{}", ts.base, auth()))
        .header("Range", "bytes=0-3")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 206);
    assert_eq!(
        resp.headers()["content-range"].to_str().unwrap(),
        format!("bytes 0-3/{}", FILE_BYTES.len())
    );
    assert_eq!(resp.bytes().await.unwrap().as_ref(), &FILE_BYTES[0..4]);

    ts.handle.stop().await;
}

#[tokio::test]
async fn scrobble_star_and_rating_write_back() {
    let ts = setup().await;

    let env = get_json(&ts.base, "scrobble?id=tr-1&submission=true").await;
    assert_eq!(env["status"], "ok");
    let track = ts.db.tracks().get(1).await.unwrap().unwrap();
    assert_eq!(track.play_count, 1);
    let source: String = sqlx::query_scalar("SELECT source FROM play_events WHERE track_id = 1")
        .fetch_one(ts.db.inner())
        .await
        .unwrap();
    assert_eq!(source, "remote");

    // the scrobble makes the album show up in frequent + recent lists
    let env = get_json(&ts.base, "getAlbumList2?type=frequent").await;
    assert_eq!(env["albumList2"]["album"].as_array().unwrap().len(), 1);
    let env = get_json(&ts.base, "getAlbumList2?type=recent").await;
    assert_eq!(env["albumList2"]["album"][0]["name"], "Doble Vida");

    let env = get_json(&ts.base, "star?id=tr-1").await;
    assert_eq!(env["status"], "ok");
    assert!(ts.db.tracks().get(1).await.unwrap().unwrap().favorite);

    // album star is a tolerated no-op
    let env = get_json(&ts.base, "star?albumId=al-1").await;
    assert_eq!(env["status"], "ok");

    let env = get_json(&ts.base, "setRating?id=tr-1&rating=4").await;
    assert_eq!(env["status"], "ok");
    assert_eq!(
        ts.db.tracks().get(1).await.unwrap().unwrap().rating,
        Some(4)
    );

    ts.handle.stop().await;
}

#[tokio::test]
async fn cover_art_scales_on_request() {
    let ts = setup().await;

    let art = ts.dir.path().join("cover.png");
    image::RgbImage::from_pixel(600, 600, image::Rgb([200, 40, 40]))
        .save(&art)
        .unwrap();
    ts.db
        .albums()
        .set_artwork(1, &art.to_string_lossy())
        .await
        .unwrap();

    // scaled: fits the 128 bucket, comes back as jpeg
    let resp = reqwest::get(format!(
        "{}/rest/getCoverArt?id=al-1&size=100&{}",
        ts.base,
        auth()
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()["content-type"].to_str().unwrap(),
        "image/jpeg"
    );
    let scaled = image::load_from_memory(&resp.bytes().await.unwrap()).unwrap();
    assert_eq!(scaled.width(), 128);

    // no size param → original png untouched
    let resp = reqwest::get(format!("{}/rest/getCoverArt?id=al-1&{}", ts.base, auth()))
        .await
        .unwrap();
    assert_eq!(
        resp.headers()["content-type"].to_str().unwrap(),
        "image/png"
    );
    let original = image::load_from_memory(&resp.bytes().await.unwrap()).unwrap();
    assert_eq!(original.width(), 600);

    // absurd size → falls back to the original bytes
    let resp = reqwest::get(format!(
        "{}/rest/getCoverArt?id=al-1&size=9999&{}",
        ts.base,
        auth()
    ))
    .await
    .unwrap();
    assert_eq!(
        resp.headers()["content-type"].to_str().unwrap(),
        "image/png"
    );

    ts.handle.stop().await;
}

#[tokio::test]
async fn form_post_carries_params() {
    let ts = setup().await;
    let client = reqwest::Client::new();

    // auth + format entirely in the body
    let body = client
        .post(format!("{}/rest/ping", ts.base))
        .form(&[("u", "x"), ("p", PASSWORD), ("f", "json")])
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["subsonic-response"]["status"], "ok");

    // repeated ids in the body (the reason formPost exists)
    let body = client
        .post(format!("{}/rest/star", ts.base))
        .form(&[
            ("u", "x"),
            ("p", PASSWORD),
            ("f", "json"),
            ("id", "tr-1"),
            ("id", "tr-2"),
        ])
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["subsonic-response"]["status"], "ok");
    assert!(ts.db.tracks().get(1).await.unwrap().unwrap().favorite);
    assert!(ts.db.tracks().get(2).await.unwrap().unwrap().favorite);

    // extension advertised
    let env = get_json(&ts.base, "getOpenSubsonicExtensions?").await;
    assert_eq!(env["openSubsonicExtensions"][0]["name"], "formPost");

    ts.handle.stop().await;
}

#[tokio::test]
async fn unknown_endpoint_and_shutdown() {
    let ts = setup().await;

    let env = get_json(&ts.base, "getVideos?").await;
    assert_eq!(env["status"], "failed");
    assert_eq!(env["error"]["code"], 0);

    let base = ts.base.clone();
    ts.handle.stop().await;
    assert!(reqwest::get(format!("{base}/rest/ping?{}", auth()))
        .await
        .is_err());
}
