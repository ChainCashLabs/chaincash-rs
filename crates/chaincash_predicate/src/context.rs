pub struct Note {
    pub value: u64,
    pub owner: String,
    pub issuer: String,
    pub signers: Vec<String>,
}

pub trait ContextProvider {
    fn agent_notes(&self, agent: &str) -> Vec<Note>;

    fn agent_reserves(&self, agent: &str) -> u64;
}

pub struct PredicateContext<P: ContextProvider> {
    pub note: Note,
    pub provider: P,
}
