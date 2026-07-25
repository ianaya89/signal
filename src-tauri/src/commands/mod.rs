pub mod device;
pub mod edit;
pub mod info;
pub mod library;
pub mod player;
pub mod playlist;
pub mod plugins;
pub mod queue;
pub mod search;
pub mod session;
pub mod settings;
pub mod stats;
pub mod window;

use signal_core::SignalError;

pub trait DbResultExt<T> {
    fn db_err(self) -> Result<T, SignalError>;
}

impl<T> DbResultExt<T> for sqlx::Result<T> {
    fn db_err(self) -> Result<T, SignalError> {
        self.map_err(|e| SignalError::Db(e.to_string()))
    }
}
