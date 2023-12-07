use crate::context::{ContextProvider, PredicateContext};
use crate::predicates::Accept;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Collateral {
    pub(crate) percent: u16,
}

impl Accept for Collateral {
    fn accept<P: ContextProvider>(&self, context: &PredicateContext<P>) -> bool {
        todo!()
        // let ratio = (context.liabilities as f64 / context.value as f64) * 100.0;
        //
        // ratio >= self.percent as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_true_if_collaterized() {
        let context = PredicateContext {
            note: todo!(),
            provider: todo!(),
        };
        let p = Collateral { percent: 100 };

        assert!(p.accept(&context))
    }

    #[test]
    fn test_returns_true_if_over_collaterized() {
        let context = PredicateContext {
            note: todo!(),
            provider: todo!(),
        };
        let p = Collateral { percent: 100 };

        assert!(p.accept(&context))
    }

    #[test]
    fn test_returns_true_if_not_collaterized() {
        let context = PredicateContext {
            note: todo!(),
            provider: todo!(),
        };
        let p = Collateral { percent: 100 };

        assert!(!p.accept(&context))
    }
}
