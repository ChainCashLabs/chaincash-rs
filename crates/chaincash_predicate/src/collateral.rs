use serde::{Deserialize, Serialize};

use crate::{Accept, NoteContext};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Collateral {
    pub(crate) percent: u16,
}

impl Accept for Collateral {
    fn accept(&self, context: &NoteContext) -> bool {
        let ratio = (context.liabilities as f64 / context.value as f64) * 100.0;

        ratio >= self.percent as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_true_if_collaterized() {
        let context = NoteContext {
            owner: "PK1".to_string(),
            value: 50,
            liabilities: 50,
        };
        let p = Collateral { percent: 100 };

        assert!(p.accept(&context))
    }

    #[test]
    fn test_returns_true_if_over_collaterized() {
        let context = NoteContext {
            owner: "PK1".to_string(),
            value: 50,
            liabilities: 60,
        };
        let p = Collateral { percent: 100 };

        assert!(p.accept(&context))
    }

    #[test]
    fn test_returns_true_if_not_collaterized() {
        let context = NoteContext {
            owner: "PK1".to_string(),
            value: 50,
            liabilities: 48,
        };
        let p = Collateral { percent: 100 };

        assert!(!p.accept(&context))
    }
}
