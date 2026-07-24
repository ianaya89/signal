#![allow(clippy::unwrap_used)]

use signal_core::TrackTechnical;
use signal_db::{DbPool, NewTrack};

async fn test_db() -> (DbPool, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = DbPool::connect(&dir.path().join("test.db")).await.unwrap();
    (db, dir)
}

fn new_track(title: &str, artist_id: i64, album_id: i64, path: &str) -> NewTrack {
    NewTrack {
        title: title.to_owned(),
        artist_id,
        album_id: Some(album_id),
        track_no: Some(1),
        disc_no: Some(1),
        year: Some(1999),
        duration_ms: 215_000,
        genres: vec!["Rock".to_owned()],
        technical: TrackTechnical {
            codec: "FLAC".to_owned(),
            container: "FLAC".to_owned(),
            bitrate_kbps: 1024,
            bit_depth: Some(16),
            sample_rate_hz: 44_100,
            channels: 2,
            replaygain_track_gain: None,
            replaygain_album_gain: None,
            peak: None,
            dr_score: None,
            encoder: None,
            file_path: path.into(),
            file_size_bytes: 30_000_000,
            md5: None,
        },
    }
}

#[tokio::test]
async fn migrations_apply_and_roundtrip() {
    let (db, _dir) = test_db().await;

    let artist_id = db.artists().get_or_create("Gustavo Cerati").await.unwrap();
    let same = db.artists().get_or_create("Gustavo Cerati").await.unwrap();
    assert_eq!(artist_id, same);

    let album_id = db
        .albums()
        .upsert("Bocanada", artist_id, Some(1999))
        .await
        .unwrap();
    let track_id = db
        .tracks()
        .insert(&new_track(
            "Puente",
            artist_id,
            album_id,
            "/music/puente.flac",
        ))
        .await
        .unwrap();

    let track = db.tracks().get(track_id).await.unwrap().unwrap();
    assert_eq!(track.title, "Puente");
    assert_eq!(track.technical.sample_rate_hz, 44_100);
    assert_eq!(track.rating, None);

    let albums = db.albums().list().await.unwrap();
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].artist_name, "Gustavo Cerati");
    assert_eq!(albums[0].track_count, 1);

    let artists = db.artists().list().await.unwrap();
    assert_eq!(artists.len(), 1);
    assert_eq!(artists[0].album_count, 1);

    let tracks = db.albums().tracks(album_id).await.unwrap();
    assert_eq!(tracks.len(), 1);

    assert_eq!(
        db.tracks().id_by_path("/music/puente.flac").await.unwrap(),
        Some(track_id)
    );
    assert_eq!(db.tracks().id_by_path("/nope.flac").await.unwrap(), None);
}

#[tokio::test]
async fn fts_index_is_populated_by_triggers() {
    let (db, _dir) = test_db().await;

    let artist_id = db.artists().get_or_create("Radiohead").await.unwrap();
    let album_id = db
        .albums()
        .upsert("In Rainbows", artist_id, Some(2007))
        .await
        .unwrap();
    db.tracks()
        .insert(&new_track(
            "Reckoner",
            artist_id,
            album_id,
            "/music/reckoner.flac",
        ))
        .await
        .unwrap();

    let hits: Vec<i64> =
        sqlx::query_scalar("SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH 'radiohead'")
            .fetch_all(db.inner())
            .await
            .unwrap();
    assert_eq!(hits.len(), 1);

    let genre_hits: Vec<i64> =
        sqlx::query_scalar("SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH 'rock'")
            .fetch_all(db.inner())
            .await
            .unwrap();
    assert_eq!(genre_hits.len(), 1);
}

#[tokio::test]
async fn artists_and_albums_dedupe_case_insensitively() {
    let (db, _dir) = test_db().await;

    let a1 = db
        .artists()
        .get_or_create("Los Angeles Azules")
        .await
        .unwrap();
    let a2 = db
        .artists()
        .get_or_create("Los angeles azules")
        .await
        .unwrap();
    let a3 = db
        .artists()
        .get_or_create("LOS ANGELES AZULES")
        .await
        .unwrap();
    assert_eq!(a1, a2);
    assert_eq!(a1, a3);

    let al1 = db
        .albums()
        .upsert("Epoca Dorada", a1, Some(2000))
        .await
        .unwrap();
    let al2 = db.albums().upsert("epoca dorada", a1, None).await.unwrap();
    assert_eq!(al1, al2);

    // artist list only shows album artists with tracks
    db.tracks()
        .insert(&new_track("Track", a1, al1, "/music/t1.flac"))
        .await
        .unwrap();
    let feat = db
        .artists()
        .get_or_create("Los Angeles Azules feat. X")
        .await
        .unwrap();
    assert_ne!(feat, a1);
    let artists = db.artists().list().await.unwrap();
    assert_eq!(artists.len(), 1);
    assert_eq!(artists[0].name, "Los Angeles Azules");

    let by_artist = db.albums().list_by_artist(a1).await.unwrap();
    assert_eq!(by_artist.len(), 1);
}

#[tokio::test]
async fn rename_to_existing_merges() {
    let (db, _dir) = test_db().await;

    // two spellings of the same artist, each with the "same" album
    let junk = db
        .artists()
        .get_or_create("Los Angeles Azules (Www.X.Com)")
        .await
        .unwrap();
    let clean = db
        .artists()
        .get_or_create("Los Angeles Azules")
        .await
        .unwrap();
    let junk_album = db
        .albums()
        .upsert("Epoca Dorada", junk, None)
        .await
        .unwrap();
    let clean_album = db
        .albums()
        .upsert("Epoca Dorada", clean, Some(2000))
        .await
        .unwrap();
    db.tracks()
        .insert(&new_track("T1", junk, junk_album, "/m/t1.flac"))
        .await
        .unwrap();
    db.tracks()
        .insert(&new_track("T2", clean, clean_album, "/m/t2.flac"))
        .await
        .unwrap();

    // rename junk → clean name: artists merge, colliding albums fuse
    let merged = db
        .artists()
        .rename(junk, "Los Angeles Azules")
        .await
        .unwrap();
    assert!(merged);

    let artists = db.artists().list().await.unwrap();
    assert_eq!(artists.len(), 1);
    assert_eq!(artists[0].track_count, 2);

    let albums = db.albums().list_by_artist(clean).await.unwrap();
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].track_count, 2);

    // album rename → existing album of same artist merges too
    let other = db
        .albums()
        .upsert("Epoca Dorada 2", clean, None)
        .await
        .unwrap();
    db.tracks()
        .insert(&new_track("T3", clean, other, "/m/t3.flac"))
        .await
        .unwrap();
    let merged = db.albums().rename(other, "epoca dorada").await.unwrap();
    assert!(merged);
    let albums = db.albums().list_by_artist(clean).await.unwrap();
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].track_count, 3);

    // plain rename (no collision) still works
    let merged = db.artists().rename(clean, "LAA").await.unwrap();
    assert!(!merged);
}

#[tokio::test]
async fn settings_roundtrip() {
    let (db, _dir) = test_db().await;
    assert_eq!(db.settings().get("library.root").await.unwrap(), None);
    db.settings().set("library.root", "/music").await.unwrap();
    db.settings().set("library.root", "/flac").await.unwrap();
    assert_eq!(
        db.settings().get("library.root").await.unwrap().as_deref(),
        Some("/flac")
    );
}
