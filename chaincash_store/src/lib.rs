use database::ConnectionPool;

pub mod database;
pub mod error;

pub use error::Error;

trait Updatable {
    fn needs_update(&self) -> Result<bool, Error>;
    fn update(&self) -> Result<(), Error>;
}

trait Store {
    fn notes(&self) -> ();
    fn reserves(&self) -> ();
}

pub struct ChainCashStore {
    pool: ConnectionPool,
}

impl ChainCashStore {
    pub fn open<S: Into<String>>(store_url: S) -> Result<Self, crate::Error> {
        let pool = database::connect(store_url)?;

        Ok(Self { pool })
    }
}

impl Updatable for ChainCashStore {
    fn needs_update(&self) -> Result<bool, Error> {
        database::has_pending_migrations(&mut self.pool.get().unwrap())
    }

    fn update(&self) -> Result<(), Error> {
        database::run_pending_migrations(&mut self.pool.get().unwrap())
    }
}

impl Store for ChainCashStore {
    fn notes(&self) -> () {
        todo!()
    }

    fn reserves(&self) -> () {
        todo!()
    }
}
