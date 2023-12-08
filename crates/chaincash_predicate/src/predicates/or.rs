use crate::context::{ContextProvider, PredicateContext};
use crate::predicates::{Accept, Predicate};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Or {
    conditions: Vec<Predicate>,
}

impl Accept for Or {
    fn accept<P: ContextProvider>(&self, context: &PredicateContext<P>) -> bool {
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
    // use crate::predicates::{collateral::Collateral, whitelist::Whitelist};
    //
    // use super::*;
    //
    // #[test]
    // fn test_returns_true_if_any_condition_returns_true() {
    //     let context = PredicateContext {
    //         note: todo!(),
    //         provider: todo!(),
    //     };
    //     let p1 = Whitelist {
    //         agents: vec!["PK2".to_string()],
    //     };
    //     let p2 = Collateral { percent: 100 };
    //     let p = Or {
    //         conditions: vec![Predicate::Whitelist(p1), Predicate::Collateral(p2)],
    //     };
    //
    //     assert!(p.accept(&context))
    // }
    //
    // #[test]
    // fn test_returns_false_if_all_conditions_return_false() {
    //     let context = PredicateContext {
    //         note: todo!(),
    //         provider: todo!(),
    //     };
    //     let p1 = Whitelist {
    //         agents: vec!["PK2".to_string()],
    //     };
    //     let p2 = Collateral { percent: 200 };
    //     let p = Or {
    //         conditions: vec![Predicate::Whitelist(p1), Predicate::Collateral(p2)],
    //     };
    //
    //     assert!(!p.accept(&context))
    // }
}
