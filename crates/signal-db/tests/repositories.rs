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
