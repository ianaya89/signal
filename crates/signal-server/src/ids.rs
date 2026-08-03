//! Typed Subsonic ids. The API traffics in opaque strings; prefixes keep
//! track/album/artist/playlist id spaces from colliding.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sid {
    Track(i64),
    Album(i64),
    Artist(i64),
    Playlist(i64),
    SmartPlaylist(i64),
}

impl Sid {
    pub fn parse(s: &str) -> Option<Self> {
        let (prefix, raw) = s.split_once('-')?;
        let id: i64 = raw.parse().ok()?;
        match prefix {
            "tr" => Some(Self::Track(id)),
            "al" => Some(Self::Album(id)),
            "ar" => Some(Self::Artist(id)),
            "pl" => Some(Self::Playlist(id)),
            "sp" => Some(Self::SmartPlaylist(id)),
            _ => None,
        }
    }
}

impl fmt::Display for Sid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Track(id) => write!(f, "tr-{id}"),
            Self::Album(id) => write!(f, "al-{id}"),
            Self::Artist(id) => write!(f, "ar-{id}"),
            Self::Playlist(id) => write!(f, "pl-{id}"),
            Self::SmartPlaylist(id) => write!(f, "sp-{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for sid in [
            Sid::Track(7),
            Sid::Album(42),
            Sid::Artist(1),
            Sid::Playlist(9),
            Sid::SmartPlaylist(3),
        ] {
            assert_eq!(Sid::parse(&sid.to_string()), Some(sid));
        }
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(Sid::parse("xx-1"), None);
        assert_eq!(Sid::parse("tr-abc"), None);
        assert_eq!(Sid::parse("42"), None);
        assert_eq!(Sid::parse(""), None);
    }
}
