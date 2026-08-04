//! `OpenSubsonic` client — the mirror image of `signal-server`.
//!
//! Where `signal-server` answers Subsonic requests about Signal's local
//! library, this crate *makes* them against someone else's server (another
//! Signal, Navidrome, Airsonic, Gonic) so Signal can browse and stream remote
//! catalogs. See `docs/11-subsonic-client.md`.
//!
//! Deliberately a pure protocol client: it knows nothing about `signal-db`,
//! `signal-player`, or Signal's domain model. Turning a [`Child`] into
//! something playable is the caller's job — `src-tauri` builds a
//! `MediaSource::Url` from [`SubsonicClient::stream_url`].
//!
//! [`Child`]: signal_subsonic_types::Child

mod auth;
mod client;
mod error;

pub use auth::AuthMode;
pub use client::{ClientConfig, SearchLimits, ServerIdent, SubsonicClient, API_VERSION};
pub use error::ClientError;

// re-exported so consumers don't need a direct dependency on the types crate
pub use signal_subsonic_types as types;
