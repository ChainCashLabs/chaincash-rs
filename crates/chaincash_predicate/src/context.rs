pub struct Note {
    pub nanoerg: u64,
    pub owner: String,
    pub issuer: String,
    pub signers: Vec<String>,
}

pub trait ContextProvider {
    fn agent_issued_notes(&self, agent: &str) -> Vec<Note>;

    fn agent_reserves(&self, agent: &str) -> u64;
}

pub struct PredicateContext<P: ContextProvider> {
    pub note: Note,
    pub provider: P,
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    pub struct TestAgent {
        pub pk: String,
        pub issued_notes: Vec<Note>,
        pub reserves: u64,
    }

    pub struct TestContextProvider {
        pub agents: Vec<TestAgent>,
    }

    impl ContextProvider for TestContextProvider {
        fn agent_issued_notes(&self, agent: &str) -> Vec<Note> {
            self.agents
                .iter()
                .find(|n| n.pk == agent)
                .map(|a| a.issued_notes)
                .unwrap_or_default()
        }

        fn agent_reserves(&self, agent: &str) -> u64 {
            self.agents
                .iter()
                .find(|n| n.pk == agent)
                .map(|a| a.reserves)
                .unwrap_or_default()
        }
    }
}
