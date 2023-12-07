// note value = the amount of tokens * gold oracle price, i.e the note value is the ERG amount
// reserves are tied to an agent

// so now we only have a note trait & a agent is just a pk
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
