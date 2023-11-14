pub mod collateral;
pub mod or;
pub mod whitelist;

use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Predicate deserialization failed due to: {0}")]
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

pub struct NoteContext {
    owner: String,
    value: u64,
    liabilities: u64,
}

pub trait Accept {
    fn accept(&self, context: &NoteContext) -> bool;
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Predicate {
    Or(or::Or),
    Whitelist(whitelist::Whitelist),
    Collateral(collateral::Collateral),
}

impl Predicate {
    pub fn from_file(path: &PathBuf) -> Result<Self, Error> {
        let s = std::fs::read_to_string(path).map_err(|e| Error::LoadFromFile {
            source: e,
            path: path.display().to_string(),
        })?;

        Ok(toml::from_str(&s)?)
    }
}

impl Accept for Predicate {
    fn accept(&self, context: &NoteContext) -> bool {
        match self {
            Predicate::Or(p) => p.accept(context),
            Predicate::Whitelist(p) => p.accept(context),
            Predicate::Collateral(p) => p.accept(context),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate_deser() {
        let s = r#"
            type = "or"
            conditions = [
                {type = "whitelist", agents = ["PK1", "PK2"]},
                {type = "collateral", percent = 110}
            ]
            "#;
        let p = toml::from_str::<Predicate>(s).unwrap();
        let mut context = NoteContext {
            owner: "PK0".to_string(),
            value: 1,
            liabilities: 1,
        };

        assert!(!p.accept(&context));

        context.owner = "PK1".to_string();
        assert!(p.accept(&context))
    }
}
