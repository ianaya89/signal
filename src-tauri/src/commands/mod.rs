pub mod device;
pub mod edit;
pub mod library;
pub mod player;
pub mod playlist;
pub mod queue;
pub mod search;
pub mod settings;
pub mod stats;

use signal_core::SignalError;

pub trait DbResultExt<T> {
    fn db_err(self) -> Result<T, SignalError>;
}

impl<T> DbResultExt<T> for sqlx::Result<T> {
    fn db_err(self) -> Result<T, SignalError> {
        self.map_err(|e| SignalError::Db(e.to_string()))
    }
}
