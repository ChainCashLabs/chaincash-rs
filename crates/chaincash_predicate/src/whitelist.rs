use crate::{Accept, NoteContext};

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Whitelist {
    pub(crate) agents: Vec<String>,
}

impl Accept for Whitelist {
    fn accept(&self, context: &NoteContext) -> bool {
        self.agents.contains(&context.owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_true_if_owner_whitelisted() {
        let context = NoteContext {
            owner: "PK1".to_string(),
            value: 1,
            liabilities: 1,
        };
        let p = Whitelist {
            agents: vec!["PK0".to_string(), "PK1".to_string()],
        };

        assert!(p.accept(&context))
    }

    #[test]
    fn test_returns_false_if_owner_not_whitelisted() {
        let context = NoteContext {
            owner: "PK3".to_string(),
            value: 1,
            liabilities: 1,
        };
        let p = Whitelist {
            agents: vec!["PK1".to_string()],
        };

        assert!(!p.accept(&context))
    }
}
