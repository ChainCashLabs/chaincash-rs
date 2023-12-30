use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),

    #[error("pool error: {0}")]
    Pool(#[from] diesel::r2d2::PoolError),

    #[error("Failed to update store due to: {0}")]
    Update(String),

    #[error("Failed to extract spec from box")]
    BoxSpec(#[from] chaincash_specs::boxes::Error),
}
