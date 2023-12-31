use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

#[rustfmt::skip]
pub mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
pub type ConnectionPool = Pool<ConnectionManager<SqliteConnection>>;

pub fn connect<S: Into<String>>(database_url: S) -> Result<ConnectionPool, crate::Error> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);

    Ok(Pool::builder().build(manager)?)
}

pub fn has_pending_migrations(connection: &mut SqliteConnection) -> Result<bool, crate::Error> {
    connection
        .has_pending_migration(MIGRATIONS)
        .map_err(|_| crate::Error::Migration("failed to check pending migrations".to_string()))
}

pub fn run_pending_migrations(connection: &mut SqliteConnection) -> Result<(), crate::Error> {
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|_| crate::Error::Migration("failed to run pending migrations".to_string()))?;

    Ok(())
}
