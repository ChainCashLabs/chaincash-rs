//! ChainCash server Error module.
use thiserror::Error;

/// Represents errors that can occur in the ChainCash server.
#[derive(Error, Debug)]
pub enum Error {
    /// An [`std::io::Error`] occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A [`hyper::Error`] occurred, possibly during server creation.
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    /// A [`kv::Error`] occurred.
    #[error("kv store error: {0}")]
    Kv(#[from] crate::kv::Error),
}
