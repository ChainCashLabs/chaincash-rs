//! Key Value Store used as the storage solution for the database.
use heed::{types::ByteSlice, EnvOpenOptions};
use thiserror::Error;

/// Represents errors that can occur during [`KvStore`] usage.
#[derive(Error, Debug)]
pub enum Error {
    #[error("heed error: {0}")]
    Heed(#[from] heed::Error),
}

/// Key Value Store trait.
///
/// Implementations of [`KvStore] must be `Send` and `Sync` as the trait is
/// used as part of the server app state which requires these types.
pub trait KvStore: Send + Sync {
    /// Insert a value into the store using the given key.
    /// If the key already exists, the value will be overwritten.
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error>;

    /// Retrieve a value from the store.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error>;
}

pub struct HeedKvStore {
    env: heed::Env,
    db: heed::Database<ByteSlice, ByteSlice>,
}

impl HeedKvStore {
    /// Create a new [`HeedKvStore`] using the given path.
    pub fn new(path: &std::path::Path) -> Result<Self, Error> {
        let env = EnvOpenOptions::new().open(path)?;
        let db = env.create_database(None)?;

        Ok(Self { env, db })
    }
}

impl KvStore for HeedKvStore {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        let mut wtxn = self.env.write_txn()?;

        self.db.put(&mut wtxn, key, value)?;
        wtxn.commit()?;

        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let rtxn = self.env.read_txn()?;
        let value = self.db.get(&rtxn, key)?.map(|v| v.to_vec());

        Ok(value)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct InMemoryKvStore {
    db: std::collections::HashMap<Vec<u8>, Vec<u8>>,
}

#[cfg(test)]
impl KvStore for InMemoryKvStore {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.db.insert(key.to_vec(), value.to_vec());

        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let value = self.db.get(key).map(|v| v.to_vec());

        Ok(value)
    }
}
