pub mod error;
pub mod notes;
pub mod reserves;
pub mod sqlite;

pub use error::Error;
pub use notes::NoteService;
pub use reserves::ReserveService;
pub use sqlite::SqliteChainCashStore;

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    pub url: String,
}

pub trait Update {
    fn has_updates(&self) -> Result<bool, Error>;
    fn update(&self) -> Result<(), Error>;
}

pub trait ChainCashStore: Sync + Send {
    fn notes(&self) -> Box<dyn NoteService>;
    fn reserves(&self) -> Box<dyn ReserveService>;
}
