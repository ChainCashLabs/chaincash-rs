use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),

    #[error("pool error: {0}")]
    Pool(#[from] diesel::r2d2::PoolError),

    #[error("migration error: {0}")]
    Migration(String),
}
