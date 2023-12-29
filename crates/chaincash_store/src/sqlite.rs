use crate::notes::Note;
use crate::reserves::Reserve;
use crate::{ChainCashStore, Error, NoteService, ReserveService, Update};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::borrow::BorrowMut;

#[rustfmt::skip]
pub mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
type ConnectionPool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Clone)]
pub struct SqliteChainCashStore {
    pool: ConnectionPool,
}

impl SqliteChainCashStore {
    pub fn open<S: Into<String>>(store_url: S) -> Result<Self, Error> {
        let manager = ConnectionManager::<SqliteConnection>::new(store_url);

        Ok(Self {
            pool: Pool::builder().build(manager)?,
        })
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        Self::open(":memory:")
    }
}

impl Update for SqliteChainCashStore {
    fn has_updates(&self) -> Result<bool, Error> {
        self.pool
            .get()?
            .borrow_mut()
            .has_pending_migration(MIGRATIONS)
            .map_err(|_| crate::Error::Migration("failed to check pending migrations".to_string()))
    }

    fn update(&self) -> Result<(), Error> {
        self.pool
            .get()?
            .borrow_mut()
            .run_pending_migrations(MIGRATIONS)
            .map_err(|_| crate::Error::Migration("failed to run pending migrations".to_string()))?;
        Ok(())
    }
}

impl ChainCashStore for SqliteChainCashStore {
    fn notes(&self) -> Box<dyn NoteService> {
        Box::new(SqliteNoteService::new(self.pool.clone()))
    }

    fn reserves(&self) -> Box<dyn ReserveService> {
        Box::new(SqliteReserveService::new(self.pool.clone()))
    }
}

#[derive(Clone)]
pub struct SqliteNoteService {
    pool: ConnectionPool,
}

impl SqliteNoteService {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

impl NoteService for SqliteNoteService {
    fn create(&self) -> Note {
        todo!()
    }
}

#[derive(Clone)]
pub struct SqliteReserveService {
    pool: ConnectionPool,
}

impl SqliteReserveService {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

impl ReserveService for SqliteReserveService {
    fn create(&self) -> Reserve {
        todo!()
    }
}
