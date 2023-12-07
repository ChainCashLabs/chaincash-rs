use crate::context::{ContextProvider, PredicateContext};
use crate::predicates::Accept;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Whitelist {
    pub(crate) agents: Vec<String>,
}

impl Accept for Whitelist {
    fn accept<P: ContextProvider>(&self, context: &PredicateContext<P>) -> bool {
        self.agents.contains(&context.owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_true_if_owner_whitelisted() {
        let context = PredicateContext {
            note: todo!(),
            provider: todo!(),
        };
        let p = Whitelist {
            agents: vec!["PK0".to_string(), "PK1".to_string()],
        };

        assert!(p.accept(&context))
    }

    #[test]
    fn test_returns_false_if_owner_not_whitelisted() {
        let context = PredicateContext {
            note: todo!(),
            provider: todo!(),
        };
        let p = Whitelist {
            agents: vec!["PK1".to_string()],
        };

        assert!(!p.accept(&context))
    }
}
