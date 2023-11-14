use serde::{Deserialize, Serialize};

use crate::{Accept, NoteContext, Predicate};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Or {
    conditions: Vec<Predicate>,
}

impl Accept for Or {
    fn accept(&self, context: &NoteContext) -> bool {
        for condition in &self.conditions {
            if condition.accept(context) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{collateral::Collateral, whitelist::Whitelist};

    use super::*;

    #[test]
    fn test_returns_true_if_any_condition_returns_true() {
        let context = NoteContext {
            owner: "PK1".to_string(),
            value: 1,
            liabilities: 1,
        };
        let p1 = Whitelist {
            agents: vec!["PK2".to_string()],
        };
        let p2 = Collateral { percent: 100 };
        let p = Or {
            conditions: vec![Predicate::Whitelist(p1), Predicate::Collateral(p2)],
        };

        assert!(p.accept(&context))
    }

    #[test]
    fn test_returns_false_if_all_conditions_return_false() {
        let context = NoteContext {
            owner: "PK1".to_string(),
            value: 1,
            liabilities: 1,
        };
        let p1 = Whitelist {
            agents: vec!["PK2".to_string()],
        };
        let p2 = Collateral { percent: 200 };
        let p = Or {
            conditions: vec![Predicate::Whitelist(p1), Predicate::Collateral(p2)],
        };

        assert!(!p.accept(&context))
    }
}
