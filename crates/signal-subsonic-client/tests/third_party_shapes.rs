//! Parsing third-party server responses.
//!
//! The `against_signal_server` tests prove the client agrees with Signal's own
//! server, which is internally consistent by construction — it can't surface
//! the ways *other* servers differ. These fixtures cover that: extra fields the
//! shared DTOs don't model, absent optional fields, and id schemes that aren't
//! Signal's `tr-7`/`al-3`.
//!
//! The fixtures are hand-built from each server's documented and observed
//! response shape, not captured from a live instance. They pin the parsing
//! behavior Signal depends on, but they are not evidence that any specific
//! Navidrome/Gonic/Airsonic build answers exactly this — that's what the
//! against-a-real-server smoke test in `docs/11-subsonic-client.md` §7 is for.

#![allow(clippy::unwrap_used)]

use signal_subsonic_types::{AlbumWithSongs, ArtistsIndex, Envelope, SearchResult3};

fn envelope(name: &str) -> Envelope {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let raw = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&raw).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[test]
fn navidrome_album_parses_despite_unmodeled_fields() {
    let env = envelope("navidrome-getalbum.json");
    assert!(env.response.is_ok());
    assert!(env.response.open_subsonic);
    assert_eq!(env.response.server_type.as_deref(), Some("navidrome"));

    let album: AlbumWithSongs = env.response.take("album").unwrap().unwrap();
    assert_eq!(album.album.name, "Doble Vida");
    assert_eq!(album.album.year, Some(1988));
    // opaque id, nothing like Signal's own al-3 scheme
    assert_eq!(album.album.id, "6b0dc0e8b9f0d1a7c3e5f2a1b4d6c8e0");
    assert_eq!(album.song.len(), 2);

    let starred = &album.song[1];
    assert_eq!(starred.title, "En la Ciudad de la Furia");
    assert_eq!(starred.user_rating, Some(5));
    assert!(starred.starred.is_some());
    assert_eq!(starred.suffix, "flac");
    assert_eq!(starred.duration, 243);
}

#[test]
fn gonic_artists_parse_with_optional_fields_missing() {
    let env = envelope("gonic-getartists.json");
    assert!(env.response.is_ok());
    // gonic omits openSubsonic entirely — absence must not read as failure
    assert!(!env.response.open_subsonic);

    let artists: ArtistsIndex = env.response.take("artists").unwrap().unwrap();
    assert_eq!(artists.ignored_articles, "");
    assert_eq!(artists.index.len(), 2);
    assert_eq!(artists.index[0].artist[0].name, "Soda Stereo");
    // albumCount omitted for Virus; the default keeps the whole response usable
    assert_eq!(artists.index[1].artist[0].album_count, 0);
}

#[test]
fn airsonic_search_results_parse_across_all_three_kinds() {
    let env = envelope("airsonic-search3.json");
    let hits: SearchResult3 = env.response.take("searchResult3").unwrap().unwrap();
    assert_eq!(hits.artist.len(), 1);
    assert_eq!(hits.album.len(), 1);
    assert_eq!(hits.song.len(), 1);
    assert_eq!(hits.album[0].artist_id, "8");
    assert_eq!(hits.song[0].content_type, "audio/mpeg");
    assert_eq!(hits.song[0].bit_rate, 320);
    assert!(hits.song[0].starred.is_none());
}
