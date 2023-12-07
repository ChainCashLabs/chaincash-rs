use crate::context::{ContextProvider, PredicateContext};
use crate::predicates::Accept;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CollateralAlgorithm {
    Initial,
}

impl Default for CollateralAlgorithm {
    fn default() -> Self {
        Self::Initial
    }
}

impl CollateralAlgorithm {
    fn initial<P: ContextProvider>(&self, percent: u16, context: &PredicateContext<P>) -> bool {
        todo!()
    }

    pub fn eval<P: ContextProvider>(&self, percent: u16, context: &PredicateContext<P>) -> bool {
        match self {
            CollateralAlgorithm::Initial => self.initial(percent, context),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Collateral {
    #[serde(default = "CollateralAlgorithm::default")]
    algorithm: CollateralAlgorithm,
    pub(crate) percent: u16,
}

impl Accept for Collateral {
    fn accept<P: ContextProvider>(&self, context: &PredicateContext<P>) -> bool {
        self.algorithm.eval(self.percent, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{test_util::*, Note};

    // * first, take value of notes issued by issuer of a note of interest is divided by its reserves, if collateralization is enough (e.g. 100%), finish
    #[test]
    fn test_initial_returns_true_if_collaterized_by_issuer() {
        let issuer_pk = "issuer1".to_owned();
        let note_of_interest = Note {
            nanoerg: 1000,
            issuer: issuer_pk.clone(),
            owner: "owner1".to_owned(),
            signers: vec![issuer_pk.clone()],
        };
        let issuer = TestAgent {
            pk: "issuer1".to_owned(),
            issued_notes: vec![note_of_interest],
            reserves: 900,
        };
        let provider = TestContextProvider {
            agents: vec![issuer],
        };
        let context = PredicateContext {
            note: note_of_interest,
            provider,
        };
        let p = Collateral {
            percent: 85,
            algorithm: CollateralAlgorithm::Initial,
        };

        assert!(p.accept(&context))
    }

    // ^^^ but returns false if under-collaterized

    // * if not, take max of notes issued (not passed through) by second signer divided by its reserves, third etc, stop when signer with enough collateralization found

    // * if not, take max of notes issued (not passed through) by second signer divided by its reserves, third etc, stop when signer with enough collateralization found
    // somehow ensure it starts from the 2nd signer, i.e first non-issuer signer

    // * if not found in the whole signatures-chain, acceptance predicate returns false
}
