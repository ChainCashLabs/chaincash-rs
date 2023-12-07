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

// needed:
// - get issuer of note
// - get all issued notes by agent
// - get all signers of note (excluding first which is issuer, ), this is basically just holders
// except the current holder, the note is only signed when sending

pub struct Note {
    /// Ergo box id of the note
    pub id: String,
    /// Current owner of the note, public key in hex format
    pub owner: String,
    pub value: u64,
    // nominal?
}
