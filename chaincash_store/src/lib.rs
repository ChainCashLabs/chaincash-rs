use database::ConnectionPool;

pub(crate) mod database;
pub mod entities;
pub mod error;

use entities::{notes::NoteService, reserves::ReserveService};
pub use error::Error;

trait Update {
    fn has_updates(&self) -> Result<bool, Error>;
    fn update(&self) -> Result<(), Error>;
}

trait Store {
    fn notes(&self) -> &NoteService;
    fn reserves(&self) -> &ReserveService;
}

pub struct ChainCashStore {
    pool: ConnectionPool,
    notes: NoteService,
    reserves: ReserveService,
}

impl ChainCashStore {
    pub fn open<S: Into<String>>(store_url: S) -> Result<Self, Error> {
        let pool = database::connect(store_url)?;
        let notes = NoteService::new(pool.clone());
        let reserves = ReserveService::new(pool.clone());

        Ok(Self {
            pool,
            notes,
            reserves,
        })
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        Self::open(":memory:")
    }
}

impl Update for ChainCashStore {
    fn has_updates(&self) -> Result<bool, Error> {
        database::has_pending_migrations(&mut self.pool.get().unwrap())
    }

    fn update(&self) -> Result<(), Error> {
        database::run_pending_migrations(&mut self.pool.get().unwrap())
    }
}

impl Store for ChainCashStore {
    fn notes(&self) -> &NoteService {
        &self.notes
    }

    fn reserves(&self) -> &ReserveService {
        &self.reserves
    }
}
