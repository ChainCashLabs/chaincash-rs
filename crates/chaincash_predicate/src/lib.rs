pub mod context;
pub mod predicates;

use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Predicate deserialization failed")]
    Deserialization(#[from] toml::de::Error),

    #[error("Failed to load predicate from file '{path}'")]
    LoadFromFile {
        source: std::io::Error,
        path: String,
    },
}

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    /// Path to enabled predicate configuration files
    pub predicates: Vec<PathBuf>,
}
