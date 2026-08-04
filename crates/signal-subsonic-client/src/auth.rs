//! Client-side Subsonic auth — produces exactly the `t`+`s` pair that
//! `signal-server`'s `auth::check` validates, and the `p=` plaintext form for
//! servers that reject token auth.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};

/// `md5(password ++ salt)`, hex-encoded — the `t` parameter.
#[must_use]
pub fn token(password: &str, salt: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    hex::encode(hasher.finalize())
}

/// Per-request salt source.
///
/// A Subsonic salt is an anti-replay nonce, not a secret, so this deliberately
/// avoids adding a `rand` dependency the workspace doesn't otherwise carry —
/// the same call `signal-server`'s `lists.rs` shuffle already made. Clock nanos
/// mixed with a per-client counter, hashed so consecutive salts don't reveal
/// the sequence.
#[derive(Debug, Default)]
pub struct SaltGen(AtomicU64);

impl SaltGen {
    #[must_use]
    pub fn next_salt(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| u64::try_from(d.as_nanos()).ok())
            .unwrap_or(0);
        let counter = self.0.fetch_add(1, Ordering::Relaxed);
        let mut hasher = Md5::new();
        hasher.update(nanos.to_le_bytes());
        hasher.update(counter.to_le_bytes());
        hex::encode(hasher.finalize())[..16].to_owned()
    }
}

/// Which credential form to put on the wire.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AuthMode {
    /// `t=md5(password+salt)&s=salt` — preferred, and what every modern server
    /// accepts.
    #[default]
    Token,
    /// `p=password` in the clear. Only for servers that reject token auth;
    /// `remote_sources.auth_mode` persists the choice once probed.
    LegacyPlain,
}

impl AuthMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::LegacyPlain => "legacy_p",
        }
    }

    #[must_use]
    pub fn from_str_or_default(raw: &str) -> Self {
        match raw {
            "legacy_p" => Self::LegacyPlain,
            _ => Self::Token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_matches_the_servers_own_vector() {
        // same (password, salt) pair signal-server's auth tests use
        assert_eq!(
            token("sesame", "c19b2d"),
            {
                let mut h = Md5::new();
                h.update(b"sesame");
                h.update(b"c19b2d");
                hex::encode(h.finalize())
            },
            "client token generation drifted from md5(password ++ salt)"
        );
    }

    #[test]
    fn salts_are_hex_and_do_not_repeat() {
        let gen = SaltGen::default();
        let a = gen.next_salt();
        let b = gen.next_salt();
        assert_ne!(a, b);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn auth_mode_round_trips_through_its_db_string() {
        for mode in [AuthMode::Token, AuthMode::LegacyPlain] {
            assert_eq!(AuthMode::from_str_or_default(mode.as_str()), mode);
        }
        assert_eq!(AuthMode::from_str_or_default("garbage"), AuthMode::Token);
    }
}
